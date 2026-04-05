---
id: permissionchecker-and-engine
level: task
title: "PermissionChecker and engine integration — check() before tool execution"
short_code: "ARAWN-T-0074"
created_at: 2026-04-03T02:48:46.986392+00:00
updated_at: 2026-04-03T10:18:10.619423+00:00
parent: ARAWN-I-0009
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0009
---

# PermissionChecker and engine integration — check() before tool execution

## Parent Initiative

[[ARAWN-I-0009]]

## Objective

Build the `PermissionChecker` struct that evaluates rules against a tool call, and wire it into the engine loop so every tool execution goes through a permission check first.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `PermissionChecker` struct holds `PermissionConfig` + `SessionGrants` + `PermissionMode`
- [ ] `check(tool_name, tool_input) -> PermissionDecision` method implementing: deny first → allow → ask → mode fallback
- [ ] When decision is `Ask`, calls `PermissionPrompt` trait to get user response
- [ ] When decision is `Denied`, returns a `ToolResult` error to the LLM explaining the tool was blocked
- [ ] Engine calls `permission_checker.check()` before `ToolExecutor::execute()` for every tool call
- [ ] `PermissionChecker` is async-compatible (prompt may need to await user input)
- [ ] Integration test: denied tool returns error, allowed tool executes, ask tool prompts (with mock prompter)

## Implementation Notes

### Technical Approach
- `PermissionChecker` lives in the permissions crate/module
- Engine integration point: the tool execution loop in `crates/arawn-core/src/engine/`
- The checker takes a `Box<dyn PermissionPrompt>` for asking the user — TUI provides the real impl, tests provide a mock
- Error returned to LLM should be clear: "Permission denied: tool X is not allowed by your permission settings"

### Dependencies
- Depends on T-0072 (rule types + matcher)
- Depends on T-0073 (config loading)

## Status Updates

- Created `crates/arawn-engine/src/permissions/checker.rs`
- `PermissionChecker` with rules, Mutex<SessionGrants>, optional Box<dyn PermissionPrompt>
- `check()` flow: session grants → deny rules → allow rules → ask (prompt or deny) → NoMatch (allow for now)
- `PermissionPrompt` async trait, `PermissionRequest`/`PermissionResponse` types
- `SessionGrants` with grant/is_granted/clear
- Engine integration: `QueryEngine::with_permission_checker()` + check in `execute_tool()` before tool.execute()
- Denied tools return clear error: "Permission denied: tool 'X' is not allowed by your permission settings."
- 11 checker tests + full workspace builds clean
- Note: PermissionMode not yet on checker — that's T-0075