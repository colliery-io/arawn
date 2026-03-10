---
id: e2e-tests-for-routes-sessions-rs
level: task
title: "E2E tests for routes/sessions.rs (55.7% E2E coverage)"
short_code: "ARAWN-T-0305"
created_at: 2026-03-09T15:43:28.012839+00:00
updated_at: 2026-03-10T00:55:53.987066+00:00
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

# E2E tests for routes/sessions.rs (55.7% E2E coverage)

## Objective

Improve E2E test coverage for `routes/sessions.rs` from 55.7% to 80%+. The session endpoints are critical — they manage session lifecycle, listing, history retrieval, and pagination. Current E2E tests only exercise basic create/chat flows without covering session listing, individual session retrieval, pagination, or deletion paths.

## Backlog Item Details

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [x] P2 - Medium (partially tested)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] E2E tests for session list endpoint with pagination
- [ ] E2E tests for get session by ID
- [ ] E2E tests for session history/messages retrieval
- [ ] E2E tests for session deletion
- [ ] Error paths: not found, invalid session ID
- [ ] Coverage for `routes/sessions.rs` reaches at least 80% combined

## Implementation Notes

### Key Files
- `crates/arawn-server/src/routes/sessions.rs` (589/771 lines, 76% combined; 55.7% from E2E)
- Unit tests provide 75.2% — E2E should fill the gaps in pagination and deletion paths

## Status Updates

### Session 1 - Complete
- Created `crates/arawn-server/tests/e2e_sessions.rs` with 34 E2E tests
- Tests cover all 6 handlers: create, list, get, delete, update, get_messages
- CRUD: create with title, metadata, minimal; get by ID; delete + verify removal
- Pagination: limit, offset on list endpoint
- Update: title, metadata merge, title+metadata together
- Workstream paths: create/list/get/delete/messages with workstreams enabled; reassign to nonexistent ws (400); reassign without workstreams (400); invalid workstream ID (path traversal rejection)
- Error paths: invalid UUID (400), not found (404) for get/delete/update/messages
- Chat integration: session created via chat appears in list, detail shows turns, messages endpoint returns conversation
- **Coverage result: routes/sessions.rs 81.2% combined (626/771 lines)** — exceeds 80% target
- All tests pass, clippy clean