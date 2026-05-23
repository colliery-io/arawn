//! Boot-time ceremony back-fill.
//!
//! On `arawn serve` boot, walk the most recent `lookback_days` days
//! and compose a tablet for each missed daily/weekly period. Retro
//! is skipped — its detectors depend on aggregated weekly history
//! and recovering a missed retro after the fact doesn't produce
//! anything the user can act on (ARAWN-I-0052).
//!
//! The loop is idempotent: dates with an existing tablet (any
//! status) are skipped via the dispatcher's idempotency check.
//! Per-iteration errors are logged and the loop continues — one
//! day's failure doesn't poison the rest.

use chrono::{Datelike, Duration, NaiveDate, Utc};
use tracing::{info, warn};

use crate::CeremonyError;
use crate::registry::PluginRegistry;
use crate::runner::{CeremonyDispatcher, DispatchOutcome};

/// Outcome counters for one back-fill pass — surfaced in the boot
/// log so users can see what happened.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BackfillReport {
    /// Tablets the back-fill loop composed for historical dates.
    pub composed: usize,
    /// Dates already covered by an existing tablet.
    pub already_present: usize,
    /// Dates that errored mid-dispatch. Errors are logged
    /// individually; this is just the count for the boot log.
    pub failed: usize,
}

/// Walk the `lookback_days` window for each registered daily/weekly
/// plugin, dispatching for any date whose tablet is missing.
///
/// Plugin order: daily first, then weekly. Daily back-fill
/// populates the rows weekly's gather might want to scan. Retro is
/// always excluded (`kind == "retro"`).
///
/// `lookback_days = 0` disables back-fill — useful as a kill switch
/// without removing the call site. Logs one info line and returns
/// an empty report.
pub async fn run(
    registry: &PluginRegistry,
    dispatcher: &dyn CeremonyDispatcher,
    lookback_days: u32,
) -> Result<BackfillReport, CeremonyError> {
    if lookback_days == 0 {
        info!("ceremony back-fill: disabled (lookback_days = 0)");
        return Ok(BackfillReport::default());
    }

    let today = Utc::now().date_naive();
    let cap = lookback_days as i64;
    let mut report = BackfillReport::default();

    // Process daily first, then weekly — daily writes the rows that
    // weekly's gather may want to scan.
    for kind in ["daily", "weekly"] {
        if registry.get(kind).is_none() {
            // Plugin not registered — nothing to back-fill.
            continue;
        }

        let dates: Vec<NaiveDate> = match kind {
            "daily" => (1..=cap)
                .filter_map(|n| today.checked_sub_signed(Duration::days(n)))
                .collect(),
            "weekly" => weekly_mondays_in_window(today, cap),
            _ => Vec::new(),
        };

        for date in dates {
            match dispatcher.dispatch_for(kind, date).await {
                Ok(DispatchOutcome::Generated { .. }) => {
                    report.composed += 1;
                    info!(kind, date = %date, "ceremony back-fill: composed");
                }
                Ok(DispatchOutcome::Skipped { .. }) => {
                    report.already_present += 1;
                }
                Err(e) => {
                    report.failed += 1;
                    warn!(kind, date = %date, error = %e, "ceremony back-fill iteration failed");
                }
            }
        }
    }

    info!(
        composed = report.composed,
        already_present = report.already_present,
        failed = report.failed,
        lookback_days = lookback_days,
        "ceremony back-fill complete",
    );
    Ok(report)
}

