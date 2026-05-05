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
///
/// `search:read` was in the original scope list but is dropped: `slack_search`
/// is deferred (slack-morphism doesn't typed-expose `search.messages`), and
/// requesting an unused scope causes Slack to reject the OAuth flow with
/// "Invalid permissions requested" if the workspace admin hasn't pre-approved
/// it. Re-add when `slack_search` lands.
pub const SLACK_OAUTH_SCOPES: &[&str] = &[
    "channels:read",
    "channels:history",
    "groups:read",
    "groups:history",
    "im:read",
    "im:history",
    "mpim:read",
    "mpim:history",
    "chat:write",
    // Lets `slack_post` send to public channels the bot isn't a member of —
    // without it, you have to /invite the bot into every channel first.
    "chat:write.public",
    "reactions:write",
    "users:read",
    // Populates the `email` field in `slack_users_list` results.
    "users:read.email",
    // Lets `slack_history` surface file attachments instead of `[file]`
    // placeholders.
    "files:read",
    // Required by `slack_open_dm` for 1:1 DM channels — Slack's
    // conversations.open with a single user_id checks im:write, not
    // chat:write (despite both being "messaging" scopes in Slack's UI).
    "im:write",
    // Same as above but for multi-party DMs (mpim) — open conversations.open
    // with multiple user_ids.
    "mpim:write",
];

