//! Seeder for the weekly-ceremony UAT scenario.
//!
//! Writes the state the weekly plugin's gather reads:
//!
//! - Calendar events spanning the current ISO week (Mon–Sun), one or
//!   two per day plus a couple of all-day blocks, via the
//!   `CalendarEventProjection` typed writer (projections.db).
//! - A prior weekly tablet (period_key = previous ISO week) with 2
//!   un-done `ceremony_items` so `prior_weekly_inbound` is non-empty.
//! - A prior retro tablet (period_key = 2 weeks ago) plus one
//!   `ceremony_diary` row and 2 `ceremony_patterns_detected` rows so
//!   `last_retro_excerpts` has content.
//! - 3+ rolling todos with `created_at > 7 days ago` and
//!   `done_at IS NULL`, parented to the prior weekly tablet so
//!   `rolling_todo_hot` fires.
//!
//! All inserts are idempotent (`INSERT OR IGNORE` + projection
//! UPSERT). Time-based values come from `chrono::Utc::now()` so the
//! scenario stays self-contained.

#![allow(dead_code)] // Consumed only when the weekly scenario runs.

use std::path::Path;

use chrono::{DateTime, Datelike, Duration, IsoWeek, NaiveDate, Utc};
use rusqlite::{Connection, params};

use arawn_projections::ProjectionStore;
use arawn_projections::calendar::{CalendarEventProjection, FEED_TYPE as CAL_FEED};

