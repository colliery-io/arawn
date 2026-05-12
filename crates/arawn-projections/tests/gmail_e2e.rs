//! End-to-end projection flow: walk a fixture gmail feed dir, write
//! projections, search via FTS, re-run and confirm idempotency.

use arawn_projections::gmail::{self, GmailMessageProjection, FEED_TYPE};
use arawn_projections::ProjectionStore;
use serde_json::json;

fn write_msg(dir: &std::path::Path, day: &str, id: &str, msg: serde_json::Value) {
    let day_dir = dir.join(day);
    std::fs::create_dir_all(&day_dir).unwrap();
    std::fs::write(day_dir.join(format!("{id}.json")), msg.to_string()).unwrap();
}

fn fixture_msg(id: &str, internal_date_ms: i64, subject: &str, body: &str) -> serde_json::Value {
    json!({
        "id": id,
        "threadId": format!("t-{id}"),
        "internalDate": internal_date_ms.to_string(),
        "labelIds": ["INBOX"],
        "snippet": body,
        "payload": {
            "headers": [
                {"name": "From", "value": "alice@example.com"},
                {"name": "To", "value": "bob@example.com"},
                {"name": "Subject", "value": subject},
            ],
            "mimeType": "text/plain",
            "body": { "data": "" }
        }
    })
}

#[test]
fn end_to_end_walk_write_search() {
    let feed_dir = tempfile::tempdir().unwrap();
    write_msg(
        feed_dir.path(),
        "2026-05-10",
        "m1",
        fixture_msg("m1", 1_700_000_000_000, "Quarterly planning", "Discussion of Q3 targets"),
    );
    write_msg(
        feed_dir.path(),
        "2026-05-11",
        "m2",
        fixture_msg("m2", 1_700_086_400_000, "Lunch?", "Want to grab tacos at noon"),
    );

    let store = ProjectionStore::in_memory().unwrap();
    let parsed = gmail::walk_feed_dir("gmail-inbox", feed_dir.path()).unwrap();
    assert_eq!(parsed.len(), 2);
    let out = store.write_batch(&parsed).unwrap();
    assert_eq!(out.inserted, 2);
    assert_eq!(out.updated, 0);

    assert_eq!(store.count(FEED_TYPE).unwrap(), 2);

    // FTS finds the planning message but not the lunch one.
    let ids = store.fts_search(FEED_TYPE, "Quarterly", 5).unwrap();
    assert_eq!(ids.len(), 1);
    let row = store.get_row(FEED_TYPE, &ids[0]).unwrap().unwrap();
    assert_eq!(row.title, "Quarterly planning");
    assert_eq!(row.feed_id, "gmail-inbox");
    assert!(row.metadata.get("sender").and_then(|v| v.as_str()).is_some());

    // FTS finds the lunch message.
    let ids = store.fts_search(FEED_TYPE, "tacos", 5).unwrap();
    assert_eq!(ids.len(), 1);
    let row = store.get_row(FEED_TYPE, &ids[0]).unwrap().unwrap();
    assert_eq!(row.title, "Lunch?");
}

#[test]
fn rerun_is_idempotent() {
    let feed_dir = tempfile::tempdir().unwrap();
    write_msg(
        feed_dir.path(),
        "2026-05-10",
        "m1",
        fixture_msg("m1", 1_700_000_000_000, "Hello", "First copy"),
    );

    let store = ProjectionStore::in_memory().unwrap();
    let first = gmail::walk_feed_dir("gmail-inbox", feed_dir.path()).unwrap();
    let out1 = store.write_batch(&first).unwrap();
    assert_eq!(out1.inserted, 1);

    // Second walk produces the same parsed item; writer reports
    // "unchanged" (same body hash → metadata refresh only).
    let second = gmail::walk_feed_dir("gmail-inbox", feed_dir.path()).unwrap();
    let out2 = store.write_batch(&second).unwrap();
    assert_eq!(out2.unchanged, 1);
    assert_eq!(out2.inserted, 0);

    assert_eq!(store.count(FEED_TYPE).unwrap(), 1);
}

