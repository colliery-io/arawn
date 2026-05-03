//! OAuth dance composition.
//!
//! Each integration that uses OAuth supplies provider config (auth URL,
//! token URL, scopes, client_id, client_secret) plus a [`ConnectContext`]
//! to publish the auth URL through. The flow:
//!
//! 1. Bind a localhost callback listener.
//! 2. Build the authorization URL (PKCE, CSRF state).
//! 3. Publish the URL via the context (the TUI opens a browser).
//! 4. Wait for the callback, validate the CSRF state.
//! 5. Exchange the code for a `Token`.
//! 6. Persist via `TokenStore` and return.

use arawn_auth::{CallbackServer, OAuthClient, OAuthProviderConfig, Token, TokenStore};

use crate::error::IntegrationError;
use crate::integration::ConnectContext;

/// Result of a successful OAuth flow. Returned in case the caller wants
/// to do something with the token immediately (e.g. cache an access token
/// in memory).
#[derive(Debug, Clone)]
pub struct OAuthOutcome {
    pub token: Token,
}

/// Drive the OAuth dance end-to-end. Returns when the token is in the
/// `TokenStore` keyed by `service_name` (so the integration's
/// `is_connected` will return `true` immediately after).
pub async fn run_oauth_flow(
    provider_config: OAuthProviderConfig,
    token_store: &TokenStore,
    service_name: &str,
    callback_path: &str,
    ctx: &dyn ConnectContext,
) -> Result<OAuthOutcome, IntegrationError> {
    // 1. Bind the callback first so we know what redirect_uri to advertise.
    let callback = CallbackServer::bind(callback_path).await?;
    let redirect_uri = callback.redirect_uri().clone();

    // 2. Build the authorization URL.
    let client = OAuthClient::new(provider_config);
    let auth_request = client.start_flow(&redirect_uri);

    // 3. Publish the URL.
    ctx.publish_auth_url(&auth_request.authorization_url).await;
    ctx.publish_progress("waiting for browser authorization (5 min timeout)…").await;

    // 4. Wait for the callback.
    let result = callback.listen().await?;

    // 5. CSRF check + code exchange.
    if result.state != auth_request.csrf_state {
        return Err(IntegrationError::Provider(format!(
            "CSRF state mismatch on callback (expected {}, got {})",
            auth_request.csrf_state, result.state
        )));
    }
    ctx.publish_progress("exchanging authorization code for token…").await;
    let token = client
        .exchange_code(&result.code, &redirect_uri, &auth_request.pkce_verifier)
        .await?;

    // 6. Persist.
    token_store.save(service_name, &token)?;

    Ok(OAuthOutcome { token })
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::Mutex;
    use url::Url;

    /// Captures everything published; lets tests assert without a real TUI.
    struct CaptureCtx {
        service: String,
        auth_url: Mutex<Option<Url>>,
        progress: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl ConnectContext for CaptureCtx {
        fn service(&self) -> &str {
            &self.service
        }
        async fn publish_auth_url(&self, url: &Url) {
            *self.auth_url.lock().unwrap() = Some(url.clone());
        }
        async fn publish_progress(&self, message: &str) {
            self.progress.lock().unwrap().push(message.to_string());
        }
    }

    #[tokio::test]
    async fn ctx_capture_smoke() {
        // Doesn't run a real flow — this exists to lock in the ConnectContext
        // shape. Real OAuth integration tests need a mock OAuth provider
        // that's heavier than belongs in this crate.
        let ctx = CaptureCtx {
            service: "test".into(),
            auth_url: Mutex::new(None),
            progress: Mutex::new(Vec::new()),
        };
        let url: Url = "https://example.com/auth?foo=bar".parse().unwrap();
        ctx.publish_auth_url(&url).await;
        ctx.publish_progress("step 1").await;
        ctx.publish_progress("step 2").await;

        assert_eq!(ctx.service(), "test");
        assert_eq!(ctx.auth_url.lock().unwrap().as_ref().unwrap(), &url);
        assert_eq!(
            *ctx.progress.lock().unwrap(),
            vec!["step 1".to_string(), "step 2".to_string()],
        );
    }
}
