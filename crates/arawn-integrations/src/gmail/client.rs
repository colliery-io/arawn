//! Auth + Hub plumbing for the `google-gmail1` client.
//!
//! Two pieces:
//!
//! 1. [`ArawnGetToken`] — implements `google_apis_common::GetToken` against
//!    our `arawn_auth::Token`. Refreshes via the same `OAuthClient::refresh`
//!    used elsewhere in arawn, so token refresh logic stays in one place.
//!    No yup-oauth2 dependency.
//!
//! 2. [`build_gmail_hub`] — wires a `hyper-util::Client` with hyper-rustls
//!    against the `Gmail` Hub. One-liner that hides the connector setup.

use std::sync::Arc;

use arawn_auth::{OAuthClient, OAuthProviderConfig, Token, TokenStore};
use google_gmail1::{
    Gmail,
    common::{Client, GetToken},
    hyper_util::{client::legacy::Client as HyperClient, rt::TokioExecutor},
};
use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::client::legacy::connect::HttpConnector;
use tokio::sync::Mutex as AsyncMutex;
use tracing::{debug, warn};

use crate::error::IntegrationError;

use super::integration::{SERVICE_NAME, TokenStoreHandle};

/// HTTPS connector type produced by [`build_gmail_hub`]. Exported so callers
/// who hold the Hub by name don't have to reach into hyper-rustls' types.
pub type HttpsConnector = hyper_rustls::HttpsConnector<HttpConnector>;

/// Concrete Gmail Hub the integration exposes. Tools call methods on this.
pub type GmailHub = Gmail<HttpsConnector>;

/// Build the hyper-util client + HTTPS connector and return a configured
/// [`Gmail`] hub. The auth handle is an [`ArawnGetToken`] that lazily
/// refreshes through `arawn-auth` when an access token is expired.
pub fn build_gmail_hub(
    token_store: TokenStoreHandle,
    initial_token: Token,
    oauth_config: OAuthProviderConfig,
) -> GmailHub {
    let connector = HttpsConnectorBuilder::new()
        .with_native_roots()
        .expect("rustls native roots available")
        .https_or_http()
        .enable_http1()
        .enable_http2()
        .build();

    let hyper_client: HyperClient<HttpsConnector, _> =
        HyperClient::builder(TokioExecutor::new()).build(connector);

    let common_client: Client<HttpsConnector> = hyper_client;

    let auth = ArawnGetToken::new(initial_token, oauth_config, token_store);
    Gmail::new(common_client, auth)
}

/// `GetToken` impl backed by `arawn-auth`. Holds a cached `Token` behind an
/// async mutex; on every request, checks expiry and refreshes synchronously
/// (within the request) when needed. Refreshed tokens are persisted via
/// the supplied [`TokenStoreHandle`].
#[derive(Clone)]
pub struct ArawnGetToken {
    inner: Arc<ArawnGetTokenInner>,
}

struct ArawnGetTokenInner {
    /// Cached current token. Refreshed in-place when expired.
    token: AsyncMutex<Token>,
    /// Used for refresh calls; holds client_id / client_secret / token_url.
    oauth: OAuthClient,
    /// Persists refreshed tokens so process restart picks them up.
    token_store: TokenStoreHandle,
}

impl ArawnGetToken {
    pub fn new(token: Token, oauth_config: OAuthProviderConfig, token_store: TokenStoreHandle) -> Self {
        Self {
            inner: Arc::new(ArawnGetTokenInner {
                token: AsyncMutex::new(token),
                oauth: OAuthClient::new(oauth_config),
                token_store,
            }),
        }
    }
}

impl GetToken for ArawnGetToken {
    fn get_token<'a>(
        &'a self,
        _scopes: &'a [&str],
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<Option<String>, Box<dyn std::error::Error + Send + Sync>>,
                > + Send
                + 'a,
        >,
    > {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut guard = inner.token.lock().await;
            if guard.is_expired() {
                let refresh = guard.refresh.clone().ok_or_else(|| {
                    Box::<dyn std::error::Error + Send + Sync>::from(
                        "Gmail token has no refresh token; reconnect via /connect gmail",
                    )
                })?;
                debug!("refreshing Gmail access token");
                let new_token = inner
                    .oauth
                    .refresh(&refresh)
                    .await
                    .map_err(|e| Box::<dyn std::error::Error + Send + Sync>::from(e.to_string()))?;
                if let Err(e) = inner.token_store.save_token(&new_token) {
                    warn!(error = %e, "failed to persist refreshed Gmail token");
                }
                *guard = new_token;
            }
            Ok(Some(guard.access.clone()))
        })
    }
}

/// Convenience wrapper for the integration's `client()` factory: opens a
/// fresh [`TokenStore`], reads the persisted Gmail token, and returns a
/// fully-wired Hub. Used by tools that need to make API calls.
pub fn client_from_token_store(
    data_dir: std::path::PathBuf,
    oauth_config: OAuthProviderConfig,
) -> Result<GmailHub, IntegrationError> {
    let store = TokenStore::open(&data_dir)?;
    let token = store
        .load(SERVICE_NAME)?
        .ok_or_else(|| IntegrationError::NotConnected(SERVICE_NAME.to_string()))?;
    Ok(build_gmail_hub(
        TokenStoreHandle { data_dir },
        token,
        oauth_config,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke-only — confirms ArawnGetToken can be constructed and dispatches
    /// `get_token()` without compile errors. End-to-end auth is exercised by
    /// the manual smoke test documented in docs/src/integrations/gmail.md.
    #[tokio::test]
    async fn arawn_get_token_returns_unexpired_access_directly() {
        let token = Token {
            access: "live-access".into(),
            refresh: Some("live-refresh".into()),
            // future expiry — not expired
            expires_at: Some(chrono::Utc::now() + chrono::Duration::hours(1)),
            scope: None,
            token_type: "Bearer".into(),
        };
        let oauth = OAuthProviderConfig {
            auth_url: "https://example.com/auth".parse().unwrap(),
            token_url: "https://example.com/token".parse().unwrap(),
            client_id: "id".into(),
            client_secret: "secret".into(),
            scopes: vec![],
        };
        let store = TokenStoreHandle {
            data_dir: tempfile::tempdir().unwrap().path().to_path_buf(),
        };
        let getter = ArawnGetToken::new(token, oauth, store);
        let result = getter.get_token(&[]).await.unwrap();
        assert_eq!(result.as_deref(), Some("live-access"));
    }
}