/// Enumerate the Mondays that fall in `[today - lookback, today - 1d]`.
///
/// Weekly tablets key off the ISO week starting Monday; back-filling
/// one Monday per missed week is enough to recover the calendar
/// without double-firing on cron-jittered runs.
fn weekly_mondays_in_window(today: NaiveDate, lookback_days: i64) -> Vec<NaiveDate> {
    let earliest = match today.checked_sub_signed(Duration::days(lookback_days)) {
        Some(d) => d,
        None => return Vec::new(),
    };
    let mut out = Vec::new();
    let mut cursor = today - Duration::days(1);
    // Walk backwards, snapping to Monday each iteration.
    while cursor >= earliest {
        // Snap cursor back to the Monday of its ISO week.
        let weekday = cursor.weekday().num_days_from_monday() as i64;
        let monday = cursor - Duration::days(weekday);
        if monday < earliest {
            break;
        }
        if !out.contains(&monday) {
            out.push(monday);
        }
        // Jump to the prior week's Sunday so the next iteration
        // snaps to that week's Monday.
        cursor = monday - Duration::days(1);
    }
    // Drop today's Monday if it slipped in — we never back-fill the
    // *current* period; the live cron owns that.
    let weekday = today.weekday();
    let this_monday = today - Duration::days(weekday.num_days_from_monday() as i64);
    out.retain(|m| *m != this_monday);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::{
        Ceremony, CeremonyCtx, CronSchedule, InteractiveAction, NewItem, PatternDetector,
    };
    use crate::registry::PluginRegistry;
    use crate::runner::{CeremonyDispatcher, DispatchOutcome};
    use crate::types::GatheredFacts;
    use async_trait::async_trait;
    use chrono::{DateTime, Weekday};
    use std::sync::Arc;
    use std::sync::Mutex;

    /// Recording dispatcher that captures every (kind, target) pair
    /// and lets the test decide which dates already exist.
    struct RecordingDispatcher {
        calls: Mutex<Vec<(String, NaiveDate)>>,
        already_present: Vec<(String, NaiveDate)>,
        fail_on: Vec<(String, NaiveDate)>,
    }

    impl RecordingDispatcher {
        fn new() -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                already_present: Vec::new(),
                fail_on: Vec::new(),
            }
        }
        fn with_already_present(mut self, kind: &str, date: NaiveDate) -> Self {
            self.already_present.push((kind.to_string(), date));
            self
        }
        fn with_failure(mut self, kind: &str, date: NaiveDate) -> Self {
            self.fail_on.push((kind.to_string(), date));
            self
        }
        fn calls(&self) -> Vec<(String, NaiveDate)> {
            self.calls.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl CeremonyDispatcher for RecordingDispatcher {
        async fn dispatch(&self, _kind: &str) -> Result<DispatchOutcome, CeremonyError> {
            unreachable!("back-fill never calls dispatch()")
        }
        async fn dispatch_for(
            &self,
            kind: &str,
            target: NaiveDate,
        ) -> Result<DispatchOutcome, CeremonyError> {
            self.calls
                .lock()
                .unwrap()
                .push((kind.to_string(), target));
            if self.fail_on.iter().any(|(k, d)| k == kind && *d == target) {
                return Err(CeremonyError::Other("synthetic test failure".into()));
            }
            if self
                .already_present
                .iter()
                .any(|(k, d)| k == kind && *d == target)
            {
                return Ok(DispatchOutcome::Skipped {
                    reason: "test fixture: already present".into(),
                });
            }
            Ok(DispatchOutcome::Generated {
                tablet_id: format!("{kind}-{target}"),
            })
        }
    }

    /// Minimal plugin stub used purely to register kinds with the
    /// PluginRegistry; back-fill never invokes its methods.
    struct PluginStub(&'static str);
    #[async_trait]
    impl Ceremony for PluginStub {
        fn kind(&self) -> &'static str {
            self.0
        }
        fn period_key(&self, _now: DateTime<Utc>) -> String {
            "stub".into()
        }
        fn period_window(
            &self,
            _period_key: &str,
        ) -> Result<(DateTime<Utc>, DateTime<Utc>), CeremonyError> {
            let now = Utc::now();
            Ok((now, now + Duration::days(1)))
        }
        fn default_schedule(&self) -> CronSchedule {
            CronSchedule::local("0 0 * * *")
        }
        fn interactive_actions(&self) -> Vec<InteractiveAction> {
            Vec::new()
        }
        fn patterns(&self) -> Option<&dyn PatternDetector> {
            None
        }
        async fn gather(&self, _ctx: &dyn CeremonyCtx) -> Result<GatheredFacts, CeremonyError> {
            Ok(GatheredFacts::new(serde_json::json!({})))
        }
        async fn compose(
            &self,
            _ctx: &dyn CeremonyCtx,
            _facts: GatheredFacts,
        ) -> Result<Vec<NewItem>, CeremonyError> {
            Ok(Vec::new())
        }
    }

    fn registry_with(kinds: &[&'static str]) -> PluginRegistry {
        let reg = PluginRegistry::new();
        for k in kinds {
            reg.register(Arc::new(PluginStub(k))).unwrap();
        }
        reg
    }

    #[tokio::test]
    async fn zero_lookback_disables() {
        let reg = registry_with(&["daily", "weekly"]);
        let disp = RecordingDispatcher::new();
        let report = run(&reg, &disp, 0).await.unwrap();
        assert_eq!(report, BackfillReport::default());
        assert!(disp.calls().is_empty());
    }

    #[tokio::test]
    async fn fourteen_day_default_composes_daily() {
        let reg = registry_with(&["daily"]);
        let disp = RecordingDispatcher::new();
        let report = run(&reg, &disp, 14).await.unwrap();
        assert_eq!(report.composed, 14);
        assert_eq!(report.already_present, 0);
        assert_eq!(report.failed, 0);
        let calls = disp.calls();
        assert_eq!(calls.len(), 14);
        for (kind, _) in &calls {
            assert_eq!(kind, "daily");
        }
    }

    #[tokio::test]
    async fn already_present_dates_dont_get_redispatched() {
        let reg = registry_with(&["daily"]);
        let today = Utc::now().date_naive();
        // Pretend last 3 days are already in the DB.
        let mut disp = RecordingDispatcher::new();
        for n in 1..=3 {
            disp = disp.with_already_present("daily", today - Duration::days(n));
        }
        let report = run(&reg, &disp, 14).await.unwrap();
        // 14 calls total: 3 skipped, 11 composed.
        assert_eq!(report.composed, 11);
        assert_eq!(report.already_present, 3);
        assert_eq!(disp.calls().len(), 14);
    }

    #[tokio::test]
    async fn iteration_failure_does_not_abort_loop() {
        let reg = registry_with(&["daily"]);
        let today = Utc::now().date_naive();
        let disp =
            RecordingDispatcher::new().with_failure("daily", today - Duration::days(5));
        let report = run(&reg, &disp, 14).await.unwrap();
        assert_eq!(report.composed, 13);
        assert_eq!(report.failed, 1);
        assert_eq!(disp.calls().len(), 14);
    }

    #[tokio::test]
    async fn retro_is_skipped() {
        // Even with retro registered, back-fill never touches it.
        let reg = registry_with(&["daily", "weekly", "retro"]);
        let disp = RecordingDispatcher::new();
        run(&reg, &disp, 14).await.unwrap();
        assert!(disp.calls().iter().all(|(k, _)| k != "retro"));
    }

    #[tokio::test]
    async fn weekly_enumerates_mondays_only() {
        let reg = registry_with(&["weekly"]);
        let disp = RecordingDispatcher::new();
        run(&reg, &disp, 14).await.unwrap();
        for (_, date) in disp.calls() {
            assert_eq!(
                date.weekday(),
                Weekday::Mon,
                "weekly back-fill must enumerate Mondays only, got {date} ({:?})",
                date.weekday()
            );
        }
    }

    #[tokio::test]
    async fn no_kind_registered_skips_silently() {
        // Empty registry → no calls, no errors.
        let reg = PluginRegistry::new();
        let disp = RecordingDispatcher::new();
        let report = run(&reg, &disp, 14).await.unwrap();
        assert_eq!(report, BackfillReport::default());
        assert!(disp.calls().is_empty());
    }

    #[tokio::test]
    async fn daily_runs_before_weekly() {
        // Plugin ordering: daily writes the rows weekly may scan.
        // Verify the call order in dispatcher invocations.
        let reg = registry_with(&["daily", "weekly"]);
        let disp = RecordingDispatcher::new();
        run(&reg, &disp, 14).await.unwrap();
        let calls = disp.calls();
        let first_weekly = calls.iter().position(|(k, _)| k == "weekly").unwrap();
        let last_daily = calls
            .iter()
            .enumerate()
            .filter(|(_, (k, _))| k == "daily")
            .map(|(i, _)| i)
            .max()
            .unwrap();
        assert!(
            last_daily < first_weekly,
            "all daily calls must precede every weekly call"
        );
    }
}
