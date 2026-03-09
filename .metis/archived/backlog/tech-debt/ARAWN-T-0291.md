---
id: add-pipeline-factory-tests
level: task
title: "Add pipeline factory tests"
short_code: "ARAWN-T-0291"
created_at: 2026-03-08T20:21:13.183833+00:00
updated_at: 2026-03-08T23:44:46.635452+00:00
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

# Add pipeline factory tests

## Objective

`PipelineFactory` (creates pipelines from workflow definitions) has zero test coverage. Add tests covering factory creation, workflow-to-pipeline translation, error handling for invalid definitions, and edge cases.

## Backlog Item Details

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P1 - High (important for user experience)

### Technical Debt Impact
- **Current Problems**: PipelineFactory has no tests at all — any change to workflow parsing or pipeline construction is untested.
- **Benefits of Fixing**: Safe refactoring of workflow definitions, confidence in pipeline construction logic.
- **Risk Assessment**: High — factory is the entry point for all pipeline creation from user-defined workflows.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Tests for creating pipelines from valid workflow definitions
- [ ] Tests for error handling with invalid/malformed workflow definitions
- [ ] Tests for task dependency resolution during factory construction
- [ ] Tests for edge cases (empty workflows, single-task workflows, circular deps)
- [ ] `cargo test -p arawn-pipeline` passes
- [ ] `angreal check clippy` clean

## Implementation Notes

### Technical Approach
- Read `crates/arawn-pipeline/src/factory.rs` (or wherever PipelineFactory lives)
- Add unit tests for each factory method
- Add integration tests in `crates/arawn-pipeline/tests/`

### Files
- `crates/arawn-pipeline/src/factory.rs`
- `crates/arawn-pipeline/tests/factory_test.rs` (new)

## Status Updates

### Session 2 — Complete
- Added ~25 new tests to `crates/arawn-pipeline/src/definition.rs`
- Coverage areas: dependency resolution in `to_dynamic_tasks`, `effective_runtime()`/`effective_config()` edge cases for all ActionDefinition variants, validation edge cases (empty task ID, three-node cycle, linear chain), `WorkflowFile::from_file()` error paths, parsing edge cases (LLM with model, script language, schedule timezone, capabilities defaults, retry defaults), Debug/Clone impls
- All 149 unit tests + 14 integration tests pass
- Clippy clean, fmt clean
- Pipeline crate went from ~76 to ~100+ tests

*To be added during implementation*