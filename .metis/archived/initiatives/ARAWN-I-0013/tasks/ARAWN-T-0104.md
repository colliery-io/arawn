---
id: hooks-engine-integration-tests
level: task
title: "Hooks + engine integration tests"
short_code: "ARAWN-T-0104"
created_at: 2026-04-05T17:17:08.591847+00:00
updated_at: 2026-04-05T17:38:06.065619+00:00
parent: ARAWN-I-0013
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0013
---

# Hooks + engine integration tests

## Objective

Integration tests verifying hooks fire correctly when wired into the QueryEngine. Tests go in `crates/arawn-tests/tests/hooks.rs`.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Test: PreToolUse blocking hook (`exit 1` command) stops tool execution — ToolResult contains block message
- [ ] Test: PreToolUse allowing hook (`exit 0` command) permits tool execution
- [ ] Test: PostToolUse hook fires after tool completes (verify via side effect — writes to temp file)
- [ ] Test: hook with content pattern matching (regex on tool input JSON) — only fires when pattern matches
- [ ] Test: multiple hooks, one blocks → aggregated result blocks overall
- [ ] Test: no matching hooks → tool executes normally (fast path)
- [ ] All tests pass with `angreal test integration`

## Implementation Notes

### Key APIs
- `HookRunner::new(config: HookConfig, cwd: PathBuf)`
- `HookConfig` is serde-deserializable — construct via `serde_json::from_value(json!({...}))`
- `CommandHookExecutor::execute()` spawns subprocesses — use `echo`/`exit`/`touch` as hook commands
- `HookInput::PreToolUse { tool_name, tool_input }` / `HookInput::PostToolUse { ... }`

### Test pattern
```rust
let hook_config: HookConfig = serde_json::from_value(json!({
    "hooks": [{
        "event": "PreToolUse",
        "matcher": "shell",
        "command": "exit 1"
    }]
})).unwrap();
let runner = Arc::new(HookRunner::new(hook_config, tmp.path().into()));
let harness = TestHarness::builder()
    .with_tool(Box::new(ShellTool::default()))
    .with_hook_runner(runner)
    .with_script(vec![...])
    .build();
```

### PostToolUse side-effect verification
Use `touch <tempdir>/hook_fired` as the hook command, then assert the file exists after the test.

### Dependencies
Blocked by: ARAWN-T-0102 (TestHarnessBuilder extension)

## Status Updates

### 2026-04-05 — Complete
- Created `crates/arawn-tests/tests/hooks.rs` with 6 tests
- Note: blocking exit code is 2, not 1 (exit 1 = warn only). Updated tests accordingly.
- HookConfig structure uses event name as key: `{"PreToolUse": [{"matcher": "...", "hooks": [...]}]}`
- Content pattern matching uses glob syntax: `"shell(*secret*)"` matches tool input containing "secret"
- All 6 tests pass