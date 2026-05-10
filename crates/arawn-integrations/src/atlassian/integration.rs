use std::path::PathBuf;

use arawn_auth::{OAuthProviderConfig, Token, TokenStore};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::error::IntegrationError;
use crate::integration::{ConnectContext, Integration};
use crate::oauth_flow::run_oauth_flow;

/// Stable service name. Used as the [`TokenStore`] key, the integration
/// registry key, the per-service credential subdirectory, and the
/// argument to `/connect atlassian`.
pub const SERVICE_NAME: &str = "atlassian";

/// Default fixed port for the OAuth callback. Atlassian's redirect-URI
/// allowlist is exact-match (no wildcard ports), same as Slack.
pub const DEFAULT_ATLASSIAN_REDIRECT_PORT: u16 = 8080;

/// Bot scopes requested at OAuth time. Full read+write to both Jira and
/// Confluence; `offline_access` for refresh tokens (Atlassian access
/// tokens expire after ~1 hour).
///
/// Confluence v2 endpoints (`/wiki/api/v2/...`) require **granular**
/// scopes (`read:space:confluence`, etc.); the classic
/// `read:confluence-content.all` only authorizes v1. Both are listed so
/// the v1 CQL search and the v2 page/space surface both work.
pub const ATLASSIAN_OAUTH_SCOPES: &[&str] = &[
    // Jira (classic)
    "read:jira-work",
    "write:jira-work",
    "read:jira-user",
    // Confluence v1 (CQL search + space metadata + write fallback)
    "read:confluence-content.all",
    "read:confluence-content.summary",
    "search:confluence",
    "read:confluence-space.summary",
    "write:confluence-content",
    // Confluence v2 (granular — required by /wiki/api/v2/* endpoints)
    "read:space:confluence",
    "read:page:confluence",
    "write:page:confluence",
    "read:content-details:confluence",
    // Refresh tokens
    "offline_access",
];

/// One Atlassian site (workspace) the user authorized arawn to access.
/// Multi-site is real — same Atlassian account often has work + personal
/// instances. Persisted in the token's `extras` after the OAuth dance so
/// tools can route to the right `cloud_id` without re-fetching.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlassianSite {
    pub id: String,    // The cloud_id used in API URLs.
    pub url: String,   // e.g. "https://acme.atlassian.net"
    pub name: String,  // Human-readable instance name.
    #[serde(default)]
    pub scopes: Vec<String>,
}

/// Default Atlassian OAuth provider config.
pub struct AtlassianProviderConfig {
    pub auth_url: Url,
    pub token_url: Url,
    pub scopes: Vec<String>,
    pub redirect_port: u16,
}

impl Default for AtlassianProviderConfig {
    fn default() -> Self {
        Self {
            auth_url: "https://auth.atlassian.com/authorize".parse().unwrap(),
            token_url: "https://auth.atlassian.com/oauth/token".parse().unwrap(),
            scopes: ATLASSIAN_OAUTH_SCOPES.iter().map(|s| s.to_string()).collect(),
            redirect_port: DEFAULT_ATLASSIAN_REDIRECT_PORT,
        }
    }
}

impl AtlassianProviderConfig {
    pub fn into_oauth_provider(
        self,
        client_id: String,
        client_secret: String,
    ) -> OAuthProviderConfig {
        // Atlassian's authorize URL needs `audience=api.atlassian.com` and
        // `prompt=consent` to issue a refresh token. `audience` is the
        // resource we're requesting access to.
        OAuthProviderConfig {
            auth_url: self.auth_url,
            token_url: self.token_url,
            client_id,
            client_secret,
            scopes: self.scopes,
            extra_auth_params: vec![("audience".into(), "api.atlassian.com".into())],
        }
    }
}

/// Atlassian integration. Tools depend on it via `Arc<AtlassianIntegration>`.
pub struct AtlassianIntegration {
    data_dir: PathBuf,
    client_id: String,
    client_secret: String,
    provider_config: Option<AtlassianProviderConfig>,
}

impl AtlassianIntegration {
    pub fn new(data_dir: PathBuf, client_id: String, client_secret: String) -> Self {
        Self {
            data_dir,
            client_id,
            client_secret,
            provider_config: None,
        }
    }

    pub fn with_provider_config(mut self, config: AtlassianProviderConfig) -> Self {
        self.provider_config = Some(config);
        self
    }

    /// Load the persisted token. Returns `NotConnected` if absent.
    pub fn load_token(&self) -> Result<Token, IntegrationError> {
        let store = self.token_store()?;
        store
            .load(SERVICE_NAME)?
            .ok_or_else(|| IntegrationError::NotConnected(SERVICE_NAME.to_string()))
    }

    /// Persist the (potentially-refreshed) token back to disk.
    pub fn save_token(&self, token: &Token) -> Result<(), IntegrationError> {
        let store = self.token_store()?;
        store.save(SERVICE_NAME, token)?;
        Ok(())
    }

