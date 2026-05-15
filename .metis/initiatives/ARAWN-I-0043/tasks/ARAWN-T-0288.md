---
---
id: retro-pattern-catalog-v1-priority
level: task
title: "Retro pattern catalog v1 — priority-completion, rollover heat, workstream neglect"
short_code: "ARAWN-T-0288"
created_at: 2026-05-15T23:45:45.253757+00:00
updated_at: 2026-05-15T23:45:45.253757+00:00
parent: ARAWN-I-0043
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


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
