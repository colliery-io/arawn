---
id: binary-wiring-daily-plugin-into
level: task
title: "Binary wiring — daily plugin into ceremony runner"
short_code: "ARAWN-T-0299"
created_at: 2026-05-16T14:00:00.000000+00:00
updated_at: 2026-05-16T14:00:00.000000+00:00
parent: ARAWN-I-0041
blocked_by:
  - ARAWN-T-0296
  - ARAWN-T-0297
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0041
---

# Binary wiring — daily plugin into ceremony runner

## Parent Initiative

[[ARAWN-I-0041]]

## Objective

Construct `DailyCeremony` in `main.rs` alongside `RetroCeremony`,
register on the shared `PluginRegistry`, apply
`[ceremonies.daily]` config overrides, register cron via the
runner. The ceremony engine, RPC dispatch, and event channel are
already in place from [[ARAWN-T-0292]]; this task slots a second
plugin into the existing wiring.

## Acceptance Criteria

- [ ] `main.rs`'s ceremony block constructs and registers
      `DailyCeremony` next to `RetroCeremony`:
      - Calendar + attention sources built once (production impls
        from [[ARAWN-T-0297]]).
      - `[ceremonies.daily]` config consulted: enabled-flag,
        model override, schedule override applied via
        `runner.register_one_with_schedule("daily", ...)`.
      - Default schedule (`0 7 * * MON-FRI` local) when no
        override.
- [ ] The current gate (`if workflow_runner && retro_enabled`)
      restructured so retro and daily enable independently:
      - Construct the shared infra (conn, plugin registry,
        dispatcher, service, runner, sweep task) when the workflow
        runner is up.
      - Each plugin's construction + tool registration guarded by
        its own `<kind>_enabled` flag.
- [ ] Default `arawn.toml` template grows a commented-out
      `[ceremonies.daily]` block matching the retro one.
- [ ] Binary builds; existing retro UAT continues to pass
      (regression-safety with no config present).

## Implementation Notes

### Technical Approach

1. Refactor the existing `if let Some(workflow_runner) = ... && retro_enabled`
   block so the workflow-runner gate guards the shared infra and
   per-plugin gates guard each plugin. Roughly:
   ```rust
   if let Some(workflow_runner) = ... {
       // build conn, registry, dispatcher, service, runner, sweep
       if retro_enabled { register retro + retro tools }
       if daily_enabled { register daily + daily tools }
   }
   ```
2. `register_one_with_schedule` from [[ARAWN-T-0295]] is the call
   site for both ceremonies. No new runner API needed.
3. The nightly sweep already only operates on retro tablets; leave
   it alone.

### Dependencies

- Blocked by [[ARAWN-T-0296]] (the `DailyCeremony` impl) and
  [[ARAWN-T-0297]] (the real source adapters).
- Unblocks [[ARAWN-T-0298]] (agent tools) and [[ARAWN-T-0300]]
  (UAT scenario).

### Risk Considerations

- **Refactor scope creep**: the gate restructure touches a
  load-bearing block of `main.rs`. Keep the diff focused on
  splitting the gates — no opportunistic refactors elsewhere.
- **Cron collision**: cloacina's workflow names are per-kind, so
  retro + daily live side by side without conflict. Already
  handled by `workflow_name(kind)` in runner.rs.

## Status Updates

*To be added during implementation*
