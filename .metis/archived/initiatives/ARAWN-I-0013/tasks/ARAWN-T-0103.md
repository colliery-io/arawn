---
id: permission-engine-integration-tests
level: task
title: "Permission + engine integration tests"
short_code: "ARAWN-T-0103"
created_at: 2026-04-05T17:17:07.609544+00:00
updated_at: 2026-04-05T17:36:09.605487+00:00
parent: ARAWN-I-0013
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0013
---

# Permission + engine integration tests

## Objective

Integration tests verifying the permission system works correctly when wired into the QueryEngine. Tests go in `crates/arawn-tests/tests/permissions.rs`.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Test: deny rule blocks tool call — engine receives error message, LLM gets "permission denied" in ToolResult
- [ ] Test: allow rule permits tool call — tool executes normally
- [ ] Test: `BypassPermissions` mode auto-allows all tools including write operations
- [ ] Test: `AcceptEdits` mode allows file_write/file_edit but blocks shell
- [ ] Test: ask rule with `MockModalPrompt` allowing → tool executes
- [ ] Test: ask rule with `MockModalPrompt` denying → tool blocked
- [ ] Test: session grants — allow-always on ask persists across turns in same session
- [ ] All tests pass with `angreal test integration`

## Implementation Notes

### Key APIs
- `PermissionChecker::new(rules).with_mode(mode).with_prompter(prompter)`
- `PermissionRule::new(RuleKind::Deny, "shell")` — rule construction
- `MockModalPrompt::always(Some(0))` for allow, `MockModalPrompt::always(None)` for deny
- `PermissionMode::Default`, `AcceptEdits`, `BypassPermissions`

### Test pattern
```rust
let checker = Arc::new(
    PermissionChecker::new(vec![PermissionRule::new(RuleKind::Deny, "shell")])
        .with_prompter(Box::new(MockModalPrompt::always(None)))
);
let harness = TestHarness::builder()
    .with_tool(Box::new(ShellTool::default()))
    .with_permission_checker(checker)
    .with_script(vec![
        MockResponse::tool_call("c1", "shell", r#"{"command":"echo hi"}"#),
        MockResponse::text("Tool was denied"),
    ])
    .build();
let result = harness.run("run echo").await;
// Assert ToolResult contains permission denied message
```

### Dependencies
Blocked by: ARAWN-T-0102 (TestHarnessBuilder extension)

## Status Updates

### 2026-04-05 — Complete
- Created `crates/arawn-tests/tests/permissions.rs` with 7 tests covering all acceptance criteria
- All 7 tests pass