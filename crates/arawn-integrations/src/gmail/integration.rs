use std::path::PathBuf;

use arawn_auth::{OAuthProviderConfig, TokenStore};
use async_trait::async_trait;
use url::Url;

use crate::error::IntegrationError;
use crate::integration::{ConnectContext, Integration};
use crate::oauth_flow::run_oauth_flow;

use super::client::{client_from_token_store, GmailHub};

/// Stable service name. Used as the [`TokenStore`] key, the integration
/// registry key, the per-service credential subdirectory, and the
/// argument to `/connect gmail`.
pub const SERVICE_NAME: &str = "gmail";

/// Standard Gmail OAuth provider configuration. Encapsulated as a builder
/// so tests can swap in a fake authorization endpoint.
pub struct GmailProviderConfig {
    pub auth_url: Url,
    pub token_url: Url,
    pub scopes: Vec<String>,
}

impl Default for GmailProviderConfig {
    fn default() -> Self {
        Self {
            auth_url: "https://accounts.google.com/o/oauth2/v2/auth".parse().unwrap(),
            token_url: "https://oauth2.googleapis.com/token".parse().unwrap(),
            scopes: vec![
                "https://www.googleapis.com/auth/gmail.readonly".to_string(),
                "https://www.googleapis.com/auth/gmail.send".to_string(),
                "https://www.googleapis.com/auth/gmail.modify".to_string(),
            ],
        }
    }
}

impl GmailProviderConfig {
    /// Build the underlying [`OAuthProviderConfig`] given a client_id /
    /// client_secret pair (typically read from `ARAWN_GMAIL_CLIENT_ID` /
    /// `ARAWN_GMAIL_CLIENT_SECRET` env vars at server startup).
    pub fn into_oauth_provider(self, client_id: String, client_secret: String) -> OAuthProviderConfig {
        OAuthProviderConfig {
            auth_url: self.auth_url,
            token_url: self.token_url,
            client_id,
            client_secret,
            scopes: self.scopes,
        }
    }
}

/// Gmail integration. Built once at server startup; tools depend on it
/// via `Arc<GmailIntegration>` (or via the dynamic registry).
pub struct GmailIntegration {
    /// Data directory; used to open a fresh `TokenStore` per call (cheap).
    data_dir: PathBuf,
    /// Gmail OAuth client_id (typically loaded from env at startup).
    client_id: String,
    /// Gmail OAuth client_secret (typically loaded from env at startup).
    client_secret: String,
    /// Allows the OAuth flow to use a different provider (e.g. fake auth
    /// endpoint) for tests.
    provider_config: Option<GmailProviderConfig>,
}

impl GmailIntegration {
    /// Standard constructor.
    pub fn new(data_dir: PathBuf, client_id: String, client_secret: String) -> Self {
        Self {
            data_dir,
            client_id,
            client_secret,
            provider_config: None,
        }
    }

    /// Override the OAuth provider config — used by tests.
    pub fn with_provider_config(mut self, config: GmailProviderConfig) -> Self {
        self.provider_config = Some(config);
        self
    }

    /// Build a fully-wired `Gmail` Hub for tools. Returns `NotConnected` if
    /// the user hasn't run `/connect gmail` yet.
    pub fn hub(&self) -> Result<GmailHub, IntegrationError> {
        let provider = self.oauth_config();
        client_from_token_store(self.data_dir.clone(), provider)
    }

    fn oauth_config(&self) -> OAuthProviderConfig {
        let provider = self
            .provider_config
            .as_ref()
            .map(|c| GmailProviderConfig {
                auth_url: c.auth_url.clone(),
                token_url: c.token_url.clone(),
                scopes: c.scopes.clone(),
            })
            .unwrap_or_default();
        provider.into_oauth_provider(self.client_id.clone(), self.client_secret.clone())
    }

    fn token_store(&self) -> Result<TokenStore, IntegrationError> {
        Ok(TokenStore::open(&self.data_dir)?)
    }
}

#[async_trait]
impl Integration for GmailIntegration {
    fn name(&self) -> &str {
        SERVICE_NAME
    }

    async fn is_connected(&self) -> bool {
        // Cheap disk-only check per the trait contract — no API round-trip.
        // A "true" here doesn't guarantee the token still works on the wire;
        // tools that fail mid-call surface that via the engine error chain.
        match self.token_store() {
            Ok(store) => store.load(SERVICE_NAME).ok().flatten().is_some(),
            Err(_) => false,
        }
    }

    async fn connect(&self, ctx: &dyn ConnectContext) -> Result<(), IntegrationError> {
        let store = self.token_store()?;
        let oauth_config = self.oauth_config();
        run_oauth_flow(oauth_config, &store, SERVICE_NAME, "/oauth/callback", None, ctx).await?;
        Ok(())
    }

    async fn disconnect(&self) -> Result<(), IntegrationError> {
        let store = self.token_store()?;
        store.delete(SERVICE_NAME)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_provider_has_three_gmail_scopes() {
        let provider = GmailProviderConfig::default();
        assert_eq!(provider.scopes.len(), 3);
        assert!(provider.scopes.iter().any(|s| s.ends_with("/gmail.readonly")));
        assert!(provider.scopes.iter().any(|s| s.ends_with("/gmail.send")));
        assert!(provider.scopes.iter().any(|s| s.ends_with("/gmail.modify")));
    }

    #[test]
    fn provider_lifts_into_oauth_config() {
        let cfg = GmailProviderConfig::default()
            .into_oauth_provider("cid".into(), "csecret".into());
        assert_eq!(cfg.client_id, "cid");
        assert_eq!(cfg.client_secret, "csecret");
        assert_eq!(cfg.scopes.len(), 3);
        assert_eq!(cfg.token_url.as_str(), "https://oauth2.googleapis.com/token");
    }
}
