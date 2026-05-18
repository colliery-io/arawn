//! `github/issues-and-prs` — open + recently-closed issues and PRs
//! the user authored or is assigned to (I-0045 T-0320).
//!
//! Hits `/search/issues` three times per tick:
//!   - `is:open assignee:@me`
//!   - `is:open author:@me`
//!   - `is:closed assignee:@me closed:>=<30d-ago>` (retro context)
//!
//! Storage:
//! ```text
//! <feed_dir>/issues_and_prs/<owner>__<repo>__<number>.json
//! ```
//!
//! The same issue/PR can come back across queries; the file path
//! (`<owner>__<repo>__<number>.json`) deduplicates naturally. Each
//! write is atomic (tmp + rename) so a row never lands half-written.
//!
//! Cursor: no time-floor cursor — search is bounded by `is:open`
//! and the 30-day closed window. Storing a `since` would force us
//! to also expire stale rows, which the dispatch pass handles via
//! body-hash comparison.

use std::path::{Path, PathBuf};
use std::time::Instant;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use serde_json::{Value, json};
use tracing::warn;

use crate::error::FeedError;
use crate::template::{DiscoveryRow, FeedTemplate, RunOutcome, TemplateCtx};
use crate::types::{FeedDefaults, RunSummary, TemplateParams};

pub struct IssuesAndPrsTemplate;

const NAME: &str = "github/issues-and-prs";
const DEFAULT_PER_PAGE: u32 = 100;
const MAX_PAGES_PER_QUERY: u32 = 5; // 500 results per query; comfortably under the search rate budget
const CLOSED_WINDOW_DAYS: i64 = 30;

