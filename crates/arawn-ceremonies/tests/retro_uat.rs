//! T-0291 UAT: end-to-end retro run against synthetic 4-week history.
//!
//! Drives the full chain: register the retro plugin in a real
//! `EngineDispatcher`, dispatch via `run_once`, assert tablet +
//! cited items + pattern rows + broadcast events fired, then run
//! `service.upsert_diary` and confirm the diary row + status
//! transition + `DiaryUpdated` event.
//!
//! Two scenarios:
//! - **Full history (4 weeks).** Pattern detectors with
//!   `require_history_weeks ≤ 3` (priority_completion, rollover_heat,
//!   lens_neglect) all run; the run produces ≥ 1 pattern row.
//! - **Bootstrap (no prior history).** `lens_neglect` skips;
//!   the retro still ships and the `what_happened` section
//!   populates from the daily tablets seeded for the current week.

use std::sync::Arc;

use chrono::{Datelike, Utc};
use rusqlite::{Connection, params};
use tempfile::TempDir;

use arawn_ceremonies::{
    CeremonyDispatcher, CeremonyEvent, CeremonyService, ConnHandle, DispatchOutcome,
    EngineDispatcher, PluginRegistry, RetroCeremony, event_channel, retro_v1_catalog,
};
use arawn_llm::{MockLlmClient, MockResponse};

fn open_test_db() -> (TempDir, ConnHandle) {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("test.db");
    let _db = arawn_storage::Database::open(&db_path).expect("migrations");
    drop(_db);
    let conn = Connection::open(&db_path).expect("open conn");
    (tmp, ConnHandle::new(conn))
}

/// Stable LLM response that cites a seed item id. The seeded daily
/// tablets each include an `item-…` row whose id is predictable.
fn mock_compose_response(citations: &[(&str, &str, &str)]) -> Arc<MockLlmClient> {
    // citations: (section, body, citation_id)
    let items_json = citations
        .iter()
        .map(|(section, body, cite)| {
            serde_json::json!({
                "section": section,
                "citation_id": cite,
                "body": { "text": body },
            })
        })
        .collect::<Vec<_>>();
    let text = serde_json::to_string(&items_json).unwrap();
    Arc::new(MockLlmClient::new(vec![MockResponse::text(text)]))
}

fn seed_daily_tablet(conn: &ConnHandle, id: &str, date: &str, item_id: &str, todo_body: &str) {
    let c = conn.0.lock().unwrap();
    c.execute(
        "INSERT INTO ceremony_tablets (id, kind, period_key, generated_at, status, lenses_scanned) \
         VALUES (?1, 'daily', ?2, ?3, 'reviewed', '[]')",
        params![id, date, format!("{date}T07:00:00Z")],
    )
    .unwrap();
    c.execute(
        "INSERT INTO ceremony_items (id, tablet_id, section_key, ordinal, kind, body, citation_id, created_at) \
         VALUES (?1, ?2, 'todo', 0, 'todo', ?3, NULL, ?4)",
        params![
            item_id,
            id,
            format!(r#"{{"text":"{todo_body}"}}"#),
            format!("{date}T07:30:00Z"),
        ],
    )
    .unwrap();
}

fn seed_weekly_tablet_with_priorities(
    conn: &ConnHandle,
    id: &str,
    iso_week: &str,
    priorities: &[(&str, bool, bool)],
) {
    let c = conn.0.lock().unwrap();
    c.execute(
        "INSERT INTO ceremony_tablets (id, kind, period_key, generated_at, status, lenses_scanned) \
         VALUES (?1, 'weekly', ?2, '2026-05-11T07:00:00Z', 'reviewed', '[]')",
        params![id, iso_week],
    )
    .unwrap();
    for (i, (pid, confirmed, done)) in priorities.iter().enumerate() {
        let todo_id = format!("td-{pid}");
        c.execute(
            "INSERT INTO todos (id, body, rationale, kind, lens, created_at, \
                                due_at, done_at, archived_at, attrs) \
             VALUES (?1, 'body', 'rationale', 'weekly_priority', NULL, '2026-05-11T07:00:00Z', \
                     NULL, ?2, NULL, '{}')",
            params![
                &todo_id,
                if *done {
                    Some("2026-05-15T17:00:00Z")
                } else {
                    None
                },
            ],
        )
        .unwrap();
        c.execute(
            "INSERT INTO ceremony_priorities (id, tablet_id, todo_id, confirmed_at, ordinal) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                pid,
                id,
                &todo_id,
                if *confirmed {
                    Some("2026-05-11T08:00:00Z")
                } else {
                    None
                },
                i as i64,
            ],
        )
        .unwrap();
    }
}

