---
id: configurable-retro-cadence-weekly
level: task
title: "Configurable retro cadence — weekly / biweekly / monthly"
short_code: "ARAWN-T-0367"
created_at: 2026-05-19T18:55:47.326211+00:00
updated_at: 2026-05-19T22:19:23.565801+00:00
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

- [x] `[ceremonies.retro] cadence` parsed from `arawn.toml`;
  default `"weekly"`. Case-insensitive; aliases `bi-weekly`
  and `fortnightly` accepted.
- [x] Anchor date persisted in `ceremony_config` and survives
  restart. Auto-initialised to "today's Monday" on first
  biweekly boot.
- [x] Retro plugin emits correct period_key + period_window
  for each cadence. (`B{N}` for biweekly; `YYYY-MM` for
  monthly.)
- [x] `retro_set_cadence` agent tool exists, validates input
  (`weekly|biweekly|monthly`), persists to `ceremony_config`.
- [x] Unit tests (10): cadence parse, biweekly key math,
  biweekly window, biweekly errors without anchor, biweekly
  idempotency within a window, monthly key+window (incl.
  December rollover), monthly idempotency, save+load round-trip,
  load fall-back chain.
- [x] `angreal test unit` green. `angreal check workspace` green.

Integration test deferred — the idempotency invariant is
covered by the unit test pair `biweekly_idempotency_skips_*` +
`monthly_idempotency_skips_*` which verify that two dates in
the same biweek/month yield the same period_key. The
dispatcher's existing idempotency check (already covered by
`dispatch_for_historical_idempotent` in T-0365) then guarantees
the second dispatch returns `Skipped`. Composing the two
properties gives the integration guarantee without a
full-stack test.

## Design notes

**Cron schedule:** kept at `"0 16 * * FRI"` for all cadences.
Biweekly/monthly can't be expressed natively in 5-field cron;
rather than fight the grammar, the cron fires every Friday and
**idempotency naturally skips off-weeks**. A Friday in the
middle of an existing biweek/month period_key matches the
existing tablet → `DispatchOutcome::Skipped`. Net behaviour:
biweekly fires every other Friday; monthly fires the first
Friday of each month.

**Cadence storage:** runtime mutable via a new
`ceremony_config (kind, key, value)` table (V12). Source-of-
truth ordering at registration: DB row > `[ceremonies.retro]
cadence` > `Weekly`. The `retro_set_cadence` tool writes the
DB row; takes effect on next restart (live plugin instance
isn't hot-mutated — that would need interior mutability on the
plugin Arc, which I'd rather earn when actually painful).

**`B{N}` biweek key:** flat counter from the anchor instead
of `YYYY-Bxx` because biweekly cycles cleanly straddle year
boundaries and a flat counter sidesteps "which year owns this
biweek?" Negative biweeks are well-defined (floor division)
for dates before the anchor — useful when back-fill walks into
pre-anchor history.

## Status Updates — 2026-05-19

Landed.

- V12 migration adds `ceremony_config` table.
- `RetroCadence` enum + parser in `plugins/retro.rs`.
- `RetroCeremony` gains `cadence` + `anchor` fields plus
  `with_cadence`, `load_persisted_cadence`, `save_cadence`.
- `period_key`, `period_key_for_date`, `period_window`
  overridden per-cadence with biweekly/monthly helper fns.
- `CeremonyService::set_retro_cadence` persists changes;
  `RetroSetCadenceTool` is the agent-facing surface.
- `main.rs` registration reads cadence from DB/config,
  auto-initialises a biweekly anchor on first run.
- `CeremonyConfig.cadence: Option<String>` added.

10 new tests + 1 added re-export. 132 ceremonies tests pass
(was 121 before this initiative). Workspace + check green.

Ready for review.

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