//! Integration tests for GitHub projections (I-0045 T-0318).
//!
//! Verifies that the three new feed types round-trip through
//! `ProjectionStore` end-to-end: schema gets created lazily,
//! rows write, FTS picks up the title/body, count + get_row work.

use arawn_projections::github::{
    GithubIssueOrPrCommentProjection, GithubIssueOrPrProjection, GithubNotificationProjection,
    GithubRepoCommitProjection, GithubRepoIssueProjection, GithubRepoPrProjection,
    GithubReviewRequestProjection, ISSUES_AND_PRS_FEED_TYPE, ISSUE_OR_PR_COMMENTS_FEED_TYPE,
    NOTIFICATIONS_FEED_TYPE, REPO_COMMITS_FEED_TYPE, REPO_ISSUES_FEED_TYPE, REPO_PRS_FEED_TYPE,
    REVIEW_QUEUE_FEED_TYPE,
};
use arawn_projections::ProjectionStore;
use chrono::{TimeZone, Utc};

fn notif(id: &str, title: &str) -> GithubNotificationProjection {
    GithubNotificationProjection {
        id: format!("feed-1:{id}"),
        feed_id: "feed-1".into(),
        source_id: id.into(),
        source_ts: Utc.with_ymd_and_hms(2026, 5, 18, 10, 0, 0).unwrap(),
        owner: "arawn-dev".into(),
        repo: "arawn".into(),
        thread_id: id.into(),
        subject_title: title.into(),
        subject_url: Some(
            "https://api.github.com/repos/arawn-dev/arawn/pulls/42".into(),
        ),
        reason: "review_requested".into(),
        kind: "PullRequest".into(),
        unread: true,
        updated_at: Utc.with_ymd_and_hms(2026, 5, 18, 10, 0, 0).unwrap(),
        last_read_at: None,
    }
}

fn issue(number: i64, title: &str, body: &str) -> GithubIssueOrPrProjection {
    GithubIssueOrPrProjection {
        id: format!("feed-1:owner/repo#{number}"),
        feed_id: "feed-1".into(),
        source_id: format!("owner/repo#{number}"),
        source_ts: Utc.with_ymd_and_hms(2026, 5, 18, 10, 0, 0).unwrap(),
        owner: "owner".into(),
        repo: "repo".into(),
        number,
        kind: "issue".into(),
        title: title.into(),
        url: format!("https://github.com/owner/repo/issues/{number}"),
        state: "open".into(),
        author: "alice".into(),
        assignees: vec!["bob".into()],
        labels: vec!["bug".into()],
        body_excerpt: body.into(),
        created_at: Utc.with_ymd_and_hms(2026, 5, 10, 9, 0, 0).unwrap(),
        updated_at: Utc.with_ymd_and_hms(2026, 5, 18, 10, 0, 0).unwrap(),
        closed_at: None,
        merged_at: None,
    }
}

fn review(pr_number: i64, title: &str) -> GithubReviewRequestProjection {
    GithubReviewRequestProjection {
        id: format!("feed-1:owner/repo#{pr_number}"),
        feed_id: "feed-1".into(),
        source_id: format!("owner/repo#{pr_number}"),
        source_ts: Utc.with_ymd_and_hms(2026, 5, 18, 10, 0, 0).unwrap(),
        owner: "owner".into(),
        repo: "repo".into(),
        pr_number,
        title: title.into(),
        url: format!("https://github.com/owner/repo/pull/{pr_number}"),
        author: "alice".into(),
        requested_at: Utc.with_ymd_and_hms(2026, 5, 18, 10, 0, 0).unwrap(),
        draft: false,
    }
}

#[test]
fn notifications_write_count_get() {
    let store = ProjectionStore::in_memory().unwrap();
    store.write(&notif("n1", "Ship the GitHub integration")).unwrap();
    store.write(&notif("n2", "Wire the feed templates")).unwrap();
    assert_eq!(store.count(NOTIFICATIONS_FEED_TYPE).unwrap(), 2);
    let row = store
        .get_row(NOTIFICATIONS_FEED_TYPE, "feed-1:n1")
        .unwrap()
        .unwrap();
    assert_eq!(row.title, "Ship the GitHub integration");
    assert_eq!(row.metadata["owner"], "arawn-dev");
    assert_eq!(row.metadata["unread"], true);
}

