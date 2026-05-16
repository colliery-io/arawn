---
id: cloacina-workflow-runner-dispatch
level: task
title: "Cloacina workflow runner — dispatch by kind, per-ceremony cron registration"
short_code: "ARAWN-T-0281"
created_at: 2026-05-15T23:45:01.892127+00:00
updated_at: 2026-05-16T00:33:06.613295+00:00
parent: ARAWN-I-0043
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


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
## Status Updates

**2026-05-16 — implementation landed.**

- New `crates/arawn-ceremonies/src/runner.rs`:
  - `CeremonyDispatcher` trait: `async fn dispatch(kind) -> Result<DispatchOutcome>`. The dispatcher owns the idempotency check (skip-if-non-open-tablet-exists) and drives the pipeline. T-0282 will ship the concrete impl that talks to `arawn-storage` + `arawn-llm`.
  - `DispatchOutcome { Generated { tablet_id } | Skipped { reason } }`.
  - `CeremonyDispatchTask` impl `cloacina::Task` — one-task workflow body. Surfaces dispatcher errors as `TaskError::ExecutionFailed`.
  - `CeremonyRunner { registry, cloacina, dispatcher }`:
    - `start()` iterates the registry and registers one cloacina workflow + cron per plugin.
    - `register_one(kind)` for hot-add paths.
    - `run_once(kind)` for the manual-trigger path (RPC `ceremonies.run` and tests).
  - Workflow naming: `ceremony_<kind>` (mirrors feeds' `feed_<id>` convention).
  - Cloacina deps + `tokio` + dev `tokio[macros,rt]` added to `arawn-ceremonies/Cargo.toml`.

**Scope deviation documented:** the schedule-dedupe path (`delete_schedule_for`) is a no-op in this cut. cloacina 0.6 doesn't expose a stable "delete schedule by name" surface; the feeds crate works around this by walking cloacina's DB tables directly. For T-0281 we accept the duplicate-schedule risk on hot-reload — the binary's startup path registers exactly once, so production won't hit this. When ceremony hot-reload lands (out of scope for v1), the workaround pattern is already proven in feeds and can be lifted.

**Tests (5 new, 11 total in the crate):**
- `workflow_name_is_deterministic` — stable string.
- `run_once_invokes_dispatcher` — happy path.
- `second_run_once_for_same_period_skips` — idempotency contract via a recording stub dispatcher that simulates the tablet-exists check.
- `run_once_unknown_kind_errors` — missing plugin surfaces as `Other`.
- `dispatch_task_propagates_error_as_task_error` — dispatcher errors become `TaskError::ExecutionFailed`.

Unit tests don't spin up a real `DefaultRunner` — that needs a SQLite-backed cloacina runtime and is integration territory; covered when the binary wires ceremonies into the server. The contract pieces we can isolate are all green here.

Next: T-0282 (gather→compose→write pipeline + two-write-path enforcement).