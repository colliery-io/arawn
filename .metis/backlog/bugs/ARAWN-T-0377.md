---
id: ceremony-cron-registration-fails
level: task
title: "Ceremony cron registration fails with timezone='Local' — schedules never fire"
short_code: "ARAWN-T-0377"
created_at: 2026-05-21T12:45:00+00:00
updated_at: 2026-05-21T12:48:19.477913+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#bug"
  - "#ceremonies"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Ceremony cron registration fails with timezone='Local'

## Objective

The ceremony plugin defaults (and arawn.toml's `[ceremonies.<kind>]
timezone` fallback) pass the literal string `"Local"` to cloacina's
`register_cron_workflow`. Cloacina rejects it: `Configuration
error: Invalid cron expression or timezone: Invalid timezone:
Local`. Every ceremony's scheduled run is silently skipped on every
boot.

Server boot log shows three identical warnings:

```
WARN ceremony runner failed to register retro cron — manual runs still work
  error=cloacina register_cron_workflow for 'ceremony_retro':
  Configuration error: Invalid cron expression or timezone:
  Invalid timezone: Local
WARN ceremony runner failed to register daily cron — manual runs still work
WARN ceremony runner failed to register weekly cron — manual runs still work
```

This makes the "watch / catch up" model in I-0052 a no-op for the
forward-going path: only manual `daily_run`/`weekly_run`/`retro_run`
calls + boot back-fill compose tablets. The product loses its
scheduled-process pitch.

## Root cause

`crates/arawn-ceremonies/src/plugin.rs::CronSchedule::local` sets
`timezone` to the literal string `"Local"`. The binary's
override path (`crates/arawn/src/main.rs:1849`) falls back to
`"Local"` when `[ceremonies.<kind>] timezone` is unset. The string
is then forwarded verbatim into
`cloacina.register_cron_workflow(&name, &expr, &timezone)` at
`crates/arawn-ceremonies/src/runner.rs:182`.

Cloacina (and `chrono-tz` underneath) only accepts IANA zone names
(`UTC`, `America/Los_Angeles`, etc.) — not the convenience string
`"Local"`. There's existing parallel logic in
`crates/arawn/src/main.rs::resolve_ceremony_tz` that resolves the
same string for *gather-window math* (T-0364) — it converts
`"Local"` → `chrono_tz::UTC` so back-fill works. But the cron-side
string never went through that resolver.

## Fix

Normalize the timezone string at the runner boundary in
`register_one_with_schedule` before the cloacina call. When the
string is `"Local"` (case-insensitive) or empty, detect the OS
IANA zone via `iana-time-zone::get_timezone()` (already in the
workspace's dep tree via chrono-tz) and use that instead. Fall
back to `"UTC"` with a logged warning if detection fails.

Pass-through behaviour for explicit IANA strings stays unchanged.
Log the resolved zone so users can confirm it.

## Acceptance criteria

- [ ] `arawn-ceremonies` depends on `iana-time-zone` directly
  (rather than via chrono-tz's internal use).
- [ ] `register_one_with_schedule` resolves `"Local"` / empty to a
  real IANA zone via `iana_time_zone::get_timezone()`, with a UTC
  fallback + warning on detection failure.
- [ ] Unit test on the resolver: passes through explicit IANA,
  resolves `"Local"`/`""` to a non-`"Local"` IANA-shaped string.
- [ ] `info!("ceremony registered")` log line includes the
  resolved timezone alongside the schedule.
- [ ] Restart shows `ceremony registered` for daily / weekly /
  retro with **no** "register cron — manual runs still work"
  warnings.
- [ ] `angreal test unit` green. `angreal check workspace` green.

Surfaced during the I-0038 docs UAT setup, 2026-05-21.