---
id: extend-testharnessbuilder-for
level: task
title: "Extend TestHarnessBuilder for cross-subsystem integration testing"
short_code: "ARAWN-T-0102"
created_at: 2026-04-05T17:17:07.308020+00:00
updated_at: 2026-04-05T17:31:38.074062+00:00
parent: ARAWN-I-0013
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0013
---

# Extend TestHarnessBuilder for cross-subsystem integration testing

## Objective

Extend `TestHarnessBuilder` in `crates/arawn-engine/src/testing.rs` to support wiring permissions, hooks, skills, and plugins into the engine — enabling cross-subsystem integration tests. This is the foundation task that unblocks all other integration tests in the initiative.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `TestHarnessBuilder` has `.with_permission_checker(Arc<PermissionChecker>)` method
- [ ] `TestHarnessBuilder` has `.with_hook_runner(Arc<HookRunner>)` method
- [ ] `TestHarnessBuilder` has `.with_skill_registry(Arc<SkillRegistry>)` method
- [ ] `TestHarness::run()` wires all configured subsystems into `QueryEngine` via existing `with_*` methods
- [ ] `HarnessResult` exposes session messages for assertion (already does — verify sufficient)
- [ ] Existing tests in `testing.rs` continue to pass unchanged
- [ ] At least one new test in `testing.rs` demonstrates permission checker + engine integration
- [ ] `angreal test unit` passes

## Implementation Notes

### File to modify
`crates/arawn-engine/src/testing.rs`

### Current state
- `TestHarnessBuilder` has: `with_workstream_file()`, `with_tool()`, `with_tools()`, `with_script()`, `with_max_iterations()`
- `TestHarness::run()` creates `QueryEngine::with_config()` but never calls `with_permission_checker()`, `with_hook_runner()`, etc.
- `QueryEngine` already has all `with_*` methods (lines 119-154 of query_engine.rs)

### Approach
1. Add optional fields to `TestHarnessBuilder`: `permission_checker`, `hook_runner`, `skill_registry`
2. Add corresponding `with_*` builder methods
3. In `TestHarness::run()`, chain `with_*` calls on the engine when the optional is `Some`
4. Keep backward compatibility — existing tests that don't set these fields work identically

### Dependencies
None — this is the foundation task.

## Status Updates

### 2026-04-05 — Complete
- Added `permission_checker`, `hook_runner`, `skill_registry` optional fields to `TestHarnessBuilder` and `TestHarness`
- Added `.with_permission_checker()`, `.with_hook_runner()`, `.with_skill_registry()` builder methods
- Extracted `build_engine()` helper on `TestHarness` to DRY up engine construction in `run()` and `run_expect_error()`
- `build_engine()` chains `with_*` calls when optional subsystems are `Some`
- Added 2 new tests: `harness_permission_checker_blocks_tool` and `harness_permission_checker_allows_tool`
- All 11 tests pass (9 existing + 2 new), zero regressions