//! I-0050 T-0328 — end-to-end smoke for the `github/repo-mirror`
//! feed template + the new projection dispatch.
//!
//! Pure unit test: a `FakeGithub` implementing [`GithubFeedClient`]
//! returns canned commits, issues, PRs, and comments. The template
//! writes them to a tempdir feed_dir; `project_feed_dir` then walks
//! the dir and pushes typed rows into an in-memory `ProjectionStore`.
//! Asserts:
//!
//! - All four kinds round-trip (counts > 0).
//! - The dispatch arm reads the nested `<owner>/<repo>/<kind>` layout
//!   correctly.
//! - Cursor advances across the tick.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::{Value, json};
use tempfile::TempDir;

use arawn_feeds::clients::{
    AtlassianFeedClient, CalendarFeedClient, DriveFeedClient, FeedClients, GithubFeedClient,
    GmailFeedClient, SlackFeedClient,
};
use arawn_feeds::template::{FeedTemplate, TemplateCtx};
use arawn_feeds::templates::github::RepoMirrorTemplate;
use arawn_feeds::types::TemplateParams;
use arawn_projections::ProjectionStore;
use arawn_projections::github::{
    ISSUE_OR_PR_COMMENTS_FEED_TYPE, REPO_COMMITS_FEED_TYPE, REPO_ISSUES_FEED_TYPE,
    REPO_PRS_FEED_TYPE,
};

#[derive(Default)]
struct FakeGithub {
    commits: Mutex<Vec<Value>>,
    issues: Mutex<Vec<Value>>,
    prs: Mutex<Vec<Value>>,
    issue_comments: Mutex<Vec<Value>>,
    pr_comments: Mutex<Vec<Value>>,
}

#[async_trait]
impl GithubFeedClient for FakeGithub {
    async fn list_notifications(
        &self,
        _since: Option<chrono::DateTime<chrono::Utc>>,
        _per_page: u32,
        _all: bool,
    ) -> Result<Vec<Value>, arawn_feeds::FeedError> {
        Ok(Vec::new())
    }
    async fn search_issues(
        &self,
        _query: &str,
        _per_page: u32,
        _max_pages: u32,
    ) -> Result<Vec<Value>, arawn_feeds::FeedError> {
        Ok(Vec::new())
    }
    async fn list_repo_commits(
        &self,
        _owner: &str,
        _repo: &str,
        _since: Option<chrono::DateTime<chrono::Utc>>,
        _max_pages: u32,
    ) -> Result<Vec<Value>, arawn_feeds::FeedError> {
        Ok(self.commits.lock().unwrap().clone())
    }
    async fn list_repo_issues(
        &self,
        _owner: &str,
        _repo: &str,
        _state: &str,
        _since: Option<chrono::DateTime<chrono::Utc>>,
        _max_pages: u32,
    ) -> Result<Vec<Value>, arawn_feeds::FeedError> {
        Ok(self.issues.lock().unwrap().clone())
    }
    async fn list_repo_prs(
        &self,
        _owner: &str,
        _repo: &str,
        _state: &str,
        _since: Option<chrono::DateTime<chrono::Utc>>,
        _max_pages: u32,
    ) -> Result<Vec<Value>, arawn_feeds::FeedError> {
        Ok(self.prs.lock().unwrap().clone())
    }
    async fn list_issue_comments(
        &self,
        _owner: &str,
        _repo: &str,
        _since: Option<chrono::DateTime<chrono::Utc>>,
        _max_pages: u32,
    ) -> Result<Vec<Value>, arawn_feeds::FeedError> {
        Ok(self.issue_comments.lock().unwrap().clone())
    }
    async fn list_pr_review_comments(
        &self,
        _owner: &str,
        _repo: &str,
        _since: Option<chrono::DateTime<chrono::Utc>>,
        _max_pages: u32,
    ) -> Result<Vec<Value>, arawn_feeds::FeedError> {
        Ok(self.pr_comments.lock().unwrap().clone())
    }
    async fn list_org_repos(
        &self,
        _owner: &str,
        _max_pages: u32,
    ) -> Result<Vec<Value>, arawn_feeds::FeedError> {
        Ok(Vec::new())
    }
}

