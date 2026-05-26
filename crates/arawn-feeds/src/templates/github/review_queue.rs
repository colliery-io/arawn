//! `github/review-queue` — PRs the user has been asked to review
//! (I-0045 T-0321).
//!
//! Single search query per tick: `is:open is:pr review-requested:@me`.
//! Smallest of the three GitHub templates; reuses the shared
//! search-pagination machinery via `GithubFeedClient::search_issues`.
//!
//! Storage:
//! ```text
//! <feed_dir>/review_queue/<owner>__<repo>__<number>.json
//! ```

use std::path::{Path, PathBuf};
use std::time::Instant;

use async_trait::async_trait;
use serde_json::{Value, json};
use tracing::warn;

use crate::error::FeedError;
use crate::template::{DiscoveryRow, FeedTemplate, RunOutcome, TemplateCtx};
use crate::types::{FeedDefaults, RunSummary, TemplateParams};

pub struct ReviewQueueTemplate;

const NAME: &str = "github/review-queue";
const DEFAULT_PER_PAGE: u32 = 100;
const MAX_PAGES: u32 = 3;
const QUERY: &str = "is:open is:pr review-requested:@me";

#[async_trait]
impl FeedTemplate for ReviewQueueTemplate {
    fn name(&self) -> &'static str {
        NAME
    }

    fn validate(&self, _params: &TemplateParams) -> Result<(), FeedError> {
        Ok(())
    }

    /// No params — open PRs where the current user is requested as a reviewer.
    fn param_schema(&self) -> Vec<crate::param_schema::ParamSpec> {
        Vec::new()
    }

    fn defaults(&self, _params: &TemplateParams) -> FeedDefaults {
        FeedDefaults {
            cadence: "*/30 * * * *".into(),
            initial_cursor: json!({}),
        }
    }

    async fn run(
        &self,
        ctx: &TemplateCtx,
        _params: &TemplateParams,
        feed_dir: &Path,
        _cursor: &Value,
    ) -> Result<RunOutcome, FeedError> {
        let started = Instant::now();
        let github = ctx
            .clients()
            .github()
            .ok_or_else(|| FeedError::Auth("github integration not connected".into()))?;

        let dir = feed_dir.join("review_queue");
        std::fs::create_dir_all(&dir).map_err(|e| {
            FeedError::Storage(format!("create {}: {e}", dir.display()))
        })?;

        let items = github
            .search_issues(QUERY, DEFAULT_PER_PAGE, MAX_PAGES)
            .await?;

        let mut total_items: u64 = 0;
        let mut total_bytes: u64 = 0;

        for item in &items {
            let path = match path_for_item(item, &dir) {
                Some(p) => p,
                None => {
                    warn!(target: "arawn::feeds", feed = NAME, "item missing html_url; skipping");
                    continue;
                }
            };
            match write_json(&path, item) {
                Ok(bytes) => {
                    total_items += 1;
                    total_bytes += bytes;
                }
                Err(e) => {
                    warn!(
                        target: "arawn::feeds",
                        feed = NAME,
                        error = %e,
                        "skipping item write"
                    );
                }
            }
        }

        let status = if total_items == 0 {
            "no-new-items".to_string()
        } else {
            "ok".to_string()
        };

        Ok(RunOutcome {
            cursor: json!({}),
            summary: RunSummary {
                items_written: total_items,
                bytes_written: total_bytes,
                duration: started.elapsed(),
            },
            status,
        })
    }

    async fn discover(&self, _ctx: &TemplateCtx) -> Result<Option<Vec<DiscoveryRow>>, FeedError> {
        Ok(None)
    }
}

fn path_for_item(item: &Value, dir: &Path) -> Option<PathBuf> {
    let url = item.get("html_url")?.as_str()?;
    let path = url.strip_prefix("https://github.com/")?;
    let mut parts = path.splitn(4, '/');
    let owner = parts.next()?;
    let repo = parts.next()?;
    let _kind = parts.next()?;
    let rest = parts.next()?;
    let number_str = rest.split(['/', '#', '?']).next().unwrap_or(rest);
    let number = number_str.parse::<u64>().ok()?;
    Some(dir.join(format!(
        "{}__{}__{}.json",
        sanitize(owner),
        sanitize(repo),
        number,
    )))
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
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
    use crate::clients::{FeedClients, GithubFeedClient};
    use crate::template::TemplateCtx;
    use std::sync::{Arc, Mutex};
    use tempfile::TempDir;

    struct FakeGithub {
        queries: Mutex<Vec<String>>,
        response: Mutex<Vec<Value>>,
    }

    #[async_trait]
    impl GithubFeedClient for FakeGithub {
        async fn list_notifications(
            &self,
            _since: Option<chrono::DateTime<chrono::Utc>>,
            _per_page: u32,
            _all: bool,
        ) -> Result<Vec<Value>, FeedError> {
            Ok(Vec::new())
        }
        async fn search_issues(
            &self,
            query: &str,
            _per_page: u32,
            _max_pages: u32,
        ) -> Result<Vec<Value>, FeedError> {
            self.queries.lock().unwrap().push(query.to_string());
            Ok(self.response.lock().unwrap().clone())
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

    fn pr(number: u64, draft: bool) -> Value {
        json!({
            "html_url": format!("https://github.com/o/r/pull/{number}"),
            "title": format!("PR {number}"),
            "user": {"login":"alice"},
            "draft": draft,
            "updated_at": "2026-05-18T10:00:00Z"
        })
    }

    #[tokio::test]
    async fn writes_review_queue_files() {
        let tmp = TempDir::new().unwrap();
        let fake = Arc::new(FakeGithub {
            queries: Mutex::new(Vec::new()),
            response: Mutex::new(vec![pr(1, false), pr(2, true)]),
        });
        let gh: Arc<dyn GithubFeedClient> = fake.clone();
        let clients: Arc<dyn FeedClients> = Arc::new(WithFakeGithub { gh });
        let ctx = TemplateCtx::new(clients);
        let out = ReviewQueueTemplate
            .run(&ctx, &TemplateParams::default(), tmp.path(), &json!({}))
            .await
            .unwrap();
        assert_eq!(out.summary.items_written, 2);
        let q = fake.queries.lock().unwrap();
        assert_eq!(q.len(), 1);
        assert!(q[0].contains("review-requested:@me"));
        assert!(q[0].contains("is:pr"));
        assert!(tmp.path().join("review_queue").join("o__r__1.json").exists());
        assert!(tmp.path().join("review_queue").join("o__r__2.json").exists());
    }

    #[tokio::test]
    async fn empty_response_is_no_new_items() {
        let tmp = TempDir::new().unwrap();
        let fake = Arc::new(FakeGithub {
            queries: Mutex::new(Vec::new()),
            response: Mutex::new(Vec::new()),
        });
        let gh: Arc<dyn GithubFeedClient> = fake;
        let clients: Arc<dyn FeedClients> = Arc::new(WithFakeGithub { gh });
        let ctx = TemplateCtx::new(clients);
        let out = ReviewQueueTemplate
            .run(&ctx, &TemplateParams::default(), tmp.path(), &json!({}))
            .await
            .unwrap();
        assert_eq!(out.status, "no-new-items");
        assert_eq!(out.summary.items_written, 0);
    }

    #[test]
    fn defaults_have_30min_cadence() {
        assert_eq!(
            ReviewQueueTemplate.defaults(&TemplateParams::default()).cadence,
            "*/30 * * * *"
        );
    }
}
