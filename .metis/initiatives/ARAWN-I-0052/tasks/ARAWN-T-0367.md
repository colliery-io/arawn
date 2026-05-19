---
id: configurable-retro-cadence
level: task
title: "Configurable retro cadence — weekly / biweekly / monthly"
short_code: "ARAWN-T-0367"
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

# Configurable retro cadence — weekly / biweekly / monthly

## Objective

Make the retro cadence configurable (weekly / biweekly /
monthly) via `arawn.toml` and via an agent tool. Today
`RetroCeremony::default_schedule()` returns a hard-coded weekly
cron; users wanting biweekly or monthly retros have no knob.

Depends on [[ARAWN-T-0364]] — the pinned-windows refactor is
the foundation that lets retro's `period_window()` switch on
cadence cleanly.

## Scope

- Config: add `[ceremonies.retro] cadence = "weekly" | "biweekly" | "monthly"` (default `"weekly"`).
- For biweekly / monthly: anchor date is the first run after
  enablement, persisted in a new `ceremony_config` row so the
  cadence is stable across restarts.
- Retro plugin reads cadence at registration and adjusts three
  methods:
  - `default_schedule()` → corresponding cron expression.
  - `period_key()` → ISO week for weekly; `YYYY-Bxx` for
    biweekly (anchor-relative biweek number); `YYYY-MM` for
    monthly.
  - `period_window()` → corresponding window.
- New agent tool `retro_set_cadence` that writes the cadence
  knob into `arawn.toml` (or the runtime config, depending on
  where T-0283 landed) and re-registers the retro schedule.

## Acceptance criteria

- [ ] `[ceremonies.retro] cadence` parsed from `arawn.toml`;
  default is `"weekly"`.
- [ ] Anchor date persisted on first enablement of biweekly /
  monthly; survives restart.
- [ ] Retro plugin emits the correct schedule + period_key +
  period_window for each cadence.
- [ ] `retro_set_cadence` agent tool exists, validates input,
  and re-registers the retro schedule.
- [ ] Unit tests cover each cadence: schedule, period_key for
  an arbitrary date, period_window for an arbitrary period_key.
- [ ] Integration test: set biweekly, fire retro twice across
  the cadence boundary, confirm second run is skipped on the
  off-week.
- [ ] `angreal test unit` green. `angreal check workspace` green.

## Implementation notes

- The biweekly cron can't be expressed natively in 5-field cron
  ("every other Friday"). Easiest: register a weekly cron and
  have the retro plugin check anchor parity at dispatch time —
  if it's an off-week, return `Skipped`. Document the choice.
- Monthly cron similarly: weekly cron + a "last-Friday-of-month"
  runtime filter. Cleaner than fighting cron grammar.
- The anchor row in `ceremony_config` is keyed by `(kind,
  knob_name)`; future tasks may add more cadence-bearing
  ceremonies.

Parent: [[ARAWN-I-0052]].
