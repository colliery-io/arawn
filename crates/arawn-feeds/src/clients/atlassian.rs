//! Atlassian — what feeds need from Confluence (and later Jira),
//! plus the production adapter over `arawn-integrations`.
//!
//! This module introduces the shared `AtlassianFeedClient` trait. The
//! Confluence task (T-0222) lands the trait with Confluence-only
//! surface; the Jira task (T-0223) extends it with Jira methods. Both
//! Atlassian providers share an OAuth token + site selection, so it
//! makes sense to keep them on a single client trait rather than
//! split into two.
//!
//! Templates depend on the trait. Tests fake it externally;
//! production wires [`RealAtlassianClient`], which reuses the same
//! `AtlassianIntegration` (and persisted token) the existing
//! Confluence/Jira tools use.

use std::sync::Arc;

use arawn_integrations::atlassian::{AtlassianClient, AtlassianIntegration};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::FeedError;

/// Page metadata as feeds care about it. Subset of Confluence v2's
/// page resource — only what templates need.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfluencePageMeta {
    pub id: String,
    pub title: String,
    /// Space key the page lives in (e.g. `ENG`).
    pub space_key: String,
    /// Current version number.
    pub version: Option<i64>,
    /// RFC3339; Confluence's `lastModified` / version `createdAt`.
    pub modified_time: Option<String>,
    /// Best-effort web URL, when the API returns one.
    pub url: Option<String>,
}

/// Body of a Confluence page in storage format (raw XML).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfluencePageBody {
    pub id: String,
    /// Raw `body.storage.value` XML. May be `None` if the page has no
    /// body (placeholder pages, restricted access).
    pub storage_xml: Option<String>,
    pub version: Option<i64>,
}

