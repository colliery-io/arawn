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
use jira_v3_openapi::apis::{issue_search_api, issues_api, projects_api};
use jira_v3_openapi::models::SearchAndReconcileRequestBean;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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

/// Lightweight Jira issue summary returned by [`AtlassianFeedClient::jql_search`].
/// Only the fields feeds actually use to decide what to fetch.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JiraIssueMeta {
    pub key: String,
    pub id: String,
    /// `fields.updated` — RFC3339. Used for cursor advancement.
    pub updated: Option<String>,
    /// `fields.summary`, when requested.
    pub summary: Option<String>,
}

/// Full issue snapshot — meta + raw fields blob + optional changelog
/// histories and comments. Returned by [`AtlassianFeedClient::issue_full`].
///
/// `fields` is the verbatim `Issue.fields` object so templates can write
/// it to disk without lossy translation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraIssueDetail {
    pub meta: JiraIssueMeta,
    pub fields: Value,
    /// Comment objects from `fields.comment.comments`. `None` when the
    /// caller didn't request comments.
    pub comments: Option<Vec<Value>>,
    /// Changelog history entries from `changelog.histories`. Each
    /// entry has `id`, `created`, `author`, `items`. `None` when the
    /// caller didn't request changelog.
    pub changelog: Option<Vec<Value>>,
}

/// What feeds need from Atlassian.
///
/// Confluence methods landed in T-0222; Jira methods (jql_search,
/// issue_full, resolve_project) landed in T-0223.
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

    /// Run a JQL search and return up to `max_results` issues' meta.
    /// Adapter follows pagination. Templates own the JQL — including
    /// any `updated >=` clause for incremental cursoring.
    async fn jql_search(
        &self,
        jql: &str,
        max_results: u32,
    ) -> Result<Vec<JiraIssueMeta>, FeedError>;

    /// Fetch a full issue. When `want_changelog` or `want_comments`
    /// is true, the corresponding fields on the returned detail are
    /// populated; otherwise they're `None`. Lets `assignee-tracker`
    /// avoid paying the changelog/comments cost it doesn't need.
    async fn issue_full(
        &self,
        key: &str,
        want_changelog: bool,
        want_comments: bool,
    ) -> Result<JiraIssueDetail, FeedError>;

    /// Resolve a project key (or id) to its canonical id. Used at
    /// registration time so a typo in the `project` param fails fast
    /// instead of silently returning empty results forever.
    async fn resolve_project(&self, key_or_id: &str) -> Result<String, FeedError>;
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

    async fn jql_search(
        &self,
        jql: &str,
        max_results: u32,
    ) -> Result<Vec<JiraIssueMeta>, FeedError> {
        let client = AtlassianClient::new(Arc::clone(&self.integration));
        let cfg = client.jira_config(None).await.map_err(integ_err)?;
        let req = SearchAndReconcileRequestBean {
            jql: Some(jql.into()),
            max_results: Some(max_results.min(100) as i32),
            fields: Some(vec!["summary".into(), "updated".into()]),
            ..Default::default()
        };
        let resp = issue_search_api::search_and_reconsile_issues_using_jql_post(&cfg, req)
            .await
            .map_err(jira_err)?;
        let issues = resp.issues.unwrap_or_default();
        let mut out = Vec::with_capacity(issues.len());
        for issue in issues {
            let key = issue.key.clone().unwrap_or_default();
            let id = issue.id.clone().unwrap_or_default();
            let fields = issue
                .fields
                .as_ref()
                .map(|f| serde_json::to_value(f).unwrap_or(Value::Null))
                .unwrap_or(Value::Null);
            let updated = fields
                .get("updated")
                .and_then(|v| v.as_str())
                .map(str::to_string);
            let summary = fields
                .get("summary")
                .and_then(|v| v.as_str())
                .map(str::to_string);
            out.push(JiraIssueMeta { key, id, updated, summary });
        }
        Ok(out)
    }

    async fn issue_full(
        &self,
        key: &str,
        want_changelog: bool,
        want_comments: bool,
    ) -> Result<JiraIssueDetail, FeedError> {
        let client = AtlassianClient::new(Arc::clone(&self.integration));
        let cfg = client.jira_config(None).await.map_err(integ_err)?;

        // Build expand list. `comment` is a field, not an expand —
        // request via `fields=*all` (we always want everything for
        // the on-disk snapshot anyway). `changelog` is a true expand.
        let mut expand_parts: Vec<&str> = Vec::new();
        if want_changelog {
            expand_parts.push("changelog");
        }
        let expand = if expand_parts.is_empty() {
            None
        } else {
            Some(expand_parts.join(","))
        };

        // Always pull all fields. Comments come back inside
        // `fields.comment.comments` when present.
        let fields_list: Vec<String> = vec!["*all".into()];

        let issue = issues_api::get_issue(
            &cfg,
            key,
            Some(fields_list),
            None,
            expand.as_deref(),
            None,
            None,
            None,
        )
        .await
        .map_err(jira_err)?;

        let raw_fields = issue
            .fields
            .as_ref()
            .map(|f| serde_json::to_value(f).unwrap_or(Value::Null))
            .unwrap_or(Value::Null);
        let updated = raw_fields
            .get("updated")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let summary = raw_fields
            .get("summary")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let meta = JiraIssueMeta {
            key: issue.key.clone().unwrap_or_default(),
            id: issue.id.clone().unwrap_or_default(),
            updated,
            summary,
        };

        let comments = if want_comments {
            Some(
                raw_fields
                    .get("comment")
                    .and_then(|v| v.get("comments"))
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default(),
            )
        } else {
            None
        };

        let changelog = if want_changelog {
            // The openapi model for issue puts changelog at the top
            // level of the response, not inside fields. Fall back to
            // serializing the entire issue and pulling it from there.
            let raw_issue = serde_json::to_value(&issue).unwrap_or(Value::Null);
            Some(
                raw_issue
                    .get("changelog")
                    .and_then(|v| v.get("histories"))
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default(),
            )
        } else {
            None
        };

        Ok(JiraIssueDetail {
            meta,
            fields: raw_fields,
            comments,
            changelog,
        })
    }

    async fn resolve_project(&self, key_or_id: &str) -> Result<String, FeedError> {
        let client = AtlassianClient::new(Arc::clone(&self.integration));
        let cfg = client.jira_config(None).await.map_err(integ_err)?;
        let project = projects_api::get_project(&cfg, key_or_id, None, None)
            .await
            .map_err(|e| match e {
                jira_v3_openapi::apis::Error::ResponseError(r) if r.status.as_u16() == 404 => {
                    FeedError::InvalidParams(format!(
                        "no Jira project '{key_or_id}'"
                    ))
                }
                other => jira_err(other),
            })?;
        project
            .id
            .ok_or_else(|| FeedError::Schema("project response missing id".into()))
    }
}

fn jira_err<E: std::fmt::Debug>(e: jira_v3_openapi::apis::Error<E>) -> FeedError {
    use jira_v3_openapi::apis::Error;
    match e {
        Error::ResponseError(r) => match r.status.as_u16() {
            401 | 403 => FeedError::Auth(format!("jira {}: {}", r.status, r.content)),
            404 => FeedError::InvalidParams(format!("jira not found: {}", r.content)),
            410 => FeedError::Schema(format!("jira gone: {}", r.content)),
            429 => FeedError::RateLimited { retry_after: None },
            _ => FeedError::Provider(format!("jira {}: {}", r.status, r.content)),
        },
        other => FeedError::Provider(format!("jira: {other:?}")),
    }
}
