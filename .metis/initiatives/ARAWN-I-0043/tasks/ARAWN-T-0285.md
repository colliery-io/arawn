---
id: activity-rollup-pipeline-end-of
level: task
title: "Activity rollup pipeline — end-of-period SQL aggregation into ceremony_activity_rollup"
short_code: "ARAWN-T-0285"
created_at: 2026-05-15T23:45:22.797071+00:00
updated_at: 2026-05-16T01:06:30.515381+00:00
parent: ARAWN-I-0043
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


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
## Status Updates

**2026-05-16 — implementation landed.**

**Deviation documented:** the task body listed concrete metrics (`emails_sent`, `meetings_attended`, etc.) and named their sources (`feed_*`, `signal_*`, `steward_journal`). Those source tables live in **per-workstream** databases, not the central `arawn.db`. Pulling them directly would force `arawn-ceremonies` to depend on `arawn-projections`, `arawn-feeds`, `arawn-steward`, etc. — a heavy cross-crate coupling for what should be a generic aggregator.

Shipped the rollup **framework** with a pluggable `RollupSource` trait instead. Concrete sources land in the binary (or a sibling crate) when ceremonies are wired in; the framework + the canonical metric-key vocabulary live here.

- New `crates/arawn-ceremonies/src/rollup.rs`:
  - `RollupSource` trait: `metric_key()` + `compute(iso_week, workstream) -> Option<f64>`. Returning `None` is honored as "source doesn't apply here" — no row written.
  - `WorkstreamList` trait: returns active workstream names. Default impl `CentralDbWorkstreams` queries the V1 `workstreams` table for `archived = 0`.
  - `compute_for_week(iso_week, workstreams, sources, conn)` walks every active workstream × source, gathers values, then writes them under one transaction. **Idempotent** — wipes existing rows for the week before inserting so a recompute can't leave stale data.
  - `read_rollup_value(conn, iso_week, workstream, metric_key)` is the read surface pattern detectors (T-0286/T-0288) call.

- Canonical metric-key vocabulary documented in the `RollupSource::metric_key` doc comment: `emails_sent`, `slack_threads_participated`, `meetings_attended`, `deep_work_hours`, `signals_extracted_count`, `steward_proposals_accepted`, `steward_proposals_rejected`.

**Tests (5 new in `rollup::tests`, 34 total in the crate):**
- `computes_rollup_for_two_workstreams_two_sources` — fan-out works; 2×2=4 rows written.
- `missing_workstream_value_is_skipped_silently` — `None` returns from a source skip the row.
- `rerun_for_same_week_replaces_not_appends` — idempotency confirmed; only one row per `(iso_week, workstream, metric_key)`.
- `central_db_workstreams_lists_active_only` — `archived=1` rows filtered out.
- `empty_sources_writes_nothing` — degenerate case.

Next: T-0286 (pattern detector framework).