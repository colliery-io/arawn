---
id: tool-artifact-validation-tests
level: task
title: "Tool artifact validation tests — verify every tool that produces external artifacts"
short_code: "ARAWN-T-0169"
created_at: 2026-04-15T01:40:32.141965+00:00
updated_at: 2026-04-15T16:31:15.959285+00:00
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

# Tool artifact validation tests — verify every tool that produces external artifacts

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective
Add integration tests that call each artifact-producing tool directly and verify the output is valid — not just that the tool returned OK, but that the artifact it created actually works. The UAT found that `workflow_create` returned "success" patterns but the compiled output was broken. This class of bug can't be caught by unit tests or MockLLM tests.

## Backlog Item Details

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P1 - High (important for user experience)

### Technical Debt Impact
- **Current Problems**: `workflow_create` has been broken since it was built — nobody ever ran the generated Rust. MockLLM tests verify the tool accepts input and returns output, not that the output is correct. This gap applies to any tool that generates code, compiles artifacts, or mutates external state.
- **Benefits of Fixing**: Every tool that produces artifacts has a "golden path" test proving the output works. Regressions in scaffolding, templates, or external integrations are caught in CI.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `workflow_create`: call with a 2-task DAG, assert compilation succeeds, package.toml exists, dylib/binary produced (blocked by T-0168 — scaffold must be fixed first)
- [ ] `workflow_create`: call with the skill guide's "daily-summary" example verbatim, assert it compiles
- [ ] `workflow_list`: create a workflow, then list, assert it appears
- [ ] `workflow_delete`: create then delete, assert gone
- [ ] `file_write` + `file_read` roundtrip: write a file, read it back, assert content matches
- [ ] `file_edit`: write a file, edit it, read back, assert edit applied correctly
- [ ] `memory_store` + `memory_search` roundtrip with embeddings: store entity, search by paraphrase, assert found (already exists in memory_tools.rs — verify it passes with embedder)
- [ ] `shell`: run a command that produces output, assert output captured correctly
- [ ] All tests in `crates/arawn-tests/tests/tool_artifacts.rs` (new file)

## Implementation Notes

### Approach
Each test calls the tool's `execute()` method directly (not through MockLLM) with known-good parameters, then validates the produced artifact:

- **workflow_create**: needs a tempdir for the packages dir, calls execute, then checks for compiled output. This is slow (~30s compile time) so mark as `#[ignore]` for CI.
- **file tools**: use TestHarness workspace, write/edit/read back
- **memory tools**: already covered in `memory_tools.rs` — just verify embedder path works
- **shell**: execute `echo hello` and `ls`, verify output

### Key principle
The test doesn't validate that the tool parses parameters correctly (unit tests do that). It validates that the **output artifact is correct and usable** — the file compiles, the entity is searchable, the edit was applied.

### Dependencies
- T-0168 (workflow scaffold fix) must be done first for workflow_create tests to pass

## Status Updates
*To be added during implementation*