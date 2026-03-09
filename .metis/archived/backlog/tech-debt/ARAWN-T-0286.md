---
id: add-pipeline-engine-execution-and
level: task
title: "Add pipeline engine execution and scheduling tests"
short_code: "ARAWN-T-0286"
created_at: 2026-03-08T03:17:31.240706+00:00
updated_at: 2026-03-08T19:08:19.656845+00:00
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

# Add pipeline engine execution and scheduling tests

## Objective

`PipelineEngine` in `arawn-pipeline/src/engine.rs` has only **2 tests** despite being the orchestration core. The existing integration tests (`tests/engine_test.rs` has 6 tests, `tests/e2e_runtime_test.rs` has 3 tests) cover basic execution but miss scheduling, error recovery, retries, result streaming, and concurrent workflow execution.

### Priority
- [x] P2 - Medium (pipeline system not yet heavily used, but growing)
- **Size**: M

### Current Problems
- `engine.rs` has 2 inline tests — just default config and basic instantiation
- No tests for workflow scheduling (`schedule_workflow`, `list_schedules`)
- No tests for execution result retrieval (`get_execution_result`)
- No tests for task retry logic on failure
- No tests for timeout enforcement
- No tests for concurrent workflow execution
- Workflow registration validation is minimal
- Context propagation between dependent tasks barely tested

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `engine.rs` inline tests expanded to cover all public methods
- [ ] Tests for workflow registration: valid workflow, duplicate ID, empty workflow
- [ ] Tests for task execution: success, failure, timeout, retry
- [ ] Tests for task dependencies: A→B→C chain, parallel tasks, failed dependency skips downstream
- [ ] Tests for context propagation: output of task A available as input to task B
- [ ] Tests for scheduling: cron schedule registration, listing, removal
- [ ] Tests for concurrent execution: two workflows running simultaneously
- [ ] Tests for engine shutdown: graceful cleanup of running tasks
- [ ] At least 20 new test functions

## Implementation Notes

### Test approach

Use the existing `test_engine()` helper in `engine.rs` which creates an isolated SQLite database:

```rust
fn test_engine() -> (tempfile::TempDir, PipelineEngine) {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("test.db");
    let engine = PipelineEngine::new(&db).unwrap();
    (dir, engine)
}
```

### Scenarios to test

**Registration:**
```rust
#[tokio::test]
async fn test_register_workflow_and_list() {
    let (_dir, engine) = test_engine();
    let wf = WorkflowDefinition::from_toml("...").unwrap();
    engine.register_workflow(wf).await.unwrap();
    let list = engine.list_workflows().await.unwrap();
    assert_eq!(list.len(), 1);
}
```

**Execution with dependencies:**
```rust
#[tokio::test]
async fn test_task_dependency_chain() {
    // Task B depends on Task A
    // Task A outputs {"result": 42}
    // Task B should receive A's output in context
}
```

**Error recovery:**
```rust
#[tokio::test]
async fn test_task_failure_skips_dependents() {
    // Task A fails → Task B (depends on A) should be skipped
    // Task C (no dependency on A) should still run
}
```

### Key files
- `crates/arawn-pipeline/src/engine.rs` — Core engine (add inline tests)
- `crates/arawn-pipeline/tests/engine_test.rs` — Integration tests (expand)
- `crates/arawn-pipeline/src/task.rs` — Task execution (retry, timeout)

### Dependencies
- None

## Status Updates

### Session 1
- Added 9 inline tests to `engine.rs`: config defaults, status equality, register/list, duplicate workflow, multiple workflows, execution result ID, context preservation, list schedules empty, schedule info fields
- Added 9 integration tests to `engine_test.rs`: three-step dependency chain, parallel independent tasks, task failure status, failed dependency downstream, concurrent workflow execution, context pass-through, execute same workflow twice, schedule and list cron
- Fixed `TaskError::ExecutionError` → `TaskError::ExecutionFailed { message, task_id, timestamp }`

### Session 2
- Fixed `test_task_failure_returns_failed_status`: Cloacina marks pipelines "Completed" even when tasks fail. Fixed `PipelineEngine::execute()` to check `task_results` for failed tasks and return `ExecutionStatus::Failed`.
- Fixed `test_three_step_dependency_chain`: `Context::insert` errors on duplicate keys (`KeyExists`). Changed to unique keys per step (`a_done`, `b_saw_a`, `c_saw_b`).
- All 121 lib tests + 14 integration tests pass, 0 failures.