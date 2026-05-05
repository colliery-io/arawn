use std::path::PathBuf;

use arawn_auth::{OAuthProviderConfig, TokenStore};
use async_trait::async_trait;
use url::Url;

use crate::error::IntegrationError;
use crate::integration::{ConnectContext, Integration};
use crate::oauth_flow::run_oauth_flow;

use super::client::{CalendarHub, client_from_token_store};

/// Stable service name. Used as the [`TokenStore`] key, the integration
/// registry key, the per-service credential subdirectory, and the
/// argument to `/connect google_calendar`.
pub const SERVICE_NAME: &str = "google_calendar";

/// The OAuth scope Google Calendar reads/writes need.
pub const CALENDAR_OAUTH_SCOPE: &str = "https://www.googleapis.com/auth/calendar.events";

/// Default Google Calendar OAuth provider config.
pub struct GoogleCalendarProviderConfig {
    pub auth_url: Url,
    pub token_url: Url,
    pub scopes: Vec<String>,
}

impl Default for GoogleCalendarProviderConfig {
    fn default() -> Self {
        Self {
            auth_url: "https://accounts.google.com/o/oauth2/v2/auth".parse().unwrap(),
            token_url: "https://oauth2.googleapis.com/token".parse().unwrap(),
            scopes: vec![CALENDAR_OAUTH_SCOPE.to_string()],
        }
    }
}

impl GoogleCalendarProviderConfig {
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

/// Google Calendar integration.
pub struct GoogleCalendarIntegration {
    data_dir: PathBuf,
    client_id: String,
    client_secret: String,
    provider_config: Option<GoogleCalendarProviderConfig>,
}

impl GoogleCalendarIntegration {
    pub fn new(data_dir: PathBuf, client_id: String, client_secret: String) -> Self {
        Self {
            data_dir,
            client_id,
            client_secret,
            provider_config: None,
        }
    }

    pub fn with_provider_config(mut self, config: GoogleCalendarProviderConfig) -> Self {
        self.provider_config = Some(config);
        self
    }

    /// Build a fully-wired `CalendarHub` for tools. Returns `NotConnected`
    /// if the user hasn't run `/connect google_calendar` yet.
    pub fn hub(&self) -> Result<CalendarHub, IntegrationError> {
        client_from_token_store(self.data_dir.clone(), self.oauth_config())
    }

    fn oauth_config(&self) -> OAuthProviderConfig {
        let provider = self
            .provider_config
            .as_ref()
            .map(|c| GoogleCalendarProviderConfig {
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
impl Integration for GoogleCalendarIntegration {
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
    fn default_provider_has_calendar_events_scope() {
        let provider = GoogleCalendarProviderConfig::default();
        assert_eq!(provider.scopes, vec![CALENDAR_OAUTH_SCOPE.to_string()]);
    }

    #[test]
    fn provider_lifts_into_oauth_config() {
        let cfg = GoogleCalendarProviderConfig::default()
            .into_oauth_provider("cid".into(), "csecret".into());
        assert_eq!(cfg.client_id, "cid");
        assert_eq!(cfg.client_secret, "csecret");
        assert_eq!(cfg.scopes, vec![CALENDAR_OAUTH_SCOPE.to_string()]);
    }
}
