---
id: ceremony-gather-pinned-date
level: task
title: "Ceremony gather: pinned date windows derived from period_key"
short_code: "ARAWN-T-0364"
created_at: 2026-05-19T18:55:47.326211+00:00
updated_at: 2026-05-19T18:55:47.326211+00:00
parent: ARAWN-I-0052
blocked_by: []
archived: false

tags:
  - "#task"
  - "#refactor"
  - "#ceremonies"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0052
---

# Ceremony gather: pinned date windows derived from period_key

## Objective

Replace the `Utc::now() - Duration::X` pattern in the daily and
weekly gather paths with **pinned date windows** computed from
the tablet's `period_key`. Foundation for historical dispatch
(T-0365) and back-fill (T-0366). No user-visible behavior change
when running on the canonical cron tick.

## Scope

- Add `Ceremony::period_window(&self, period_key: &str) -> Result<(DateTime<Utc>, DateTime<Utc>), CeremonyError>` to the trait.
- Implement for `DailyCeremony`, `WeeklyCeremony`, `RetroCeremony`:
  - Daily: `period_key` is `YYYY-MM-DD` interpreted as **local**;
    window is `[date 00:00 local, (date+1) 00:00 local)` in UTC.
  - Weekly: `period_key` is `YYYY-Www` (ISO week); window is
    `[Monday 00:00 local, next Monday 00:00 local)` in UTC.
  - Retro: same as weekly. T-0367 will broaden this for biweekly /
    monthly.
- Expose `EngineCtx::period_window()` so gather paths can read it.
- Rewrite gather queries in `crates/arawn-ceremonies/src/plugins/{daily,weekly}.rs` to use `ctx.period_window().start` instead of `Utc::now() - Duration::X`.
- Local timezone resolution: source from runtime config (the same path `CronSchedule::local` already uses).

## Non-scope

- No new dispatch surface (T-0365).
- No back-fill loop (T-0366).
- Retro's existing weekly semantics unchanged here (T-0367
  expands them).

## Acceptance criteria

- [ ] `Ceremony::period_window` exists on the trait and is
  implemented by all three plugins.
- [ ] `EngineCtx::period_window()` exposed; gather paths read
  from it.
- [ ] No `Utc::now() - Duration` calls remain in `daily.rs`
  or `weekly.rs` gather logic.
- [ ] Unit tests verify each plugin's window is correct for an
  arbitrary `period_key` (not just "today").
- [ ] Existing daily/weekly gather tests pass unchanged when
  `period_key` corresponds to "today" — proves no behavior
  drift on the canonical cron path.
- [ ] `angreal test unit` green. `angreal check workspace` green.

## Implementation notes

- The DST edge case matters: `[date 00:00 local, (date+1) 00:00 local)` is *not* always 24h. Use `chrono`'s `Local::from_local_datetime` and pick the `single()` resolution; on ambiguous boundaries fall back to the earlier offset and log.
- Two tablets at the DST flip may have overlapping or short windows by design — that's the truthful answer; don't paper over it.
- `period_window` returns `Result` so retro can surface "unknown cadence" later without changing the trait surface.

Parent: [[ARAWN-I-0052]] — initiative defines the broader rationale and the 14-day cap.
