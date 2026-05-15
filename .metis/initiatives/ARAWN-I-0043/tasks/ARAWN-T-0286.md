---
---
id: pattern-detector-framework
level: task
title: "Pattern detector framework — pluggable Detector trait + ceremony_patterns_detected wiring"
short_code: "ARAWN-T-0286"
created_at: 2026-05-15T23:45:27.770490+00:00
updated_at: 2026-05-15T23:45:27.770490+00:00
parent: ARAWN-I-0043
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


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
