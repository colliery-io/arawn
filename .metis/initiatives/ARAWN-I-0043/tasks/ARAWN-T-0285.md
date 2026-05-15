---
---
id: activity-rollup-pipeline-end-of
level: task
title: "Activity rollup pipeline — end-of-period SQL aggregation into ceremony_activity_rollup"
short_code: "ARAWN-T-0285"
created_at: 2026-05-15T23:45:22.797071+00:00
updated_at: 2026-05-15T23:45:22.797071+00:00
parent: ARAWN-I-0043
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0043
---

# Activity rollup pipeline

## Goal
Deterministic SQL aggregation that, given an ISO week, computes per-workstream metrics and stores them in `ceremony_activity_rollup`. Runs end-of-week before retro generation. Generic — usable by any ceremony, not retro-specific.

## Reference
I-0043 §Data model — `ceremony_activity_rollup` + Compose Chain stage 2 setup.

## Acceptance
- New `ceremonies::rollup::compute_for_week(iso_week, conn)` function.
- Metrics computed in v1: `emails_sent`, `slack_threads_participated`, `meetings_attended`, `deep_work_hours`, `signals_extracted_count`, `steward_proposals_accepted`, `steward_proposals_rejected`.
- Sources: existing `feed_*` tables, `signal_*` tables, calendar feed, steward journal.
- Idempotent: re-running for the same week overwrites the rows (one transaction).
- Tests: seed synthetic feeds for one week, run compute, assert expected metric values.

## Out of scope
Pattern detection — that consumes the rollup but is a separate framework (T-0286).
Per-day rollups (only weekly in v1; daily ceremonies query feeds directly).
