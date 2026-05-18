//! Integration tests for GitHub projections (I-0045 T-0318).
//!
//! Verifies that the three new feed types round-trip through
//! `ProjectionStore` end-to-end: schema gets created lazily,
//! rows write, FTS picks up the title/body, count + get_row work.

use arawn_projections::github::{
    GithubIssueOrPrProjection, GithubNotificationProjection, GithubReviewRequestProjection,
    ISSUES_AND_PRS_FEED_TYPE, NOTIFICATIONS_FEED_TYPE, REVIEW_QUEUE_FEED_TYPE,
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