#[test]
fn issues_write_fts_and_metadata_round_trip() {
    let store = ProjectionStore::in_memory().unwrap();
    store
        .write(&issue(
            42,
            "Reproduce the panic",
            "Steps: cargo run, hit ctrl-c, panic.",
        ))
        .unwrap();
    store
        .write(&issue(
            43,
            "Add the docs",
            "We need an integration setup page.",
        ))
        .unwrap();
    assert_eq!(store.count(ISSUES_AND_PRS_FEED_TYPE).unwrap(), 2);
    // FTS finds the body content.
    let hits = store.fts_search(ISSUES_AND_PRS_FEED_TYPE, "panic", 5).unwrap();
    assert_eq!(hits.len(), 1);
    assert!(hits[0].contains("#42"));
    // Metadata round-trips for the typed fields the morning brief needs.
    let row = store
        .get_row(ISSUES_AND_PRS_FEED_TYPE, "feed-1:owner/repo#42")
        .unwrap()
        .unwrap();
    assert_eq!(row.metadata["kind"], "issue");
    assert_eq!(row.metadata["assignees"][0], "bob");
    assert_eq!(row.metadata["labels"][0], "bug");
    assert_eq!(row.metadata["state"], "open");
}

#[test]
fn review_queue_write_and_get() {
    let store = ProjectionStore::in_memory().unwrap();
    store.write(&review(7, "Land the migration")).unwrap();
    assert_eq!(store.count(REVIEW_QUEUE_FEED_TYPE).unwrap(), 1);
    let row = store
        .get_row(REVIEW_QUEUE_FEED_TYPE, "feed-1:owner/repo#7")
        .unwrap()
        .unwrap();
    assert_eq!(row.title, "Land the migration");
    assert_eq!(row.metadata["pr_number"], 7);
    assert_eq!(row.metadata["draft"], false);
}

#[test]
fn re_writing_same_source_id_updates_in_place() {
    let store = ProjectionStore::in_memory().unwrap();
    store.write(&issue(1, "v1", "v1 body")).unwrap();
    let mut updated = issue(1, "v2", "v2 body");
    updated.state = "closed".into();
    store.write(&updated).unwrap();
    assert_eq!(store.count(ISSUES_AND_PRS_FEED_TYPE).unwrap(), 1);
    let row = store
        .get_row(ISSUES_AND_PRS_FEED_TYPE, "feed-1:owner/repo#1")
        .unwrap()
        .unwrap();
    assert_eq!(row.title, "v2");
    assert_eq!(row.metadata["state"], "closed");
}

// ────── I-0050 T-0324 — repo-mirror projection round-trips ──────

fn ts() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 5, 18, 10, 0, 0).unwrap()
}

fn commit(sha: &str) -> GithubRepoCommitProjection {
    GithubRepoCommitProjection {
        id: format!("feed-1:openai/codex@{sha}"),
        feed_id: "feed-1".into(),
        source_id: format!("openai/codex@{sha}"),
        source_ts: ts(),
        owner: "openai".into(),
        repo: "codex".into(),
        sha: sha.into(),
        message: format!("commit {sha}: do the thing"),
        author: "alice".into(),
        parents: vec!["parent".into()],
        html_url: format!("https://github.com/openai/codex/commit/{sha}"),
    }
}

fn repo_issue(n: i64) -> GithubRepoIssueProjection {
    GithubRepoIssueProjection {
        id: format!("feed-1:openai/codex#{n}"),
        feed_id: "feed-1".into(),
        source_id: format!("openai/codex#{n}"),
        source_ts: ts(),
        owner: "openai".into(),
        repo: "codex".into(),
        number: n,
        title: format!("issue {n}"),
        state: "open".into(),
        labels: vec!["bug".into()],
        body_excerpt: "details".into(),
        author: "alice".into(),
        assignees: vec!["bob".into()],
        url: format!("https://github.com/openai/codex/issues/{n}"),
        created_at: ts(),
        updated_at: ts(),
        closed_at: None,
    }
}

