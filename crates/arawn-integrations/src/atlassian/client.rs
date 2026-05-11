//! Thin HTTP client over Atlassian Cloud's Jira and Confluence REST APIs.
//!
//! Atlassian's Cloud APIs live under `https://api.atlassian.com/ex/...`
//! with a per-site `cloud_id` substituted in. We don't pull in a Rust
//! Atlassian SDK because the available crates are sparse / unmaintained;
//! a hand-rolled client over `reqwest` is ~200 LOC and exactly what we
//! need.
//!
//! Refresh: Atlassian access tokens are 1-hour-lived. This client checks
//! expiry on each call and refreshes via [`arawn_auth::OAuthClient`]
//! when needed, persisting the new token through the integration.

use std::sync::Arc;

use arawn_auth::{OAuthClient, Token};
use chrono::Utc;
use jira_v3_openapi::apis::configuration::Configuration as JiraConfig;
use reqwest::{Client, Method, Response};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::IntegrationError;

use super::integration::{AtlassianIntegration, AtlassianSite};

/// Refresh-aware Atlassian HTTP client. Holds a reference to the
/// integration so it can re-load and persist tokens across refreshes.
pub struct AtlassianClient {
    integration: Arc<AtlassianIntegration>,
    http: Client,
}

impl AtlassianClient {
    pub fn new(integration: Arc<AtlassianIntegration>) -> Self {
        Self {
            integration,
            http: Client::new(),
        }
    }

    /// Resolve the target site (defaulting to the first one) and return
    /// the cloud_id-stamped API base for the given product.
    ///
    /// **Confluence is on v2 by default** (`/wiki/api/v2`). v1
    /// (`/wiki/rest/api`) is reachable via [`Self::confluence_v1_get`]
    /// for the few endpoints (CQL search) that don't have a v2 yet.
    fn product_base(
        &self,
        product: Product,
        site: Option<&str>,
    ) -> Result<(AtlassianSite, String), IntegrationError> {
        let site = self.integration.select_site(site)?;
        let base = match product {
            Product::Confluence => format!(
                "https://api.atlassian.com/ex/confluence/{}/wiki/api/v2",
                site.id
            ),
            Product::ConfluenceV1 => format!(
                "https://api.atlassian.com/ex/confluence/{}/wiki/rest/api",
                site.id
            ),
        };
        Ok((site, base))
    }

    /// Get a fresh access token. Refreshes via OAuthClient if expired.
    async fn fresh_access_token(&self) -> Result<String, IntegrationError> {
        let token = self.integration.load_token()?;
        if !is_expired(&token) {
            return Ok(token.access);
        }
        // Need refresh.
        let Some(refresh) = token.refresh.clone() else {
            return Err(IntegrationError::NotConnected("atlassian (token expired and no refresh token; reconnect)".to_string()));
        };
        let oauth = OAuthClient::new(self.integration.oauth_config());
        let mut new_token = oauth.refresh(&refresh).await?;
        // Carry forward extras (notably `sites` populated during
        // connect()'s accessible-resources discovery). Without this,
        // every refresh wipes the sites list and the next API call
        // fails with "no accessible sites — reconnect" — see
        // ARAWN-T-0235.
        merge_prior_extras(&mut new_token, &token.extras);
        self.integration.save_token(&new_token)?;
        Ok(new_token.access)
    }

    /// Build a `jira_v3_openapi::Configuration` for the selected site,
    /// pre-populated with a fresh OAuth bearer token and the cloud_id
    /// gateway base URL. The generated client appends `/rest/api/3/...`
    /// itself; we only set the host + cloud_id prefix.
    pub async fn jira_config(&self, site: Option<&str>) -> Result<JiraConfig, IntegrationError> {
        let site = self.integration.select_site(site)?;
        let access = self.fresh_access_token().await?;
        Ok(JiraConfig {
            base_path: format!("https://api.atlassian.com/ex/jira/{}", site.id),
            oauth_access_token: Some(access),
            user_agent: Some("arawn/0.1".to_string()),
            ..JiraConfig::default()
        })
    }

    /// GET a JSON-bodied resource from Confluence.
    pub async fn confluence_get<T: DeserializeOwned>(
        &self,
        path: &str,
        site: Option<&str>,
        query: &[(&str, String)],
    ) -> Result<T, IntegrationError> {
        let (_, base) = self.product_base(Product::Confluence, site)?;
        self.send_json(Method::GET, &format!("{base}{path}"), query, None::<&()>)
            .await
    }

