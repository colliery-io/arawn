//! `github/repo-mirror` — pull commits + issues + PRs + comments
//! for one repo every 30 min (I-0050 T-0325).
//!
//! Required params: `owner`, `name`. Storage layout:
//! ```text
//! <feed_dir>/<owner>/<name>/commits/<sha>.json
//! <feed_dir>/<owner>/<name>/issues/<number>.json
//! <feed_dir>/<owner>/<name>/prs/<number>.json
//! <feed_dir>/<owner>/<name>/comments/<comment_id>.json
//! ```
//! Cursor: four per-kind `since` strings in `CursorState`. Advances
//! to the max `updated_at` (or commit `committer.date`) across each
//! kind's batch. Partial failures (one kind 4xxs) log + continue;
//! other kinds still write and advance.

use std::path::{Path, PathBuf};
use std::time::Instant;

use async_trait::async_trait;
use chrono::DateTime;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tracing::warn;

use crate::clients::GithubFeedClient;
use crate::error::FeedError;
use crate::template::{DiscoveryRow, FeedTemplate, RunOutcome, TemplateCtx};
use crate::types::{FeedDefaults, RunSummary, TemplateParams};

pub struct RepoMirrorTemplate;

const NAME: &str = "github/repo-mirror";
/// Conservative pagination cap per kind per tick. With 30-min cadence
/// and core-API budget of 5000/h, one repo's tick spends at worst
/// 4 kinds × 5 pages = 20 calls; even 100 repos stays comfortably in
/// budget.
const MAX_PAGES_PER_KIND: u32 = 5;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct CursorState {
    #[serde(default)]
    commits_since: Option<String>,
    #[serde(default)]
    issues_since: Option<String>,
    #[serde(default)]
    prs_since: Option<String>,
    #[serde(default)]
    comments_since: Option<String>,
}

impl CursorState {
    fn from_value(v: &Value) -> Self {
        serde_json::from_value::<Self>(v.clone()).unwrap_or_default()
    }

    fn into_value(self) -> Value {
        json!({
            "commits_since": self.commits_since,
            "issues_since": self.issues_since,
            "prs_since": self.prs_since,
            "comments_since": self.comments_since,
        })
    }
}

