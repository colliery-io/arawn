---
id: skills-engine-integration-tests
level: task
title: "Skills + engine integration tests"
short_code: "ARAWN-T-0105"
created_at: 2026-04-05T17:17:09.410382+00:00
updated_at: 2026-04-05T17:41:19.391664+00:00
parent: ARAWN-I-0013
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0013
---

# Skills + engine integration tests

## Objective

Integration tests verifying skill loading and invocation through the engine via SkillTool. Tests go in `crates/arawn-tests/tests/skills.rs`.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Test: register skill in-memory, mock LLM calls `skill` tool with skill name, skill prompt content returned in ToolResult
- [ ] Test: write skill markdown to temp dir, load via `load_skills_dir()`, register, invoke through engine
- [ ] Test: skill not found → ToolResult contains error message
- [ ] Test: `user_invocable` filtering — only user-invocable skills returned by registry filter
- [ ] Test: plugin-namespaced skill (`plugin_name:skill_name`) accessible via SkillTool
- [ ] All tests pass with `angreal test integration`

## Implementation Notes

### Key APIs
- `SkillRegistry::new()` / `.register(skill)` / `.get(name)`
- `SkillDefinition { name, description, prompt, user_invocable, source, .. }`
- `SkillSource::Plugin(name)` for plugin-namespaced skills
- `load_skills_dir(dir, source)` — loads `.md` files from a directory
- `SkillTool::new(registry)` — the tool that the LLM calls

### Skill markdown format
```markdown
---
name: test-skill
description: A test skill
user_invocable: true
---
This is the skill prompt content that gets returned.
```

### Test pattern
```rust
let skill_registry = Arc::new(SkillRegistry::new());
skill_registry.register(SkillDefinition {
    name: "test-skill".into(),
    description: "test".into(),
    prompt: "Do the thing".into(),
    ..Default::default()
});
let harness = TestHarness::builder()
    .with_skill_registry(skill_registry)
    .with_script(vec![
        MockResponse::tool_call("c1", "skill", r#"{"name":"test-skill"}"#),
        MockResponse::text("Done"),
    ])
    .build();
```

### Dependencies
Blocked by: ARAWN-T-0102 (TestHarnessBuilder extension)

## Status Updates

### 2026-04-05 — Complete
- Created `crates/arawn-tests/tests/skills.rs` with 5 tests
- Updated `TestHarnessBuilder::build()` to auto-register `SkillTool` when a skill registry is provided (mirrors main.rs behavior)
- Tool name is `"Skill"` (capital S), params use `"skill"` key (not `"name"`)
- All 5 tests pass