#[async_trait]
impl FeedTemplate for IssuesAndPrsTemplate {
    fn name(&self) -> &'static str {
        NAME
    }

    fn validate(&self, _params: &TemplateParams) -> Result<(), FeedError> {
        Ok(())
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

        let closed_floor = (Utc::now() - Duration::days(CLOSED_WINDOW_DAYS))
            .format("%Y-%m-%d")
            .to_string();
        let queries = [
            "is:open assignee:@me".to_string(),
            "is:open author:@me".to_string(),
            format!("is:closed assignee:@me closed:>={closed_floor}"),
        ];

        let dir = feed_dir.join("issues_and_prs");
        std::fs::create_dir_all(&dir).map_err(|e| {
            FeedError::Storage(format!("create {}: {e}", dir.display()))
        })?;

        let mut total_items: u64 = 0;
        let mut total_bytes: u64 = 0;
        let mut seen_paths: std::collections::HashSet<PathBuf> =
            std::collections::HashSet::new();

        for query in &queries {
            let items = match github
                .search_issues(query, DEFAULT_PER_PAGE, MAX_PAGES_PER_QUERY)
                .await
            {
                Ok(v) => v,
                Err(e) => {
                    warn!(
                        target: "arawn::feeds",
                        feed = NAME,
                        query = %query,
                        error = %e,
                        "search failed; skipping this query"
                    );
                    continue;
                }
            };
            for item in &items {
                let path = match path_for_item(item, &dir) {
                    Some(p) => p,
                    None => {
                        warn!(target: "arawn::feeds", feed = NAME, "item missing html_url; skipping");
                        continue;
                    }
                };
                if seen_paths.contains(&path) {
                    continue; // dedupe across queries
                }
                match write_json(&path, item) {
                    Ok(bytes) => {
                        total_items += 1;
                        total_bytes += bytes;
                        seen_paths.insert(path);
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

/// Build the on-disk filename for a `/search/issues` row. Returns
/// `None` if the row's `html_url` doesn't look like a real issue/PR
/// URL.
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

    /// Fake that records each search query and returns canned items.
    struct FakeGithub {
        queries: Mutex<Vec<String>>,
        responses: Mutex<Vec<Vec<Value>>>, // FIFO per query
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
            let mut q = self.responses.lock().unwrap();
            if q.is_empty() {
                return Ok(Vec::new());
            }
            Ok(q.remove(0))
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

    fn issue(number: u64) -> Value {
        json!({
            "html_url": format!("https://github.com/o/r/issues/{number}"),
            "title": format!("issue {number}"),
            "state": "open",
            "user": {"login": "alice"},
            "body": "...",
            "created_at": "2026-05-10T09:00:00Z",
            "updated_at": "2026-05-18T09:00:00Z"
        })
    }

    #[tokio::test]
    async fn runs_all_three_queries_and_dedupes_by_path() {
        let tmp = TempDir::new().unwrap();
        // Q1 returns issues 1+2; Q2 returns issue 2 (dupe of Q1) +
        // issue 3; Q3 returns nothing. Expected on disk: 3 files.
        let responses = vec![
            vec![issue(1), issue(2)],
            vec![issue(2), issue(3)],
            vec![],
        ];
        let fake = Arc::new(FakeGithub {
            queries: Mutex::new(Vec::new()),
            responses: Mutex::new(responses),
        });
        let gh: Arc<dyn GithubFeedClient> = fake.clone();
        let clients: Arc<dyn FeedClients> = Arc::new(WithFakeGithub { gh });
        let ctx = TemplateCtx::new(clients);
        let out = IssuesAndPrsTemplate
            .run(&ctx, &TemplateParams::default(), tmp.path(), &json!({}))
            .await
            .unwrap();
        assert_eq!(out.summary.items_written, 3);
        assert_eq!(out.status, "ok");
        // All three queries were issued.
        let q = fake.queries.lock().unwrap();
        assert_eq!(q.len(), 3);
        assert!(q[0].contains("assignee:@me"));
        assert!(q[1].contains("author:@me"));
        assert!(q[2].contains("is:closed"));
        // Files on disk.
        for num in [1, 2, 3] {
            let p = tmp
                .path()
                .join("issues_and_prs")
                .join(format!("o__r__{num}.json"));
            assert!(p.exists(), "{p:?} missing");
        }
    }

    #[tokio::test]
    async fn empty_results_yield_no_new_items_status() {
        let tmp = TempDir::new().unwrap();
        let fake = Arc::new(FakeGithub {
            queries: Mutex::new(Vec::new()),
            responses: Mutex::new(vec![vec![], vec![], vec![]]),
        });
        let gh: Arc<dyn GithubFeedClient> = fake.clone();
        let clients: Arc<dyn FeedClients> = Arc::new(WithFakeGithub { gh });
        let ctx = TemplateCtx::new(clients);
        let out = IssuesAndPrsTemplate
            .run(&ctx, &TemplateParams::default(), tmp.path(), &json!({}))
            .await
            .unwrap();
        assert_eq!(out.summary.items_written, 0);
        assert_eq!(out.status, "no-new-items");
    }

    #[test]
    fn defaults_have_30min_cadence() {
        assert_eq!(
            IssuesAndPrsTemplate.defaults(&TemplateParams::default()).cadence,
            "*/30 * * * *"
        );
    }

    #[test]
    fn path_parser_handles_issues_and_prs_paths() {
        let dir = std::path::Path::new("/tmp/test");
        let issue = json!({"html_url":"https://github.com/foo/bar/issues/42"});
        assert_eq!(
            path_for_item(&issue, dir).unwrap(),
            std::path::PathBuf::from("/tmp/test/foo__bar__42.json")
        );
        let pr = json!({"html_url":"https://github.com/foo/bar/pull/7"});
        assert_eq!(
            path_for_item(&pr, dir).unwrap(),
            std::path::PathBuf::from("/tmp/test/foo__bar__7.json")
        );
        let bad = json!({"html_url":"https://example.com/x/y/z/1"});
        assert!(path_for_item(&bad, dir).is_none());
    }
}
