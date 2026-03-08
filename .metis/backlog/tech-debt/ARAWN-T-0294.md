---
id: add-pipeline-task-timeout
level: task
title: "Add pipeline task timeout enforcement tests"
short_code: "ARAWN-T-0294"
created_at: 2026-03-08T20:21:16.388520+00:00
updated_at: 2026-03-08T20:21:16.388520+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#tech-debt"


exit_criteria_met: false
initiative_id: NULL
---

# Add pipeline task timeout enforcement tests

## Objective

No test verifies that pipeline tasks exceeding their configured timeout are actually killed and reported as failed. Add tests that create tasks with short timeouts and verify they are terminated.

## Backlog Item Details

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P2 - Medium (nice to have)

### Technical Debt Impact
- **Current Problems**: Timeout behavior is implemented but untested. If timeout enforcement breaks, long-running tasks could hang indefinitely.
- **Benefits of Fixing**: Confidence that timeout enforcement works correctly.
- **Risk Assessment**: Medium — timeouts are a safety mechanism; silent breakage is dangerous.

## Acceptance Criteria

- [ ] Test that a task exceeding its timeout is killed
- [ ] Test that timeout produces a Failed status with appropriate error message
- [ ] Test that other tasks in the pipeline are unaffected by one task's timeout
- [ ] Tests use short timeouts (100ms-500ms) to avoid slow test suite
- [ ] `cargo test -p arawn-pipeline` passes
- [ ] `angreal check clippy` clean

## Implementation Notes

### Technical Approach
- Create a DynamicTask that sleeps longer than the configured timeout
- Execute via PipelineEngine and verify the result is Failed with timeout error
- Test both single-task and multi-task pipelines with one timing out

### Files
- `crates/arawn-pipeline/src/engine.rs` or `crates/arawn-pipeline/tests/`

## Status Updates

*To be added during implementation*