    /// Read the persisted set of accessible Atlassian sites (cloud_ids
    /// with their URLs and names). Empty if never connected or if the
    /// post-OAuth resource discovery failed.
    pub fn sites(&self) -> Result<Vec<AtlassianSite>, IntegrationError> {
        let token = self.load_token()?;
        let raw = token.extras.get("sites").cloned();
        let Some(value) = raw else {
            return Ok(Vec::new());
        };
        serde_json::from_value::<Vec<AtlassianSite>>(value).map_err(|e| {
            IntegrationError::Format(format!("decode atlassian sites: {e}"))
        })
    }

    /// Resolve a site by URL or name (e.g. `"acme.atlassian.net"`). When
    /// `which` is `None`, returns the first site (default).
    pub fn select_site(
        &self,
        which: Option<&str>,
    ) -> Result<AtlassianSite, IntegrationError> {
        let sites = self.sites()?;
        if sites.is_empty() {
            return Err(IntegrationError::NotConnected(format!(
                "{SERVICE_NAME} (no accessible sites — reconnect)"
            )));
        }
        match which {
            None => Ok(sites[0].clone()),
            Some(label) => {
                let needle = label.trim_start_matches("https://").trim_end_matches('/');
                sites
                    .into_iter()
                    .find(|s| {
                        let url_norm = s
                            .url
                            .trim_start_matches("https://")
                            .trim_end_matches('/');
                        url_norm == needle || s.name == needle || s.id == needle
                    })
                    .ok_or_else(|| {
                        IntegrationError::Format(format!(
                            "no Atlassian site matching '{label}' — \
                             available sites in capabilities_summary"
                        ))
                    })
            }
        }
    }

    /// Read the granted scope set from the persisted token.
    pub fn granted_scopes(
        &self,
    ) -> Result<std::collections::HashSet<String>, IntegrationError> {
        let token = self.load_token()?;
        let raw = token.scope.unwrap_or_default();
        Ok(raw
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect())
    }

    /// Compare the persisted token's scopes against what the current
    /// build expects. Returns the set the user's token is *missing*.
    /// A non-empty result means the token was minted by an older
    /// version of arawn and needs `/disconnect atlassian` +
    /// `/connect atlassian` to mint a fresh one.
    ///
    /// Returns `None` if no token is persisted (`granted_scopes` will
    /// fail with `NotConnected`, which is the integration-not-connected
    /// case, not a scope mismatch).
    pub fn missing_scopes(&self) -> Option<Vec<String>> {
        let granted = self.granted_scopes().ok()?;
        let required: std::collections::HashSet<&str> =
            ATLASSIAN_OAUTH_SCOPES.iter().copied().collect();
        let missing: Vec<String> = required
            .iter()
            .filter(|s| !granted.contains(**s))
            .map(|s| s.to_string())
            .collect();
        if missing.is_empty() {
            None
        } else {
            Some(missing)
        }
    }

    pub fn oauth_config(&self) -> OAuthProviderConfig {
        self.provider().into_oauth_provider(
            self.client_id.clone(),
            self.client_secret.clone(),
        )
    }

