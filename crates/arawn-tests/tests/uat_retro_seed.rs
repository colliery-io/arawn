//! Seeder for the retro-ceremony UAT scenario.
//!
//! Writes the full set of ceremony tables (tablets, priorities,
//! rollup, todos, prior retro diary) needed for all three v1
//! detectors to fire on a real LLM-judged retro run. Time-based
//! values are computed from `chrono::Utc::now()` so the scenario is
//! self-contained: no fixture dates to keep in sync with the
//! present, and the seed always produces a "this week" that lines
//! up with whatever `RetroCeremony::iso_week(Utc::now())` returns.
//!
//! The seeder opens its own rusqlite connection to `arawn.db` — the
//! database lives under the scenario's data dir and was migrated
//! by `Store::open` during fixture apply.

#![allow(dead_code)] // Consumed only when retro scenario runs.

use std::path::Path;

use chrono::{DateTime, Datelike, Duration, IsoWeek, NaiveDate, Utc};
use rusqlite::{Connection, params};

/// Seed ceremony state for the retro UAT scenario rooted at
/// `data_dir`. Idempotent — re-running is a no-op if the rows
/// already exist (uses `INSERT OR IGNORE`).
pub fn apply(data_dir: &Path) -> Result<SeedSummary, String> {
    let db_path = data_dir.join("arawn.db");
    let conn = Connection::open(&db_path).map_err(|e| format!("open {db_path:?}: {e}"))?;

    let now = Utc::now();
    let cur_iso = iso_week_str(now);
    let (monday, sunday) = monday_sunday(now);

    let prior_weeks = (1..=3)
        .map(|n| now - Duration::weeks(n))
        .collect::<Vec<_>>();

    let mut summary = SeedSummary::default();

    seed_workstream_rollup(&conn, &cur_iso, &prior_weeks, &mut summary)?;
    seed_weekly_tablet_with_priorities(&conn, &cur_iso, &mut summary)?;
    seed_daily_tablets_and_todos(&conn, &cur_iso, monday, sunday, &mut summary)?;
    seed_prior_retro_with_diary(&conn, &prior_weeks[1], &mut summary)?;

    Ok(summary)
}

#[derive(Debug, Default)]
pub struct SeedSummary {
    pub rollup_rows: usize,
    pub daily_tablets: usize,
    pub rolling_todos: usize,
    pub priorities: usize,
    pub prior_retros: usize,
}

// ─────────────────────────────────────────────────────────────────────────
// Section seeders
// ─────────────────────────────────────────────────────────────────────────

/// Three workstreams across 3 prior weeks + current week.
/// `proj-c` produces no rollup in the current week → triggers
/// `WorkstreamNeglectDetector`.
fn seed_workstream_rollup(
    conn: &Connection,
    cur_iso: &str,
    prior_weeks: &[DateTime<Utc>],
    sum: &mut SeedSummary,
) -> Result<(), String> {
    // Prior weeks: all three workstreams active.
    for dt in prior_weeks {
        let iso = iso_week_str(*dt);
        for ws in ["proj-a", "proj-b", "proj-c"] {
            for (metric, value) in [("emails_sent", 8.0), ("slack_threads_participated", 6.0)] {
                conn.execute(
                    "INSERT OR IGNORE INTO ceremony_activity_rollup \
                     (iso_week, workstream, metric_key, value) VALUES (?1, ?2, ?3, ?4)",
                    params![&iso, ws, metric, value],
                )
                .map_err(|e| format!("rollup insert prior: {e}"))?;
                sum.rollup_rows += 1;
            }
        }
    }
    // Current week: proj-a + proj-b active; proj-c absent.
    for ws in ["proj-a", "proj-b"] {
        for (metric, value) in [
            ("emails_sent", 11.0),
            ("slack_threads_participated", 5.0),
            ("deep_work_hours", 7.5),
        ] {
            conn.execute(
                "INSERT OR IGNORE INTO ceremony_activity_rollup \
                 (iso_week, workstream, metric_key, value) VALUES (?1, ?2, ?3, ?4)",
                params![cur_iso, ws, metric, value],
            )
            .map_err(|e| format!("rollup insert current: {e}"))?;
            sum.rollup_rows += 1;
        }
    }
    Ok(())
}

