//! Per-rule pattern detection scaffolding.
//!
//! T-0279 shipped the engine-facing [`crate::plugin::PatternDetector`]
//! trait (one method, returns a `Vec<DetectedPattern>`). T-0286
//! decomposes that into individual rules: each rule implements
//! [`Detector`] and reads from the rollup tables + recent tablets
//! via [`DetectorCtx`]. A [`DetectorRegistry`] aggregates rules and
//! implements `PatternDetector` itself, so a plugin returns a
//! single registry from `Ceremony::patterns()` and the framework
//! does the fan-out.
//!
//! Bootstrap fallback: each rule declares the minimum number of
//! prior weeks it needs (`require_history_weeks`). When history is
//! shorter the rule returns no rows, and the engine ships a retro
//! without the patterns section. T-0288's catalog rules use this.

use std::sync::Arc;

use async_trait::async_trait;
use rusqlite::params;

use crate::CeremonyError;
use crate::engine::ConnHandle;
use crate::plugin::{CeremonyCtx, PatternDetector};
use crate::types::DetectedPattern;

/// One rule that contributes zero or more pattern rows. Rules are
/// strictly deterministic — SQL aggregations only, no LLM.
#[async_trait]
pub trait Detector: Send + Sync {
    /// Stable identifier (e.g. `"priority_completion_ratio"`).
    /// Used in `DetectedPattern.pattern_key` and as the dispatch
    /// key for telemetry.
    fn key(&self) -> &'static str;

    /// Minimum number of prior ISO weeks of rollup history this
    /// rule requires. Rules that compare against historical
    /// baselines return their lookback window here; rules that
    /// only need the current week return 0. The framework calls
    /// [`DetectorCtx::weeks_of_history`] and short-circuits to
    /// empty when insufficient.
    fn require_history_weeks(&self) -> u32 {
        0
    }

    /// Run the rule for `current_iso_week`. The ctx exposes the
    /// rollup + tablet history queries.
    async fn detect(
        &self,
        ctx: &DetectorCtx<'_>,
    ) -> Result<Vec<DetectedPattern>, CeremonyError>;
}

/// Read-only surface a [`Detector`] uses to query historical data.
/// Bound to the rule's `current_iso_week` so detectors don't have
/// to thread the week through every call.
pub struct DetectorCtx<'a> {
    pub current_iso_week: String,
    pub conn: &'a ConnHandle,
}

impl<'a> DetectorCtx<'a> {
    pub fn new(current_iso_week: String, conn: &'a ConnHandle) -> Self {
        Self {
            current_iso_week,
            conn,
        }
    }

    /// Distinct iso_weeks present in `ceremony_activity_rollup`
    /// strictly before `current_iso_week`. Used by detectors to
    /// decide whether they have enough history to fire.
    pub fn weeks_of_history(&self) -> Result<u32, CeremonyError> {
        let conn = self
            .conn
            .0
            .lock()
            .map_err(|_| CeremonyError::Storage("connection mutex poisoned".into()))?;
        let count: u32 = conn
            .query_row(
                "SELECT COUNT(DISTINCT iso_week) FROM ceremony_activity_rollup \
                 WHERE iso_week < ?1",
                params![&self.current_iso_week],
                |row| row.get(0),
            )
            .map_err(|e| CeremonyError::Storage(format!("weeks_of_history: {e}")))?;
        Ok(count)
    }

    /// Sum of a metric for one workstream across the trailing
    /// `lookback_weeks` (exclusive of `current_iso_week`). Returns
    /// `None` when no rows match.
    pub fn metric_sum_trailing(
        &self,
        workstream: &str,
        metric_key: &str,
        lookback_weeks: u32,
    ) -> Result<Option<f64>, CeremonyError> {
        let conn = self
            .conn
            .0
            .lock()
            .map_err(|_| CeremonyError::Storage("connection mutex poisoned".into()))?;
        // Sort iso_week strings lexicographically — ISO week format
        // (`YYYY-Www`) sorts correctly as text.
        let mut stmt = conn
            .prepare(
                "SELECT iso_week, value FROM ceremony_activity_rollup \
                 WHERE workstream = ?1 AND metric_key = ?2 AND iso_week < ?3 \
                 ORDER BY iso_week DESC LIMIT ?4",
            )
            .map_err(|e| CeremonyError::Storage(format!("metric_sum prepare: {e}")))?;
        let rows = stmt
            .query_map(
                params![workstream, metric_key, &self.current_iso_week, lookback_weeks],
                |row| Ok(row.get::<_, f64>(1)?),
            )
            .map_err(|e| CeremonyError::Storage(format!("metric_sum query: {e}")))?;
        let mut total = 0.0;
        let mut seen = 0;
        for r in rows {
            total += r.map_err(|e| CeremonyError::Storage(format!("metric_sum row: {e}")))?;
            seen += 1;
        }
        if seen == 0 { Ok(None) } else { Ok(Some(total)) }
    }

