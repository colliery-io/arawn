//! Shared plumbing for Google API integrations (Gmail, Calendar, ...).
//!
//! Each Google API client crate (`google-gmail1`, `google-calendar3`, etc.)
//! produces its own typed `Hub<C>` but shares:
//!
//! - The same `google_apis_common::GetToken` trait for auth.
//! - The same `google_apis_common::Client<C>` type alias (a hyper-util client).
//! - The same `hyper-rustls` connector requirements.
//! - The same OAuth refresh story (we sit on top of `arawn_auth::OAuthClient`).
//!
//! This module hosts the pieces that aren't service-specific so each
//! integration just builds its Hub via `Hub::new(client, auth)` with values
//! we hand it.

use std::path::PathBuf;
use std::sync::Arc;

use arawn_auth::{OAuthClient, OAuthProviderConfig, Token, TokenStore};
use google_apis_common::{Client, GetToken};
use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::client::legacy::{Client as HyperClient, connect::HttpConnector};
use hyper_util::rt::TokioExecutor;
use tokio::sync::Mutex as AsyncMutex;
use tracing::{debug, warn};

use crate::error::IntegrationError;

/// HTTPS connector flavour we wire all Google integrations against.
pub type HttpsConnector = hyper_rustls::HttpsConnector<HttpConnector>;

/// Build the shared hyper-util client every Google integration uses.
/// Concretely: HTTPS-or-HTTP, native roots, HTTP/1.1 + HTTP/2.
pub fn build_https_client() -> Client<HttpsConnector> {
    let connector = HttpsConnectorBuilder::new()
        .with_native_roots()
        .expect("rustls native roots available")
        .https_or_http()
        .enable_http1()
        .enable_http2()
        .build();
    let client: HyperClient<HttpsConnector, _> =
        HyperClient::builder(TokioExecutor::new()).build(connector);
    client
}

/// Per-service `arawn-auth::TokenStore` handle. Each Google integration
/// holds one of these so its [`ArawnGetToken`] can persist refreshed
/// tokens back to disk under the right service key.
#[derive(Clone)]
pub struct TokenStoreHandle {
    pub data_dir: PathBuf,
    pub service_name: String,
}

impl TokenStoreHandle {
    pub fn new(data_dir: PathBuf, service_name: impl Into<String>) -> Self {
        Self {
            data_dir,
            service_name: service_name.into(),
        }
    }

    pub fn save_token(&self, token: &Token) -> Result<(), IntegrationError> {
        let store = TokenStore::open(&self.data_dir)?;
        store.save(&self.service_name, token)?;
        Ok(())
    }

    pub fn load_token(&self) -> Result<Option<Token>, IntegrationError> {
        let store = TokenStore::open(&self.data_dir)?;
        Ok(store.load(&self.service_name)?)
    }
}

/// `google_apis_common::GetToken` impl backed by `arawn-auth`. Holds a
/// cached `Token` in an async mutex; every API call inspects expiry and
/// refreshes through `OAuthClient::refresh` when needed. Refreshed tokens
/// are persisted via the supplied [`TokenStoreHandle`] so a process
/// restart picks them up.
#[derive(Clone)]
pub struct ArawnGetToken {
    inner: Arc<ArawnGetTokenInner>,
}

struct ArawnGetTokenInner {
    token: AsyncMutex<Token>,
    oauth: OAuthClient,
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
                    Box::<dyn std::error::Error + Send + Sync>::from(format!(
                        "{} token has no refresh token; reconnect via /connect {}",
                        inner.token_store.service_name, inner.token_store.service_name
                    ))
                })?;
                debug!(
                    service = %inner.token_store.service_name,
                    "refreshing access token"
                );
                let new_token = inner
                    .oauth
                    .refresh(&refresh)
                    .await
                    .map_err(|e| Box::<dyn std::error::Error + Send + Sync>::from(e.to_string()))?;
                if let Err(e) = inner.token_store.save_token(&new_token) {
                    warn!(
                        service = %inner.token_store.service_name,
                        error = %e,
                        "failed to persist refreshed token"
                    );
                }
                *guard = new_token;
            }
            Ok(Some(guard.access.clone()))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unexpired_token_returned_directly_no_refresh() {
        let token = Token {
            access: "live-access".into(),
            refresh: Some("live-refresh".into()),
            expires_at: Some(chrono::Utc::now() + chrono::Duration::hours(1)),
            scope: None,
            token_type: "Bearer".into(),
            extras: serde_json::Map::new(),
        };
        let oauth = OAuthProviderConfig {
            auth_url: "https://example.com/auth".parse().unwrap(),
            token_url: "https://example.com/token".parse().unwrap(),
            client_id: "id".into(),
            client_secret: "secret".into(),
            scopes: vec![],
            extra_auth_params: Vec::new(),
        };
        let store = TokenStoreHandle::new(
            tempfile::tempdir().unwrap().path().to_path_buf(),
            "test_service",
        );
        let getter = ArawnGetToken::new(token, oauth, store);
        let result = getter.get_token(&[]).await.unwrap();
        assert_eq!(result.as_deref(), Some("live-access"));
    }
}