/// Weekly tablet for the current week with 3 confirmed priorities,
/// none marked done → `PriorityCompletionDetector` fires (ratio 0/3).
fn seed_weekly_tablet_with_priorities(
    conn: &Connection,
    cur_iso: &str,
    sum: &mut SeedSummary,
) -> Result<(), String> {
    let weekly_id = format!("weekly-{cur_iso}");
    let generated = (Utc::now() - Duration::days(4)).to_rfc3339();
    let confirmed_at = (Utc::now() - Duration::days(4)).to_rfc3339();
    conn.execute(
        "INSERT OR IGNORE INTO ceremony_tablets \
         (id, kind, period_key, generated_at, status, workstreams_scanned, priorities_confirmed_at) \
         VALUES (?1, 'weekly', ?2, ?3, 'reviewed', '[\"proj-a\",\"proj-b\",\"proj-c\"]', ?4)",
        params![&weekly_id, cur_iso, &generated, &confirmed_at],
    )
    .map_err(|e| format!("weekly tablet insert: {e}"))?;

    let priorities = [
        (
            "prio-001",
            "Ship the proj-a postgres migration",
            "Carried over from last week.",
        ),
        (
            "prio-002",
            "Close the proj-b on-call rotation cleanup",
            "Six rollover todos in this area.",
        ),
        (
            "prio-003",
            "Pair with the proj-c team on the shared schema",
            "Calendar said Tuesday.",
        ),
    ];
    for (idx, (id, body, rationale)) in priorities.iter().enumerate() {
        conn.execute(
            "INSERT OR IGNORE INTO ceremony_priorities \
             (id, tablet_id, body, rationale, citation_id, confirmed_at, done_at, ordinal) \
             VALUES (?1, ?2, ?3, ?4, NULL, ?5, NULL, ?6)",
            params![id, &weekly_id, body, rationale, &confirmed_at, idx as i64],
        )
        .map_err(|e| format!("priority insert: {e}"))?;
        sum.priorities += 1;
    }
    Ok(())
}

/// Five daily tablets across the current week (Mon–Fri) plus three
/// rolling todos whose `created_at` is in the prior week and
/// `last_seen_tablet_id` points to a daily this week → fires
/// `RolloverHeatDetector` (count ≥ 3).
fn seed_daily_tablets_and_todos(
    conn: &Connection,
    _cur_iso: &str,
    monday: NaiveDate,
    sunday: NaiveDate,
    sum: &mut SeedSummary,
) -> Result<(), String> {
    let mut daily_ids: Vec<String> = Vec::new();
    for offset in 0..5 {
        let date = monday + Duration::days(offset);
        if date > sunday {
            break;
        }
        let id = format!("daily-{}", date.format("%Y-%m-%d"));
        let generated = date.and_hms_opt(7, 0, 0).unwrap().and_utc().to_rfc3339();
        conn.execute(
            "INSERT OR IGNORE INTO ceremony_tablets \
             (id, kind, period_key, generated_at, status, workstreams_scanned) \
             VALUES (?1, 'daily', ?2, ?3, 'reviewed', '[\"proj-a\",\"proj-b\"]')",
            params![&id, &date.format("%Y-%m-%d").to_string(), &generated],
        )
        .map_err(|e| format!("daily tablet insert: {e}"))?;
        daily_ids.push(id);
        sum.daily_tablets += 1;
    }
    // The rolling todos must be `created_at < monday`. Pick a
    // date 10 days before monday.
    let created_before = (monday - Duration::days(10))
        .and_hms_opt(9, 0, 0)
        .unwrap()
        .and_utc()
        .to_rfc3339();
    let origin_id = daily_ids
        .first()
        .cloned()
        .unwrap_or_else(|| "daily-origin".into());
    let last_seen_id = daily_ids
        .last()
        .cloned()
        .unwrap_or_else(|| origin_id.clone());
    let todos = [
        ("todo-001", "Reply to proj-b on-call escalation thread"),
        ("todo-002", "Draft the proj-a migration runbook"),
        (
            "todo-003",
            "Re-share the proj-c schema RFC with the SRE team",
        ),
        ("todo-004", "Schedule the proj-b retro post-mortem"),
    ];
    for (id, body) in todos {
        conn.execute(
            "INSERT OR IGNORE INTO ceremony_todos_rolling \
             (todo_id, body, origin_tablet_id, created_at, done_at, last_seen_tablet_id) \
             VALUES (?1, ?2, ?3, ?4, NULL, ?5)",
            params![id, body, &origin_id, &created_before, &last_seen_id],
        )
        .map_err(|e| format!("todo insert: {e}"))?;
        sum.rolling_todos += 1;
    }
    Ok(())
}