/// Split a Slack-style scope string (comma- or whitespace-delimited)
/// into a deduped set.
fn parse_scope_string(s: &str) -> std::collections::HashSet<String> {
    s.split(|c: char| c == ',' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

/// User-token scopes — the second leg of Slack's dual-token OAuth model.
///
/// The bot token (above) acts as the bot user; the user token acts as
/// the human who installed the app. Bots can only see private channels
/// they've been explicitly invited to; users see every channel they're
/// in. So for *read* operations we use the user token to get coverage
/// without requiring the bot to be invited everywhere. *Write*
/// operations stay on the bot token so messages still appear as
/// "arawn" rather than as the user themselves.
///
/// `:read` scopes enumerate channels (drive `conversations.list`);
/// `:history` scopes read messages within them. Both are needed for
/// the dual-token read path to actually surface private channels.
///
/// Routed at the tool level — see `SlackIntegration::user_context()`.
pub const SLACK_OAUTH_USER_SCOPES: &[&str] = &[
    "channels:read",
    "channels:history",
    "groups:read",
    "groups:history",
    "im:read",
    "im:history",
    "mpim:read",
    "mpim:history",
    "users:read",
    "search:read",
];

/// Slack OAuth v2 provider config. Default values match Slack's standard
/// endpoints; tests can override.
pub struct SlackProviderConfig {
    pub auth_url: Url,
    pub token_url: Url,
    pub scopes: Vec<String>,
    /// Pinned port for the local OAuth callback. Slack's redirect-URI
    /// allowlist is exact-match — no wildcard ports — so we bind a known
    /// port and the user adds `http://127.0.0.1:<port>/oauth/callback` to
    /// the Slack app config exactly once.
    pub redirect_port: u16,
}

/// Default callback port for Slack. The user adds
/// `http://127.0.0.1:8080/oauth/callback` to the Slack app's redirect
/// allowlist; the binary always binds 8080. Override via
/// [`SlackProviderConfig`] if 8080 is taken on your machine.
pub const DEFAULT_SLACK_REDIRECT_PORT: u16 = 8080;

impl Default for SlackProviderConfig {
    fn default() -> Self {
        Self {
            auth_url: "https://slack.com/oauth/v2/authorize".parse().unwrap(),
            token_url: "https://slack.com/api/oauth.v2.access".parse().unwrap(),
            scopes: SLACK_OAUTH_SCOPES.iter().map(|s| s.to_string()).collect(),
            redirect_port: DEFAULT_SLACK_REDIRECT_PORT,
        }
    }
}

impl SlackProviderConfig {
    pub fn into_oauth_provider(self, client_id: String, client_secret: String) -> OAuthProviderConfig {
        // Slack's OAuth v2 takes user-level scopes via a separate
        // `user_scope` query param (alongside the regular `scope` for
        // bot scopes). Pass our user-scope set there so the OAuth
        // response includes both a bot token and an authed_user token.
        let user_scope = SLACK_OAUTH_USER_SCOPES.join(",");
        OAuthProviderConfig {
            auth_url: self.auth_url,
            token_url: self.token_url,
            client_id,
            client_secret,
            scopes: self.scopes,
            extra_auth_params: vec![("user_scope".into(), user_scope)],
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

    /// Build a `SlackContext` backed by the **bot** token. Used by write
    /// operations (`slack_post`, `slack_react`) so messages appear as the
    /// bot identity rather than as the user.
    pub fn context(&self) -> Result<SlackContext, IntegrationError> {
        self.bot_context()
    }

    /// Same as [`Self::context`] — kept as the canonical name for the
    /// bot-token side of the dual-token model.
    pub fn bot_context(&self) -> Result<SlackContext, IntegrationError> {
        let token = self.load_token()?;
        Ok(build_slack_client(&token))
    }

    /// Build a `SlackContext` backed by the **user** token (the half of
    /// Slack's OAuth response that authorized arawn-as-this-user). Used
    /// by read operations (`slack_history`, `slack_list_channels`, etc.)
    /// so we see private channels the user is in without needing the
    /// bot to be invited.
    ///
    /// Returns `NotConnected` if the user token isn't present in the
    /// persisted token's `extras` (e.g. older OAuth flows that didn't
    /// request `user_scope`). Caller is expected to fall back to
    /// `bot_context()` for graceful degradation.
    pub fn user_context(&self) -> Result<SlackContext, IntegrationError> {
        let token = self.load_token()?;
        let user_access = token
            .extras
            .get("authed_user")
            .and_then(|v| v.get("access_token"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                IntegrationError::NotConnected(format!("{SERVICE_NAME} (user token)"))
            })?;
        // Synthesize a Token-shaped struct just for the user side so we
        // can reuse build_slack_client. We deliberately drop refresh /
        // expiry — Slack user tokens follow the same lifecycle as the
        // bot token (see ARAWN-T-0204 status updates re: rotation).
        let user_token = arawn_auth::Token {
            access: user_access.to_string(),
            refresh: None,
            expires_at: None,
            scope: token
                .extras
                .get("authed_user")
                .and_then(|v| v.get("scope"))
                .and_then(|v| v.as_str())
                .map(String::from),
            token_type: "Bearer".into(),
            extras: serde_json::Map::new(),
        };
        Ok(build_slack_client(&user_token))
    }

    fn load_token(&self) -> Result<arawn_auth::Token, IntegrationError> {
        let store = self.token_store()?;
        store
            .load(SERVICE_NAME)?
            .ok_or_else(|| IntegrationError::NotConnected(SERVICE_NAME.to_string()))
    }

    /// Bot-token scope set from the persisted token's `scope` field.
    /// Slack returns scopes comma-delimited; we accept either comma or
    /// whitespace as a separator. Returns `NotConnected` if no token
    /// is on disk.
    pub fn granted_scopes(&self) -> Result<std::collections::HashSet<String>, IntegrationError> {
        let token = self.load_token()?;
        Ok(parse_scope_string(token.scope.as_deref().unwrap_or("")))
    }

    /// User-token scope set from `extras.authed_user.scope`. Returns an
    /// empty set if no user token is present (older single-token
    /// installs that haven't reconnected since dual-token landed).
    pub fn granted_user_scopes(
        &self,
    ) -> Result<std::collections::HashSet<String>, IntegrationError> {
        let token = self.load_token()?;
        let scope_str = token
            .extras
            .get("authed_user")
            .and_then(|v| v.get("scope"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        Ok(parse_scope_string(scope_str))
    }

    fn oauth_config(&self) -> OAuthProviderConfig {
        self.provider().into_oauth_provider(self.client_id.clone(), self.client_secret.clone())
    }

    fn provider(&self) -> SlackProviderConfig {
        self.provider_config
            .as_ref()
            .map(|c| SlackProviderConfig {
                auth_url: c.auth_url.clone(),
                token_url: c.token_url.clone(),
                scopes: c.scopes.clone(),
                redirect_port: c.redirect_port,
            })
            .unwrap_or_default()
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
        let port = self.provider().redirect_port;
        run_oauth_flow(
            oauth_config,
            &store,
            SERVICE_NAME,
            "/oauth/callback",
            Some(port),
            ctx,
        )
        .await?;
        Ok(())
    }

    async fn disconnect(&self) -> Result<(), IntegrationError> {
        let store = self.token_store()?;
        store.delete(SERVICE_NAME)?;
        Ok(())
    }

    async fn capabilities_summary(&self) -> Option<String> {
        // Cheap path only — read persisted token, format both scope sets.
        // No network. If anything fails (no token, decrypt error, etc.),
        // return None and let the agent figure it out from tool errors.
        let bot_scopes = self.granted_scopes().ok()?;
        if bot_scopes.is_empty() {
            return None;
        }
        let user_scopes = self.granted_user_scopes().unwrap_or_default();

        let mut bot_sorted: Vec<&String> = bot_scopes.iter().collect();
        bot_sorted.sort();
        let bot_list = bot_sorted
            .into_iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(", ");

        if user_scopes.is_empty() {
            return Some(format!("slack (connected; bot scopes: {bot_list})"));
        }

        let mut user_sorted: Vec<&String> = user_scopes.iter().collect();
        user_sorted.sort();
        let user_list = user_sorted
            .into_iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        Some(format!(
            "slack (connected; bot scopes: {bot_list}; user scopes: {user_list}). \
             Reads use the user token (sees private channels you're in without bot invite); \
             posts use the bot token (appear as the bot)."
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_provider_carries_sixteen_bot_scopes() {
        let provider = SlackProviderConfig::default();
        assert_eq!(provider.scopes.len(), 16);
        assert!(provider.scopes.iter().any(|s| s == "chat:write"));
        assert!(provider.scopes.iter().any(|s| s == "chat:write.public"));
        assert!(provider.scopes.iter().any(|s| s == "channels:history"));
        assert!(provider.scopes.iter().any(|s| s == "users:read"));
        assert!(provider.scopes.iter().any(|s| s == "users:read.email"));
        assert!(provider.scopes.iter().any(|s| s == "files:read"));
        assert!(provider.scopes.iter().any(|s| s == "mpim:read"));
        assert!(provider.scopes.iter().any(|s| s == "im:write"));
        assert!(provider.scopes.iter().any(|s| s == "mpim:write"));
        assert!(!provider.scopes.iter().any(|s| s == "search:read"));
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
