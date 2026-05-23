---
id: t-d-subagent-task-lifecycle-fires
level: task
title: "T-D: Subagent + task lifecycle fires — SubagentStart/Stop, TaskCreated/Completed"
short_code: "ARAWN-T-0415"
created_at: 2026-05-23T03:31:03.000000+00:00
updated_at: 2026-05-23T03:31:03.000000+00:00
parent: ARAWN-I-0056
blocked_by: [ARAWN-T-0412]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0056
---

# T-D: Subagent + task lifecycle fires

## Parent Initiative

[[ARAWN-I-0056]]

## Objective

Add fire sites for the four subagent + background-task lifecycle events. All non-blocking.

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

*To be added during implementation*
