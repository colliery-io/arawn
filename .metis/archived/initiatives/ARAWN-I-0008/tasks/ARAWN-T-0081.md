---
id: hookrunner-and-engine-integration
level: task
title: "HookRunner and engine integration — matching, execution, PreToolUse/PostToolUse in query engine"
short_code: "ARAWN-T-0081"
created_at: 2026-04-04T02:16:26.120775+00:00
updated_at: 2026-04-04T02:35:00.408105+00:00
parent: ARAWN-I-0008
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0008
---

# HookRunner and engine integration — matching, execution, PreToolUse/PostToolUse in query engine

## Objective

Build the `HookRunner` that ties everything together: match hooks from config against an event, execute matching hooks via `CommandHookExecutor`, aggregate results. Wire it into the query engine so PreToolUse/PostToolUse hooks fire around every tool call.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `HookRunner::new(config: HookConfig)` constructor
- [ ] `HookRunner::run(event, tool_name, tool_input) -> AggregatedHookResult` async method
- [ ] Matches hooks from config by event type, then filters by matcher
- [ ] Executes all matching hooks in parallel (via `tokio::join!` or `futures::join_all`)
- [ ] Aggregation: any `Block` result → overall `Block`; otherwise `Allow`; `Warn` results collected
- [ ] PreToolUse integration: engine calls `hook_runner.run(PreToolUse, ...)` before `tool_executor.execute()`
- [ ] If PreToolUse returns `Block` → return error message to model, skip tool execution
- [ ] PostToolUse integration: engine calls `hook_runner.run(PostToolUse, ...)` after tool execution (informational, doesn't block)
- [ ] PostToolUse hook input includes tool output in addition to tool name and input
- [ ] Engine integration test: configure a blocking PreToolUse hook, verify tool call is rejected
- [ ] Engine integration test: configure a PostToolUse hook, verify it runs after tool execution

## Implementation Notes

- Depends on T-0078 (types), T-0079 (executor), T-0080 (config loading)
- This is the main integration task — touches the query engine's tool execution path
- Look at where `PermissionChecker::check()` is called in the engine — hooks should be called in a similar location, after permission checks pass
- Keep the HookRunner behind a trait for testability (mock in engine tests)

## Status Updates

*To be added during implementation*