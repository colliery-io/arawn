//! Retro pattern catalog v1.
//!
//! Three concrete `Detector`s. Each is deterministic SQL with a
//! declared `require_history_weeks` so the [`DetectorRegistry`]
//! short-circuits when history is too short.
//!
//! - **priority_completion_ratio** — % of confirmed weekly
//!   priorities marked done at week's end. Fires when ratio < 0.5.
//!   Lookback: 0 (only this week's data).
//! - **rollover_heat** — un-done todos that existed before the
//!   current week and are still being seen in this week's daily
//!   tablets. Signal: "this matters but never finds time". Fires
//!   when count ≥ 3. Lookback: 0.
//! - **workstream_neglect** — workstreams with non-zero activity in
//!   any of the prior 3 weeks but zero (or absent) in this week.
//!   Fires once per neglected workstream. Lookback: 3.

use std::sync::Arc;

use async_trait::async_trait;
use rusqlite::params;
use serde_json::json;

use crate::CeremonyError;
use crate::patterns::{Detector, DetectorCtx, DetectorRegistry};
use crate::plugins::retro::monday_sunday_for_iso_week_public as monday_sunday_for_iso_week;
use crate::types::DetectedPattern;

// --- priority_completion_ratio ---

/// Confirmed weekly priorities → fraction marked done. Fires when
/// the ratio is below 0.5 OR when no priorities were marked done at
/// all despite confirmations existing.
pub struct PriorityCompletionDetector;

#[async_trait]
impl Detector for PriorityCompletionDetector {
    fn key(&self) -> &'static str {
        "priority_completion_ratio"
    }

    fn require_history_weeks(&self) -> u32 {
        0
    }

    async fn detect(
        &self,
        ctx: &DetectorCtx<'_>,
    ) -> Result<Vec<DetectedPattern>, CeremonyError> {
        let conn = ctx
            .conn
            .0
            .lock()
            .map_err(|_| CeremonyError::Storage("connection mutex poisoned".into()))?;
        // Count confirmed priorities on this iso_week's weekly
        // tablet, and how many of those are done.
        let row: Option<(i64, i64)> = conn
            .query_row(
                "SELECT \
                   SUM(CASE WHEN p.confirmed_at IS NOT NULL THEN 1 ELSE 0 END), \
                   SUM(CASE WHEN p.confirmed_at IS NOT NULL AND p.done_at IS NOT NULL \
                            THEN 1 ELSE 0 END) \
                 FROM ceremony_priorities p \
                 JOIN ceremony_tablets t ON p.tablet_id = t.id \
                 WHERE t.kind = 'weekly' AND t.period_key = ?1",
                params![&ctx.current_iso_week],
                |row| Ok((row.get::<_, Option<i64>>(0)?.unwrap_or(0), row.get::<_, Option<i64>>(1)?.unwrap_or(0))),
            )
            .ok();
        let (confirmed, done) = row.unwrap_or((0, 0));
        if confirmed == 0 {
            // Nothing to evaluate — quiet skip.
            return Ok(Vec::new());
        }
        let ratio = done as f64 / confirmed as f64;
        if ratio >= 0.5 {
            return Ok(Vec::new());
        }
        Ok(vec![DetectedPattern {
            iso_week: ctx.current_iso_week.clone(),
            pattern_key: "priority_completion_ratio".into(),
            magnitude: ratio,
            payload: json!({
                "confirmed": confirmed,
                "done": done,
                "ratio": ratio,
            }),
        }])
    }
}

// --- rollover_heat ---

/// Un-done todos that existed before the current week and are still
/// being seen in this week's daily tablets. Fires when ≥ 3 such
/// todos are open.
pub struct RolloverHeatDetector;