    /// POST a JSON body to Confluence.
    pub async fn confluence_post<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        site: Option<&str>,
        body: &B,
    ) -> Result<T, IntegrationError> {
        let (_, base) = self.product_base(Product::Confluence, site)?;
        self.send_json(Method::POST, &format!("{base}{path}"), &[], Some(body))
            .await
    }

    /// PUT a JSON body to Confluence (used by page update).
    pub async fn confluence_put<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        site: Option<&str>,
        body: &B,
    ) -> Result<T, IntegrationError> {
        let (_, base) = self.product_base(Product::Confluence, site)?;
        self.send_json(Method::PUT, &format!("{base}{path}"), &[], Some(body))
            .await
    }

    /// GET against the legacy Confluence v1 API. Use only for endpoints
    /// that don't have a v2 equivalent yet (notably CQL search).
    pub async fn confluence_v1_get<T: DeserializeOwned>(
        &self,
        path: &str,
        site: Option<&str>,
        query: &[(&str, String)],
    ) -> Result<T, IntegrationError> {
        let (_, base) = self.product_base(Product::ConfluenceV1, site)?;
        self.send_json(Method::GET, &format!("{base}{path}"), query, None::<&()>)
            .await
    }

    async fn send_json<B: Serialize, T: DeserializeOwned>(
        &self,
        method: Method,
        url: &str,
        query: &[(&str, String)],
        body: Option<&B>,
    ) -> Result<T, IntegrationError> {
        let resp = self.send(method, url, query, body).await?;
        let status = resp.status();
        // Capture Retry-After before consuming the body — Atlassian
        // surfaces it on 429s. parse_retry_after handles delta-seconds
        // and HTTP-date forms.
        if status.as_u16() == 429 {
            let retry_after = resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| crate::parse_retry_after(Some(s)));
            return Err(IntegrationError::RateLimited { retry_after });
        }
        let text = resp
            .text()
            .await
            .map_err(|e| IntegrationError::Provider(format!("read body: {e}")))?;
        if !status.is_success() {
            return Err(IntegrationError::Provider(format!(
                "HTTP {status}: {text}"
            )));
        }
        serde_json::from_str(&text)
            .map_err(|e| IntegrationError::Provider(format!("decode body: {e} (raw: {text})")))
    }

    async fn send<B: Serialize>(
        &self,
        method: Method,
        url: &str,
        query: &[(&str, String)],
        body: Option<&B>,
    ) -> Result<Response, IntegrationError> {
        let access = self.fresh_access_token().await?;
        let mut req = self.http.request(method, url).bearer_auth(access);
        if !query.is_empty() {
            req = req.query(query);
        }
        if let Some(b) = body {
            req = req.json(b);
        }
        req.send()
            .await
            .map_err(|e| IntegrationError::Provider(format!("network: {e}")))
    }
}

#[derive(Debug, Clone, Copy)]
enum Product {
    /// Confluence v2 — `/wiki/api/v2`. Default for new endpoints.
    Confluence,
    /// Confluence v1 — `/wiki/rest/api`. Kept only for CQL search,
    /// the one v1 endpoint with no v2 replacement.
    ConfluenceV1,
}

fn is_expired(token: &Token) -> bool {
    match token.expires_at {
        // Refresh 60s before actual expiry to avoid races on slow networks.
        Some(t) => Utc::now() + chrono::Duration::seconds(60) >= t,
        None => false,
    }
}

/// Carry the prior token's extras into the refreshed token. New
/// keys from the refresh response (rare for atlassian) take
/// precedence; everything else (notably `sites`) is preserved so
/// `select_site` keeps working post-refresh.
///
/// Extracted from `fresh_access_token` so the merge semantic is
/// testable without an HTTP mock.
fn merge_prior_extras(
    new_token: &mut Token,
    prior_extras: &serde_json::Map<String, serde_json::Value>,
) {
    for (k, v) in prior_extras.iter() {
        new_token
            .extras
            .entry(k.clone())
            .or_insert_with(|| v.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn token_with_extras(extras: serde_json::Map<String, serde_json::Value>) -> Token {
        Token {
            access: "stub-access".into(),
            refresh: Some("stub-refresh".into()),
            expires_at: None,
            scope: None,
            token_type: "Bearer".into(),
            extras,
        }
    }

    #[test]
    fn refresh_preserves_sites_when_new_token_extras_empty() {
        let prior_extras = {
            let mut m = serde_json::Map::new();
            m.insert(
                "sites".into(),
                json!([{"id":"abc","url":"https://acme.atlassian.net","name":"acme","scopes":[]}]),
            );
            m
        };
        let mut new_token = token_with_extras(serde_json::Map::new());
        merge_prior_extras(&mut new_token, &prior_extras);
        assert!(
            new_token.extras.get("sites").is_some(),
            "sites must survive the refresh"
        );
    }

    #[test]
    fn refresh_doesnt_overwrite_extras_the_provider_set() {
        // Hypothetical: provider puts a "scope" or "session_id" in
        // refresh response extras. Our merge keeps that.
        let prior_extras = {
            let mut m = serde_json::Map::new();
            m.insert("sites".into(), json!(["site-a"]));
            m.insert("session_id".into(), json!("old-id"));
            m
        };
        let mut new_token = token_with_extras({
            let mut m = serde_json::Map::new();
            m.insert("session_id".into(), json!("new-id"));
            m
        });
        merge_prior_extras(&mut new_token, &prior_extras);
        // sites carried forward
        assert_eq!(new_token.extras["sites"], json!(["site-a"]));
        // session_id from new token wins
        assert_eq!(new_token.extras["session_id"], json!("new-id"));
    }

    #[test]
    fn refresh_with_empty_prior_extras_is_no_op() {
        let mut new_token = token_with_extras({
            let mut m = serde_json::Map::new();
            m.insert("foo".into(), json!("bar"));
            m
        });
        merge_prior_extras(&mut new_token, &serde_json::Map::new());
        assert_eq!(new_token.extras["foo"], json!("bar"));
        assert_eq!(new_token.extras.len(), 1);
    }
}
