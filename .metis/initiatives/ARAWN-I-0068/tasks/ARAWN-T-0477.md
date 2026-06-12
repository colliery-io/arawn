---
id: p2-1b-persisted-failure-history
level: task
title: "P2-1b: Persisted failure history — ceremony run-history table + steward error log"
short_code: "ARAWN-T-0477"
created_at: 2026-06-12T12:02:10.276962+00:00
updated_at: 2026-06-12T12:02:10.276962+00:00
parent: ARAWN-I-0068
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0068
---

# P2-1b: Persisted failure history — ceremony run-history table + steward error log

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0068]] — P2-1 (CRITICAL), storage half. Feeds persisted failure data into the `/status` surface ([[ARAWN-T-0476]]).

## Objective **[REQUIRED]**

Persist background-job failure history so a failed run is queryable after the fact (not just a vanished `warn!` line). Add a **ceremony run-history** table (per dispatch: ceremony, period key, outcome ok/error, error text, timestamp) and a **steward error log** (per failed subroutine pass: lens, subroutine, error, timestamp). `/status` (T-0476) reads the latest rows so "why is there no tablet today?" / "did the steward fail last night?" is answerable.

**The defect:** ceremony dispatch failure leaves no row, nothing queryable (`arawn-ceremonies/src/runner.rs:256-262`) — the user only sees "no tablet today". Steward subroutine errors increment a stat and are dropped; the journal records only successes (`arawn-steward/src/runner.rs:174-182`).

### Type
- [x] Feature — persistence backing the status surface

### Priority
- [x] P1 - High (the durable half of the keystone)

## Acceptance Criteria **[REQUIRED]**

- [ ] A migration adds a `ceremony_run_history` table; every ceremony dispatch writes a row with outcome (ok/error) + error text + timestamp + period key (success AND failure).
- [ ] A steward error log (table or append-only) records each failed subroutine pass (lens, subroutine, error, ts); the existing journal stays success-only.
- [ ] Both are bounded/prunable (don't grow unbounded) — cap rows or prune by age.
- [ ] `/status` (T-0476) surfaces the latest ceremony run outcome per ceremony and the recent steward errors.
- [ ] Inline tests: a failing ceremony dispatch writes an error row; a failing steward subroutine writes an error-log entry; both readable back.
- [ ] `angreal check all` and `angreal test unit` green.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Add a numbered SQL migration in `arawn-storage` (or the ceremonies DB) for `ceremony_run_history`; write the row from the ceremony runner's dispatch path (both branches). For steward, add an error-log writer alongside the journal in `arawn-steward`. Expose read helpers consumed by `LocalService::status()`.

### Dependencies
Pairs with [[ARAWN-T-0476]] (the surface reads these). Schema migration — mind the existing migration sequence and idempotency (cf. V8 backfill convention).

### Risk Considerations
Migration on existing user DBs must be additive/safe. Don't let history tables grow without bound. Writing history must not fail the ceremony/steward run itself (best-effort insert, logged on failure).

## Status Updates **[REQUIRED]**

*To be added during implementation*