#[test]
fn body_change_updates_and_refreshes_fts() {
    let store = ProjectionStore::in_memory().unwrap();
    let mut p = GmailMessageProjection {
        id: gmail::projection_id("g", "m1"),
        feed_id: "g".into(),
        source_id: "m1".into(),
        source_ts: chrono::Utc::now(),
        sender: Some("alice@example.com".into()),
        recipients: vec!["bob@example.com".into()],
        subject: "Sprint kickoff".into(),
        body_text: "Original body about projects".into(),
        thread_id: Some("t1".into()),
        labels: vec!["INBOX".into()],
    };
    let out = store.write(&p).unwrap();
    assert_eq!(out.inserted, 1);

    // Update body — should refresh FTS so old terms drop out.
    p.body_text = "Replaced body about goldfinches".into();
    let out = store.write(&p).unwrap();
    assert_eq!(out.updated, 1);
    assert_eq!(out.inserted, 0);

    let old_hits = store.fts_search(FEED_TYPE, "projects", 5).unwrap();
    assert!(old_hits.is_empty(), "old FTS terms should be gone");
    let new_hits = store.fts_search(FEED_TYPE, "goldfinches", 5).unwrap();
    assert_eq!(new_hits.len(), 1);
}

#[test]
fn missing_source_ids_returns_unprojected() {
    let store = ProjectionStore::in_memory().unwrap();
    let p = GmailMessageProjection {
        id: gmail::projection_id("g", "m1"),
        feed_id: "g".into(),
        source_id: "m1".into(),
        source_ts: chrono::Utc::now(),
        sender: None,
        recipients: vec![],
        subject: "x".into(),
        body_text: "x".into(),
        thread_id: None,
        labels: vec![],
    };
    store.write(&p).unwrap();

    let missing = store
        .missing_source_ids(FEED_TYPE, "g", &[
            "m1".into(),
            "m2".into(),
            "m3".into(),
        ])
        .unwrap();
    assert_eq!(missing, vec!["m2".to_string(), "m3".to_string()]);
}

#[test]
fn rerun_after_partial_failure_picks_up_missing() {
    // Simulates the backfill scenario: a previous run projected 5
    // messages but missed 2 (process killed). The next run walks the
    // mirror, asks the store which ones are missing, and projects only
    // those.
    let feed_dir = tempfile::tempdir().unwrap();
    for i in 1..=5 {
        write_msg(
            feed_dir.path(),
            "2026-05-10",
            &format!("m{i}"),
            fixture_msg(&format!("m{i}"), 1_700_000_000_000 + i, "subj", "body"),
        );
    }
    let store = ProjectionStore::in_memory().unwrap();
    // Project the first 3.
    let parsed = gmail::walk_feed_dir("g", feed_dir.path()).unwrap();
    let first_three: Vec<_> = parsed.iter().take(3).cloned().collect();
    store.write_batch(&first_three).unwrap();
    assert_eq!(store.count(FEED_TYPE).unwrap(), 3);

    // Backfill-style: walk all, ask store which are missing.
    let parsed_now = gmail::walk_feed_dir("g", feed_dir.path()).unwrap();
    let candidate_ids: Vec<String> = parsed_now.iter().map(|p| p.source_id.clone()).collect();
    let missing = store
        .missing_source_ids(FEED_TYPE, "g", &candidate_ids)
        .unwrap();
    let to_project: Vec<_> = parsed_now
        .into_iter()
        .filter(|p| missing.contains(&p.source_id))
        .collect();
    let out = store.write_batch(&to_project).unwrap();
    assert_eq!(out.inserted, 2);
    assert_eq!(store.count(FEED_TYPE).unwrap(), 5);
}
