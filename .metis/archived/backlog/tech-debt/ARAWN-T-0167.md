---
id: skill-system-test-gaps-built-in
level: task
title: "Skill system test gaps — built-in loading, prompt injection, and skill-to-tool chaining"
short_code: "ARAWN-T-0167"
created_at: 2026-04-13T17:21:55.310036+00:00
updated_at: 2026-04-16T12:32:10.680920+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Skill system test gaps — built-in loading, prompt injection, and skill-to-tool chaining

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective
Close test coverage gaps in the skill system identified during UAT. Existing tests cover basic registration and invocation but miss built-in skill loading, system prompt injection, and the multi-step skill→tool chaining pattern that failed in production.

## Backlog Item Details

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P1 - High (important for user experience)

### Technical Debt Impact
- **Current Problems**: UAT showed the agent saw the "workflows" skill in the system prompt listing but never invoked it. We have no tests validating that built-in skills load, appear in the prompt, or chain into domain-specific tool use.
- **Benefits of Fixing**: Catch regressions in the skill→tool pipeline that directly affect agent capability. The work protocol prompt fix addresses the model behavior, but we need tests to ensure the plumbing stays intact.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Test: built-in "workflows" skill loads via `SkillRegistry::new()` — assert it's in `registry.all()` with correct name, description, and `user_invocable: true`
- [ ] Test: `format_skill_listing` includes built-in skills — assert the formatted string contains "workflows" and its description
- [ ] Test: skill listing appears in assembled system prompt — build a `SystemPromptBuilder` with a skill registry, assert the output contains the skill listing section
- [ ] Test: skill→tool chaining — MockLLM script: model calls `skill(name="workflows")` → gets workflow guide → then calls `workflow_create` with a valid workflow spec. Assert both tool calls occur and the workflow guide was returned as the skill output.
- [ ] Test: skill description matching scenarios — given a registry with 3+ skills, assert `format_skill_listing` produces descriptions that would help a model select the right skill for "create a daily pipeline" vs "commit my changes" vs "review this PR"

## Implementation Notes
- Add to `crates/arawn-tests/tests/skills.rs` (existing file, extend it)
- For the system prompt injection test, use `SystemPromptBuilder` directly (it's in `arawn-engine/src/system_prompt.rs`)
- For the chaining test, the MockLLM script needs 3 responses: skill call → workflow_create call → final text. The workflow_create tool needs `workflows_dir` which can be a tempdir.
- The built-in skill loading test can be a unit test in `skills/loader.rs` or an integration test

## Status Updates
*To be added during implementation*