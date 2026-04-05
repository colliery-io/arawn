---
id: hot-reload-api-integration-tests
level: task
title: "Hot-reload API integration tests"
short_code: "ARAWN-T-0108"
created_at: 2026-04-05T17:17:12.342303+00:00
updated_at: 2026-04-05T17:48:47.564029+00:00
parent: ARAWN-I-0013
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0013
---

# Hot-reload API integration tests

## Objective

Integration tests verifying the hot-reload APIs on PermissionChecker work correctly mid-session. Tests go in `crates/arawn-tests/tests/hot_reload.rs`. These test the programmatic API (`update_rules()`, `update_mode()`), NOT the filesystem ConfigWatcher.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Test: `update_rules()` changes permission behavior — tool denied before update, allowed after
- [ ] Test: `update_mode()` changes behavior — Default mode denies shell, switch to BypassPermissions allows it
- [ ] Test: engine uses updated rules on next tool call without restart
- [ ] Test: session grants cleared on mode change (if applicable)
- [ ] All tests pass with `angreal test integration`

## Implementation Notes

### Key APIs
- `PermissionChecker::update_rules(rules: Vec<PermissionRule>)` — hot-swaps rules via RwLock
- `PermissionChecker::update_mode(mode: PermissionMode)` — hot-swaps mode via RwLock
- Both are `&self` methods (no `&mut self`) — safe to call while engine holds `Arc<PermissionChecker>`

### Test pattern
```rust
let checker = Arc::new(
    PermissionChecker::new(vec![PermissionRule::new(RuleKind::Deny, "shell")])
);

// Turn 1: shell denied
let harness = TestHarness::builder()
    .with_tool(Box::new(ShellTool::default()))
    .with_permission_checker(checker.clone())
    .with_script(vec![
        MockResponse::tool_call("c1", "shell", r#"{"command":"echo hi"}"#),
        MockResponse::text("denied"),
    ])
    .build();
let result = harness.run("run echo").await;
// Assert denied

// Hot-reload: allow shell
checker.update_rules(vec![PermissionRule::new(RuleKind::Allow, "shell")]);

// Turn 2: shell allowed (same checker, updated rules)
// ... run again with new script, assert allowed
```

### Dependencies
Blocked by: ARAWN-T-0102 (TestHarnessBuilder extension)

## Status Updates

### 2026-04-05 — Complete
- Created `crates/arawn-tests/tests/hot_reload.rs` with 3 tests
- Note: session grants clearing on mode change not tested separately — `update_mode` doesn't auto-clear grants (separate concern)
- All 3 tests pass