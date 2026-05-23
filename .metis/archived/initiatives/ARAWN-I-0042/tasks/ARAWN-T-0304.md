---
id: binary-wiring-weekly-plugin-into
level: task
title: "Binary wiring — weekly plugin into ceremony runner"
short_code: "ARAWN-T-0304"
created_at: 2026-05-16T16:38:05.817688+00:00
updated_at: 2026-05-16T16:57:57.202115+00:00
parent: ARAWN-I-0042
blocked_by: [ARAWN-T-0301, ARAWN-T-0303]
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0042
---

# Binary wiring — weekly plugin

## Parent Initiative

[[ARAWN-I-0042]]

## Objective

Slot the weekly plugin into the existing ceremony wiring in
`main.rs` next to retro + daily. Same pattern as
[[ARAWN-T-0299]] but for the third plugin kind.

## Acceptance Criteria

## Acceptance Criteria

- [ ] `main.rs` ceremony block constructs and registers
      `WeeklyCeremony` alongside retro + daily:
      - `[ceremonies.weekly]` config consulted for enabled,
        model, schedule.
      - Calendar + attention sources reused (already built once
        for daily).
      - `runner.register_one_with_schedule("weekly", ...)`.
      - Seven `weekly_*` tools registered under
        `weekly_actually_enabled`.
- [ ] Outer guard extended to
      `if workflow_runner && (retro || daily || weekly)`.
- [ ] Default `arawn.toml` template grows a commented-out
      `[ceremonies.weekly]` block.
- [ ] Binary builds; existing retro + daily UATs continue to
      pass (regression-safety with no config present).

## Implementation Notes

### Technical Approach

1. Extend the existing per-plugin gate pattern from
   [[ARAWN-T-0299]]. Adding a third arm is mechanical.
2. The `daily_actually_enabled` projections-required gate exists
   because the daily plugin needs `Arc<ProjectionStore>`. Weekly
   has the same dependency for its `CalendarSource` +
   `AttentionSource` — apply the same gate.
3. The new keyword set `weekly|priority|priorities|week` is
   added to query_engine.rs alongside the existing
   `retro|ceremony|standup|diary|daily|today|brief` set.

### Dependencies

- Blocked by [[ARAWN-T-0301]] (plugin) and [[ARAWN-T-0303]]
  (tools).
- Unblocks [[ARAWN-T-0305]] (UAT).

### Risk Considerations

- **Three-way gate complexity**: at this point the ceremony
  wiring block is ~150 lines of conditional construction. Worth
  considering a `register_ceremony(kind, plugin, ...)` helper
  to keep main.rs readable. Defer to a follow-up unless this
  task balloons.

## Status Updates

### 2026-05-16 — weekly wired into binary

- `main.rs` ceremony block extended for the third plugin:
  `weekly_cfg`/`weekly_enabled`/`weekly_actually_enabled` flags
  threaded through alongside retro + daily. Outer gate now
  fires when any of the three is enabled.
- Hoisted the calendar + attention `Arc<dyn>` source pair out
  of the daily block so weekly reuses the same constructions
  (no duplicated `ProjectionsCalendarSource::new` calls).
- WeeklyCeremony registered on the shared `PluginRegistry`,
  cron registered via
  `runner.register_one_with_schedule("weekly", sched)`, all 7
  weekly_* tools registered under
  `weekly_actually_enabled`.
- `info!` line on success reports retro + daily + weekly flags.
- `arawn.toml` template grows a commented-out
  `[ceremonies.weekly]` block matching retro/daily.
- Binary compiles clean; arawn-engine lib suite (657 incl.
  weekly tools) green; arawn lib (57) green.

Completed 2026-05-16.