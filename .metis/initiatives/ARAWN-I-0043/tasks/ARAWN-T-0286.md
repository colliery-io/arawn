---
id: pattern-detector-framework
level: task
title: "Pattern detector framework — pluggable Detector trait + ceremony_patterns_detected wiring"
short_code: "ARAWN-T-0286"
created_at: 2026-05-15T23:45:27.770490+00:00
updated_at: 2026-05-16T01:16:50.378480+00:00
parent: ARAWN-I-0043
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0043
---

# Pattern detector framework

## Goal
Pluggable framework that lets each ceremony register its own pattern detectors. Each detector is a `Fn(ctx) -> Vec<DetectedPattern>` that reads from rollup + tablets + patterns history and writes findings into `ceremony_patterns_detected`.

## Reference
I-0043 Plugin Contract `Ceremony::patterns()` + Compose Chain stage 2.

## Acceptance
- New `PatternDetector` trait:
  ```rust
  trait PatternDetector: Send + Sync {
      fn detectors(&self) -> Vec<Box<dyn Detector>>;
  }
  trait Detector: Send + Sync {
      fn key(&self) -> &'static str;
      fn detect(&self, ctx: &DetectorCtx) -> Result<Vec<DetectedPattern>, CeremonyError>;
  }
  ```
- Pipeline calls `plugin.patterns()` → for each detector, runs it and writes rows via `CeremonyCtx::write_pattern_row` (returns the row id for use as `citation_id` on dependent composed items).
- `DetectorCtx` exposes the rollup + tablet history queries detectors need.
- Bootstrap fallback: detectors that need ≥N weeks of history return empty when history is short, with a tracing debug.
- Tests: stub detector + synthetic rollup data; assert rows written + bootstrap path returns empty.

## Out of scope
The concrete pattern catalog for retro — T-0288.

## Notes
Deterministic stats, no LLM in detectors. Keep this strict — patterns are the load-bearing grounded layer.
## Status Updates

**2026-05-16 — implementation landed.**

T-0279 already shipped the engine-facing aggregator `PatternDetector` (one method, returns `Vec<DetectedPattern>`). T-0286 decomposed this into per-rule scaffolding:

- New `crates/arawn-ceremonies/src/patterns.rs`:
  - `Detector` trait — single rule, `key() + require_history_weeks() + detect(&DetectorCtx)`.
  - `DetectorCtx<'a>` — read-only ctx scoped to the current ISO week. Exposes:
    - `weeks_of_history()` — distinct prior weeks in `ceremony_activity_rollup`.
    - `metric_sum_trailing(workstream, metric_key, lookback_weeks)` — trailing sum.
    - `current_metric_value(workstream, metric_key)` — exact lookup (returns `Some(0.0)` for "we measured zero", `None` for "no row").
  - `DetectorRegistry` — aggregates a `Vec<Arc<dyn Detector>>` and implements `PatternDetector`. Plugins build one via `DetectorRegistry::new().with(rule_a).with(rule_b)` and return it from `Ceremony::patterns()`. The engine still calls the same aggregator method; the registry does the fan-out.
- **Bootstrap fallback** is in the registry, not per-rule. Before invoking each detector, the registry compares `detector.require_history_weeks()` against `DetectorCtx::weeks_of_history()` and skips with a tracing-debug when history is short. T-0288's catalog rules declare their lookback windows here; T-0287's retro plugin returns the registry from `patterns()`.

**Trait surface change:** added a default `fn conn_handle(&self) -> Option<&ConnHandle> { None }` to `CeremonyCtx`. `EngineCtx` overrides to return `Some`. This lets `DetectorRegistry` ask "do I have SQL access?" without a runtime downcast — stub ctxs return `None` and the registry errors with a clear message; production runs always have a real `EngineCtx`.

**Tests (8 new in `patterns::tests`, 42 total in the crate):**
- `weeks_of_history_counts_distinct_prior_weeks`
- `metric_sum_trailing_sums_lookback`
- `current_metric_value_returns_some_for_present_zero`
- `current_metric_value_returns_none_for_absent_row`
- `registry_aggregates_multiple_detectors`
- `registry_skips_detectors_with_insufficient_history` — bootstrap fallback verified.
- `registry_with_enough_history_fires_all`
- `registry_errors_when_ctx_is_not_engine_ctx` — confirms the conn-handle requirement.

Next: T-0287 (retro plugin — gather + compose chain).