/// Seed weekly ceremony state under `data_dir`. Idempotent.
pub fn apply(data_dir: &Path) -> Result<WeeklySeedSummary, String> {
    let db_path = data_dir.join("arawn.db");
    let conn = Connection::open(&db_path).map_err(|e| format!("open {db_path:?}: {e}"))?;

    let now = Utc::now();
    let cur_iso = iso_week_str(now);
    let prior_iso = iso_week_str(now - Duration::weeks(1));
    let retro_iso = iso_week_str(now - Duration::weeks(2));
    let (monday, sunday) = monday_sunday(now);

    let mut summary = WeeklySeedSummary::default();

    // 1. Prior weekly tablet with 2 open inbound items.
    let prior_weekly_id = format!("weekly-{prior_iso}");
    let prior_weekly_generated = (now - Duration::days(7)).to_rfc3339();
    conn.execute(
        "INSERT OR IGNORE INTO ceremony_tablets \
         (id, kind, period_key, generated_at, status, lenses_scanned) \
         VALUES (?1, 'weekly', ?2, ?3, 'reviewed', '[\"proj-a\",\"proj-b\",\"proj-c\"]')",
        params![&prior_weekly_id, &prior_iso, &prior_weekly_generated],
    )
    .map_err(|e| format!("prior weekly tablet: {e}"))?;
    summary.prior_weekly_tablets += 1;

    let inbound_items = [
        (
            "inbound-001",
            "Follow up with vendor on proj-a contract renewal",
        ),
        ("inbound-002", "Circle back on proj-b retro action items"),
    ];
    for (idx, (id, body)) in inbound_items.iter().enumerate() {
        conn.execute(
            "INSERT OR IGNORE INTO ceremony_items \
             (id, tablet_id, section_key, ordinal, kind, body, citation_id, done_at, created_at) \
             VALUES (?1, ?2, 'inbound', ?3, 'freeform', ?4, NULL, NULL, ?5)",
            params![
                id,
                &prior_weekly_id,
                idx as i64,
                body,
                &prior_weekly_generated,
            ],
        )
        .map_err(|e| format!("inbound item: {e}"))?;
        summary.inbound_items += 1;
    }

    // 2. Prior retro tablet (2 weeks ago) + diary + 2 patterns.
    let retro_tablet_id = format!("retro-{retro_iso}");
    let retro_generated = (now - Duration::days(14)).to_rfc3339();
    conn.execute(
        "INSERT OR IGNORE INTO ceremony_tablets \
         (id, kind, period_key, generated_at, status, lenses_scanned) \
         VALUES (?1, 'retro', ?2, ?3, 'reviewed', '[\"proj-a\",\"proj-b\",\"proj-c\"]')",
        params![&retro_tablet_id, &retro_iso, &retro_generated],
    )
    .map_err(|e| format!("prior retro tablet: {e}"))?;
    summary.prior_retros += 1;

    let diary_body =
        "Felt scattered; proj-c kept getting pushed even though we called it a priority.";
    let diary_written = (now - Duration::days(12)).to_rfc3339();
    let word_count: i64 = diary_body.split_whitespace().count() as i64;
    conn.execute(
        "INSERT OR IGNORE INTO ceremony_diary \
         (tablet_id, body, written_at, word_count) VALUES (?1, ?2, ?3, ?4)",
        params![&retro_tablet_id, diary_body, &diary_written, word_count],
    )
    .map_err(|e| format!("prior diary: {e}"))?;
    summary.prior_diaries += 1;

    let patterns = [
        ("pat-weekly-001", "rollover_heat", 0.82_f64),
        ("pat-weekly-002", "lens_neglect", 0.66_f64),
    ];
    for (id, key, mag) in patterns {
        conn.execute(
            "INSERT OR IGNORE INTO ceremony_patterns_detected \
             (id, iso_week, pattern_key, magnitude, payload, surfaced_in_retro) \
             VALUES (?1, ?2, ?3, ?4, '{}', 1)",
            params![id, &retro_iso, key, mag],
        )
        .map_err(|e| format!("pattern insert: {e}"))?;
        summary.patterns += 1;
    }

    // 3. Rolling hot todos — created > 7d ago, un-done, parented to
    //    the prior weekly tablet (FK target). Weekly's gather filters
    //    on `created_at < now - 7d`.
    let created_old = (now - Duration::days(10)).to_rfc3339();
    let todos = [
        (
            "weekly-todo-001",
            "Wire the OAuth refresh job into the cron",
        ),
        ("weekly-todo-002", "Land the proj-a runbook approval"),
        (
            "weekly-todo-003",
            "Re-share the proj-c RFC with the SRE team",
        ),
        (
            "weekly-todo-004",
            "Confirm Saturday on-call coverage backfill",
        ),
    ];
    for (id, body) in todos {
        conn.execute(
            "INSERT OR IGNORE INTO todos \
             (id, body, rationale, kind, lens, created_at, due_at, done_at, archived_at, attrs) \
             VALUES (?1, ?2, NULL, 'rollover', NULL, ?3, NULL, NULL, NULL, \
                     json_object('origin_tablet_id', ?4, 'last_seen_tablet_id', ?4))",
            params![id, body, &created_old, &prior_weekly_id],
        )
        .map_err(|e| format!("hot todo: {e}"))?;
        summary.rolling_todos += 1;
    }

    // 4. Calendar events for the current ISO week — written to
    //    projections.db via the typed projection writer. ~10 events:
    //    a daily standup Mon–Fri, plus a few targeted meetings and an
    //    all-day focus block.
    let store = ProjectionStore::open(&data_dir.join("projections.db"))
        .map_err(|e| format!("open projections.db: {e}"))?;
    store
        .ensure_feed_type(CAL_FEED)
        .map_err(|e| format!("ensure {CAL_FEED}: {e}"))?;
    let feed_id = "uat-weekly-calendar";

    let mut events: Vec<CalendarEventProjection> = Vec::new();

    // Mon..Fri standups at 09:00–09:15 UTC.
    for offset in 0..5i64 {
        let date = monday + Duration::days(offset);
        if date > sunday {
            break;
        }
        let id = format!("weekly-evt-standup-{}", date);
        events.push(build_event(
            &id,
            feed_id,
            date,
            (9, 0),
            Some((9, 15)),
            false,
            "Daily standup",
            "Team standup",
            vec!["alice@acme.com", "pat@acme.com"],
        ));
    }

    // Targeted meetings scattered through the week.
    let mid_week_days: Vec<(i64, (u32, u32), (u32, u32), &str, &str, Vec<&str>)> = vec![
        (
            1,
            (11, 0),
            (12, 0),
            "Postgres cutover go/no-go",
            "Final cutover decision for proj-a.",
            vec!["dba@acme.com", "pat@acme.com"],
        ),
        (
            2,
            (14, 0),
            (15, 0),
            "proj-c RFC review",
            "Shared schema RFC walkthrough.",
            vec!["architect@acme.com", "pat@acme.com"],
        ),
        (
            3,
            (10, 0),
            (10, 30),
            "1:1 with manager",
            "Weekly 1:1.",
            vec!["manager@acme.com", "pat@acme.com"],
        ),
        (
            4,
            (16, 0),
            (16, 30),
            "On-call handoff",
            "Hand off pager + open incidents.",
            vec!["sre@acme.com", "pat@acme.com"],
        ),
    ];
    for (offset, (sh, sm), (eh, em), title, desc, attendees) in mid_week_days {
        let date = monday + Duration::days(offset);
        if date > sunday {
            continue;
        }
        let id = format!(
            "weekly-evt-{}-{}",
            title.replace(' ', "-").to_lowercase(),
            date
        );
        events.push(build_event(
            &id,
            feed_id,
            date,
            (sh, sm),
            Some((eh, em)),
            false,
            title,
            desc,
            attendees,
        ));
    }

    // All-day deep-work block on Wednesday.
    let wed = monday + Duration::days(2);
    if wed <= sunday {
        let id = format!("weekly-evt-focus-{}", wed);
        events.push(build_event(
            &id,
            feed_id,
            wed,
            (0, 0),
            None,
            true,
            "Focus day",
            "All-day focus block — proj-a runbook + RFC review.",
            vec!["pat@acme.com"],
        ));
    }

    summary.calendar_events = events.len();
    store
        .write_batch(&events)
        .map_err(|e| format!("write calendar events: {e}"))?;
    // Discard cur_iso warning by binding to _.
    let _ = cur_iso;

    Ok(summary)
}

