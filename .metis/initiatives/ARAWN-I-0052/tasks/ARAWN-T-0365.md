---
id: dispatch-for-recovered-column
level: task
title: "dispatch_for(kind, NaiveDate) + ceremony_tablets.recovered column"
short_code: "ARAWN-T-0365"
created_at: 2026-05-19T18:55:47.326211+00:00
updated_at: 2026-05-19T18:55:47.326211+00:00
parent: ARAWN-I-0052
blocked_by: ["ARAWN-T-0364"]
archived: false

tags:
  - "#task"
  - "#feature"
  - "#ceremonies"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0052
---

# dispatch_for(kind, NaiveDate) + ceremony_tablets.recovered column

## Objective

Expose a `dispatch_for(kind, target: NaiveDate)` entry point on
the ceremony dispatcher so a tablet can be composed for an
arbitrary historical date, and add a `recovered` flag on
`ceremony_tablets` so the UI / API can distinguish recovered
tablets from live ones.

Depends on [[ARAWN-T-0364]] — gather must read pinned windows
before historical dispatch is meaningful.

## Scope

- Add `EngineDispatcher::dispatch_for(&self, kind: &str, target: NaiveDate) -> Result<DispatchOutcome>`.
- Convert the existing `dispatch(kind)` into `dispatch_for(kind, today_local())`.
- `period_key` for the dispatch comes from a date-taking variant on the plugin trait (or compute from `target.and_hms_opt(0,0,0)` and call the existing `period_key`).
- `generated_at` stays wall-clock (truthful audit trail).
- Migration: add `ceremony_tablets.recovered BOOLEAN NOT NULL DEFAULT 0` (sqlite). Set to `true` when `target != today_local()` at dispatch time.
- Surface `recovered` on the existing RPC tablet payload and the daily/weekly/retro show responses.

## Acceptance criteria

- [ ] `EngineDispatcher::dispatch_for(kind, NaiveDate)` exists
  and composes a tablet stamped with `period_key` derived from
  the target date.
- [ ] `dispatch(kind)` is a thin wrapper around
  `dispatch_for(kind, today_local())`.
- [ ] Migration adds `ceremony_tablets.recovered` with default
  `false`; pre-existing tablets stay `false`.
- [ ] `recovered = true` on tablets composed for a non-today
  target; `false` when `target == today_local()`.
- [ ] Idempotency unchanged: re-dispatching a date with an
  existing non-`open` tablet returns `Skipped`.
- [ ] `recovered` is exposed in the RPC tablet payload.
- [ ] Unit tests cover: live dispatch (recovered=false),
  historical dispatch (recovered=true), idempotency on
  historical re-dispatch.
- [ ] `angreal test unit` green. `angreal check workspace` green.

## Implementation notes

- Storage migration in `crates/arawn-storage/migrations/`.
- Update the tablet row-builder (`insert_tablet` in
  `arawn-ceremonies/src/engine.rs`) to take and write the flag.
- The "today" computation must use the same local-timezone
  source as T-0364's window code — share a helper
  (`fn today_local() -> NaiveDate`) so timezone semantics
  don't drift.

Parent: [[ARAWN-I-0052]].
