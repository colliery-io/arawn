---
id: boot-time-backfill
level: task
title: "Boot-time back-fill loop for missed daily/weekly ceremonies"
short_code: "ARAWN-T-0366"
created_at: 2026-05-19T18:55:47.326211+00:00
updated_at: 2026-05-19T18:55:47.326211+00:00
parent: ARAWN-I-0052
blocked_by: ["ARAWN-T-0365"]
archived: false

tags:
  - "#task"
  - "#feature"
  - "#ceremonies"
  - "#phase/todo"


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

- [ ] `backfill::run` exists and is called from server startup.
- [ ] Default lookback is 14 days; `[ceremonies] backfill_lookback_days` overrides.
- [ ] `0` disables back-fill (single log line saying so).
- [ ] Only daily + weekly are back-filled; retro is skipped.
- [ ] Each back-filled tablet has `recovered = true`.
- [ ] Dates with an existing tablet (any status) are not redispatched.
- [ ] Back-fill respects per-ceremony `enabled = false` config.
- [ ] Boot log line reports N/M/K counts.
- [ ] Unit tests cover: empty DB → composes all 14 days for daily; DB with last 3 days present → composes only the older 11; lookback=0 → composes nothing; weekly enumerates Mondays correctly.
- [ ] Integration test: end-to-end boot path actually invokes back-fill (test harness in `crates/arawn-tests`).
- [ ] `angreal test unit` green. `angreal check workspace` green.

## Implementation notes

- Plugin ordering: daily first, then weekly. Daily back-fill
  populates the rows weekly detectors might want to scan.
- Back-fill happens once per boot, before the cron loop —
  prevents races between back-fill and a freshly-firing cron.
- Errors in one back-fill iteration log a warning but don't
  abort the loop; remaining dates still get attempted.

Parent: [[ARAWN-I-0052]] — see the "Why the 14-day cap" section
for the UX rationale.