#[async_trait]
impl FeedTemplate for RepoMirrorTemplate {
    fn name(&self) -> &'static str {
        NAME
    }

    fn validate(&self, params: &TemplateParams) -> Result<(), FeedError> {
        let owner = params
            .0
            .get("owner")
            .and_then(|v| v.as_str())
            .ok_or_else(|| FeedError::InvalidParams("missing required param: owner".into()))?;
        let name = params
            .0
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| FeedError::InvalidParams("missing required param: name".into()))?;
        if owner.trim().is_empty() || name.trim().is_empty() {
            return Err(FeedError::InvalidParams(
                "owner and name must not be empty".into(),
            ));
        }
        Ok(())
    }

    fn defaults(&self, _params: &TemplateParams) -> FeedDefaults {
        FeedDefaults {
            cadence: "*/30 * * * *".into(),
            initial_cursor: CursorState::default().into_value(),
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
        let owner = params
            .0
            .get("owner")
            .and_then(|v| v.as_str())
            .ok_or_else(|| FeedError::InvalidParams("missing owner".into()))?
            .to_string();
        let name = params
            .0
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| FeedError::InvalidParams("missing name".into()))?
            .to_string();

        let mut state = CursorState::from_value(cursor);
        let repo_dir = feed_dir.join(&owner).join(&name);

        let mut total_items: u64 = 0;
        let mut total_bytes: u64 = 0;

        // ── commits ────────────────────────────────────────────
        {
            let since = state.commits_since.as_deref().and_then(parse_iso);
            let commits = match github
                .list_repo_commits(&owner, &name, since, MAX_PAGES_PER_KIND)
                .await
            {
                Ok(v) => v,
                Err(e) => {
                    warn!(target: "arawn::feeds", feed = NAME, owner = %owner, name = %name,
                          error = %e, "commits fetch failed; skipping kind this tick");
                    Vec::new()
                }
            };
            let kind_dir = repo_dir.join("commits");
            let (n, b, latest) = write_batch(
                &kind_dir,
                &commits,
                |v| v.get("sha").and_then(|s| s.as_str()).map(|s| s.to_string()),
                |v| {
                    v.get("commit")
                        .and_then(|c| c.get("author"))
                        .and_then(|a| a.get("date"))
                        .and_then(|d| d.as_str())
                        .map(|s| s.to_string())
                },
            )?;
            total_items += n;
            total_bytes += b;
            state.commits_since = advance(state.commits_since.clone(), latest);
        }

        // ── issues ─────────────────────────────────────────────
        {
            let since = state.issues_since.as_deref().and_then(parse_iso);
            let issues = match github
                .list_repo_issues(&owner, &name, "all", since, MAX_PAGES_PER_KIND)
                .await
            {
                Ok(v) => v,
                Err(e) => {
                    warn!(target: "arawn::feeds", feed = NAME, owner = %owner, name = %name,
                          error = %e, "issues fetch failed; skipping kind this tick");
                    Vec::new()
                }
            };
            let kind_dir = repo_dir.join("issues");
            let (n, b, latest) = write_batch(
                &kind_dir,
                &issues,
                |v| {
                    v.get("number")
                        .and_then(|n| n.as_i64())
                        .map(|n| n.to_string())
                },
                |v| {
                    v.get("updated_at")
                        .and_then(|t| t.as_str())
                        .map(|s| s.to_string())
                },
            )?;
            total_items += n;
            total_bytes += b;
            state.issues_since = advance(state.issues_since.clone(), latest);
        }

        // ── prs ────────────────────────────────────────────────
        {
            let since = state.prs_since.as_deref().and_then(parse_iso);
            let prs = match github
                .list_repo_prs(&owner, &name, "all", since, MAX_PAGES_PER_KIND)
                .await
            {
                Ok(v) => v,
                Err(e) => {
                    warn!(target: "arawn::feeds", feed = NAME, owner = %owner, name = %name,
                          error = %e, "prs fetch failed; skipping kind this tick");
                    Vec::new()
                }
            };
            let kind_dir = repo_dir.join("prs");
            let (n, b, latest) = write_batch(
                &kind_dir,
                &prs,
                |v| {
                    v.get("number")
                        .and_then(|n| n.as_i64())
                        .map(|n| n.to_string())
                },
                |v| {
                    v.get("updated_at")
                        .and_then(|t| t.as_str())
                        .map(|s| s.to_string())
                },
            )?;
            total_items += n;
            total_bytes += b;
            state.prs_since = advance(state.prs_since.clone(), latest);
        }

        // ── comments (issue + PR review) ───────────────────────
        // Fetch both endpoints; merge into one /comments dir on
        // disk. The projection layer's parser picks `kind` from
        // which URL field is present.
        {
            let since = state.comments_since.as_deref().and_then(parse_iso);
            let mut combined: Vec<Value> = Vec::new();
            match github
                .list_issue_comments(&owner, &name, since, MAX_PAGES_PER_KIND)
                .await
            {
                Ok(v) => combined.extend(v),
                Err(e) => warn!(target: "arawn::feeds", feed = NAME, owner = %owner, name = %name,
                                error = %e, "issue comments fetch failed; skipping"),
            }
            match github
                .list_pr_review_comments(&owner, &name, since, MAX_PAGES_PER_KIND)
                .await
            {
                Ok(v) => combined.extend(v),
                Err(e) => warn!(target: "arawn::feeds", feed = NAME, owner = %owner, name = %name,
                                error = %e, "pr review comments fetch failed; skipping"),
            }
            let kind_dir = repo_dir.join("comments");
            let (n, b, latest) = write_batch(
                &kind_dir,
                &combined,
                |v| v.get("id").and_then(|i| i.as_i64()).map(|n| n.to_string()),
                |v| {
                    v.get("updated_at")
                        .and_then(|t| t.as_str())
                        .map(|s| s.to_string())
                },
            )?;
            total_items += n;
            total_bytes += b;
            state.comments_since = advance(state.comments_since.clone(), latest);
        }

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
        Ok(None)
    }
}

/// Write each row of `batch` to `<kind_dir>/<id_extractor(row)>.json`
/// atomically. Returns (count, byte-total, latest_updated_iso).
fn write_batch(
    kind_dir: &Path,
    batch: &[Value],
    id_of: impl Fn(&Value) -> Option<String>,
    ts_of: impl Fn(&Value) -> Option<String>,
) -> Result<(u64, u64, Option<String>), FeedError> {
    if batch.is_empty() {
        return Ok((0, 0, None));
    }
    std::fs::create_dir_all(kind_dir).map_err(|e| {
        FeedError::Storage(format!("create {}: {e}", kind_dir.display()))
    })?;
    let mut count: u64 = 0;
    let mut bytes: u64 = 0;
    let mut latest: Option<String> = None;
    for v in batch {
        let Some(id) = id_of(v) else {
            warn!(target: "arawn::feeds", feed = NAME, "row missing id; skipping");
            continue;
        };
        let safe = sanitize(&id);
        let path = kind_dir.join(format!("{safe}.json"));
        match write_json(&path, v) {
            Ok(n) => {
                count += 1;
                bytes += n;
            }
            Err(e) => {
                warn!(target: "arawn::feeds", feed = NAME, error = %e, "skipping write");
                continue;
            }
        }
        if let Some(ts) = ts_of(v)
            && latest.as_deref().map(|x| ts.as_str() > x).unwrap_or(true)
        {
            latest = Some(ts);
        }
    }
    Ok((count, bytes, latest))
}

