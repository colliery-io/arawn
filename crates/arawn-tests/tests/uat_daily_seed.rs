//! Seeder for the daily-ceremony UAT scenario.
//!
//! Writes the full set of state the daily plugin's gather reads:
//!
//! - A placeholder daily tablet for "yesterday" (origin for rolling
//!   todos + cursor for the attention adapter).
//! - 3+ open rolling todos in `ceremony_todos_rolling`.
//! - A weekly tablet for the current ISO week with 2 confirmed
//!   priorities (so the `alignment` section has rows).
//! - 5+ calendar events for "today" written to the `calendar_events`
//!   projection table via the `CalendarEventProjection` writer (the
//!   projection store lives at `<data_dir>/projections.db`, NOT in
//!   the main `arawn.db`).
//!
//! All inserts are idempotent (`INSERT OR IGNORE` for sqlite rows,
//! the projection writer's UPSERT for projection rows). Time-based
//! values come from `chrono::Utc::now()` so the scenario stays
//! self-contained.

#![allow(dead_code)] // Consumed only when the daily scenario runs.

use std::path::Path;

use chrono::{DateTime, Datelike, Duration, IsoWeek, Utc};
use rusqlite::{Connection, params};

use arawn_projections::ProjectionStore;
use arawn_projections::calendar::{CalendarEventProjection, FEED_TYPE as CAL_FEED};

/// Seed daily ceremony state under `data_dir`. Idempotent.
pub fn apply(data_dir: &Path) -> Result<DailySeedSummary, String> {
    let db_path = data_dir.join("arawn.db");
    let conn = Connection::open(&db_path).map_err(|e| format!("open {db_path:?}: {e}"))?;

    let now = Utc::now();
    let today = now.date_naive();
    let cur_iso = iso_week_str(now);

    let mut summary = DailySeedSummary::default();

    // 1. Placeholder daily tablet for yesterday (FK target for todos
    //    + cursor anchor for the attention adapter).
    let yesterday = today - Duration::days(1);
    let origin_id = format!("daily-{}", yesterday.format("%Y-%m-%d"));
    let origin_generated = yesterday
        .and_hms_opt(7, 0, 0)
        .unwrap()
        .and_utc()
        .to_rfc3339();
    conn.execute(
        "INSERT OR IGNORE INTO ceremony_tablets \
         (id, kind, period_key, generated_at, status, workstreams_scanned) \
         VALUES (?1, 'daily', ?2, ?3, 'reviewed', '[\"proj-a\",\"proj-b\",\"proj-c\"]')",
        params![
            &origin_id,
            &yesterday.format("%Y-%m-%d").to_string(),
            &origin_generated,
        ],
    )
    .map_err(|e| format!("origin daily tablet: {e}"))?;
    summary.daily_tablets += 1;

    // 2. Open rolling todos, all parented to the origin daily tablet.
    let created_at = (now - Duration::days(2)).to_rfc3339();
    let todos = [
        ("daily-todo-001", "Review proj-a postgres cutover runbook"),
        (
            "daily-todo-002",
            "Reply to proj-b pager storm post-mortem draft",
        ),
        (
            "daily-todo-003",
            "Schedule async sync for proj-c shared schema RFC",
        ),
        (
            "daily-todo-004",
            "Confirm Saturday on-call coverage backfill",
        ),
    ];
    for (id, body) in todos {
        conn.execute(
            "INSERT OR IGNORE INTO ceremony_todos_rolling \
             (todo_id, body, origin_tablet_id, created_at, done_at, last_seen_tablet_id) \
             VALUES (?1, ?2, ?3, ?4, NULL, ?3)",
            params![id, body, &origin_id, &created_at],
        )
        .map_err(|e| format!("rolling todo insert: {e}"))?;
        summary.rolling_todos += 1;
    }

    // 3. Weekly tablet for this ISO week + 2 confirmed priorities.
    let weekly_id = format!("weekly-{cur_iso}");
    let weekly_generated = (now - Duration::days(2)).to_rfc3339();
    conn.execute(
        "INSERT OR IGNORE INTO ceremony_tablets \
         (id, kind, period_key, generated_at, status, workstreams_scanned, priorities_confirmed_at) \
         VALUES (?1, 'weekly', ?2, ?3, 'reviewed', '[\"proj-a\",\"proj-b\",\"proj-c\"]', ?3)",
        params![&weekly_id, &cur_iso, &weekly_generated],
    )
    .map_err(|e| format!("weekly tablet insert: {e}"))?;
    let priorities = [
        (
            "daily-prio-001",
            "Land the proj-a postgres migration this week",
            "Cutover is scheduled; blocker for proj-c.",
        ),
        (
            "daily-prio-002",
            "Close out the proj-b on-call cleanup",
            "Post-mortem actions still open; pager risk remains.",
        ),
    ];
    for (idx, (id, body, rationale)) in priorities.iter().enumerate() {
        conn.execute(
            "INSERT OR IGNORE INTO ceremony_priorities \
             (id, tablet_id, body, rationale, citation_id, confirmed_at, done_at, ordinal) \
             VALUES (?1, ?2, ?3, ?4, NULL, ?5, NULL, ?6)",
            params![
                id,
                &weekly_id,
                body,
                rationale,
                &weekly_generated,
                idx as i64
            ],
        )
        .map_err(|e| format!("priority insert: {e}"))?;
        summary.priorities += 1;
    }

    // 4. Calendar events for "today" — written to projections.db via
    //    the typed projection writer.
    let store = ProjectionStore::open(&data_dir.join("projections.db"))
        .map_err(|e| format!("open projections.db: {e}"))?;
    store
        .ensure_feed_type(CAL_FEED)
        .map_err(|e| format!("ensure {CAL_FEED}: {e}"))?;

    let feed_id = "uat-daily-calendar";
    let events: Vec<CalendarEventProjection> = vec![
        (
            "daily-evt-001",
            "Standup",
            9,
            0,
            9,
            15,
            vec!["alice@acme.com", "pat@acme.com"],
            "Daily team standup",
        ),
        (
            "daily-evt-002",
            "Postgres cutover go/no-go",
            11,
            0,
            12,
            0,
            vec!["dba@acme.com", "pat@acme.com"],
            "Final cutover decision for proj-a.",
        ),
        (
            "daily-evt-003",
            "1:1 with manager",
            13,
            30,
            14,
            0,
            vec!["manager@acme.com", "pat@acme.com"],
            "Weekly 1:1.",
        ),
        (
            "daily-evt-004",
            "proj-c RFC review",
            15,
            0,
            16,
            0,
            vec!["architect@acme.com", "pat@acme.com"],
            "Shared schema RFC walkthrough.",
        ),
        (
            "daily-evt-005",
            "On-call handoff",
            17,
            0,
            17,
            30,
            vec!["sre@acme.com", "pat@acme.com"],
            "Hand off pager + open incidents.",
        ),
        (
            "daily-evt-006",
            "Deep work block",
            8,
            0,
            9,
            0,
            vec!["pat@acme.com"],
            "Focus block — review proj-a runbook.",
        ),
    ]
    .into_iter()
    .map(|(id, title, sh, sm, eh, em, attendees, desc)| {
        let start = today.and_hms_opt(sh, sm, 0).unwrap().and_utc();
        let end = today.and_hms_opt(eh, em, 0).unwrap().and_utc();
        CalendarEventProjection {
            id: id.to_string(),
            feed_id: feed_id.to_string(),
            source_id: id.to_string(),
            source_ts: start,
            calendar_id: Some("primary".into()),
            summary: title.to_string(),
            description: desc.to_string(),
            location: None,
            start_ts: start,
            end_ts: Some(end),
            all_day: false,
            organizer: Some("pat@acme.com".into()),
            attendees: attendees.into_iter().map(String::from).collect(),
            status: Some("confirmed".into()),
            recurring_event_id: None,
        }
    })
    .collect();
    summary.calendar_events = events.len();
    store
        .write_batch(&events)
        .map_err(|e| format!("write calendar events: {e}"))?;

    Ok(summary)
}