fn repo_pr(n: i64) -> GithubRepoPrProjection {
    GithubRepoPrProjection {
        id: format!("feed-1:openai/codex#{n}"),
        feed_id: "feed-1".into(),
        source_id: format!("openai/codex#{n}"),
        source_ts: ts(),
        owner: "openai".into(),
        repo: "codex".into(),
        number: n,
        title: format!("PR {n}"),
        state: "open".into(),
        labels: vec![],
        body_excerpt: "PR body".into(),
        author: "alice".into(),
        head_ref: "feature".into(),
        base_ref: "main".into(),
        requested_reviewers: vec!["reviewer".into()],
        draft: false,
        url: format!("https://github.com/openai/codex/pull/{n}"),
        created_at: ts(),
        updated_at: ts(),
        closed_at: None,
        merged_at: None,
    }
}

fn comment(id_n: i64, parent: i64, kind: &str) -> GithubIssueOrPrCommentProjection {
    GithubIssueOrPrCommentProjection {
        id: format!("feed-1:openai/codex@{kind}/{id_n}"),
        feed_id: "feed-1".into(),
        source_id: format!("openai/codex@{kind}/{id_n}"),
        source_ts: ts(),
        owner: "openai".into(),
        repo: "codex".into(),
        parent_number: parent,
        body_excerpt: format!("comment {id_n} body"),
        author: "alice".into(),
        url: format!(
            "https://github.com/openai/codex/issues/{parent}#issuecomment-{id_n}"
        ),
        kind: kind.into(),
        created_at: ts(),
        updated_at: ts(),
    }
}

#[test]
fn repo_commits_round_trip() {
    let store = ProjectionStore::in_memory().unwrap();
    store.write(&commit("aaa")).unwrap();
    store.write(&commit("bbb")).unwrap();
    assert_eq!(store.count(REPO_COMMITS_FEED_TYPE).unwrap(), 2);
    let row = store
        .get_row(REPO_COMMITS_FEED_TYPE, "feed-1:openai/codex@aaa")
        .unwrap()
        .unwrap();
    assert_eq!(row.title, "commit aaa: do the thing");
    assert_eq!(row.metadata["owner"], "openai");
    assert_eq!(row.metadata["parents"][0], "parent");
}

#[test]
fn repo_issues_round_trip_with_fts_hit() {
    let store = ProjectionStore::in_memory().unwrap();
    store.write(&repo_issue(1)).unwrap();
    store.write(&repo_issue(2)).unwrap();
    assert_eq!(store.count(REPO_ISSUES_FEED_TYPE).unwrap(), 2);
    let hits = store.fts_search(REPO_ISSUES_FEED_TYPE, "details", 5).unwrap();
    assert_eq!(hits.len(), 2);
}

#[test]
fn repo_prs_round_trip_preserves_head_base() {
    let store = ProjectionStore::in_memory().unwrap();
    store.write(&repo_pr(7)).unwrap();
    let row = store
        .get_row(REPO_PRS_FEED_TYPE, "feed-1:openai/codex#7")
        .unwrap()
        .unwrap();
    assert_eq!(row.metadata["head_ref"], "feature");
    assert_eq!(row.metadata["base_ref"], "main");
    assert_eq!(row.metadata["requested_reviewers"][0], "reviewer");
}

#[test]
fn comments_round_trip_with_kind_discriminator() {
    let store = ProjectionStore::in_memory().unwrap();
    store.write(&comment(101, 1, "issue_comment")).unwrap();
    store.write(&comment(202, 7, "pr_review_comment")).unwrap();
    assert_eq!(store.count(ISSUE_OR_PR_COMMENTS_FEED_TYPE).unwrap(), 2);
    let row = store
        .get_row(
            ISSUE_OR_PR_COMMENTS_FEED_TYPE,
            "feed-1:openai/codex@issue_comment/101",
        )
        .unwrap()
        .unwrap();
    assert_eq!(row.metadata["kind"], "issue_comment");
    assert_eq!(row.metadata["parent_number"], 1);
    let row = store
        .get_row(
            ISSUE_OR_PR_COMMENTS_FEED_TYPE,
            "feed-1:openai/codex@pr_review_comment/202",
        )
        .unwrap()
        .unwrap();
    assert_eq!(row.metadata["kind"], "pr_review_comment");
}
