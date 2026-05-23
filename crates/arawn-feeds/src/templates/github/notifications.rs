//! `github/notifications` — mirror the user's `/notifications` inbox
//! (I-0045 T-0319).
//!
//! No required params (the GitHub App installation already scopes
//! what we can see). Optional params:
//! - `all: bool` (default `false`) — include already-read.
//! - `per_page: u32` (default 50, max 50 per GitHub).
//!
//! Storage:
//! ```text
//! <feed_dir>/notifications/<id>.json   # raw GitHub Notification
//! ```
//!
//! Cursor: `{ latest_updated_iso }` — advances to the most recent
//! `updated_at` we wrote. Next tick uses `since=<iso>`.

use std::path::{Path, PathBuf};
use std::time::Instant;

use async_trait::async_trait;
use chrono::DateTime;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tracing::warn;

use crate::error::FeedError;
use crate::template::{DiscoveryRow, FeedTemplate, RunOutcome, TemplateCtx};
use crate::types::{FeedDefaults, RunSummary, TemplateParams};

pub struct NotificationsTemplate;

const NAME: &str = "github/notifications";
const DEFAULT_PER_PAGE: u32 = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CursorState {
    #[serde(default)]
    latest_updated_iso: Option<String>,
}

impl CursorState {
    fn from_value(v: &Value) -> Self {
        serde_json::from_value::<Self>(v.clone()).unwrap_or(Self {
            latest_updated_iso: None,
        })
    }

    fn into_value(self) -> Value {
        json!({ "latest_updated_iso": self.latest_updated_iso })
    }
}

#[async_trait]
impl FeedTemplate for NotificationsTemplate {
    fn name(&self) -> &'static str {
        NAME
    }

    fn validate(&self, _params: &TemplateParams) -> Result<(), FeedError> {
        Ok(())
    }

    fn defaults(&self, _params: &TemplateParams) -> FeedDefaults {
        FeedDefaults {
            // I-0045 locked at 30 min for all GitHub feeds. User can
            // override per-feed via `[feeds.<id>.cadence]`.
            cadence: "*/30 * * * *".into(),
            initial_cursor: json!({ "latest_updated_iso": Value::Null }),
        }
    }

    async fn run(
        &self,
        ctx: &TemplateCtx,
        params: &TemplateParams,
        feed_dir: &Path,
        cursor: &Value,
    ) -> Result<RunOutcome, FeedError> {
        let started = Instant::now();
        let github = ctx
            .clients()
            .github()
            .ok_or_else(|| FeedError::Auth("github integration not connected".into()))?;

        let mut state = CursorState::from_value(cursor);
        let since = state
            .latest_updated_iso
            .as_deref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|d| d.with_timezone(&chrono::Utc));

        let per_page = params
            .0
            .get("per_page")
            .and_then(|v| v.as_u64())
            .map(|n| n as u32)
            .unwrap_or(DEFAULT_PER_PAGE)
            .min(50);
        let all = params
            .0
            .get("all")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let notifs = github.list_notifications(since, per_page, true).await?;

        let dir = feed_dir.join("notifications");
        std::fs::create_dir_all(&dir).map_err(|e| {
            FeedError::Storage(format!("create {}: {e}", dir.display()))
        })?;

        let mut total_items: u64 = 0;
        let mut total_bytes: u64 = 0;
        let mut new_latest = state.latest_updated_iso.clone();

        for n in &notifs {
            let id = match n.get("id").and_then(|v| v.as_str()) {
                Some(s) => s.to_string(),
                None => {
                    warn!(target: "arawn::feeds", feed = NAME, "notification missing id; skipping");
                    continue;
                }
            };
            let updated_at = n
                .get("updated_at")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let safe = sanitize_for_path(&id);
            let path = dir.join(format!("{safe}.json"));
            match write_json(&path, n) {
                Ok(bytes) => {
                    total_items += 1;
                    total_bytes += bytes;
                }
                Err(e) => {
                    warn!(
                        target: "arawn::feeds",
                        feed = NAME,
                        id = %id,
                        error = %e,
                        "skipping notification write"
                    );
                    continue;
                }
            }
            if let Some(ts) = updated_at
                && new_latest.as_deref().map(|n| ts.as_str() > n).unwrap_or(true)
            {
                new_latest = Some(ts);
            }
        }

        state.latest_updated_iso = new_latest;
        let _ = all; // captured into the request above
        let status = if total_items == 0 {
            "no-new-items".to_string()
        } else {
            "ok".to_string()
        };

        Ok(RunOutcome {
            cursor: state.into_value(),
            summary: RunSummary {
                items_written: total_items,
                bytes_written: total_bytes,
                duration: started.elapsed(),
            },
            status,
        })
    }

    async fn discover(&self, _ctx: &TemplateCtx) -> Result<Option<Vec<DiscoveryRow>>, FeedError> {
        // No discovery — the feed has no required params and the
        // installation already scopes what we can see.
        Ok(None)
    }
}

