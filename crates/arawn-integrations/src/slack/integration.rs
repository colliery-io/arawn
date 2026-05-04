use std::path::PathBuf;

use arawn_auth::{OAuthProviderConfig, TokenStore};
use async_trait::async_trait;
use url::Url;

use crate::error::IntegrationError;
use crate::integration::{ConnectContext, Integration};
use crate::oauth_flow::run_oauth_flow;

use super::client::{SlackContext, build_slack_client};

/// Stable service name. Used as the [`TokenStore`] key, the integration
/// registry key, and the argument to `/connect slack`.
pub const SERVICE_NAME: &str = "slack";

/// Bot scopes requested at OAuth time. Per ADR-0001 § 4.
pub const SLACK_OAUTH_SCOPES: &[&str] = &[
    "channels:read",
    "channels:history",
    "groups:read",
    "groups:history",
    "im:read",
    "im:history",
    "mpim:history",
    "chat:write",
    "reactions:write",
    "search:read",
    "users:read",
];

/// Slack OAuth v2 provider config. Default values match Slack's standard
/// endpoints; tests can override.
pub struct SlackProviderConfig {
    pub auth_url: Url,
    pub token_url: Url,
    pub scopes: Vec<String>,
}

impl Default for SlackProviderConfig {
    fn default() -> Self {
        Self {
            auth_url: "https://slack.com/oauth/v2/authorize".parse().unwrap(),
            token_url: "https://slack.com/api/oauth.v2.access".parse().unwrap(),
            scopes: SLACK_OAUTH_SCOPES.iter().map(|s| s.to_string()).collect(),
        }
    }
}

impl SlackProviderConfig {
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

/// Slack integration. Tools depend on it via `Arc<SlackIntegration>`.
pub struct SlackIntegration {
    data_dir: PathBuf,
    client_id: String,
    client_secret: String,
    provider_config: Option<SlackProviderConfig>,
}

impl SlackIntegration {
    pub fn new(data_dir: PathBuf, client_id: String, client_secret: String) -> Self {
        Self {
            data_dir,
            client_id,
            client_secret,
            provider_config: None,
        }
    }

    pub fn with_provider_config(mut self, config: SlackProviderConfig) -> Self {
        self.provider_config = Some(config);
        self
    }

    /// Build a fresh `SlackContext` for tool calls. Cheap (Arc-clones the
    /// shared client + reads the persisted token). Returns `NotConnected`
    /// if the user hasn't run `/connect slack` yet.
    pub fn context(&self) -> Result<SlackContext, IntegrationError> {
        let store = self.token_store()?;
        let token = store
            .load(SERVICE_NAME)?
            .ok_or_else(|| IntegrationError::NotConnected(SERVICE_NAME.to_string()))?;
        Ok(build_slack_client(&token))
    }

    fn oauth_config(&self) -> OAuthProviderConfig {
        let provider = self
            .provider_config
            .as_ref()
            .map(|c| SlackProviderConfig {
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
impl Integration for SlackIntegration {
    fn name(&self) -> &str {
        SERVICE_NAME
    }

    async fn is_connected(&self) -> bool {
        match self.token_store() {
            Ok(store) => store.load(SERVICE_NAME).ok().flatten().is_some(),
            Err(_) => false,
        }
    }

    async fn connect(&self, ctx: &dyn ConnectContext) -> Result<(), IntegrationError> {
        let store = self.token_store()?;
        let oauth_config = self.oauth_config();
        run_oauth_flow(oauth_config, &store, SERVICE_NAME, "/oauth/callback", ctx).await?;
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
    fn default_provider_carries_eleven_bot_scopes() {
        let provider = SlackProviderConfig::default();
        assert_eq!(provider.scopes.len(), 11);
        assert!(provider.scopes.iter().any(|s| s == "chat:write"));
        assert!(provider.scopes.iter().any(|s| s == "channels:history"));
        assert!(provider.scopes.iter().any(|s| s == "search:read"));
    }

    #[test]
    fn provider_lifts_into_oauth_config() {
        let cfg = SlackProviderConfig::default()
            .into_oauth_provider("cid".into(), "csecret".into());
        assert_eq!(cfg.client_id, "cid");
        assert_eq!(cfg.client_secret, "csecret");
        assert_eq!(cfg.auth_url.as_str(), "https://slack.com/oauth/v2/authorize");
        assert_eq!(cfg.token_url.as_str(), "https://slack.com/api/oauth.v2.access");
    }
}
