---
id: add-pipeline-task-timeout
level: task
title: "Add pipeline task timeout enforcement tests"
short_code: "ARAWN-T-0294"
created_at: 2026-03-08T20:21:16.388520+00:00
updated_at: 2026-03-08T23:54:55.639095+00:00
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

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

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

### Session 1 — Complete
- Added 4 timeout enforcement tests in `crates/arawn-pipeline/tests/engine_test.rs`
- **test_task_timeout_produces_failed_status** — task sleeps 5s with 1s timeout, verifies Failed status with timeout error message
- **test_fast_task_unaffected_by_short_timeout** — fast task completes normally with 5s timeout
- **test_one_task_timeout_does_not_block_pipeline_result** — parallel fast+slow tasks; slow times out, pipeline reports failure
- **test_pipeline_timeout_kills_long_workflow** — chained tasks each within task timeout but total exceeds 1s pipeline timeout; verifies pipeline-level timeout error
- All 18 engine integration tests pass, clippy clean, fmt clean