fn sanitize_for_path(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' })
        .collect()
}

fn write_json(path: &PathBuf, v: &Value) -> Result<u64, FeedError> {
    let body = serde_json::to_vec_pretty(v)
        .map_err(|e| FeedError::Storage(format!("serialize: {e}")))?;
    let len = body.len() as u64;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, &body)
        .map_err(|e| FeedError::Storage(format!("write {}: {e}", tmp.display())))?;
    std::fs::rename(&tmp, path)
        .map_err(|e| FeedError::Storage(format!("rename {}: {e}", path.display())))?;
    Ok(len)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::{FeedClients, GithubFeedClient, NoopClients};
    use crate::template::TemplateCtx;
    use std::sync::Arc;
    use tempfile::TempDir;

    struct FakeGithub {
        pages: Vec<Vec<Value>>,
    }

    #[async_trait]
    impl GithubFeedClient for FakeGithub {
        async fn list_notifications(
            &self,
            _since: Option<chrono::DateTime<chrono::Utc>>,
            _per_page: u32,
            _all: bool,
        ) -> Result<Vec<Value>, FeedError> {
            Ok(self.pages.iter().flatten().cloned().collect())
        }
        async fn search_issues(
            &self,
            _query: &str,
            _per_page: u32,
            _max_pages: u32,
        ) -> Result<Vec<Value>, FeedError> {
            Ok(Vec::new())
        }

        async fn list_repo_commits(
            &self,
            _owner: &str,
            _repo: &str,
            _since: Option<chrono::DateTime<chrono::Utc>>,
            _max_pages: u32,
        ) -> Result<Vec<Value>, FeedError> {
            Ok(Vec::new())
        }
        async fn list_repo_issues(
            &self,
            _owner: &str,
            _repo: &str,
            _state: &str,
            _since: Option<chrono::DateTime<chrono::Utc>>,
            _max_pages: u32,
        ) -> Result<Vec<Value>, FeedError> {
            Ok(Vec::new())
        }
        async fn list_repo_prs(
            &self,
            _owner: &str,
            _repo: &str,
            _state: &str,
            _since: Option<chrono::DateTime<chrono::Utc>>,
            _max_pages: u32,
        ) -> Result<Vec<Value>, FeedError> {
            Ok(Vec::new())
        }
        async fn list_issue_comments(
            &self,
            _owner: &str,
            _repo: &str,
            _since: Option<chrono::DateTime<chrono::Utc>>,
            _max_pages: u32,
        ) -> Result<Vec<Value>, FeedError> {
            Ok(Vec::new())
        }
        async fn list_pr_review_comments(
            &self,
            _owner: &str,
            _repo: &str,
            _since: Option<chrono::DateTime<chrono::Utc>>,
            _max_pages: u32,
        ) -> Result<Vec<Value>, FeedError> {
            Ok(Vec::new())
        }
        async fn list_org_repos(
            &self,
            _owner: &str,
            _max_pages: u32,
        ) -> Result<Vec<Value>, FeedError> {
            Ok(Vec::new())
        }
    }

    struct WithFakeGithub {
        gh: Arc<dyn GithubFeedClient>,
    }
    impl FeedClients for WithFakeGithub {
        fn slack(&self) -> Option<Arc<dyn crate::clients::SlackFeedClient>> {
            None
        }
        fn calendar(&self) -> Option<Arc<dyn crate::clients::CalendarFeedClient>> {
            None
        }
        fn gmail(&self) -> Option<Arc<dyn crate::clients::GmailFeedClient>> {
            None
        }
        fn drive(&self) -> Option<Arc<dyn crate::clients::DriveFeedClient>> {
            None
        }
        fn atlassian(&self) -> Option<Arc<dyn crate::clients::AtlassianFeedClient>> {
            None
        }
        fn github(&self) -> Option<Arc<dyn GithubFeedClient>> {
            Some(self.gh.clone())
        }
    }

    fn notif_json(id: &str, updated: &str) -> Value {
        json!({
            "id": id,
            "unread": true,
            "reason": "review_requested",
            "updated_at": updated,
            "last_read_at": null,
            "subject": {
                "title": format!("notif {id}"),
                "url": format!("https://api.github.com/repos/o/r/issues/{id}"),
                "type": "Issue"
            },
            "repository": { "name": "r", "owner": { "login": "o" } }
        })
    }

    #[tokio::test]
    async fn runs_with_no_clients_returns_auth_error() {
        let tmp = TempDir::new().unwrap();
        let clients: Arc<dyn FeedClients> = Arc::new(NoopClients);
        let ctx = TemplateCtx::new(clients);
        let cursor = json!({ "latest_updated_iso": null });
        let err = NotificationsTemplate
            .run(&ctx, &TemplateParams::default(), tmp.path(), &cursor)
            .await
            .unwrap_err();
        assert!(
            matches!(err, FeedError::Auth(_)),
            "unexpected variant: {err:?}"
        );
    }

    #[tokio::test]
    async fn writes_notification_files_and_advances_cursor() {
        let tmp = TempDir::new().unwrap();
        let pages = vec![vec![
            notif_json("100", "2026-05-18T10:00:00Z"),
            notif_json("101", "2026-05-18T11:00:00Z"),
            notif_json("102", "2026-05-18T09:00:00Z"),
        ]];
        let gh: Arc<dyn GithubFeedClient> = Arc::new(FakeGithub { pages });
        let clients: Arc<dyn FeedClients> = Arc::new(WithFakeGithub { gh });
        let ctx = TemplateCtx::new(clients);
        let cursor = json!({ "latest_updated_iso": null });
        let out = NotificationsTemplate
            .run(&ctx, &TemplateParams::default(), tmp.path(), &cursor)
            .await
            .unwrap();
        assert_eq!(out.summary.items_written, 3);
        assert_eq!(out.status, "ok");
        // Files on disk.
        for id in ["100", "101", "102"] {
            let path = tmp.path().join("notifications").join(format!("{id}.json"));
            assert!(path.exists(), "{path:?} missing");
        }
        // Cursor advanced to the max updated_at across the batch.
        assert_eq!(
            out.cursor
                .get("latest_updated_iso")
                .and_then(|v| v.as_str()),
            Some("2026-05-18T11:00:00Z")
        );
    }

    #[tokio::test]
    async fn empty_batch_returns_no_new_items_and_preserves_cursor() {
        let tmp = TempDir::new().unwrap();
        let gh: Arc<dyn GithubFeedClient> = Arc::new(FakeGithub { pages: vec![] });
        let clients: Arc<dyn FeedClients> = Arc::new(WithFakeGithub { gh });
        let ctx = TemplateCtx::new(clients);
        let cursor = json!({ "latest_updated_iso": "2026-05-10T00:00:00Z" });
        let out = NotificationsTemplate
            .run(&ctx, &TemplateParams::default(), tmp.path(), &cursor)
            .await
            .unwrap();
        assert_eq!(out.status, "no-new-items");
        assert_eq!(
            out.cursor
                .get("latest_updated_iso")
                .and_then(|v| v.as_str()),
            Some("2026-05-10T00:00:00Z")
        );
    }

    #[test]
    fn defaults_have_30min_cadence() {
        let d = NotificationsTemplate.defaults(&TemplateParams::default());
        assert_eq!(d.cadence, "*/30 * * * *");
    }

    #[test]
    fn validate_accepts_empty_params() {
        NotificationsTemplate
            .validate(&TemplateParams::default())
            .unwrap();
    }

    #[test]
    fn sanitize_replaces_unsafe_chars() {
        assert_eq!(sanitize_for_path("MDEx:OTM3MDAzOTE"), "MDEx_OTM3MDAzOTE");
        assert_eq!(sanitize_for_path("abc-123"), "abc-123");
    }
}