/// What feeds need from Atlassian.
///
/// This task (T-0222) introduces the trait with Confluence-only
/// methods; T-0223 extends it with Jira surface (`jql_search`,
/// `issue_changelog`, `issue_comments`).
#[async_trait]
pub trait AtlassianFeedClient: Send + Sync {
    /// List pages in `space_key` modified after `since`. Returns
    /// metadata only — bodies fetched separately via
    /// [`Self::page_body_storage`].
    ///
    /// `since == None` means a full sweep (first run). The adapter
    /// must follow pagination and return every match.
    async fn space_pages_modified_since(
        &self,
        space_key: &str,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<ConfluencePageMeta>, FeedError>;

    /// Fetch a page's body in storage format (raw XML).
    async fn page_body_storage(&self, page_id: &str)
        -> Result<ConfluencePageBody, FeedError>;
}

// ─── Production adapter ──────────────────────────────────────────────

pub struct RealAtlassianClient {
    integration: Arc<AtlassianIntegration>,
}

impl RealAtlassianClient {
    pub fn new(integration: Arc<AtlassianIntegration>) -> Self {
        Self { integration }
    }
}

fn integ_err(e: arawn_integrations::IntegrationError) -> FeedError {
    use arawn_integrations::IntegrationError;
    match &e {
        IntegrationError::NotConnected(msg) => FeedError::Auth(msg.clone()),
        IntegrationError::Provider(msg) => classify_provider_error(msg),
        _ => FeedError::Provider(e.user_message()),
    }
}

/// Provider errors arrive as opaque strings from the Atlassian client.
/// Sniff out the well-known shapes we care about: rate-limit, gone,
/// auth. Everything else stays a generic Provider error.
fn classify_provider_error(msg: &str) -> FeedError {
    let lc = msg.to_ascii_lowercase();
    if lc.contains("429") || lc.contains("rate limit") || lc.contains("too many requests")
    {
        FeedError::RateLimited { retry_after: None }
    } else if lc.contains("410") || lc.contains("gone") {
        FeedError::Schema(format!("atlassian gone: {msg}"))
    } else if lc.contains("401")
        || lc.contains("403")
        || lc.contains("unauthorized")
        || lc.contains("invalid_grant")
        || lc.contains("token expired")
    {
        FeedError::Auth(msg.to_string())
    } else {
        FeedError::Provider(msg.to_string())
    }
}

// ── v1 CQL search response shape (we only need the bits we use) ──

#[derive(Debug, Deserialize)]
struct V1SearchResp {
    #[serde(default)]
    results: Vec<V1SearchResult>,
    #[serde(rename = "_links", default)]
    links: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct V1SearchResult {
    title: Option<String>,
    #[serde(default)]
    content: Option<V1Content>,
    #[serde(rename = "lastModified")]
    last_modified: Option<String>,
    #[serde(rename = "_links", default)]
    links: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct V1Content {
    id: String,
    space: Option<V1Space>,
    version: Option<V1Version>,
}

#[derive(Debug, Deserialize)]
struct V1Space {
    key: Option<String>,
}

#[derive(Debug, Deserialize)]
struct V1Version {
    number: Option<i64>,
    when: Option<String>,
}

// ── v2 page detail response shape ──

#[derive(Debug, Deserialize)]
struct V2PageDetail {
    id: String,
    body: Option<V2Body>,
    version: Option<V2Version>,
}

#[derive(Debug, Deserialize)]
struct V2Body {
    storage: Option<V2BodyStorage>,
}

#[derive(Debug, Deserialize)]
struct V2BodyStorage {
    value: Option<String>,
}

#[derive(Debug, Deserialize)]
struct V2Version {
    number: Option<i64>,
}

#[async_trait]
impl AtlassianFeedClient for RealAtlassianClient {
    async fn space_pages_modified_since(
        &self,
        space_key: &str,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<ConfluencePageMeta>, FeedError> {
        let client = AtlassianClient::new(Arc::clone(&self.integration));
        // CQL: page-type only, scoped to space, optionally filtered by
        // lastmodified. CQL has a relative-time form; we use the ISO
        // form directly: lastmodified > "yyyy-MM-dd HH:mm".
        //
        // CQL is what the integration's confluence_search uses too
        // (see comment in confluence.rs: "CQL search has no v2
        // equivalent yet; v1 /search remains functional per
        // Atlassian's deprecation table").
        let mut cql = format!("space = \"{space_key}\" AND type = \"page\"");
        if let Some(t) = since {
            cql.push_str(&format!(
                " AND lastmodified > \"{}\"",
                t.format("%Y-%m-%d %H:%M")
            ));
        }
        cql.push_str(" ORDER BY lastmodified ASC");

        let mut all: Vec<ConfluencePageMeta> = Vec::new();
        let mut start: u64 = 0;
        loop {
            let resp: V1SearchResp = client
                .confluence_v1_get(
                    "/search",
                    None,
                    &[
                        ("cql", cql.clone()),
                        ("limit", "100".into()),
                        ("start", start.to_string()),
                        ("expand", "content.version,content.space".into()),
                    ],
                )
                .await
                .map_err(integ_err)?;
            let page_count = resp.results.len();
            for r in resp.results {
                let content = match r.content {
                    Some(c) => c,
                    None => continue,
                };
                let space = content
                    .space
                    .and_then(|s| s.key)
                    .unwrap_or_else(|| space_key.to_string());
                let modified_time = r
                    .last_modified
                    .or_else(|| content.version.as_ref().and_then(|v| v.when.clone()));
                let version = content.version.and_then(|v| v.number);
                let url = r
                    .links
                    .get("webui")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                all.push(ConfluencePageMeta {
                    id: content.id,
                    title: r.title.unwrap_or_default(),
                    space_key: space,
                    version,
                    modified_time,
                    url,
                });
            }
            if page_count < 100 {
                break;
            }
            // v1 /search uses `start` + `limit`; cursor semantics live
            // in `_links.next`, but `start += limit` is reliable.
            start += 100;
            if !resp.links.contains_key("next") {
                break;
            }
        }
        Ok(all)
    }

    async fn page_body_storage(
        &self,
        page_id: &str,
    ) -> Result<ConfluencePageBody, FeedError> {
        let client = AtlassianClient::new(Arc::clone(&self.integration));
        let detail: V2PageDetail = client
            .confluence_get(
                &format!("/pages/{page_id}"),
                None,
                &[("body-format", "storage".into())],
            )
            .await
            .map_err(integ_err)?;
        Ok(ConfluencePageBody {
            id: detail.id,
            storage_xml: detail.body.and_then(|b| b.storage.and_then(|s| s.value)),
            version: detail.version.and_then(|v| v.number),
        })
    }
}