#[derive(Debug, Default)]
pub struct WeeklySeedSummary {
    pub prior_weekly_tablets: usize,
    pub inbound_items: usize,
    pub prior_retros: usize,
    pub prior_diaries: usize,
    pub patterns: usize,
    pub rolling_todos: usize,
    pub calendar_events: usize,
}

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn iso_week_str(dt: DateTime<Utc>) -> String {
    let iso: IsoWeek = dt.iso_week();
    format!("{:04}-W{:02}", iso.year(), iso.week())
}

fn monday_sunday(dt: DateTime<Utc>) -> (NaiveDate, NaiveDate) {
    let date = dt.naive_utc().date();
    let weekday_from_monday = date.weekday().num_days_from_monday() as i64;
    let monday = date - Duration::days(weekday_from_monday);
    let sunday = monday + Duration::days(6);
    (monday, sunday)
}

fn build_event(
    id: &str,
    feed_id: &str,
    date: NaiveDate,
    start_hm: (u32, u32),
    end_hm: Option<(u32, u32)>,
    all_day: bool,
    title: &str,
    description: &str,
    attendees: Vec<&str>,
) -> CalendarEventProjection {
    let start = date
        .and_hms_opt(start_hm.0, start_hm.1, 0)
        .unwrap()
        .and_utc();
    let end = end_hm.map(|(h, m)| date.and_hms_opt(h, m, 0).unwrap().and_utc());
    CalendarEventProjection {
        id: id.to_string(),
        feed_id: feed_id.to_string(),
        source_id: id.to_string(),
        source_ts: start,
        calendar_id: Some("primary".into()),
        summary: title.to_string(),
        description: description.to_string(),
        location: None,
        start_ts: start,
        end_ts: end,
        all_day,
        organizer: Some("pat@acme.com".into()),
        attendees: attendees.into_iter().map(String::from).collect(),
        status: Some("confirmed".into()),
        recurring_event_id: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn apply_seeds_all_sections() {
        let tmp = tempdir().unwrap();
        let dd = tmp.path();
        let _ = arawn_storage::Store::open(dd).expect("migrations");

        let summary = apply(dd).expect("seed");
        assert_eq!(summary.prior_weekly_tablets, 1);
        assert_eq!(summary.inbound_items, 2);
        assert_eq!(summary.prior_retros, 1);
        assert_eq!(summary.prior_diaries, 1);
        assert_eq!(summary.patterns, 2);
        assert!(summary.rolling_todos >= 3, "{}", summary.rolling_todos);
        assert!(summary.calendar_events >= 8, "{}", summary.calendar_events);

        let conn = Connection::open(dd.join("arawn.db")).unwrap();
        let n_inbound: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ceremony_items WHERE done_at IS NULL",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(n_inbound >= 2, "open inbound: {n_inbound}");
        let n_hot: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ceremony_todos_rolling WHERE done_at IS NULL",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(n_hot >= 3, "hot todos: {n_hot}");
        let n_patterns: i64 = conn
            .query_row("SELECT COUNT(*) FROM ceremony_patterns_detected", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(n_patterns, 2);

        let store = ProjectionStore::open(&dd.join("projections.db")).unwrap();
        store.ensure_feed_type(CAL_FEED).unwrap();
        let conn = store.conn().lock().unwrap();
        let n_cal: i64 = conn
            .query_row("SELECT COUNT(*) FROM calendar_events", [], |r| r.get(0))
            .unwrap();
        assert!(n_cal >= 8, "calendar rows: {n_cal}");
    }

    #[test]
    fn apply_is_idempotent() {
        let tmp = tempdir().unwrap();
        let dd = tmp.path();
        let _ = arawn_storage::Store::open(dd).expect("migrations");
        let first = apply(dd).expect("first");
        let second = apply(dd).expect("second");
        assert_eq!(first.inbound_items, second.inbound_items);
        assert_eq!(first.rolling_todos, second.rolling_todos);
        assert_eq!(first.calendar_events, second.calendar_events);
        assert_eq!(first.patterns, second.patterns);

        let conn = Connection::open(dd.join("arawn.db")).unwrap();
        let n_todos: i64 = conn
            .query_row("SELECT COUNT(*) FROM ceremony_todos_rolling", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(n_todos as usize, first.rolling_todos);
    }
}
