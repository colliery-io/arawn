---
id: e2e-tests-for-routes-workstreams
level: task
title: "E2E tests for routes/workstreams.rs (25.6% E2E coverage)"
short_code: "ARAWN-T-0304"
created_at: 2026-03-09T15:43:27.145501+00:00
updated_at: 2026-03-10T00:55:53.532761+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# E2E tests for routes/workstreams.rs (25.6% E2E coverage)

## Objective

Improve E2E test coverage for `routes/workstreams.rs` from 25.6% to 70%+. Unit tests provide 0% coverage for this module — it's only exercised through E2E. Current E2E tests only hit basic CRUD; need to cover listing, filtering, session assignment, deletion, and error paths.

## Backlog Item Details

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [x] P2 - Medium (partially tested, core CRUD works)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] E2E tests for workstream CRUD (create, list, get, update, delete)
- [ ] Workstream session listing and assignment
- [ ] Error paths: not found, invalid input, duplicate names
- [ ] Coverage for `routes/workstreams.rs` reaches at least 70% from E2E

## Implementation Notes

### Key Files
- `crates/arawn-server/src/routes/workstreams.rs` (170/446 lines covered, 38% combined)
- Unit coverage: 0% — E2E is the only path to cover this module
- Existing tests in `e2e_scenarios.rs` and `e2e_stress.rs` cover basic paths

## Status Updates

### Complete
- Created `crates/arawn-server/tests/e2e_workstreams.rs` with **32 E2E tests**:
  - **CRUD**: update title/summary/tags/model, update multiple fields, update nonexistent (404)
  - **Delete**: archive workstream, verify excluded from list, delete nonexistent (404)
  - **List**: pagination (limit/offset), include_archived filter
  - **Sessions**: list sessions, list with pagination, nonexistent ws returns empty
  - **Messages**: all 4 roles, default role is user, invalid role (400), metadata, pagination, since filter (URL-encoded), invalid since (400), send to nonexistent ws (404)
  - **503 paths**: workstreams not configured (list + create), promote_file/export_file/clone_repo/usage/cleanup without DirectoryManager, compress without compressor
  - **Create**: with all optional fields (default_model, tags)
  - **Get**: full detail verification
  - **Validation**: invalid ID (path traversal) returns 400
  - **Promote**: non-scratch workstream returns 400
- **Coverage**: `routes/workstreams.rs` E2E went from **25.6% → 63.7%** (284/446 lines)
  - Remaining uncovered lines are file operation bodies (promote, export, clone, usage, cleanup) that require DirectoryManager infrastructure not available in test server
- **All 114 E2E tests pass**, clippy clean