#[derive(Debug, Default)]
pub struct DailySeedSummary {
    pub daily_tablets: usize,
    pub rolling_todos: usize,
    pub priorities: usize,
    pub calendar_events: usize,
}

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn iso_week_str(dt: DateTime<Utc>) -> String {
    let iso: IsoWeek = dt.iso_week();
    format!("{:04}-W{:02}", iso.year(), iso.week())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn apply_seeds_all_sections() {
        let tmp = tempdir().unwrap();
        let dd = tmp.path();
        // Initialise the ceremony schema (V6 migrations).
        let _ = arawn_storage::Store::open(dd).expect("migrations");

        let summary = apply(dd).expect("seed");
        assert!(summary.daily_tablets >= 1, "{}", summary.daily_tablets);
        assert!(summary.rolling_todos >= 3, "{}", summary.rolling_todos);
        assert_eq!(summary.priorities, 2);
        assert!(summary.calendar_events >= 5, "{}", summary.calendar_events);

        // Verify the rows landed in arawn.db.
        let conn = Connection::open(dd.join("arawn.db")).unwrap();
        let n_todos: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ceremony_todos_rolling WHERE done_at IS NULL",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(n_todos >= 3, "open todos: {n_todos}");
        let n_prio: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ceremony_priorities WHERE confirmed_at IS NOT NULL",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n_prio, 2);

        // Verify calendar rows landed in projections.db.
        let store = ProjectionStore::open(&dd.join("projections.db")).unwrap();
        store.ensure_feed_type(CAL_FEED).unwrap();
        let conn = store.conn().lock().unwrap();
        let n_cal: i64 = conn
            .query_row("SELECT COUNT(*) FROM calendar_events", [], |r| r.get(0))
            .unwrap();
        assert!(n_cal >= 5, "calendar rows: {n_cal}");
    }

    #[test]
    fn apply_is_idempotent() {
        let tmp = tempdir().unwrap();
        let dd = tmp.path();
        let _ = arawn_storage::Store::open(dd).expect("migrations");
        let first = apply(dd).expect("first");
        let second = apply(dd).expect("second");
        // Counts equal — same fixed ids on both runs.
        assert_eq!(first.rolling_todos, second.rolling_todos);
        assert_eq!(first.priorities, second.priorities);
        assert_eq!(first.calendar_events, second.calendar_events);

        // No duplicates in arawn.db.
        let conn = Connection::open(dd.join("arawn.db")).unwrap();
        let n_todos: i64 = conn
            .query_row("SELECT COUNT(*) FROM ceremony_todos_rolling", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(n_todos as usize, first.rolling_todos);
    }
}