fn seed_rollup_row(conn: &ConnHandle, iso_week: &str, ws: &str, key: &str, val: f64) {
    let c = conn.0.lock().unwrap();
    c.execute(
        "INSERT INTO ceremony_activity_rollup (iso_week, lens, metric_key, value) \
         VALUES (?1, ?2, ?3, ?4)",
        params![iso_week, ws, key, val],
    )
    .unwrap();
}

fn seed_rolling_todo(
    conn: &ConnHandle,
    id: &str,
    created_at: &str,
    last_seen_tablet: &str,
    done: bool,
) {
    let c = conn.0.lock().unwrap();
    c.execute(
        "INSERT INTO todos (id, body, rationale, kind, lens, created_at, \
                            due_at, done_at, archived_at, attrs) \
         VALUES (?1, 'rolled-over', NULL, 'rollover', NULL, ?2, NULL, ?3, NULL, \
                 json_object('origin_tablet_id', ?4, 'last_seen_tablet_id', ?4))",
        params![
            id,
            created_at,
            if done {
                Some("2026-05-15T17:00:00Z")
            } else {
                None
            },
            last_seen_tablet,
        ],
    )
    .unwrap();
}

#[tokio::test]
async fn uat_4_week_retro_with_pattern_detection() {
    let (_tmp, conn) = open_test_db();
    // Current ISO week is whatever Utc::now() returns. Compute it
    // and seed the surrounding fixture relative to that, so the
    // test is stable across time.
    let now = Utc::now();
    let iso_now = RetroCeremony::iso_week(now);
    // Seed prior 3 weeks of rollup so lens_neglect's
    // require_history_weeks = 3 is satisfied.
    let prior_weeks: Vec<String> = (1..=3)
        .map(|i| {
            let prior = now - chrono::Duration::weeks(i);
            RetroCeremony::iso_week(prior)
        })
        .collect();
    for w in &prior_weeks {
        seed_rollup_row(&conn, w, "proj-a", "emails_sent", 5.0);
        seed_rollup_row(&conn, w, "proj-b", "meetings_attended", 2.0);
    }
    // This week: proj-a is active; proj-b is *not* (lens_neglect
    // fires for proj-b).
    seed_rollup_row(&conn, &iso_now, "proj-a", "emails_sent", 5.0);

    // Daily tablets within this week's Mon..Sun. Just one to keep
    // the fixture compact; the gather query collects whatever is
    // present.
    let monday = now - chrono::Duration::days(now.weekday().num_days_from_monday() as i64);
    let monday_str = monday.format("%Y-%m-%d").to_string();
    seed_daily_tablet(
        &conn,
        "daily-day1",
        &monday_str,
        "item-day1",
        "ship the feature",
    );

    // Weekly tablet with mixed priority completion — 1 of 3 done →
    // ratio 0.33 → priority_completion_ratio fires.
    seed_weekly_tablet_with_priorities(
        &conn,
        "weekly-now",
        &iso_now,
        &[
            ("p1", true, true),  // confirmed + done
            ("p2", true, false), // confirmed + not done
            ("p3", true, false), // confirmed + not done
        ],
    );

    // Three un-done rolling todos created before this week, last
    // seen this week → rollover_heat fires.
    for i in 0..3 {
        seed_rolling_todo(
            &conn,
            &format!("roll-{i}"),
            "2026-05-01T00:00:00Z",
            "daily-day1",
            false,
        );
    }

    // Build the retro plugin with the v1 catalog. The LLM is
    // mocked to return one composed item citing the daily item id.
    let llm = mock_compose_response(&[("what_happened", "shipped the feature", "item-day1")]);
    let plugin =
        Arc::new(RetroCeremony::new(llm, "hint:medium").with_detectors(retro_v1_catalog()));
    let reg = PluginRegistry::new();
    reg.register(plugin).unwrap();

    // Events channel: subscribe before dispatching so we see every
    // event fired during the run.
    let (tx, mut rx) = event_channel();
    let dispatcher = Arc::new(EngineDispatcher::new(conn.clone(), reg).with_events(tx.clone()));

    // === step 1: dispatch the retro ===
    let outcome = dispatcher.dispatch("retro").await.unwrap();
    let tablet_id = match outcome {
        DispatchOutcome::Generated { tablet_id } => tablet_id,
        other => panic!("expected Generated, got {other:?}"),
    };
    assert_eq!(tablet_id, format!("retro-{iso_now}"));

    // Drain events; expect at least one TabletGenerated and ≥ 1
    // PatternDetected.
    let mut saw_tablet = false;
    let mut saw_pattern = 0;
    while let Ok(event) = rx.try_recv() {
        match event {
            CeremonyEvent::TabletGenerated {
                tablet_id: ref tid, ..
            } if *tid == tablet_id => {
                saw_tablet = true;
            }
            CeremonyEvent::PatternDetected { .. } => saw_pattern += 1,
            _ => {}
        }
    }
    assert!(saw_tablet, "expected TabletGenerated event");
    assert!(
        saw_pattern >= 1,
        "expected at least one PatternDetected event, got {saw_pattern}"
    );

    // === step 2: assert tablet + items + patterns rows ===
    let c = conn.0.lock().unwrap();
    let item_count: i64 = c
        .query_row(
            "SELECT COUNT(*) FROM ceremony_items WHERE tablet_id = ?1",
            params![&tablet_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(item_count, 1, "exactly one composed item written");

    // Every composed item carries a non-NULL citation_id.
    let uncited: i64 = c
        .query_row(
            "SELECT COUNT(*) FROM ceremony_items WHERE tablet_id = ?1 AND citation_id IS NULL",
            params![&tablet_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(uncited, 0, "composed items must all carry citations");

    let pattern_count: i64 = c
        .query_row(
            "SELECT COUNT(*) FROM ceremony_patterns_detected WHERE iso_week = ?1",
            params![&iso_now],
            |row| row.get(0),
        )
        .unwrap();
    assert!(
        pattern_count >= 1,
        "expected at least one pattern row, got {pattern_count}"
    );
    drop(c);

    // === step 3: drive upsert_diary via the service ===
    let service = CeremonyService::new(conn.clone(), dispatcher.clone()).with_events(tx);
    service
        .upsert_diary(&tablet_id, "Productive but interrupted.")
        .unwrap();

    // Diary row landed; status flipped to reviewed; DiaryUpdated event fired.
    let dto = service.get_by_period("retro", &iso_now).unwrap().unwrap();
    assert_eq!(dto.status, "reviewed");
    let c = conn.0.lock().unwrap();
    let diary_body: String = c
        .query_row(
            "SELECT body FROM ceremony_diary WHERE tablet_id = ?1",
            params![&tablet_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(diary_body, "Productive but interrupted.");
    drop(c);

    // Find the DiaryUpdated event.
    let mut saw_diary = false;
    while let Ok(event) = rx.try_recv() {
        if let CeremonyEvent::DiaryUpdated { tablet_id: tid } = event {
            if tid == tablet_id {
                saw_diary = true;
                break;
            }
        }
    }
    assert!(saw_diary, "expected DiaryUpdated event");
}

#[tokio::test]
async fn uat_bootstrap_no_history_still_ships_retro() {
    // Only one week of data — lens_neglect (require_history=3)
    // is skipped by the registry. priority_completion (require=0)
    // and rollover_heat (require=0) run but quietly return empty
    // when there's no signal.
    let (_tmp, conn) = open_test_db();
    let now = Utc::now();
    let iso_now = RetroCeremony::iso_week(now);

    // Only this week's rollup — no priors.
    seed_rollup_row(&conn, &iso_now, "proj-a", "emails_sent", 5.0);

    // Daily tablet for the LLM to cite.
    let monday = now - chrono::Duration::days(now.weekday().num_days_from_monday() as i64);
    let monday_str = monday.format("%Y-%m-%d").to_string();
    seed_daily_tablet(
        &conn,
        "daily-bootstrap",
        &monday_str,
        "item-bs",
        "first week",
    );

    let llm = mock_compose_response(&[("what_happened", "shipped the first thing", "item-bs")]);
    let plugin =
        Arc::new(RetroCeremony::new(llm, "hint:medium").with_detectors(retro_v1_catalog()));
    let reg = PluginRegistry::new();
    reg.register(plugin).unwrap();
    let dispatcher = EngineDispatcher::new(conn.clone(), reg);

    let outcome = dispatcher.dispatch("retro").await.unwrap();
    assert!(matches!(outcome, DispatchOutcome::Generated { .. }));

    // No errors, no pattern rows (lens_neglect skipped;
    // others fire-empty).
    let c = conn.0.lock().unwrap();
    let pattern_count: i64 = c
        .query_row(
            "SELECT COUNT(*) FROM ceremony_patterns_detected WHERE iso_week = ?1",
            params![&iso_now],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(pattern_count, 0, "bootstrap path produces no pattern rows");

    // The composed item still landed.
    let item_count: i64 = c
        .query_row(
            "SELECT COUNT(*) FROM ceremony_items WHERE tablet_id LIKE 'retro-%'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(item_count, 1);
}
