//! End-of-week activity rollup.
//!
//! The retro plugin's pattern detectors (T-0286, T-0288) read from
//! `ceremony_activity_rollup` rather than from the raw `feed_*` /
//! `signal_*` / `steward_journal` tables, because:
//!
//! - The raw tables live in *per-workstream* databases. Pulling
//!   them from the central `arawn.db` would force a cross-crate
//!   coupling we'd rather avoid.
//! - Detectors want pre-aggregated `(iso_week, workstream,
//!   metric_key, value)` triples, not raw rows.
//!
//! The rollup pipeline is **pluggable**: each metric is contributed
//! by a [`RollupSource`] implementation. Concrete sources land
//! later in the binary (or in a sibling crate that has direct
//! access to per-workstream DBs); this crate ships the framework
//! plus stub sources for tests.

use std::sync::Arc;

use async_trait::async_trait;
use rusqlite::params;

use crate::CeremonyError;
use crate::engine::ConnHandle;

/// One contributor to the activity rollup. Computes a numeric value
/// for `(iso_week, workstream, metric_key)`. Multiple sources may
/// share an `iso_week` but each owns its own `metric_key`.
#[async_trait]
pub trait RollupSource: Send + Sync {
    /// The metric this source produces. Stable string — pattern
    /// detectors filter on it. Recommended keys: `emails_sent`,
    /// `slack_threads_participated`, `meetings_attended`,
    /// `deep_work_hours`, `signals_extracted_count`,
    /// `steward_proposals_accepted`, `steward_proposals_rejected`.
    fn metric_key(&self) -> &str;

    /// Compute the numeric value for one `(iso_week, workstream)`
    /// slice. Sources read from whichever store they own; the
    /// pipeline aggregates the returns into the central rollup
    /// table.
    ///
    /// Returning `Ok(None)` is honored as "this source does not
    /// apply to this workstream for this week" — the row is
    /// simply not written. Use this for sources that only apply
    /// to some workstreams (e.g. Slack-only metrics on a
    /// workstream without Slack configured).
    async fn compute(
        &self,
        iso_week: &str,
        workstream: &str,
    ) -> Result<Option<f64>, CeremonyError>;
}

/// Active workstreams the rollup walks. Sourced from `workstreams`
/// in `arawn.db` (the central table from V1). Kept as a tiny trait
/// so tests can stub the list without standing up the full
/// `arawn-storage::WorkstreamStore`.
pub trait WorkstreamList: Send + Sync {
    fn active_workstreams(&self) -> Result<Vec<String>, CeremonyError>;
}

/// Default impl that queries the central `workstreams` table for
/// non-archived rows.
pub struct CentralDbWorkstreams {
    pub conn: ConnHandle,
}

impl WorkstreamList for CentralDbWorkstreams {
    fn active_workstreams(&self) -> Result<Vec<String>, CeremonyError> {
        let conn = self
            .conn
            .0
            .lock()
            .map_err(|_| CeremonyError::Storage("connection mutex poisoned".into()))?;
        let mut stmt = conn
            .prepare("SELECT name FROM workstreams WHERE archived = 0")
            .map_err(|e| CeremonyError::Storage(format!("active_workstreams prepare: {e}")))?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| CeremonyError::Storage(format!("active_workstreams query: {e}")))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| {
                CeremonyError::Storage(format!("active_workstreams row: {e}"))
            })?);
        }
        Ok(out)
    }
}