fn parse_iso(s: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.with_timezone(&chrono::Utc))
}

/// Pick the later of the previous cursor and the latest seen this
/// tick. String comparison works because both are RFC3339 normalized.
fn advance(prev: Option<String>, latest: Option<String>) -> Option<String> {
    match (prev, latest) {
        (None, None) => None,
        (Some(p), None) => Some(p),
        (None, Some(l)) => Some(l),
        (Some(p), Some(l)) => {
            if l.as_str() > p.as_str() {
                Some(l)
            } else {
                Some(p)
            }
        }
    }
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

#[allow(dead_code)]
fn _force_use_traits() {
    // Reference imports so removing them from this file breaks the
    // build deliberately.
    let _: Option<Box<dyn GithubFeedClient>> = None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::{FeedClients, GithubFeedClient as Gh};
    use crate::template::TemplateCtx;
    use std::sync::{Arc, Mutex};
    use tempfile::TempDir;

    /// Fake that returns canned responses per kind. Records each
    /// `since` floor passed in so cursor-advance tests can verify.
    struct FakeGithub {
        commits: Mutex<Vec<Value>>,
        issues: Mutex<Vec<Value>>,
        prs: Mutex<Vec<Value>>,
        issue_comments: Mutex<Vec<Value>>,
        pr_comments: Mutex<Vec<Value>>,
        // Recorded `since` floors per kind (in order).
        commits_since: Mutex<Vec<Option<chrono::DateTime<chrono::Utc>>>>,
        issues_since: Mutex<Vec<Option<chrono::DateTime<chrono::Utc>>>>,
        prs_since: Mutex<Vec<Option<chrono::DateTime<chrono::Utc>>>>,
        comments_since: Mutex<Vec<Option<chrono::DateTime<chrono::Utc>>>>,
        /// If true, `list_repo_issues` returns a Provider error to
        /// exercise partial-failure tolerance.
        issues_error: bool,
    }

    impl Default for FakeGithub {
        fn default() -> Self {
            Self {
                commits: Mutex::new(Vec::new()),
                issues: Mutex::new(Vec::new()),
                prs: Mutex::new(Vec::new()),
                issue_comments: Mutex::new(Vec::new()),
                pr_comments: Mutex::new(Vec::new()),
                commits_since: Mutex::new(Vec::new()),
                issues_since: Mutex::new(Vec::new()),
                prs_since: Mutex::new(Vec::new()),
                comments_since: Mutex::new(Vec::new()),
                issues_error: false,
            }
        }
    }

    #[async_trait]
    impl Gh for FakeGithub {
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
            since: Option<chrono::DateTime<chrono::Utc>>,
            _max_pages: u32,
        ) -> Result<Vec<Value>, FeedError> {
            self.commits_since.lock().unwrap().push(since);
            Ok(self.commits.lock().unwrap().clone())
        }
        async fn list_repo_issues(
            &self,
            _owner: &str,
            _repo: &str,
            _state: &str,
            since: Option<chrono::DateTime<chrono::Utc>>,
            _max_pages: u32,
        ) -> Result<Vec<Value>, FeedError> {
            self.issues_since.lock().unwrap().push(since);
            if self.issues_error {
                return Err(FeedError::Provider("simulated 4xx".into()));
            }
            Ok(self.issues.lock().unwrap().clone())
        }
        async fn list_repo_prs(
            &self,
            _owner: &str,
            _repo: &str,
            _state: &str,
            since: Option<chrono::DateTime<chrono::Utc>>,
            _max_pages: u32,
        ) -> Result<Vec<Value>, FeedError> {
            self.prs_since.lock().unwrap().push(since);
            Ok(self.prs.lock().unwrap().clone())
        }
        async fn list_issue_comments(
            &self,
            _owner: &str,
            _repo: &str,
            since: Option<chrono::DateTime<chrono::Utc>>,
            _max_pages: u32,
        ) -> Result<Vec<Value>, FeedError> {
            self.comments_since.lock().unwrap().push(since);
            Ok(self.issue_comments.lock().unwrap().clone())
        }
        async fn list_pr_review_comments(
            &self,
            _owner: &str,
            _repo: &str,
            _since: Option<chrono::DateTime<chrono::Utc>>,
            _max_pages: u32,
        ) -> Result<Vec<Value>, FeedError> {
            Ok(self.pr_comments.lock().unwrap().clone())
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
        gh: Arc<dyn Gh>,
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
        fn github(&self) -> Option<Arc<dyn Gh>> {
            Some(self.gh.clone())
        }
    }

    fn params() -> TemplateParams {
        TemplateParams(json!({"owner": "openai", "name": "codex"}))
    }

    fn commit(sha: &str, date: &str) -> Value {
        json!({
            "sha": sha,
            "html_url": format!("https://github.com/openai/codex/commit/{sha}"),
            "commit": {
                "message": format!("commit {sha}"),
                "author": {"name": "alice", "date": date}
            },
            "parents": []
        })
    }

    fn issue(number: i64, updated: &str) -> Value {
        json!({
            "number": number,
            "title": format!("issue {number}"),
            "state": "open",
            "user": {"login": "alice"},
            "body": "...",
            "html_url": format!("https://github.com/openai/codex/issues/{number}"),
            "created_at": "2026-05-10T09:00:00Z",
            "updated_at": updated
        })
    }

    fn pr(number: i64, updated: &str) -> Value {
        json!({
            "number": number,
            "title": format!("PR {number}"),
            "state": "open",
            "user": {"login":"alice"},
            "html_url": format!("https://github.com/openai/codex/pull/{number}"),
            "body": "x",
            "head": {"ref": "feature"},
            "base": {"ref": "main"},
            "created_at": "2026-05-10T09:00:00Z",
            "updated_at": updated
        })
    }

    fn comment(id: i64, updated: &str, parent_url: &str) -> Value {
        json!({
            "id": id,
            "body": format!("comment {id}"),
            "user": {"login": "alice"},
            "html_url": parent_url,
            "issue_url": parent_url,
            "created_at": "2026-05-18T10:00:00Z",
            "updated_at": updated
        })
    }

    #[test]
    fn validate_requires_owner_and_name() {
        assert!(RepoMirrorTemplate.validate(&TemplateParams::default()).is_err());
        let p = TemplateParams(json!({"owner": "x"}));
        assert!(RepoMirrorTemplate.validate(&p).is_err());
        let p = TemplateParams(json!({"owner": "", "name": "x"}));
        assert!(RepoMirrorTemplate.validate(&p).is_err());
        let p = TemplateParams(json!({"owner": "x", "name": "y"}));
        RepoMirrorTemplate.validate(&p).unwrap();
    }

    #[test]
    fn defaults_use_30min_cadence() {
        assert_eq!(RepoMirrorTemplate.defaults(&params()).cadence, "*/30 * * * *");
    }

    #[tokio::test]
    async fn writes_each_kind_under_owner_repo_layout() {
        let tmp = TempDir::new().unwrap();
        let fake = Arc::new(FakeGithub {
            commits: Mutex::new(vec![commit("aaa", "2026-05-18T10:00:00Z")]),
            issues: Mutex::new(vec![issue(1, "2026-05-18T10:00:00Z")]),
            prs: Mutex::new(vec![pr(7, "2026-05-18T10:00:00Z")]),
            issue_comments: Mutex::new(vec![comment(
                100,
                "2026-05-18T10:00:00Z",
                "https://github.com/openai/codex/issues/1",
            )]),
            pr_comments: Mutex::new(vec![]),
            ..Default::default()
        });
        let gh: Arc<dyn Gh> = fake.clone();
        let clients: Arc<dyn FeedClients> = Arc::new(WithFakeGithub { gh });
        let ctx = TemplateCtx::new(clients);
        let out = RepoMirrorTemplate
            .run(&ctx, &params(), tmp.path(), &json!({}))
            .await
            .unwrap();
        assert_eq!(out.summary.items_written, 4);
        assert_eq!(out.status, "ok");
        let base = tmp.path().join("openai/codex");
        assert!(base.join("commits/aaa.json").exists());
        assert!(base.join("issues/1.json").exists());
        assert!(base.join("prs/7.json").exists());
        assert!(base.join("comments/100.json").exists());
        // Cursors all advanced.
        let cur = out.cursor;
        assert_eq!(cur["commits_since"], "2026-05-18T10:00:00Z");
        assert_eq!(cur["issues_since"], "2026-05-18T10:00:00Z");
        assert_eq!(cur["prs_since"], "2026-05-18T10:00:00Z");
        assert_eq!(cur["comments_since"], "2026-05-18T10:00:00Z");
    }

    #[tokio::test]
    async fn one_kind_4xx_doesnt_block_others() {
        let tmp = TempDir::new().unwrap();
        let fake = Arc::new(FakeGithub {
            commits: Mutex::new(vec![commit("ccc", "2026-05-18T11:00:00Z")]),
            issues: Mutex::new(vec![issue(2, "2026-05-18T11:00:00Z")]),
            issues_error: true,
            ..Default::default()
        });
        let gh: Arc<dyn Gh> = fake.clone();
        let clients: Arc<dyn FeedClients> = Arc::new(WithFakeGithub { gh });
        let ctx = TemplateCtx::new(clients);
        let out = RepoMirrorTemplate
            .run(&ctx, &params(), tmp.path(), &json!({}))
            .await
            .unwrap();
        // commits still wrote even though issues 4xx'd.
        let base = tmp.path().join("openai/codex");
        assert!(base.join("commits/ccc.json").exists());
        assert!(!base.join("issues/2.json").exists());
        assert!(out.summary.items_written >= 1);
    }

    #[tokio::test]
    async fn empty_response_yields_no_new_items_and_preserves_cursor() {
        let tmp = TempDir::new().unwrap();
        let fake = Arc::new(FakeGithub::default());
        let gh: Arc<dyn Gh> = fake.clone();
        let clients: Arc<dyn FeedClients> = Arc::new(WithFakeGithub { gh });
        let ctx = TemplateCtx::new(clients);
        let prev = json!({
            "commits_since": "2026-05-10T00:00:00Z",
            "issues_since": "2026-05-10T00:00:00Z",
            "prs_since": "2026-05-10T00:00:00Z",
            "comments_since": "2026-05-10T00:00:00Z"
        });
        let out = RepoMirrorTemplate
            .run(&ctx, &params(), tmp.path(), &prev)
            .await
            .unwrap();
        assert_eq!(out.status, "no-new-items");
        // Cursors unchanged.
        assert_eq!(out.cursor["commits_since"], "2026-05-10T00:00:00Z");
        assert_eq!(out.cursor["issues_since"], "2026-05-10T00:00:00Z");
    }

    #[tokio::test]
    async fn passes_since_floors_from_cursor_to_client() {
        let tmp = TempDir::new().unwrap();
        let fake = Arc::new(FakeGithub::default());
        let gh: Arc<dyn Gh> = fake.clone();
        let clients: Arc<dyn FeedClients> = Arc::new(WithFakeGithub { gh: gh.clone() });
        let ctx = TemplateCtx::new(clients);
        let prev = json!({"commits_since": "2026-05-17T00:00:00Z"});
        RepoMirrorTemplate
            .run(&ctx, &params(), tmp.path(), &prev)
            .await
            .unwrap();
        let cs = fake.commits_since.lock().unwrap();
        assert_eq!(cs.len(), 1);
        let floor = cs[0].expect("commits_since passed in");
        assert_eq!(floor.to_rfc3339(), "2026-05-17T00:00:00+00:00");
    }

    #[tokio::test]
    async fn missing_clients_returns_auth_error() {
        use crate::clients::NoopClients;
        let tmp = TempDir::new().unwrap();
        let clients: Arc<dyn FeedClients> = Arc::new(NoopClients);
        let ctx = TemplateCtx::new(clients);
        let err = RepoMirrorTemplate
            .run(&ctx, &params(), tmp.path(), &json!({}))
            .await
            .unwrap_err();
        assert!(matches!(err, FeedError::Auth(_)));
    }

    #[test]
    fn advance_picks_newer_string() {
        assert_eq!(
            advance(Some("2026-05-10".into()), Some("2026-05-18".into())),
            Some("2026-05-18".into())
        );
        assert_eq!(
            advance(Some("2026-05-18".into()), Some("2026-05-10".into())),
            Some("2026-05-18".into())
        );
        assert_eq!(advance(None, Some("2026-05-10".into())), Some("2026-05-10".into()));
        assert_eq!(advance(Some("2026-05-10".into()), None), Some("2026-05-10".into()));
        assert_eq!(advance(None, None), None);
    }
}