struct WithFakeGithub {
    gh: Arc<dyn GithubFeedClient>,
}
impl FeedClients for WithFakeGithub {
    fn slack(&self) -> Option<Arc<dyn SlackFeedClient>> {
        None
    }
    fn calendar(&self) -> Option<Arc<dyn CalendarFeedClient>> {
        None
    }
    fn gmail(&self) -> Option<Arc<dyn GmailFeedClient>> {
        None
    }
    fn drive(&self) -> Option<Arc<dyn DriveFeedClient>> {
        None
    }
    fn atlassian(&self) -> Option<Arc<dyn AtlassianFeedClient>> {
        None
    }
    fn github(&self) -> Option<Arc<dyn GithubFeedClient>> {
        Some(self.gh.clone())
    }
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

fn issue(number: i64) -> Value {
    json!({
        "number": number,
        "title": format!("issue {number}"),
        "state": "open",
        "user": {"login": "alice"},
        "body": "panic when foo bar",
        "html_url": format!("https://github.com/openai/codex/issues/{number}"),
        "created_at": "2026-05-10T09:00:00Z",
        "updated_at": "2026-05-18T10:00:00Z"
    })
}

fn pr(number: i64) -> Value {
    json!({
        "number": number,
        "title": format!("PR {number}"),
        "state": "open",
        "user": {"login": "alice"},
        "html_url": format!("https://github.com/openai/codex/pull/{number}"),
        "body": "ship the thing",
        "head": {"ref": "feature"},
        "base": {"ref": "main"},
        "created_at": "2026-05-10T09:00:00Z",
        "updated_at": "2026-05-18T10:00:00Z"
    })
}

fn issue_comment(id: i64, parent: i64) -> Value {
    json!({
        "id": id,
        "body": format!("comment {id}"),
        "user": {"login": "alice"},
        "html_url": format!("https://github.com/openai/codex/issues/{parent}#issuecomment-{id}"),
        "issue_url": format!("https://api.github.com/repos/openai/codex/issues/{parent}"),
        "created_at": "2026-05-18T10:00:00Z",
        "updated_at": "2026-05-18T10:00:00Z"
    })
}

#[tokio::test]
async fn full_repo_mirror_round_trips_through_projection_store() {
    let tmp = TempDir::new().unwrap();
    let feed_dir = tmp.path();
    let fake = Arc::new(FakeGithub {
        commits: Mutex::new(vec![
            commit("aaa", "2026-05-18T10:00:00Z"),
            commit("bbb", "2026-05-18T11:00:00Z"),
        ]),
        issues: Mutex::new(vec![issue(1), issue(2)]),
        prs: Mutex::new(vec![pr(7)]),
        issue_comments: Mutex::new(vec![issue_comment(100, 1), issue_comment(101, 1)]),
        pr_comments: Mutex::new(vec![]),
    });
    let gh: Arc<dyn GithubFeedClient> = fake.clone();
    let clients: Arc<dyn FeedClients> = Arc::new(WithFakeGithub { gh });
    let ctx = TemplateCtx::new(clients);
    let params = TemplateParams(json!({"owner": "openai", "name": "codex"}));

    // ── Step 1: template tick writes JSON files to feed_dir ──
    let outcome = RepoMirrorTemplate
        .run(&ctx, &params, feed_dir, &json!({}))
        .await
        .unwrap();
    assert_eq!(outcome.status, "ok");
    // 2 commits + 2 issues + 1 pr + 2 comments = 7 rows.
    assert_eq!(outcome.summary.items_written, 7);

    let base = feed_dir.join("openai/codex");
    assert!(base.join("commits/aaa.json").exists());
    assert!(base.join("commits/bbb.json").exists());
    assert!(base.join("issues/1.json").exists());
    assert!(base.join("issues/2.json").exists());
    assert!(base.join("prs/7.json").exists());
    assert!(base.join("comments/100.json").exists());
    assert!(base.join("comments/101.json").exists());

    // Cursors advanced.
    let cur = &outcome.cursor;
    assert_eq!(cur["commits_since"], "2026-05-18T11:00:00Z");
    assert_eq!(cur["issues_since"], "2026-05-18T10:00:00Z");
    assert_eq!(cur["prs_since"], "2026-05-18T10:00:00Z");
    assert_eq!(cur["comments_since"], "2026-05-18T10:00:00Z");

    // ── Step 2: dispatch pass walks the feed_dir into projections ──
    let store = ProjectionStore::in_memory().unwrap();
    arawn_projections::project_feed_dir(
        &store,
        "github/repo-mirror",
        "github-repo:openai/codex",
        feed_dir,
    )
    .unwrap();

    assert_eq!(store.count(REPO_COMMITS_FEED_TYPE).unwrap(), 2);
    assert_eq!(store.count(REPO_ISSUES_FEED_TYPE).unwrap(), 2);
    assert_eq!(store.count(REPO_PRS_FEED_TYPE).unwrap(), 1);
    assert_eq!(store.count(ISSUE_OR_PR_COMMENTS_FEED_TYPE).unwrap(), 2);

    // FTS hits on body content prove the projection picked up the
    // bodies, not just the titles.
    let hits = store
        .fts_search(REPO_ISSUES_FEED_TYPE, "panic", 5)
        .unwrap();
    assert_eq!(hits.len(), 2, "both issues should match `panic`");
    let hits = store.fts_search(REPO_PRS_FEED_TYPE, "ship", 5).unwrap();
    assert_eq!(hits.len(), 1);

    // Metadata round-trip on one commit row.
    let row = store
        .get_row(
            REPO_COMMITS_FEED_TYPE,
            "github-repo:openai/codex:openai/codex@aaa",
        )
        .unwrap()
        .unwrap();
    assert_eq!(row.metadata["owner"], "openai");
    assert_eq!(row.metadata["repo"], "codex");
    assert_eq!(row.metadata["sha"], "aaa");

    // Comment kind derivation — both should be `issue_comment`.
    let row = store
        .get_row(
            ISSUE_OR_PR_COMMENTS_FEED_TYPE,
            "github-repo:openai/codex:openai/codex@issue_comment/100",
        )
        .unwrap()
        .unwrap();
    assert_eq!(row.metadata["kind"], "issue_comment");
    assert_eq!(row.metadata["parent_number"], 1);
}

#[tokio::test]
async fn second_tick_with_no_new_data_yields_no_new_items() {
    let tmp = TempDir::new().unwrap();
    let feed_dir = tmp.path();
    // First tick has data.
    let fake = Arc::new(FakeGithub {
        commits: Mutex::new(vec![commit("aaa", "2026-05-18T10:00:00Z")]),
        ..Default::default()
    });
    let gh: Arc<dyn GithubFeedClient> = fake.clone();
    let clients: Arc<dyn FeedClients> = Arc::new(WithFakeGithub { gh });
    let ctx = TemplateCtx::new(clients);
    let params = TemplateParams(json!({"owner": "openai", "name": "codex"}));
    let first = RepoMirrorTemplate
        .run(&ctx, &params, feed_dir, &json!({}))
        .await
        .unwrap();
    assert_eq!(first.status, "ok");
    // Drain the fake so the second tick sees nothing new.
    fake.commits.lock().unwrap().clear();
    let second = RepoMirrorTemplate
        .run(&ctx, &params, feed_dir, &first.cursor)
        .await
        .unwrap();
    assert_eq!(second.status, "no-new-items");
    // Cursor preserved from first tick.
    assert_eq!(
        second.cursor["commits_since"],
        first.cursor["commits_since"]
    );
}

/// Sanity: dispatch with an empty feed_dir is a clean no-op, not an
/// error. Protects against the dispatch arm assuming every kind dir
/// exists.
#[test]
fn dispatch_with_empty_feed_dir_is_a_noop() {
    let tmp = TempDir::new().unwrap();
    let store = ProjectionStore::in_memory().unwrap();
    let outcome = arawn_projections::project_feed_dir(
        &store,
        "github/repo-mirror",
        "github-repo:openai/codex",
        tmp.path(),
    )
    .unwrap();
    assert_eq!(outcome.inserted, 0);
    assert_eq!(outcome.updated, 0);
    // None of the tables get rows.
    assert_eq!(store.count(REPO_COMMITS_FEED_TYPE).unwrap(), 0);
    assert_eq!(store.count(REPO_ISSUES_FEED_TYPE).unwrap(), 0);
    assert_eq!(store.count(REPO_PRS_FEED_TYPE).unwrap(), 0);
    assert_eq!(store.count(ISSUE_OR_PR_COMMENTS_FEED_TYPE).unwrap(), 0);
}