    fn provider(&self) -> AtlassianProviderConfig {
        self.provider_config
            .as_ref()
            .map(|c| AtlassianProviderConfig {
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
impl Integration for AtlassianIntegration {
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
        let outcome = run_oauth_flow(
            oauth_config,
            &store,
            SERVICE_NAME,
            "/oauth/callback",
            Some(port),
            ctx,
        )
        .await?;

        // Post-token: discover accessible_resources (the cloud_id
        // list) and stash it in the token's extras. Without sites,
        // every API call fails with "no accessible sites — reconnect"
        // — discovery success is a hard requirement, not best-effort
        // (see ARAWN-T-0235 Bug A). Retry with backoff on transient
        // failures, then refuse to keep the token if discovery still
        // can't populate a non-empty sites list.
        ctx.publish_progress("discovering accessible Atlassian sites…")
            .await;
        let sites = retry_accessible_resources(&outcome.token.access, 3).await;
        match sites {
            Ok(sites) if !sites.is_empty() => {
                let mut token = outcome.token.clone();
                token.extras.insert(
                    "sites".to_string(),
                    serde_json::to_value(&sites).unwrap_or(serde_json::Value::Null),
                );
                store.save(SERVICE_NAME, &token)?;
                ctx.publish_progress(&format!(
                    "found {} Atlassian site(s): {}",
                    sites.len(),
                    sites
                        .iter()
                        .map(|s| s.url.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
                .await;
            }
            Ok(_empty) => {
                // OAuth succeeded but the user has no Atlassian
                // workspaces accessible to this token — almost
                // certainly a missing or wrong scope grant. Drop the
                // token and surface the error.
                store.delete(SERVICE_NAME)?;
                return Err(IntegrationError::NotConnected(
                    "atlassian connect succeeded but no accessible sites were returned — \
                     verify the OAuth app has Jira+Confluence scopes and your account is \
                     a member of at least one site, then run /connect atlassian again"
                        .into(),
                ));
            }
            Err(e) => {
                tracing::warn!(error = %e, "atlassian accessible-resources discovery failed");
                store.delete(SERVICE_NAME)?;
                return Err(IntegrationError::NotConnected(format!(
                    "atlassian accessible-resources discovery failed after retries ({e}). \
                     Token discarded — re-run /connect atlassian to retry."
                )));
            }
        }
        Ok(())
    }

    async fn disconnect(&self) -> Result<(), IntegrationError> {
        let store = self.token_store()?;
        store.delete(SERVICE_NAME)?;
        Ok(())
    }

    async fn capabilities_summary(&self) -> Option<String> {
        let sites = self.sites().ok()?;
        if sites.is_empty() {
            return None;
        }
        let scopes = self.granted_scopes().unwrap_or_default();
        let mut sorted: Vec<&String> = scopes.iter().collect();
        sorted.sort();
        let scope_list = sorted
            .into_iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let site_list = sites
            .iter()
            .map(|s| {
                s.url
                    .trim_start_matches("https://")
                    .trim_end_matches('/')
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join(", ");
        Some(format!(
            "atlassian (connected; sites: {site_list}; scopes: {scope_list}). \
             Jira tools (jira_*) and Confluence tools (confluence_*) both available. \
             Tools default to the first site; pass `site` arg to switch."
        ))
    }
}

/// Atlassian's accessible-resources response shape (snake-case-d to
/// match the `AtlassianSite` we persist).
#[derive(Debug, Clone, Deserialize)]
struct RawAccessibleResource {
    id: String,
    url: String,
    name: String,
    #[serde(default)]
    scopes: Vec<String>,
}

/// Hit `https://api.atlassian.com/oauth/token/accessible-resources` to
/// learn which cloud sites the freshly-issued token has access to.
/// Wrap `fetch_accessible_resources` with up to `attempts` retries
/// on transient failure. Backoff is 200ms, 800ms, 3.2s — fits well
/// inside a normal /connect interaction without making the user wait
/// long if Atlassian is up. Used by `connect()` so a flaky network
/// during the OAuth dance doesn't silently produce an empty-sites
/// token.
async fn retry_accessible_resources(
    access: &str,
    attempts: u32,
) -> Result<Vec<AtlassianSite>, IntegrationError> {
    let mut last_err: Option<IntegrationError> = None;
    for attempt in 0..attempts {
        if attempt > 0 {
            let delay_ms = 200u64 * 4u64.pow(attempt - 1);
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
        }
        match fetch_accessible_resources(access).await {
            Ok(sites) => return Ok(sites),
            Err(e) => {
                tracing::warn!(
                    attempt = attempt + 1,
                    max = attempts,
                    error = %e,
                    "accessible-resources discovery attempt failed"
                );
                last_err = Some(e);
            }
        }
    }
    Err(last_err.unwrap_or_else(|| {
        IntegrationError::Format(
            "accessible-resources discovery failed with no error captured".into(),
        )
    }))
}

async fn fetch_accessible_resources(
    access_token: &str,
) -> Result<Vec<AtlassianSite>, IntegrationError> {
    let client = reqwest::Client::new();
    let resp = client
        .get("https://api.atlassian.com/oauth/token/accessible-resources")
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| IntegrationError::Provider(format!("network: {e}")))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(IntegrationError::Provider(format!(
            "accessible-resources HTTP {status}: {body}"
        )));
    }
    let raw: Vec<RawAccessibleResource> = resp
        .json()
        .await
        .map_err(|e| IntegrationError::Provider(format!("decode resources: {e}")))?;
    Ok(raw
        .into_iter()
        .map(|r| AtlassianSite {
            id: r.id,
            url: r.url,
            name: r.name,
            scopes: r.scopes,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_provider_carries_jira_classic_and_confluence_v2_scopes() {
        let provider = AtlassianProviderConfig::default();
        assert_eq!(provider.scopes.len(), ATLASSIAN_OAUTH_SCOPES.len());
        // Jira classic scopes
        assert!(provider.scopes.iter().any(|s| s == "read:jira-work"));
        assert!(provider.scopes.iter().any(|s| s == "write:jira-work"));
        // Confluence v1 (CQL search + space metadata)
        assert!(
            provider
                .scopes
                .iter()
                .any(|s| s == "read:confluence-content.all")
        );
        assert!(
            provider
                .scopes
                .iter()
                .any(|s| s == "read:confluence-space.summary")
        );
        // Confluence v2 (granular)
        assert!(provider.scopes.iter().any(|s| s == "read:space:confluence"));
        assert!(provider.scopes.iter().any(|s| s == "read:page:confluence"));
        assert!(provider.scopes.iter().any(|s| s == "write:page:confluence"));
        assert!(provider.scopes.iter().any(|s| s == "offline_access"));
    }

    #[test]
    fn provider_lifts_into_oauth_config_with_audience() {
        let cfg = AtlassianProviderConfig::default()
            .into_oauth_provider("cid".into(), "csecret".into());
        assert_eq!(cfg.client_id, "cid");
        assert!(
            cfg.extra_auth_params
                .iter()
                .any(|(k, v)| k == "audience" && v == "api.atlassian.com"),
            "audience param required for Atlassian OAuth"
        );
    }
}