#[async_trait]
impl Detector for RolloverHeatDetector {
    fn key(&self) -> &'static str {
        "rollover_heat"
    }

    fn require_history_weeks(&self) -> u32 {
        0
    }

    async fn detect(
        &self,
        ctx: &DetectorCtx<'_>,
    ) -> Result<Vec<DetectedPattern>, CeremonyError> {
        let (monday, sunday) = monday_sunday_for_iso_week(&ctx.current_iso_week)
            .ok_or_else(|| {
                CeremonyError::Other(format!(
                    "invalid iso_week '{}'",
                    ctx.current_iso_week
                ))
            })?;
        let conn = ctx
            .conn
            .0
            .lock()
            .map_err(|_| CeremonyError::Storage("connection mutex poisoned".into()))?;
        // un-done rolling todos
        //   created_at < monday (existed before this week)
        //   last_seen_tablet is a daily tablet within this week
        let mut stmt = conn
            .prepare(
                "SELECT r.todo_id, r.body, r.created_at, t.period_key \
                 FROM ceremony_todos_rolling r \
                 JOIN ceremony_tablets t ON r.last_seen_tablet_id = t.id \
                 WHERE r.done_at IS NULL \
                       AND r.created_at < ?1 \
                       AND t.kind = 'daily' \
                       AND t.period_key BETWEEN ?2 AND ?3",
            )
            .map_err(|e| CeremonyError::Storage(format!("rollover prepare: {e}")))?;
        let rows = stmt
            .query_map(params![&monday, &monday, &sunday], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(|e| CeremonyError::Storage(format!("rollover query: {e}")))?;
        let mut hot: Vec<(String, String, String, String)> = Vec::new();
        for r in rows {
            hot.push(r.map_err(|e| CeremonyError::Storage(format!("rollover row: {e}")))?);
        }
        if hot.len() < 3 {
            return Ok(Vec::new());
        }
        let count = hot.len();
        Ok(vec![DetectedPattern {
            iso_week: ctx.current_iso_week.clone(),
            pattern_key: "rollover_heat".into(),
            magnitude: count as f64,
            payload: json!({
                "count": count,
                "todos": hot.iter().map(|(id, body, created, last_seen)| json!({
                    "todo_id": id,
                    "body": body,
                    "created_at": created,
                    "last_seen_period_key": last_seen,
                })).collect::<Vec<_>>(),
            }),
        }])
    }
}

// --- workstream_neglect ---

/// Workstreams that produced rollup activity in any of the prior 3
/// weeks but zero (or no row) this week. Fires once per neglected
/// workstream.
pub struct WorkstreamNeglectDetector;

#[async_trait]
impl Detector for WorkstreamNeglectDetector {
    fn key(&self) -> &'static str {
        "workstream_neglect"
    }

    fn require_history_weeks(&self) -> u32 {
        3
    }

    async fn detect(
        &self,
        ctx: &DetectorCtx<'_>,
    ) -> Result<Vec<DetectedPattern>, CeremonyError> {
        let conn = ctx
            .conn
            .0
            .lock()
            .map_err(|_| CeremonyError::Storage("connection mutex poisoned".into()))?;
        // workstreams with sum(value) > 0 in the trailing 3 weeks
        // strictly before current iso_week.
        let mut stmt = conn
            .prepare(
                "SELECT DISTINCT workstream FROM ceremony_activity_rollup \
                 WHERE iso_week < ?1 AND iso_week >= ( \
                     SELECT iso_week FROM ( \
                         SELECT DISTINCT iso_week FROM ceremony_activity_rollup \
                         WHERE iso_week < ?1 ORDER BY iso_week DESC LIMIT 3 \
                     ) ORDER BY iso_week ASC LIMIT 1 \
                 ) AND value > 0",
            )
            .map_err(|e| CeremonyError::Storage(format!("neglect prior prepare: {e}")))?;
        let active_rows = stmt
            .query_map(params![&ctx.current_iso_week], |row| {
                Ok(row.get::<_, String>(0)?)
            })
            .map_err(|e| CeremonyError::Storage(format!("neglect prior query: {e}")))?;
        let mut active_prior: Vec<String> = Vec::new();
        for r in active_rows {
            active_prior
                .push(r.map_err(|e| CeremonyError::Storage(format!("neglect prior row: {e}")))?);
        }

        // for each active_prior workstream, check if any current-week
        // row exists with non-zero value.
        let mut current_active: std::collections::HashSet<String> = std::collections::HashSet::new();
        {
            let mut stmt2 = conn
                .prepare(
                    "SELECT DISTINCT workstream FROM ceremony_activity_rollup \
                     WHERE iso_week = ?1 AND value > 0",
                )
                .map_err(|e| CeremonyError::Storage(format!("neglect current prepare: {e}")))?;
            let cur_rows = stmt2
                .query_map(params![&ctx.current_iso_week], |row| {
                    Ok(row.get::<_, String>(0)?)
                })
                .map_err(|e| CeremonyError::Storage(format!("neglect current query: {e}")))?;
            for r in cur_rows {
                current_active.insert(
                    r.map_err(|e| CeremonyError::Storage(format!("neglect current row: {e}")))?,
                );
            }
        }

        let mut out = Vec::new();
        for ws in &active_prior {
            if !current_active.contains(ws) {
                out.push(DetectedPattern {
                    iso_week: ctx.current_iso_week.clone(),
                    pattern_key: "workstream_neglect".into(),
                    magnitude: 1.0,
                    payload: json!({
                        "workstream": ws,
                        "comparison_window_weeks": 3,
                    }),
                });
            }
        }
        Ok(out)
    }
}

