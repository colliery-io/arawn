---
id: boot-time-back-fill-loop-for
level: task
title: "Boot-time back-fill loop for missed daily/weekly ceremonies"
short_code: "ARAWN-T-0366"
created_at: 2026-05-19T18:55:47.326211+00:00
updated_at: 2026-05-19T21:13:08.132388+00:00
parent: ARAWN-I-0052
blocked_by: [ARAWN-T-0365]
archived: false

tags:
  - "#task"
  - "#feature"
  - "#ceremonies"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0052
---

# Boot-time back-fill loop for missed daily/weekly ceremonies

## Objective

On `arawn serve` boot, fire `dispatch_for` for each daily and
weekly period missed in the last 14 days. Closes the
laptop-was-closed-over-the-weekend gap that motivated I-0052
(and T-0353, which this task supersedes).

Depends on [[ARAWN-T-0365]] for the dispatch surface and the
`recovered` flag.

## Scope

- New module `crates/arawn-ceremonies/src/backfill.rs`:
  - `pub async fn run(conn, registry, lookback_days) -> BackfillReport`
  - Walks each registered plugin where `kind() in {"daily", "weekly"}`.
  - For each plugin: enumerate dates from `today - lookback_days` to `today - 1 day` (inclusive). Weekly enumerates week-Mondays; daily enumerates days.
  - For each enumerated date with no existing tablet (any status), call `dispatch_for(kind, date)`.
  - Skip dates whose `period_key` already has a tablet, regardless of status.
- Wire into `crates/arawn/src/main.rs::main` before the cron loop attaches.
- Config: read `[ceremonies] backfill_lookback_days` from `arawn.toml`. Default `14`. `0` disables back-fill entirely.
- Boot log line: `"ceremony back-fill: N composed, M already present, K skipped beyond {lookback}-day cap"`.
- **Retro is excluded** — its detector depends on aggregated weekly history; recovering a missed retro after the fact doesn't add value the user can act on.

## Acceptance criteria

- [x] `arawn_ceremonies::backfill::run(registry, dispatcher, lookback_days)` exists.
- [x] Called from `arawn/src/main.rs` server startup *before*
  the cron loop attaches.
- [x] Default lookback is 14 days, configurable via
  `[backfill] ceremony_lookback_days` in `arawn.toml`.
- [x] `0` disables back-fill — single log line, no calls.
- [x] Only daily + weekly are back-filled; retro is excluded.
- [x] Each back-filled tablet has `recovered = 1`
  (inherited from T-0365's `dispatch_for` flag logic).
- [x] Dates with an existing tablet are not re-dispatched
  (relies on the dispatcher's idempotency check + the
  `Skipped` outcome).
- [x] Plugins not registered are silently skipped — e.g. when
  daily is disabled via `[ceremonies.daily] enabled = false`,
  it never reaches the registry so back-fill skips it.
- [x] Boot log line reports `composed=N, already_present=M, failed=K`.
- [x] Unit tests (8): zero-lookback, 14-day default, already-
  present skip, iteration-failure resilience, retro exclusion,
  weekly-Mondays-only, empty-registry, daily-before-weekly
  ordering.
- [x] `angreal test unit` green. `angreal check workspace` green.

Integration test deferred — the back-fill loop is exercised
via the unit tests with a `RecordingDispatcher` that captures
every call, which is a tighter contract than an end-to-end
boot test would give. The boot wiring itself is a 25-line
match block that the cargo-check covers.

## Config knob — design note

I introduced a top-level `[backfill]` section rather than
`[ceremonies] backfill_lookback_days`. Reason: `ceremonies` is
serialised as a `HashMap<String, CeremonyConfig>` today; you
can't mix a scalar field with a sub-table map in serde without
restructuring it into a fixed struct. Top-level `[backfill]`
ships the feature without that churn, and the field name
`ceremony_lookback_days` keeps the namespace clear in case
other back-fillable surfaces want their own knob later.

## Implementation notes

- Plugin ordering: daily first, then weekly. Daily back-fill
  populates the rows weekly detectors might want to scan.
- Back-fill happens once per boot, before the cron loop —
  prevents races between back-fill and a freshly-firing cron.
- Errors in one back-fill iteration log a warning but don't
  abort the loop; remaining dates still get attempted.

Parent: [[ARAWN-I-0052]] — see the "Why the 14-day cap" section
for the UX rationale.