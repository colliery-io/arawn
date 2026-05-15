---
---
id: cloacina-workflow-runner-dispatch
level: task
title: "Cloacina workflow runner — dispatch by kind, per-ceremony cron registration"
short_code: "ARAWN-T-0281"
created_at: 2026-05-15T23:45:01.892127+00:00
updated_at: 2026-05-15T23:45:01.892127+00:00
parent: ARAWN-I-0043
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0043
---

# Cloacina workflow runner — dispatch by kind

## Goal
On startup, the ceremony engine asks the `PluginRegistry` for every registered ceremony, reads each plugin's `default_schedule()`, and registers a cloacina workflow per plugin. Each workflow execution dispatches `gather → pattern_detect (optional) → compose → write` for one plugin instance.

## Reference
I-0043 Sequencing §1 + Plugin Contract.

## Acceptance
- `CeremonyRunner` constructed with a `PluginRegistry` and a cloacina handle.
- On `start`: registers one workflow per plugin; cron pulled from `plugin.default_schedule()`.
- Each workflow fires `runner.run(plugin_kind)` which:
  1. computes `period_key(now)`
  2. checks if a tablet for that `(kind, period_key)` already exists; if yes and `status != open`, skip (idempotent re-run protection)
  3. drives the pipeline (delegated to T-0282)
- Manual trigger via `runner.run_once(kind)` callable from the RPC layer (T-0283).
- Tests: dummy plugin with 5s cron, fake clock; assert workflow fires + idempotent re-run skips.

## Out of scope
- Per-plugin config overrides for cron (later; the RPC `config_update_schedule` method lands with T-0283).
- The actual gather/compose pipeline (T-0282).

## Notes
Cloacina is already in the workspace via `arawn-feeds`. Reuse its runner — no new scheduler.
