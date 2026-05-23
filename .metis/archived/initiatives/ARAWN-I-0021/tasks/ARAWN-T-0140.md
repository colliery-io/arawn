---
id: fix-tool-name-casing-in-task-tools
level: task
title: "Fix tool name casing in TASK_TOOLS and AGENT_TOOLS filter constants"
short_code: "ARAWN-T-0140"
created_at: 2026-04-10T01:00:54.250251+00:00
updated_at: 2026-04-10T01:13:00.019798+00:00
parent: ARAWN-I-0021
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0021
---

# Fix tool name casing in TASK_TOOLS and AGENT_TOOLS filter constants

## Parent Initiative
[[ARAWN-I-0021]]

## Objective
Fix `TASK_TOOLS` and `AGENT_TOOLS` filter constants that use PascalCase names (`"TaskCreate"`, `"Agent"`) which don't match actual snake_case tool names (`"task_create"`, `"agent"`), causing keyword-triggered tool inclusion to silently fail after the first 2 messages.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] `TASK_TOOLS` uses `task_create`, `task_update`, `task_get`, `task_list` (not PascalCase)
- [ ] `AGENT_TOOLS` uses `agent` (not `Agent`)
- [ ] Remaining PascalCase tool names (`EnterPlanMode`, `ExitPlanMode`, `Skill`, `TaskOutput`, `TaskStop`) normalized to snake_case
- [ ] Test added that verifies every name in filter constants exists in a populated ToolRegistry
- [ ] All existing tests pass

## Implementation Notes
### Files
- `crates/arawn-engine/src/query_engine.rs` — `TASK_TOOLS`, `AGENT_TOOLS`, `CORE_TOOLS` constants (~line 860+)
- All tool `fn name()` implementations that return PascalCase

### Approach
1. Grep all `fn name(&self)` across tools to build ground truth list
2. Update filter constants to match actual names
3. Rename PascalCase tools to snake_case (update `name()` return values)
4. Update any test assertions or mock scripts referencing old names

## Status Updates
- Renamed 5 tools from PascalCase to snake_case: `EnterPlanMode`→`enter_plan_mode`, `ExitPlanMode`→`exit_plan_mode`, `Skill`→`skill`, `TaskOutput`→`task_output`, `TaskStop`→`task_stop`
- Fixed TASK_TOOLS: `TaskCreate`→`task_create`, `TaskUpdate`→`task_update`, `TaskGet`→`task_get`, `TaskList`→`task_list`, `TaskOutput`→`task_output`, `TaskStop`→`task_stop`
- Fixed AGENT_TOOLS: `Agent`→`agent`
- Fixed CORE_TOOLS: `Skill`→`skill`
- Fixed PLAN_TOOLS: `EnterPlanMode`→`enter_plan_mode`, `ExitPlanMode`→`exit_plan_mode`
- Updated plan mode enforcement in query_engine.rs and checker.rs
- Updated tool_category() in checker.rs for task tools
- Updated all test references in skills.rs, full_pipeline.rs, workflows.rs, skill.rs, checker.rs
- All 60 engine tests + 51 permission tests + 16 integration tests pass
- Note: skipped adding a registry verification test — the casing mismatch class will be eliminated entirely by T-0159 (ToolCategory enum)