/// Computes the rollup for one ISO week. Walks every active
/// workstream × every registered source, and writes the resulting
/// `(iso_week, workstream, metric_key, value)` rows to
/// `ceremony_activity_rollup`. The write is idempotent — rerunning
/// for the same week first deletes the existing rows so a partial
/// recompute can't leave stale data behind.
///
/// Wraps the write in a single transaction so a mid-source failure
/// rolls back cleanly.
pub async fn compute_for_week(
    iso_week: &str,
    workstreams: &dyn WorkstreamList,
    sources: &[Arc<dyn RollupSource>],
    conn: &ConnHandle,
) -> Result<usize, CeremonyError> {
    let active = workstreams.active_workstreams()?;

    // Gather all values first (async per-source), then write under
    // a single transaction so a failure mid-write doesn't leave
    // partial rows.
    let mut values: Vec<(String, String, f64)> = Vec::new();
    for ws in &active {
        for source in sources {
            if let Some(value) = source.compute(iso_week, ws).await? {
                values.push((ws.clone(), source.metric_key().to_string(), value));
            }
        }
    }

    let conn = conn
        .0
        .lock()
        .map_err(|_| CeremonyError::Storage("connection mutex poisoned".into()))?;
    conn.execute("BEGIN IMMEDIATE", [])
        .map_err(|e| CeremonyError::Storage(format!("rollup BEGIN: {e}")))?;
    // Idempotency: wipe existing rows for this week.
    if let Err(e) = conn.execute(
        "DELETE FROM ceremony_activity_rollup WHERE iso_week = ?1",
        params![iso_week],
    ) {
        let _ = conn.execute("ROLLBACK", []);
        return Err(CeremonyError::Storage(format!("rollup wipe: {e}")));
    }
    for (ws, key, value) in &values {
        if let Err(e) = conn.execute(
            "INSERT INTO ceremony_activity_rollup (iso_week, workstream, metric_key, value) \
             VALUES (?1, ?2, ?3, ?4)",
            params![iso_week, ws, key, *value],
        ) {
            let _ = conn.execute("ROLLBACK", []);
            return Err(CeremonyError::Storage(format!("rollup insert: {e}")));
        }
    }
    conn.execute("COMMIT", [])
        .map_err(|e| CeremonyError::Storage(format!("rollup COMMIT: {e}")))?;
    Ok(values.len())
}

/// Convenience: read a single rollup value back. Pattern detectors
/// call this when they need one metric for one workstream/week.
pub fn read_rollup_value(
    conn: &ConnHandle,
    iso_week: &str,
    workstream: &str,
    metric_key: &str,
) -> Result<Option<f64>, CeremonyError> {
    let conn = conn
        .0
        .lock()
        .map_err(|_| CeremonyError::Storage("connection mutex poisoned".into()))?;
    let value: Option<f64> = conn
        .query_row(
            "SELECT value FROM ceremony_activity_rollup \
             WHERE iso_week = ?1 AND workstream = ?2 AND metric_key = ?3",
            params![iso_week, workstream, metric_key],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| CeremonyError::Storage(format!("read_rollup_value: {e}")))?;
    Ok(value)
}

