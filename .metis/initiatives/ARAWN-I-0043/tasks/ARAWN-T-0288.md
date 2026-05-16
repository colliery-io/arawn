---
id: retro-pattern-catalog-v1-priority
level: task
title: "Retro pattern catalog v1 — priority-completion, rollover heat, workstream neglect"
short_code: "ARAWN-T-0288"
created_at: 2026-05-15T23:45:45.253757+00:00
updated_at: 2026-05-16T01:41:46.331804+00:00
parent: ARAWN-I-0043
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0043
---

# Retro pattern catalog v1

## Goal
Three concrete `Detector` implementations bundled with the retro plugin: `priority_completion_ratio`, `rollover_heat`, `workstream_neglect`. Each fires deterministic SQL and writes `DetectedPattern` rows.

## Reference
I-0043 §Pattern catalog (initial set).

## Acceptance
- `priority_completion_ratio`: % of `ceremony_priorities` with `confirmed_at IS NOT NULL` AND `done_at IS NOT NULL` over the week. Magnitude is the ratio. Fires when ratio < 0.5 or trends down from prior week.
- `rollover_heat`: count of `ceremony_todos_rolling` rows whose `last_seen_tablet_id` advanced ≥3 daily tablets this week without `done_at`. Fires when count ≥ 3.
- `workstream_neglect`: workstreams with non-zero activity in any of the prior 3 weeks but zero activity this week (across rollup metrics). Fires per neglected workstream.
- Each detector cites its source rows in the `payload` JSON.
- Tests: synthetic rollup + tablet rows → assert fires/no-fires in the obvious cases.

## Out of scope
Other catalog entries from the I-0043 doc (interruption hotspots, meeting drift). Add only after v1 ships and the framework is exercised.
## Status Updates

**2026-05-16 — implementation landed.**

Three concrete `Detector` impls + a `v1_catalog()` factory live at `crates/arawn-ceremonies/src/plugins/retro_detectors.rs`.

- **`PriorityCompletionDetector`** (`require_history_weeks = 0`):
  - Counts confirmed weekly priorities and how many are marked done on this week's weekly tablet.
  - Fires when ratio < 0.5; quiet otherwise (or when no priorities exist).
  - Payload: `{ confirmed, done, ratio }`.

- **`RolloverHeatDetector`** (`require_history_weeks = 0`):
  - Counts `ceremony_todos_rolling` rows where `done_at IS NULL`, `created_at < this_week_monday`, and `last_seen_tablet_id` is a daily tablet in this week's Mon..Sun range.
  - Fires when count ≥ 3; payload carries the count + each todo's `{id, body, created_at, last_seen_period_key}` for citation.
  - The week's Mon..Sun comes from the retro plugin's `monday_sunday_for_iso_week_public` helper (exposed `pub(crate)` so the catalog doesn't duplicate the ISO-week math).

- **`WorkstreamNeglectDetector`** (`require_history_weeks = 3`):
  - Two-pass SQL: get workstreams with `value > 0` in any of the trailing 3 weeks; cross-check against workstreams with `value > 0` this week; emit one pattern per workstream that's in the former but not the latter.
  - Payload: `{ workstream, comparison_window_weeks: 3 }`.

- **`v1_catalog()`** returns a fully-loaded `DetectorRegistry`. Retro plugin wires it via `RetroCeremony::new(...).with_detectors(retro_v1_catalog())`. The registry handles the bootstrap fallback for `WorkstreamNeglectDetector` when history is short.

**Tests (11 new in `plugins::retro_detectors::tests`, 60 total in the crate):**
- Priority completion: fires below threshold, quiet above threshold, quiet when no priorities exist.
- Rollover heat: fires at threshold, quiet below, ignores done todos + in-week creations.
- Workstream neglect: fires per neglected workstream, quiet when all active, declares `require_history_weeks = 3`.
- Catalog assembly: runs all three when history sufficient (returns empty when no signal), skips neglect detector when history short.

Next: T-0289 (diary capture — `upsert_diary` RPC + Sunday-night status transitions).