    /// Current-week value for one workstream/metric. Returns `None`
    /// when no row exists for the current week (vs. zero, which is
    /// an explicit "we measured and got zero").
    pub fn current_metric_value(
        &self,
        workstream: &str,
        metric_key: &str,
    ) -> Result<Option<f64>, CeremonyError> {
        crate::rollup::read_rollup_value(self.conn, &self.current_iso_week, workstream, metric_key)
    }
}

/// Aggregates a set of [`Detector`]s and implements
/// [`PatternDetector`]. Plugins return this from
/// `Ceremony::patterns()` and the engine fans out.
pub struct DetectorRegistry {
    detectors: Vec<Arc<dyn Detector>>,
}

impl Default for DetectorRegistry {
    fn default() -> Self {
        Self {
            detectors: Vec::new(),
        }
    }
}

impl DetectorRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(mut self, detector: Arc<dyn Detector>) -> Self {
        self.detectors.push(detector);
        self
    }

    pub fn detectors(&self) -> &[Arc<dyn Detector>] {
        &self.detectors
    }
}

/// The PatternDetector impl that the engine calls. Walks every
/// registered [`Detector`], skipping those whose
/// `require_history_weeks` exceed the available history.
///
/// Requires a ctx with a `conn_handle()` (i.e. the production
/// `EngineCtx`). Stub ctxs in tests that intentionally lack SQL
/// access surface as `CeremonyError::Other` — detectors cannot
/// run without history.
#[async_trait]
impl PatternDetector for DetectorRegistry {
    async fn detect(
        &self,
        ctx: &dyn CeremonyCtx,
    ) -> Result<Vec<DetectedPattern>, CeremonyError> {
        let conn = ctx.conn_handle().ok_or_else(|| {
            CeremonyError::Other(
                "DetectorRegistry requires a ctx with conn_handle (production EngineCtx)".into(),
            )
        })?;
        let dctx = DetectorCtx::new(ctx.period_key().to_string(), conn);

        let available_history = dctx.weeks_of_history()?;
        let mut out = Vec::new();
        for detector in &self.detectors {
            let need = detector.require_history_weeks();
            if need > available_history {
                tracing::debug!(
                    rule = detector.key(),
                    need_weeks = need,
                    have_weeks = available_history,
                    "skipping detector — insufficient history (bootstrap)"
                );
                continue;
            }
            let rows = detector.detect(&dctx).await?;
            tracing::debug!(rule = detector.key(), rows = rows.len(), "detector fired");
            out.extend(rows);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::EngineCtx;
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

    fn seed_rollup(
        conn: &ConnHandle,
        rows: &[(&str, &str, &str, f64)], // (iso_week, workstream, metric_key, value)
    ) {
        let c = conn.0.lock().unwrap();
        for (week, ws, key, val) in rows {
            c.execute(
                "INSERT INTO ceremony_activity_rollup (iso_week, workstream, metric_key, value) \
                 VALUES (?1, ?2, ?3, ?4)",
                params![week, ws, key, *val],
            )
            .unwrap();
        }
    }

    // --- DetectorCtx queries ---

    #[tokio::test]
    async fn weeks_of_history_counts_distinct_prior_weeks() {
        let (_tmp, conn) = open_test_db();
        seed_rollup(
            &conn,
            &[
                ("2026-W17", "proj-a", "x", 1.0),
                ("2026-W18", "proj-a", "x", 1.0),
                ("2026-W19", "proj-a", "x", 1.0),
                ("2026-W20", "proj-a", "x", 1.0), // current
            ],
        );
        let dctx = DetectorCtx::new("2026-W20".into(), &conn);
        assert_eq!(dctx.weeks_of_history().unwrap(), 3);
    }

    #[tokio::test]
    async fn metric_sum_trailing_sums_lookback() {
        let (_tmp, conn) = open_test_db();
        seed_rollup(
            &conn,
            &[
                ("2026-W17", "proj-a", "x", 1.0),
                ("2026-W18", "proj-a", "x", 2.0),
                ("2026-W19", "proj-a", "x", 4.0),
                ("2026-W20", "proj-a", "x", 8.0), // current — excluded
            ],
        );
        let dctx = DetectorCtx::new("2026-W20".into(), &conn);
        // Last 2 trailing weeks: W19 + W18 = 6.
        assert_eq!(
            dctx.metric_sum_trailing("proj-a", "x", 2).unwrap().unwrap(),
            6.0
        );
        // Last 3 trailing weeks: W19 + W18 + W17 = 7.
        assert_eq!(
            dctx.metric_sum_trailing("proj-a", "x", 3).unwrap().unwrap(),
            7.0
        );
    }

    #[tokio::test]
    async fn current_metric_value_returns_some_for_present_zero() {
        let (_tmp, conn) = open_test_db();
        seed_rollup(&conn, &[("2026-W20", "proj-a", "x", 0.0)]);
        let dctx = DetectorCtx::new("2026-W20".into(), &conn);
        assert_eq!(
            dctx.current_metric_value("proj-a", "x").unwrap(),
            Some(0.0)
        );
    }

    #[tokio::test]
    async fn current_metric_value_returns_none_for_absent_row() {
        let (_tmp, conn) = open_test_db();
        let dctx = DetectorCtx::new("2026-W20".into(), &conn);
        assert!(dctx.current_metric_value("proj-a", "x").unwrap().is_none());
    }

    // --- DetectorRegistry + bootstrap ---

    struct AlwaysFiresDetector {
        key: &'static str,
        history: u32,
    }
    #[async_trait]
    impl Detector for AlwaysFiresDetector {
        fn key(&self) -> &'static str {
            self.key
        }
        fn require_history_weeks(&self) -> u32 {
            self.history
        }
        async fn detect(
            &self,
            ctx: &DetectorCtx<'_>,
        ) -> Result<Vec<DetectedPattern>, CeremonyError> {
            Ok(vec![DetectedPattern {
                iso_week: ctx.current_iso_week.clone(),
                pattern_key: self.key.to_string(),
                magnitude: 1.0,
                payload: serde_json::json!({"rule": self.key}),
            }])
        }
    }

    #[tokio::test]
    async fn registry_aggregates_multiple_detectors() {
        let (_tmp, conn) = open_test_db();
        let ctx = EngineCtx::new(conn.clone(), "retro-2026-W20".into(), "2026-W20".into());
        let reg = DetectorRegistry::new()
            .with(Arc::new(AlwaysFiresDetector { key: "a", history: 0 }))
            .with(Arc::new(AlwaysFiresDetector { key: "b", history: 0 }));
        let rows = PatternDetector::detect(&reg, &ctx).await.unwrap();
        assert_eq!(rows.len(), 2);
        let keys: Vec<_> = rows.iter().map(|p| p.pattern_key.as_str()).collect();
        assert!(keys.contains(&"a"));
        assert!(keys.contains(&"b"));
    }

    #[tokio::test]
    async fn registry_skips_detectors_with_insufficient_history() {
        // No rollup history seeded → weeks_of_history = 0.
        // Detector requires 4 → should be skipped.
        let (_tmp, conn) = open_test_db();
        let ctx = EngineCtx::new(conn.clone(), "retro-2026-W20".into(), "2026-W20".into());
        let reg = DetectorRegistry::new()
            .with(Arc::new(AlwaysFiresDetector {
                key: "needs_history",
                history: 4,
            }))
            .with(Arc::new(AlwaysFiresDetector {
                key: "needs_nothing",
                history: 0,
            }));
        let rows = PatternDetector::detect(&reg, &ctx).await.unwrap();
        // Only the no-history rule fires.
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].pattern_key, "needs_nothing");
    }

    #[tokio::test]
    async fn registry_with_enough_history_fires_all() {
        let (_tmp, conn) = open_test_db();
        seed_rollup(
            &conn,
            &[
                ("2026-W17", "proj-a", "x", 1.0),
                ("2026-W18", "proj-a", "x", 1.0),
                ("2026-W19", "proj-a", "x", 1.0),
            ],
        );
        let ctx = EngineCtx::new(conn.clone(), "retro-2026-W20".into(), "2026-W20".into());
        let reg = DetectorRegistry::new().with(Arc::new(AlwaysFiresDetector {
            key: "needs_3_weeks",
            history: 3,
        }));
        let rows = PatternDetector::detect(&reg, &ctx).await.unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[tokio::test]
    async fn registry_errors_when_ctx_is_not_engine_ctx() {
        // A custom CeremonyCtx that isn't EngineCtx should surface
        // as `Other` because the framework needs SQL access.
        struct DummyCtx;
        #[async_trait]
        impl CeremonyCtx for DummyCtx {
            fn period_key(&self) -> &str {
                "2026-W20"
            }
            fn tablet_id(&self) -> &str {
                "retro-2026-W20"
            }
            async fn write_pattern_row(
                &self,
                _pattern: DetectedPattern,
            ) -> Result<String, CeremonyError> {
                Ok("x".into())
            }
        }
        let reg = DetectorRegistry::new()
            .with(Arc::new(AlwaysFiresDetector { key: "a", history: 0 }));
        let err = PatternDetector::detect(&reg, &DummyCtx).await.unwrap_err();
        assert!(matches!(err, CeremonyError::Other(_)));
    }
}