/// Prior retro from two weeks ago with a diary the gather payload
/// will surface. The `prior_retro_diaries` slice of the gather
/// payload pulls the last 3; one is plenty for the LLM to "remember"
/// a theme.
fn seed_prior_retro_with_diary(
    conn: &Connection,
    prior_two_weeks: &DateTime<Utc>,
    sum: &mut SeedSummary,
) -> Result<(), String> {
    let iso = iso_week_str(*prior_two_weeks);
    let tablet_id = format!("retro-{iso}");
    let generated = (*prior_two_weeks).to_rfc3339();
    conn.execute(
        "INSERT OR IGNORE INTO ceremony_tablets \
         (id, kind, period_key, generated_at, status, workstreams_scanned) \
         VALUES (?1, 'retro', ?2, ?3, 'reviewed', '[\"proj-a\",\"proj-b\",\"proj-c\"]')",
        params![&tablet_id, &iso, &generated],
    )
    .map_err(|e| format!("prior retro insert: {e}"))?;
    let body =
        "Felt scattered. proj-c kept getting deferred even though we said it was a priority.";
    let written = (*prior_two_weeks + Duration::days(2)).to_rfc3339();
    let word_count: i64 = body.split_whitespace().count() as i64;
    conn.execute(
        "INSERT OR IGNORE INTO ceremony_diary \
         (tablet_id, body, written_at, word_count) VALUES (?1, ?2, ?3, ?4)",
        params![&tablet_id, body, &written, word_count],
    )
    .map_err(|e| format!("prior diary insert: {e}"))?;
    sum.prior_retros += 1;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn iso_week_str(dt: DateTime<Utc>) -> String {
    let iso: IsoWeek = dt.iso_week();
    format!("{:04}-W{:02}", iso.year(), iso.week())
}

/// Monday and Sunday of the ISO week containing `dt`, as
/// `NaiveDate`s. ISO week starts Monday.
fn monday_sunday(dt: DateTime<Utc>) -> (NaiveDate, NaiveDate) {
    let date = dt.naive_utc().date();
    let weekday_from_monday = date.weekday().num_days_from_monday() as i64;
    let monday = date - Duration::days(weekday_from_monday);
    let sunday = monday + Duration::days(6);
    (monday, sunday)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn monday_sunday_brackets_the_week() {
        // 2026-05-13 is a Wednesday.
        let dt: DateTime<Utc> = "2026-05-13T12:00:00Z".parse().unwrap();
        let (mon, sun) = monday_sunday(dt);
        assert_eq!(mon.weekday(), chrono::Weekday::Mon);
        assert_eq!(sun.weekday(), chrono::Weekday::Sun);
        assert_eq!((sun - mon).num_days(), 6);
    }

    #[test]
    fn apply_seeds_all_sections() {
        let tmp = tempdir().unwrap();
        let dd = tmp.path();
        // Initialise the schema by going through Store::open (runs
        // migrations); drop the store so we can re-open the DB
        // directly.
        let _ = arawn_storage::Store::open(dd).expect("migrations");

        let summary = apply(dd).expect("seed");
        assert!(summary.rollup_rows >= 18, "{}", summary.rollup_rows);
        assert!(summary.daily_tablets >= 4, "{}", summary.daily_tablets);
        assert!(summary.rolling_todos >= 3, "{}", summary.rolling_todos);
        assert_eq!(summary.priorities, 3);
        assert_eq!(summary.prior_retros, 1);
    }

    #[test]
    fn apply_is_idempotent() {
        let tmp = tempdir().unwrap();
        let dd = tmp.path();
        let _ = arawn_storage::Store::open(dd).expect("migrations");
        let _ = apply(dd).expect("first");
        // Second call must not error on duplicate PKs.
        let _ = apply(dd).expect("second");
    }
}
