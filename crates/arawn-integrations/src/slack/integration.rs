use std::path::PathBuf;

use arawn_auth::{OAuthProviderConfig, TokenStore};
use async_trait::async_trait;
use url::Url;

use crate::error::IntegrationError;
use crate::integration::{ConnectContext, Integration};
use crate::oauth_flow::{obtain_oauth_token, run_oauth_flow};

use super::client::{SlackContext, build_slack_client};

/// The Slack app: `[integrations.slack]`, and the argument to
/// `/connect slack`. Each connected workspace is its own integration,
/// named [`workspace_name`] (`slack@<alias>`).
pub const SERVICE_NAME: &str = "slack";

/// Longest workspace alias. With the longest tool op the tool name is
/// `slack_` + 32 + `_list_channels`, 52 characters: inside the 64 that
/// every LLM provider allows.
pub const ALIAS_MAX_LEN: usize = 32;

/// The alias when the team has neither a usable name nor an id.
pub const DEFAULT_ALIAS: &str = "default";

/// `slack@<alias>`: the integration name, the [`TokenStore`] key and the
/// credentials dir of one workspace.
pub fn workspace_name(alias: &str) -> String {
    format!("{SERVICE_NAME}@{alias}")
}

/// The alias of a workspace name (`slack@acme` → `acme`), or `None` when
/// `name` is not a valid workspace name.
pub fn alias_of(name: &str) -> Option<&str> {
    name.strip_prefix("slack@").filter(|a| is_valid_alias(a))
}

/// `[a-z0-9-]{1,32}`.
pub fn is_valid_alias(alias: &str) -> bool {
    !alias.is_empty()
        && alias.len() <= ALIAS_MAX_LEN
        && alias
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// An alias from a Slack team: the lowercased name, every run of other
/// characters made one `-`, trimmed and cut to [`ALIAS_MAX_LEN`]. A name
/// with non-ASCII letters, or nothing left, gives the team id instead.
/// `None` when neither gives an alias.
pub fn sanitize_alias(team_name: &str, team_id: &str) -> Option<String> {
    fn clean(s: &str) -> String {
        let mut out = String::new();
        for c in s.chars() {
            if c.is_ascii_alphanumeric() {
                out.push(c.to_ascii_lowercase());
            } else if !out.is_empty() && !out.ends_with('-') {
                out.push('-');
            }
        }
        out.truncate(ALIAS_MAX_LEN);
        out.trim_end_matches('-').to_string()
    }
    let from_name = if team_name.is_ascii() {
        clean(team_name)
    } else {
        String::new()
    };
    let alias = if from_name.is_empty() {
        clean(team_id)
    } else {
        from_name
    };
    (!alias.is_empty()).then_some(alias)
}

/// The alias for a team that just connected through `/connect slack`.
/// A team that is already connected keeps its alias (a reconnect
/// refreshes its token). Otherwise [`sanitize_alias`] of the team, or
/// [`DEFAULT_ALIAS`]; a taken alias gets `-2`, `-3`… `existing` is each
/// connected alias with its stored team.
pub fn pick_alias(team: Option<&SlackTeam>, existing: &[(String, Option<SlackTeam>)]) -> String {
    if let Some(t) = team.filter(|t| !t.id.is_empty())
        && let Some((alias, _)) = existing
            .iter()
            .find(|(_, e)| e.as_ref().is_some_and(|e| e.id == t.id))
    {
        return alias.clone();
    }
    let base = team
        .and_then(|t| sanitize_alias(&t.name, &t.id))
        .unwrap_or_else(|| DEFAULT_ALIAS.to_string());
    let taken = |a: &str| existing.iter().any(|(e, _)| e == a);
    if !taken(&base) {
        return base;
    }
    (2..)
        .map(|n| {
            let suffix = format!("-{n}");
            let mut stem = base.clone();
            stem.truncate(ALIAS_MAX_LEN - suffix.len());
            format!("{}{suffix}", stem.trim_end_matches('-'))
        })
        .find(|a| !taken(a))
        .expect("an unused suffix exists")
}

/// The Slack team a token was granted in. Slack's `oauth.v2.access`
/// returns `team: {id, name}`, kept in the token's `extras`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlackTeam {
    pub id: String,
    pub name: String,
}

/// The team stored in a token, if any.
pub fn team_of(token: &arawn_auth::Token) -> Option<SlackTeam> {
    let team = token.extras.get("team")?;
    let field = |k: &str| {
        team.get(k)
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string()
    };
    let t = SlackTeam {
        id: field("id"),
        name: field("name"),
    };
    (!t.id.is_empty() || !t.name.is_empty()).then_some(t)
}

