---
id: full-pipeline-integration-test-all
level: task
title: "Full pipeline integration test — all subsystems wired"
short_code: "ARAWN-T-0107"
created_at: 2026-04-05T17:17:11.666998+00:00
updated_at: 2026-04-05T17:51:15.331224+00:00
parent: ARAWN-I-0013
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0013
---

# Full pipeline integration test — all subsystems wired

## Objective

The capstone integration test — a multi-turn conversation with ALL subsystems (permissions, hooks, skills, tools) wired into the QueryEngine simultaneously. Verifies the subsystems compose correctly and don't interfere with each other. Tests go in `crates/arawn-tests/tests/full_pipeline.rs`.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Test: multi-turn conversation exercising all subsystems:
  - Turn 1: LLM calls `think` (allowed by permissions, no hook match) → succeeds
  - Turn 2: LLM calls `shell` (denied by permission rule) → denied, error fed back
  - Turn 3: LLM calls `skill` tool (skill registered) → skill prompt returned
  - Turn 4: LLM calls `file_read` (ask rule → mock allows, PreToolUse hook allows) → succeeds
- [ ] Verify permission checks fired for each tool call
- [ ] Verify hooks fired only for matching tools
- [ ] Verify full message history is correct (user, assistant, tool_result for each turn)
- [ ] Test passes with `angreal test integration`

## Implementation Notes

### Setup
```rust
// Permissions: allow think, deny shell, ask file_read
let rules = vec![
    PermissionRule::new(RuleKind::Allow, "think"),
    PermissionRule::new(RuleKind::Deny, "shell"),
    PermissionRule::new(RuleKind::Ask, "file_read"),
];
let checker = Arc::new(
    PermissionChecker::new(rules)
        .with_prompter(Box::new(MockModalPrompt::always(Some(0))))
);

// Hooks: PreToolUse on file_read → allow (exit 0)
let hook_config = serde_json::from_value(json!({
    "hooks": [{ "event": "PreToolUse", "matcher": "file_read", "command": "exit 0" }]
})).unwrap();
let runner = Arc::new(HookRunner::new(hook_config, tmp.path().into()));

// Skills
let skill_registry = Arc::new(SkillRegistry::new());
skill_registry.register(test_skill);

// Wire everything
let harness = TestHarness::builder()
    .with_tool(Box::new(ThinkTool))
    .with_tool(Box::new(ShellTool::default()))
    .with_tool(Box::new(FileReadTool))
    .with_permission_checker(checker)
    .with_hook_runner(runner)
    .with_skill_registry(skill_registry)
    .with_script(vec![/* 4 turns of mock responses */])
    .build();
```

### Dependencies
Blocked by: ARAWN-T-0102, ARAWN-T-0103, ARAWN-T-0104, ARAWN-T-0105, ARAWN-T-0106
(All prior test tasks should be complete so patterns are validated individually first)

## Status Updates

### 2026-04-05 — Complete
- Created `crates/arawn-tests/tests/full_pipeline.rs` with 1 comprehensive capstone test
- Exercises all 4 subsystems in a single multi-turn conversation:
  - think → allowed by permission + PostToolUse hook side-effect verified
  - shell → denied by permission rule
  - Skill → greet skill prompt returned
  - file_read → ask→allow + PreToolUse hook→allow
- Verifies full 10-message history (1 user + 4 tool rounds + 1 final)
- Full workspace suite: 0 failures