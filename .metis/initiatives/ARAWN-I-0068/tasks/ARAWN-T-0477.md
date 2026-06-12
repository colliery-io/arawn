---
id: p2-1b-persisted-failure-history
level: task
title: "P2-1b: Persisted failure history — ceremony run-history table + steward error log"
short_code: "ARAWN-T-0477"
created_at: 2026-06-12T12:02:10.276962+00:00
updated_at: 2026-06-12T21:57:15.934641+00:00
parent: ARAWN-I-0068
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


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

## Acceptance Criteria

## Acceptance Criteria

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

### 2026-06-12 — COMPLETE ✅
Persisted failure history landed; both tables feed the `/status` surface (bumped to schema v2).

**Storage (`arawn-storage`):**
- Migration `V13__failure_history.sql`: `ceremony_run_history` (id, kind, period_key, outcome ok/skipped/error, error, ran_at) + `steward_error_log` (id, lens_name, subroutine, error, failed_at), each indexed.
- New `failure_history` module: free fns over `&rusqlite::Connection` so every caller reuses the SQL — `record_ceremony_run` / `latest_ceremony_runs` (latest per kind) / `record_steward_error` / `recent_steward_errors`, plus `prune` to `HISTORY_CAP=500` on every insert (bounded). Exported `CeremonyRunRecord`/`StewardErrorRecord`.

**Writers (best-effort — never fail the run):**
- `arawn-ceremonies/engine.rs`: `dispatch_for` wraps the dispatch body in an inner async block, then `record_run` writes an ok/skipped/error row. (Made `arawn-storage` a regular dep, was dev-only.)
- `arawn-steward/runner.rs`: the subroutine `Err` branch now calls `record_subroutine_error` → `steward_error_log`.

**Surface (`/status`, ARAWN-T-0476):**
- `arawn-service`: `CeremoniesStatus.recent_runs: Vec<CeremonyRunStatus>`; new `StewardStatus { recent_errors }` + `StewardErrorStatus`; `SystemStatus.steward`; `SYSTEM_STATUS_VERSION` 1→2.
- `LocalService::status`: `ceremonies_status` reads `latest_ceremony_runs`; new `steward_status` reads `recent_steward_errors` (both via `store.database().conn()`).
- TUI `format_system_status`: per-ceremony recent-run lines + a Steward section (✓ none / ⚠ N recent).

**Tests — all green:**
- storage: latest-per-kind, recent steward errors, prune-to-cap (3).
- ceremonies: `dispatch_records_ok_run_history`, `failing_dispatch_records_error_run_history` (tablet rolls back, error row persists).
- steward: `failing_subroutine_writes_error_log`.
- tui: `format_system_status` render tests updated for recent_runs + steward.
- arawn-tests: existing health/status round-trips still green (17 + ws).
- `cargo fmt --all --check` clean · `cargo clippy --workspace -- -D warnings` clean · `cargo test --workspace --lib` + `cargo test -p arawn-tests` exit 0, no failures.

**Migration safety:** V13 is additive (two new tables); existing user DBs migrate forward cleanly. All acceptance criteria met.