/// Bot scopes requested at OAuth time. Per ADR-0001 § 4.
///
/// Note: `search:read` is intentionally absent. Requesting an unused scope
/// causes Slack to reject the OAuth flow with "Invalid permissions requested"
/// when the workspace admin hasn't pre-approved it, so the scope list stays
/// minimal until a feature actually needs `search.messages`.
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
#[derive(Clone)]
pub struct SlackProviderConfig {
    pub auth_url: Url,
    pub token_url: Url,
    pub scopes: Vec<String>,
    /// Pinned port for the local OAuth callback. Slack's redirect-URI
    /// allowlist is exact-match — no wildcard ports — so we bind a known
    /// port and the user adds `http://localhost:<port>/oauth/callback` to
    /// the Slack app config exactly once.
    pub redirect_port: u16,
}

/// Default callback port for Slack. The user adds
/// `http://localhost:8080/oauth/callback` to the Slack app's redirect
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
    pub fn into_oauth_provider(
        self,
        client_id: String,
        client_secret: String,
    ) -> OAuthProviderConfig {
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

/// The Slack app (REQ-005): one client id and secret, installed in any
/// number of workspaces. Makes the [`SlackIntegration`] of a workspace and
/// finds the connected ones in the token store.
pub struct SlackApp {
    data_dir: PathBuf,
    client_id: String,
    client_secret: String,
    provider_config: Option<SlackProviderConfig>,
}

impl SlackApp {
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

    /// The integration of workspace `alias`, connected or not.
    pub fn workspace(&self, alias: &str) -> Result<SlackIntegration, IntegrationError> {
        if !is_valid_alias(alias) {
            return Err(IntegrationError::Provider(format!(
                "`{alias}` is not a Slack workspace alias: use 1 to {ALIAS_MAX_LEN} of a-z, 0-9 and -"
            )));
        }
        let mut ws = SlackIntegration::new(
            self.data_dir.clone(),
            alias,
            self.client_id.clone(),
            self.client_secret.clone(),
        );
        ws.provider_config = self.provider_config.clone();
        Ok(ws)
    }

    /// Aliases of the workspaces with a stored token, sorted. Reads file
    /// names only.
    pub fn connected_aliases(&self) -> Vec<String> {
        connected_aliases(&self.data_dir)
    }

    /// `/connect slack`: run the OAuth flow, then name the workspace after
    /// the team the user picked in Slack's page ([`pick_alias`]) and store
    /// the token under it.
    pub async fn connect_new(
        &self,
        ctx: &dyn ConnectContext,
    ) -> Result<SlackIntegration, IntegrationError> {
        let provider = self.provider_config.clone().unwrap_or_default();
        let port = provider.redirect_port;
        let oauth =
            provider.into_oauth_provider(self.client_id.clone(), self.client_secret.clone());
        let token = obtain_oauth_token(oauth, "/oauth/callback", Some(port), ctx).await?;
        let existing: Vec<(String, Option<SlackTeam>)> = self
            .connected_aliases()
            .into_iter()
            .map(|a| {
                let team = self.workspace(&a).ok().and_then(|w| w.team());
                (a, team)
            })
            .collect();
        let alias = pick_alias(team_of(&token).as_ref(), &existing);
        TokenStore::open(&self.data_dir)?.save(&workspace_name(&alias), &token)?;
        self.workspace(&alias)
    }
}

/// Aliases of the workspaces with a stored token in `data_dir`, sorted.
pub fn connected_aliases(data_dir: &std::path::Path) -> Vec<String> {
    TokenStore::open(data_dir)
        .and_then(|s| s.keys())
        .unwrap_or_default()
        .iter()
        .filter_map(|k| alias_of(k).map(String::from))
        .collect()
}

/// One Slack workspace. Tools depend on it via `Arc<SlackIntegration>`.
pub struct SlackIntegration {
    data_dir: PathBuf,
    alias: String,
    /// `slack@<alias>`: integration name and token-store key.
    name: String,
    client_id: String,
    client_secret: String,
    provider_config: Option<SlackProviderConfig>,
}

impl SlackIntegration {
    /// The caller checks `alias` ([`is_valid_alias`]); [`SlackApp::workspace`]
    /// does.
    pub fn new(
        data_dir: PathBuf,
        alias: impl Into<String>,
        client_id: String,
        client_secret: String,
    ) -> Self {
        let alias = alias.into();
        Self {
            data_dir,
            name: workspace_name(&alias),
            alias,
            client_id,
            client_secret,
            provider_config: None,
        }
    }

    pub fn alias(&self) -> &str {
        &self.alias
    }

    /// `<data_dir>/integrations/slack@<alias>/`. Slack keeps its token in
    /// the token store; the dir is removed on disconnect if it exists.
    pub fn creds_dir(&self) -> PathBuf {
        self.data_dir.join("integrations").join(&self.name)
    }

    /// The team stored with the token, if any.
    pub fn team(&self) -> Option<SlackTeam> {
        self.load_token().ok().as_ref().and_then(team_of)
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
            .ok_or_else(|| IntegrationError::NotConnected(format!("{} (user token)", self.name)))?;
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
            .load(&self.name)?
            .ok_or_else(|| IntegrationError::NotConnected(self.name.clone()))
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
        self.provider()
            .into_oauth_provider(self.client_id.clone(), self.client_secret.clone())
    }

    fn provider(&self) -> SlackProviderConfig {
        self.provider_config.clone().unwrap_or_default()
    }

    fn token_store(&self) -> Result<TokenStore, IntegrationError> {
        Ok(TokenStore::open(&self.data_dir)?)
    }
}

#[async_trait]
impl Integration for SlackIntegration {
    fn name(&self) -> &str {
        &self.name
    }

    async fn is_connected(&self) -> bool {
        match self.token_store() {
            Ok(store) => store.load(&self.name).ok().flatten().is_some(),
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
            &self.name,
            "/oauth/callback",
            Some(port),
            ctx,
        )
        .await?;
        Ok(())
    }

    async fn disconnect(&self) -> Result<(), IntegrationError> {
        let store = self.token_store()?;
        store.delete(&self.name)?;
        match std::fs::remove_dir_all(self.creds_dir()) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(IntegrationError::Io(e)),
            _ => Ok(()),
        }
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

        let name = &self.name;
        let tools = super::tools::tool_name(&self.alias, "*");
        let team = match self.team() {
            Some(t) if !t.name.is_empty() => format!("team {}; ", t.name),
            _ => String::new(),
        };
        if user_scopes.is_empty() {
            return Some(format!(
                "{name} (connected; {team}bot scopes: {bot_list}). Tools: {tools}."
            ));
        }

        let mut user_sorted: Vec<&String> = user_scopes.iter().collect();
        user_sorted.sort();
        let user_list = user_sorted
            .into_iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        Some(format!(
            "{name} (connected; {team}bot scopes: {bot_list}; user scopes: {user_list}). \
             Reads use the user token (sees private channels you're in without bot invite); \
             posts use the bot token (appear as the bot). Tools: {tools}."
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

    fn token_for(team_id: &str, team_name: &str) -> arawn_auth::Token {
        let mut extras = serde_json::Map::new();
        extras.insert(
            "team".into(),
            serde_json::json!({ "id": team_id, "name": team_name }),
        );
        arawn_auth::Token {
            access: format!("xoxb-{team_id}"),
            refresh: None,
            expires_at: None,
            scope: Some("channels:read,chat:write".into()),
            token_type: "Bearer".into(),
            extras,
        }
    }

    fn app(dir: &std::path::Path) -> SlackApp {
        SlackApp::new(dir.to_path_buf(), "cid".into(), "csecret".into())
    }

    #[test]
    fn sanitize_alias_cases() {
        assert_eq!(sanitize_alias("Acme Corp", "T1").unwrap(), "acme-corp");
        // Punctuation runs become one dash; ends are trimmed.
        assert_eq!(
            sanitize_alias("  R&D -- Team!! ", "T1").unwrap(),
            "r-d-team"
        );
        // Non-ASCII name: the team id.
        assert_eq!(sanitize_alias("Équipe", "T0ABC").unwrap(), "t0abc");
        assert_eq!(sanitize_alias("", "T0ABC").unwrap(), "t0abc");
        assert_eq!(sanitize_alias("!!!", "T0ABC").unwrap(), "t0abc");
        assert!(sanitize_alias("", "").is_none());
        // Length cap, with no dash left at the cut.
        let long = sanitize_alias(&format!("{} b", "a".repeat(31)), "T1").unwrap();
        assert_eq!(long, "a".repeat(31));
        let cut = sanitize_alias(&"x".repeat(40), "T1").unwrap();
        assert_eq!(cut.len(), ALIAS_MAX_LEN);
        assert!(is_valid_alias(&cut));
    }

    #[test]
    fn alias_names_and_validation() {
        assert_eq!(workspace_name("acme"), "slack@acme");
        assert_eq!(alias_of("slack@acme"), Some("acme"));
        assert_eq!(alias_of("slack"), None);
        assert_eq!(alias_of("slack@Acme"), None);
        assert_eq!(alias_of(&format!("slack@{}", "a".repeat(33))), None);
        assert!(
            app(std::path::Path::new("/nonexistent"))
                .workspace("a/b")
                .is_err()
        );
    }

    #[tokio::test]
    async fn two_aliases_have_separate_tokens_and_disconnect_one_leaves_the_other() {
        let dir = tempfile::tempdir().unwrap();
        let app = app(dir.path());
        let acme = app.workspace("acme").unwrap();
        let personal = app.workspace("personal").unwrap();
        assert_eq!(acme.name(), "slack@acme");
        assert_eq!(personal.name(), "slack@personal");
        let store = TokenStore::open(dir.path()).unwrap();
        store.save("slack@acme", &token_for("TA", "Acme")).unwrap();
        store
            .save("slack@personal", &token_for("TP", "Personal"))
            .unwrap();

        assert_eq!(acme.team().unwrap().id, "TA");
        assert_eq!(personal.team().unwrap().id, "TP");
        assert!(acme.is_connected().await && personal.is_connected().await);
        assert!(
            acme.capabilities_summary()
                .await
                .unwrap()
                .starts_with("slack@acme (connected; team Acme;")
        );
        assert!(
            acme.capabilities_summary()
                .await
                .unwrap()
                .ends_with("Tools: slack_acme_*.")
        );

        acme.disconnect().await.unwrap();
        assert!(!acme.is_connected().await);
        assert!(personal.is_connected().await);
        assert_eq!(personal.team().unwrap().name, "Personal");
        assert_eq!(app.connected_aliases(), ["personal"]);
    }

    fn team(id: &str, name: &str) -> SlackTeam {
        SlackTeam {
            id: id.into(),
            name: name.into(),
        }
    }

    #[test]
    fn pick_alias_derives_from_the_team() {
        assert_eq!(pick_alias(Some(&team("TA", "Acme Corp")), &[]), "acme-corp");
        assert_eq!(pick_alias(Some(&team("T0X", "Équipe")), &[]), "t0x");
        assert_eq!(pick_alias(None, &[]), DEFAULT_ALIAS);
    }

    #[test]
    fn pick_alias_suffixes_a_collision_and_keeps_a_known_team() {
        let existing = vec![
            ("acme".to_string(), Some(team("TA", "Acme"))),
            ("acme-2".to_string(), None),
        ];
        // Another team also called Acme.
        assert_eq!(pick_alias(Some(&team("TB", "Acme")), &existing), "acme-3");
        // The same team again: a reconnect keeps its alias.
        assert_eq!(pick_alias(Some(&team("TA", "Acme Inc")), &existing), "acme");
        // The suffix fits in the length cap.
        let long = "x".repeat(ALIAS_MAX_LEN);
        let taken = vec![(long.clone(), Some(team("T1", &long)))];
        let a = pick_alias(Some(&team("T2", &long)), &taken);
        assert_eq!(a.len(), ALIAS_MAX_LEN);
        assert!(a.ends_with("-2") && is_valid_alias(&a), "{a}");
    }

    #[test]
    fn connected_aliases_come_from_the_token_store() {
        let dir = tempfile::tempdir().unwrap();
        let store = TokenStore::open(dir.path()).unwrap();
        store.save("slack@personal", &token_for("TP", "P")).unwrap();
        store.save("slack@acme", &token_for("TA", "A")).unwrap();
        // Not workspaces: another service, and the pre-workspace key.
        store.save("gmail", &token_for("x", "x")).unwrap();
        store.save("slack", &token_for("x", "x")).unwrap();
        assert_eq!(app(dir.path()).connected_aliases(), ["acme", "personal"]);
    }

    #[test]
    fn provider_lifts_into_oauth_config() {
        let cfg =
            SlackProviderConfig::default().into_oauth_provider("cid".into(), "csecret".into());
        assert_eq!(cfg.client_id, "cid");
        assert_eq!(cfg.client_secret, "csecret");
        assert_eq!(
            cfg.auth_url.as_str(),
            "https://slack.com/oauth/v2/authorize"
        );
        assert_eq!(
            cfg.token_url.as_str(),
            "https://slack.com/api/oauth.v2.access"
        );
    }
}