use rusqlite::OptionalExtension;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn open_test_db() -> (TempDir, ConnHandle) {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("test.db");
        let _db = arawn_storage::Database::open(&db_path).expect("migrations");
        drop(_db);
        let conn = rusqlite::Connection::open(&db_path).expect("open conn");
        (tmp, ConnHandle::new(conn))
    }

    struct StubWorkstreams(Vec<String>);
    impl WorkstreamList for StubWorkstreams {
        fn active_workstreams(&self) -> Result<Vec<String>, CeremonyError> {
            Ok(self.0.clone())
        }
    }

    /// Constant per-workstream source. Useful for asserting cross-
    /// section counts without needing the real per-workstream DB.
    struct ConstSource {
        key: &'static str,
        value_per_workstream: std::collections::HashMap<String, f64>,
    }
    #[async_trait]
    impl RollupSource for ConstSource {
        fn metric_key(&self) -> &str {
            self.key
        }
        async fn compute(
            &self,
            _iso_week: &str,
            workstream: &str,
        ) -> Result<Option<f64>, CeremonyError> {
            Ok(self.value_per_workstream.get(workstream).copied())
        }
    }

    fn make_source(
        key: &'static str,
        values: &[(&str, f64)],
    ) -> Arc<dyn RollupSource> {
        let mut m = std::collections::HashMap::new();
        for (k, v) in values {
            m.insert((*k).to_string(), *v);
        }
        Arc::new(ConstSource {
            key,
            value_per_workstream: m,
        })
    }

    #[tokio::test]
    async fn computes_rollup_for_two_workstreams_two_sources() {
        let (_tmp, conn) = open_test_db();
        let workstreams = StubWorkstreams(vec!["proj-a".into(), "proj-b".into()]);
        let sources: Vec<Arc<dyn RollupSource>> = vec![
            make_source("emails_sent", &[("proj-a", 12.0), ("proj-b", 4.0)]),
            make_source(
                "meetings_attended",
                &[("proj-a", 3.0), ("proj-b", 1.0)],
            ),
        ];
        let n = compute_for_week("2026-W20", &workstreams, &sources, &conn)
            .await
            .unwrap();
        assert_eq!(n, 4);
        assert_eq!(
            read_rollup_value(&conn, "2026-W20", "proj-a", "emails_sent")
                .unwrap()
                .unwrap(),
            12.0
        );
        assert_eq!(
            read_rollup_value(&conn, "2026-W20", "proj-b", "meetings_attended")
                .unwrap()
                .unwrap(),
            1.0
        );
    }

    #[tokio::test]
    async fn missing_workstream_value_is_skipped_silently() {
        let (_tmp, conn) = open_test_db();
        let workstreams = StubWorkstreams(vec!["proj-a".into(), "proj-b".into()]);
        // Source returns a value only for proj-a.
        let sources: Vec<Arc<dyn RollupSource>> =
            vec![make_source("emails_sent", &[("proj-a", 5.0)])];
        let n = compute_for_week("2026-W20", &workstreams, &sources, &conn)
            .await
            .unwrap();
        assert_eq!(n, 1);
        assert_eq!(
            read_rollup_value(&conn, "2026-W20", "proj-a", "emails_sent")
                .unwrap()
                .unwrap(),
            5.0
        );
        assert!(
            read_rollup_value(&conn, "2026-W20", "proj-b", "emails_sent")
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn rerun_for_same_week_replaces_not_appends() {
        let (_tmp, conn) = open_test_db();
        let workstreams = StubWorkstreams(vec!["proj-a".into()]);
        let sources: Vec<Arc<dyn RollupSource>> =
            vec![make_source("emails_sent", &[("proj-a", 5.0)])];
        compute_for_week("2026-W20", &workstreams, &sources, &conn)
            .await
            .unwrap();
        // Second run with a different value.
        let sources2: Vec<Arc<dyn RollupSource>> =
            vec![make_source("emails_sent", &[("proj-a", 9.0)])];
        compute_for_week("2026-W20", &workstreams, &sources2, &conn)
            .await
            .unwrap();
        // Only the latest value is present; no duplicate row.
        let count: i64 = {
            let c = conn.0.lock().unwrap();
            c.query_row(
                "SELECT COUNT(*) FROM ceremony_activity_rollup WHERE iso_week = '2026-W20'",
                [],
                |row| row.get(0),
            )
            .unwrap()
        };
        assert_eq!(count, 1);
        assert_eq!(
            read_rollup_value(&conn, "2026-W20", "proj-a", "emails_sent")
                .unwrap()
                .unwrap(),
            9.0
        );
    }

    #[tokio::test]
    async fn central_db_workstreams_lists_active_only() {
        let (_tmp, conn) = open_test_db();
        // Seed two workstreams; one archived.
        {
            let c = conn.0.lock().unwrap();
            c.execute(
                "INSERT INTO workstreams (id, name, root_dir, created_at, updated_at, archived) \
                 VALUES (?1, ?2, ?3, ?4, ?5, 0)",
                params![
                    "11111111-1111-1111-1111-111111111111",
                    "alpha",
                    "/tmp/a",
                    "2026-05-15T00:00:00Z",
                    "2026-05-15T00:00:00Z",
                ],
            )
            .unwrap();
            c.execute(
                "INSERT INTO workstreams (id, name, root_dir, created_at, updated_at, archived) \
                 VALUES (?1, ?2, ?3, ?4, ?5, 1)",
                params![
                    "22222222-2222-2222-2222-222222222222",
                    "beta",
                    "/tmp/b",
                    "2026-05-15T00:00:00Z",
                    "2026-05-15T00:00:00Z",
                ],
            )
            .unwrap();
        }
        let lister = CentralDbWorkstreams { conn };
        let mut active = lister.active_workstreams().unwrap();
        active.sort();
        assert_eq!(active, vec!["alpha".to_string()]);
    }

    #[tokio::test]
    async fn empty_sources_writes_nothing() {
        let (_tmp, conn) = open_test_db();
        let workstreams = StubWorkstreams(vec!["proj-a".into()]);
        let sources: Vec<Arc<dyn RollupSource>> = vec![];
        let n = compute_for_week("2026-W20", &workstreams, &sources, &conn)
            .await
            .unwrap();
        assert_eq!(n, 0);
    }
}