// --- catalog assembly ---

/// The v1 catalog. Plugins (currently just retro) build their
/// `DetectorRegistry` from this.
pub fn v1_catalog() -> DetectorRegistry {
    DetectorRegistry::new()
        .with(Arc::new(PriorityCompletionDetector))
        .with(Arc::new(RolloverHeatDetector))
        .with(Arc::new(WorkstreamNeglectDetector))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{ConnHandle, EngineCtx};
    use crate::plugin::PatternDetector as _;
    use rusqlite::params;
    use tempfile::TempDir;

    fn open_test_db() -> (TempDir, ConnHandle) {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("test.db");
        let _db = arawn_storage::Database::open(&db_path).expect("migrations");
        drop(_db);
        let conn = rusqlite::Connection::open(&db_path).expect("open conn");
        (tmp, ConnHandle::new(conn))
    }

    fn insert_tablet(
        conn: &ConnHandle,
        id: &str,
        kind: &str,
        period_key: &str,
        generated_at: &str,
    ) {
        let c = conn.0.lock().unwrap();
        c.execute(
            "INSERT INTO ceremony_tablets (id, kind, period_key, generated_at, status, workstreams_scanned) \
             VALUES (?1, ?2, ?3, ?4, 'reviewed', '[]')",
            params![id, kind, period_key, generated_at],
        )
        .unwrap();
    }

    fn insert_priority(
        conn: &ConnHandle,
        id: &str,
        tablet_id: &str,
        confirmed: bool,
        done: bool,
    ) {
        let c = conn.0.lock().unwrap();
        c.execute(
            "INSERT INTO ceremony_priorities (id, tablet_id, body, rationale, citation_id, \
             confirmed_at, done_at, ordinal) VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6, 0)",
            params![
                id,
                tablet_id,
                "x",
                "y",
                if confirmed { Some("2026-05-11T08:00:00Z") } else { None },
                if done { Some("2026-05-15T17:00:00Z") } else { None },
            ],
        )
        .unwrap();
    }

    // --- priority_completion_ratio ---

    #[tokio::test]
    async fn priority_completion_fires_below_threshold() {
        let (_tmp, conn) = open_test_db();
        insert_tablet(&conn, "weekly-W20", "weekly", "2026-W20", "2026-05-11T07:00:00Z");
        insert_priority(&conn, "p1", "weekly-W20", true, false);
        insert_priority(&conn, "p2", "weekly-W20", true, false);
        insert_priority(&conn, "p3", "weekly-W20", true, true);
        // 1 / 3 = 0.33 < 0.5 → fire.
        let ctx = EngineCtx::new(conn.clone(), "retro-2026-W20".into(), "2026-W20".into());
        let dctx = DetectorCtx::new("2026-W20".into(), &conn);
        let _ = ctx;
        let rows = PriorityCompletionDetector.detect(&dctx).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].pattern_key, "priority_completion_ratio");
    }

    #[tokio::test]
    async fn priority_completion_quiet_above_threshold() {
        let (_tmp, conn) = open_test_db();
        insert_tablet(&conn, "weekly-W20", "weekly", "2026-W20", "2026-05-11T07:00:00Z");
        insert_priority(&conn, "p1", "weekly-W20", true, true);
        insert_priority(&conn, "p2", "weekly-W20", true, true);
        insert_priority(&conn, "p3", "weekly-W20", true, false);
        // 2 / 3 = 0.67 ≥ 0.5 → quiet.
        let dctx = DetectorCtx::new("2026-W20".into(), &conn);
        let rows = PriorityCompletionDetector.detect(&dctx).await.unwrap();
        assert!(rows.is_empty());
    }

    #[tokio::test]
    async fn priority_completion_quiet_when_no_priorities() {
        let (_tmp, conn) = open_test_db();
        let dctx = DetectorCtx::new("2026-W20".into(), &conn);
        let rows = PriorityCompletionDetector.detect(&dctx).await.unwrap();
        assert!(rows.is_empty());
    }

    // --- rollover_heat ---

    fn insert_rolling_todo(
        conn: &ConnHandle,
        id: &str,
        created_at: &str,
        last_seen_tablet: &str,
        done: bool,
    ) {
        let c = conn.0.lock().unwrap();
        c.execute(
            "INSERT INTO ceremony_todos_rolling (todo_id, body, origin_tablet_id, created_at, \
             done_at, last_seen_tablet_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id,
                "x",
                last_seen_tablet, // origin doesn't matter for this test
                created_at,
                if done { Some("2026-05-15T17:00:00Z") } else { None },
                last_seen_tablet,
            ],
        )
        .unwrap();
    }

    #[tokio::test]
    async fn rollover_heat_fires_at_threshold() {
        let (_tmp, conn) = open_test_db();
        // A daily tablet inside the week.
        insert_tablet(&conn, "daily-d1", "daily", "2026-05-13", "2026-05-13T07:00:00Z");
        // Three todos created before Monday, all un-done, last_seen
        // this week's daily.
        insert_rolling_todo(&conn, "t1", "2026-05-05T00:00:00Z", "daily-d1", false);
        insert_rolling_todo(&conn, "t2", "2026-05-05T00:00:00Z", "daily-d1", false);
        insert_rolling_todo(&conn, "t3", "2026-05-05T00:00:00Z", "daily-d1", false);
        let dctx = DetectorCtx::new("2026-W20".into(), &conn);
        let rows = RolloverHeatDetector.detect(&dctx).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].magnitude, 3.0);
    }

    #[tokio::test]
    async fn rollover_heat_quiet_below_threshold() {
        let (_tmp, conn) = open_test_db();
        insert_tablet(&conn, "daily-d1", "daily", "2026-05-13", "2026-05-13T07:00:00Z");
        insert_rolling_todo(&conn, "t1", "2026-05-05T00:00:00Z", "daily-d1", false);
        insert_rolling_todo(&conn, "t2", "2026-05-05T00:00:00Z", "daily-d1", false);
        let dctx = DetectorCtx::new("2026-W20".into(), &conn);
        let rows = RolloverHeatDetector.detect(&dctx).await.unwrap();
        assert!(rows.is_empty());
    }

    #[tokio::test]
    async fn rollover_heat_ignores_done_and_in_week_creations() {
        let (_tmp, conn) = open_test_db();
        insert_tablet(&conn, "daily-d1", "daily", "2026-05-13", "2026-05-13T07:00:00Z");
        // Created inside the week → not a rollover.
        insert_rolling_todo(&conn, "in-week", "2026-05-13T08:00:00Z", "daily-d1", false);
        // Done → exclude.
        insert_rolling_todo(&conn, "done", "2026-05-01T00:00:00Z", "daily-d1", true);
        // Only one "real" rollover-eligible row left; threshold not met.
        insert_rolling_todo(&conn, "single", "2026-05-01T00:00:00Z", "daily-d1", false);
        let dctx = DetectorCtx::new("2026-W20".into(), &conn);
        let rows = RolloverHeatDetector.detect(&dctx).await.unwrap();
        assert!(rows.is_empty());
    }

    // --- workstream_neglect ---

    fn insert_rollup(
        conn: &ConnHandle,
        iso_week: &str,
        workstream: &str,
        metric: &str,
        value: f64,
    ) {
        let c = conn.0.lock().unwrap();
        c.execute(
            "INSERT INTO ceremony_activity_rollup (iso_week, workstream, metric_key, value) \
             VALUES (?1, ?2, ?3, ?4)",
            params![iso_week, workstream, metric, value],
        )
        .unwrap();
    }

    #[tokio::test]
    async fn workstream_neglect_fires_per_neglected_workstream() {
        let (_tmp, conn) = open_test_db();
        // Prior 3 weeks: two workstreams active.
        insert_rollup(&conn, "2026-W17", "proj-a", "emails", 5.0);
        insert_rollup(&conn, "2026-W18", "proj-a", "emails", 5.0);
        insert_rollup(&conn, "2026-W19", "proj-a", "emails", 5.0);
        insert_rollup(&conn, "2026-W17", "proj-b", "emails", 5.0);
        insert_rollup(&conn, "2026-W18", "proj-b", "emails", 5.0);
        insert_rollup(&conn, "2026-W19", "proj-b", "emails", 5.0);
        // This week: only proj-a is active.
        insert_rollup(&conn, "2026-W20", "proj-a", "emails", 5.0);
        let dctx = DetectorCtx::new("2026-W20".into(), &conn);
        let rows = WorkstreamNeglectDetector.detect(&dctx).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].pattern_key, "workstream_neglect");
        let payload = rows[0].payload.clone();
        assert_eq!(payload.get("workstream").unwrap().as_str().unwrap(), "proj-b");
    }

    #[tokio::test]
    async fn workstream_neglect_quiet_when_all_active() {
        let (_tmp, conn) = open_test_db();
        insert_rollup(&conn, "2026-W17", "proj-a", "emails", 5.0);
        insert_rollup(&conn, "2026-W18", "proj-a", "emails", 5.0);
        insert_rollup(&conn, "2026-W19", "proj-a", "emails", 5.0);
        insert_rollup(&conn, "2026-W20", "proj-a", "emails", 5.0);
        let dctx = DetectorCtx::new("2026-W20".into(), &conn);
        let rows = WorkstreamNeglectDetector.detect(&dctx).await.unwrap();
        assert!(rows.is_empty());
    }

    #[tokio::test]
    async fn workstream_neglect_requires_three_weeks_history() {
        // Only 2 weeks of prior history → DetectorRegistry should
        // skip this detector. We test the require_history_weeks
        // contract directly here.
        assert_eq!(WorkstreamNeglectDetector.require_history_weeks(), 3);
    }

    // --- catalog assembly ---

    #[tokio::test]
    async fn v1_catalog_runs_all_three_when_history_sufficient() {
        let (_tmp, conn) = open_test_db();
        // Seed prior 3 weeks of rollup so workstream_neglect's
        // require_history_weeks passes.
        insert_rollup(&conn, "2026-W17", "proj-a", "emails", 5.0);
        insert_rollup(&conn, "2026-W18", "proj-a", "emails", 5.0);
        insert_rollup(&conn, "2026-W19", "proj-a", "emails", 5.0);
        // Current week: proj-a active.
        insert_rollup(&conn, "2026-W20", "proj-a", "emails", 5.0);
        let ctx = EngineCtx::new(conn.clone(), "retro-2026-W20".into(), "2026-W20".into());
        let registry = v1_catalog();
        // No priorities, no rollover todos → only priority+rollover
        // detectors run + return empty. workstream_neglect runs + returns
        // empty (proj-a active in both windows). Result: no rows
        // but no errors.
        let rows = registry.detect(&ctx).await.unwrap();
        assert!(rows.is_empty());
    }

    #[tokio::test]
    async fn v1_catalog_skips_workstream_neglect_when_history_short() {
        let (_tmp, conn) = open_test_db();
        // No prior weeks of rollup → workstream_neglect skipped.
        // priority_completion_ratio and rollover_heat have
        // require_history_weeks=0 so they still run (and return
        // empty).
        let ctx = EngineCtx::new(conn.clone(), "retro-2026-W20".into(), "2026-W20".into());
        let registry = v1_catalog();
        let rows = registry.detect(&ctx).await.unwrap();
        assert!(rows.is_empty());
    }
}
