---
id: dispatch-for-kind-naivedate
level: task
title: "dispatch_for(kind, NaiveDate) + ceremony_tablets.recovered column"
short_code: "ARAWN-T-0365"
created_at: 2026-05-19T18:55:47.326211+00:00
updated_at: 2026-05-19T20:59:11.671794+00:00
parent: ARAWN-I-0052
blocked_by: [ARAWN-T-0364]
archived: false

tags:
  - "#task"
  - "#feature"
  - "#ceremonies"
  - "#phase/completed"


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

- [x] `EngineDispatcher::dispatch_for(kind, NaiveDate)` exists
  and composes a tablet stamped with `period_key` derived from
  the target date via `Ceremony::period_key_for_date`.
- [x] `dispatch(kind)` is a thin wrapper around
  `dispatch_for(kind, Utc::now().date_naive())`.
- [x] Migration V11 adds `ceremony_tablets.recovered` (INTEGER,
  NOT NULL, DEFAULT 0). Pre-existing tablets stay `0`.
- [x] `recovered = 1` on tablets whose target period_key differs
  from the live period_key; `0` otherwise.
  (Period-key comparison rather than raw date equality so
  weekly back-fill for a mid-week date doesn't get flagged when
  "today" is in the same ISO week.)
- [x] Idempotency unchanged.
- [x] `recovered` exposed on `TabletDto` (`#[serde(default)]` so
  older clients don't break) and read back via `row_to_tablet`.
- [x] Unit tests: `dispatch_for_today_marks_not_recovered`,
  `dispatch_for_historical_marks_recovered`,
  `dispatch_for_historical_idempotent`.
- [x] `angreal test unit` green. `angreal check workspace` green.

## Status Updates — 2026-05-19

Landed.

**Migration:** `V11__ceremony_tablets_recovered.sql` adds the
column with default 0; refinery picks it up automatically.

**Plugin trait:** new `Ceremony::period_key_for_date(NaiveDate)`
with a default impl that synthesises noon UTC and delegates to
`period_key`. Each plugin works with the default; per-plugin
overrides remain available for future weirdness.

**Dispatcher:** `CeremonyDispatcher::dispatch_for(kind, target)`
added to the trait (with a back-compat default that delegates to
`dispatch`). `EngineDispatcher`'s `dispatch` now delegates to
`dispatch_for(kind, today)`. The `recovered` decision is made by
comparing the target's period_key with the live period_key —
keeps the audit semantics correct under cadence-bearing plugins.

**Storage:** `insert_tablet` takes a `recovered: bool` and writes
to the new column.

**RPC surface:** `TabletDto.recovered: bool` (serde-default for
backwards compat). `row_to_tablet` reads column 7.

**Test stubs:** added `recovered: false` to all
out-of-DB `TabletDto` literals across render.rs and arawn-tui.
A new `DateAwarePlugin` stub in engine tests gives us a
period_key that varies with the target date (existing
`ScriptedPlugin` hard-codes a week).

**Verified:** 124 ceremonies tests pass (was 121, +3 new).
Full workspace unit suite green. `angreal check workspace`
green.

**Note on `today_local`:** I did not introduce a dedicated
helper. The dispatcher uses `Utc::now().date_naive()` and the
recovered determination is then made via period_key equivalence
through each plugin's own (tz-aware) period semantics. That
threads through the same `tz` set on the plugin in T-0364, so
no separate timezone drift is possible. T-0367 will revisit if
biweekly cadence wants a different "today" anchor.

Ready for review.

## Implementation notes

- Storage migration in `crates/arawn-storage/migrations/`.
- Update the tablet row-builder (`insert_tablet` in
  `arawn-ceremonies/src/engine.rs`) to take and write the flag.
- The "today" computation must use the same local-timezone
  source as T-0364's window code — share a helper
  (`fn today_local() -> NaiveDate`) so timezone semantics
  don't drift.

Parent: [[ARAWN-I-0052]].