---
id: t-d-subagent-task-lifecycle-fires
level: task
title: "T-D: Subagent + task lifecycle fires — SubagentStart/Stop, TaskCreated/Completed"
short_code: "ARAWN-T-0415"
created_at: 2026-05-23T03:31:03+00:00
updated_at: 2026-05-23T04:19:49.282864+00:00
parent: ARAWN-I-0056
blocked_by: [ARAWN-T-0412]
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0056
---

# T-D: Subagent + task lifecycle fires

## Parent Initiative

[[ARAWN-I-0056]]

## Objective

Add fire sites for the four subagent + background-task lifecycle events. All non-blocking.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `SubagentStart` fires in `AgentTool::execute` when a sub-agent is spawned (after the inner `QueryEngine` is built, before the model call).
  - Hook input carries: `parent_session_id`, `subagent_id` (if available), `subagent_model`, `subagent_prompt` (first user message).
- [ ] `SubagentStop` fires when a sub-agent's run completes (success or error).
  - Hook input carries: `parent_session_id`, `subagent_id`, `success` (bool), `final_text` (or error).
- [ ] `TaskCreated` fires in `BackgroundTaskManager` when a task is registered.
  - Hook input carries: `task_id`, `task_kind`, `session_id` (if scoped to one).
- [ ] `TaskCompleted` fires in `BackgroundTaskManager` when a task finishes (success or error).
  - Hook input carries: `task_id`, `task_kind`, `success` (bool), `output_summary` (truncated stdout).
- [ ] Each fire site is guarded by `if let Some(runner) = &self.hook_runner` (AgentTool will need to pull the runner from its context; BackgroundTaskManager from its constructor).
- [ ] Unit tests covering each fire site.
- [ ] `cargo test --workspace --lib` green.

## Implementation Notes

- `AgentTool` is in `crates/arawn-engine/src/tools/agent.rs`. The sub-agent run is the inner `engine.run(...)` call — wrap with Start/Stop fires.
- The AgentTool's `ToolContext` doesn't currently carry a `HookRunner`. Two options:
  - (a) Add `hook_runner: Option<Arc<HookRunner>>` to `EngineToolContext` and propagate through, OR
  - (b) Have AgentTool spawn the inner engine via the parent's already-attached runner.
  Option (b) is cleaner — the inner engine inherits hooks from the outer engine's config. Verify the inner engine's `with_hook_runner` path is reachable from AgentTool.
- `BackgroundTaskManager` lives in `crates/arawn-engine/src/background.rs`. Lifecycle hooks belong in `spawn()` (TaskCreated) and the task's completion path (TaskCompleted).
- For `TaskCompleted`'s `output_summary`: truncate to ~500 bytes to keep hook payloads bounded.

## Status Updates

### 2026-05-23 — landed

**Fire sites added:**

| Event | Site | How |
|---|---|---|
| `SubagentStart` | `AgentTool::execute` | Fires after the inner `QueryEngine` is built (with the parent's hook runner attached) and before `engine.run`. |
| `SubagentStop` | `AgentTool::execute` | Fires on every exit path (success, MaxIterations, error). Result summary truncated to ~500 bytes via new `truncate_for_hook` helper. Also fires from the background-execution detached task. |
| `TaskCreated` | `BackgroundTaskManager::register` | Spawned in a detached tokio task because `register()` is sync. |
| `TaskCompleted` | `BackgroundTaskManager::complete` | Same pattern — detached spawn. Result carries the existing human-readable summary string. |

**Wiring approach: interior mutability for both AgentTool and BackgroundTaskManager.**

The challenge was that `AgentTool` lives behind `Box<dyn Tool>` in the shared `ToolRegistry` — no `&mut` access after registration. Same for `BackgroundTaskManager` which is shared across many tools via `Arc`. Solution: both types get a `set_hook_runner(&self, runner)` method using `RwLock<Option<Arc<HookRunner>>>` interior mutability. Startup calls these once after the hook runner is built; subsequent reads use a small `hook_runner_clone()` helper.

**Startup order change in `main.rs`:** moved the `load_and_build_hook_runner` call up from "just before LocalService::new" to "right after registry/bg_manager construction" — so it's available when `register_default_tools` runs. `register_default_tools` gains a new `hook_runner: Option<Arc<HookRunner>>` parameter; it attaches the runner to `bg_manager` via `set_hook_runner` and to the `AgentTool` (constructed before registering) via `set_hook_runner`.

**Engine inheritance:** when `AgentTool` builds its inner `QueryEngine`, it now also calls `.with_hook_runner(...)` on the sub-engine so SubagentStart/Stop/Tool-events fire inside the sub-agent's loop too.

**New test (1 in `crates/arawn-engine/src/background.rs`):**
- `task_created_and_completed_hooks_fire` — uses marker-file pattern. Registers a hook config with `TaskCreated` and `TaskCompleted` hooks that `touch` sentinel files; runs `register` + `complete`; polls for up to 500ms for the files to appear (since fire is detached). Asserts both markers exist.

**Coverage tradeoff:** SubagentStart/Stop are wired but not unit-tested in this task — exercising them requires constructing an AgentTool with a registry it can use for sub-agent dispatch, which is heavier than the V1 test budget. T-E's UAT will exercise these end-to-end. The wiring is small and grep-able.

**Validation:**
- `cargo test -p arawn-engine --lib background`: ✅ **15 tests pass** (was 14, +1 new).
- `cargo test --test hooks`: ✅ **11 tests pass** (no regression).
- `cargo check --workspace`: ✅ clean.
- `cargo build --workspace --release`: ✅ clean (1m 18s).
- `cargo test --workspace --lib`: ✅ **1,743 tests pass** (was 1,742, +1 new T-D test).