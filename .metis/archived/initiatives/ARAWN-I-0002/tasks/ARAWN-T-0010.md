---
id: workstreamstore-sessionstore
level: task
title: "WorkstreamStore + SessionStore — SQLite implementation"
short_code: "ARAWN-T-0010"
created_at: 2026-03-31T22:49:35.000757+00:00
updated_at: 2026-03-31T23:10:14.240272+00:00
parent: ARAWN-I-0002
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0002
---

# WorkstreamStore + SessionStore — SQLite implementation

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0002]]

## Objective

Implement `WorkstreamStore` and `SessionStore` traits with SQLite backing. These provide CRUD operations for workstream and session metadata.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `WorkstreamStore` trait: `create`, `get`, `list`, `delete`
- [ ] `SessionStore` trait: `create`, `get`, `list_for_workstream`, `list_scratch`, `update_workstream_id`
- [ ] `SqliteWorkstreamStore` implements `WorkstreamStore` against the `Database`
- [ ] `SqliteSessionStore` implements `SessionStore` against the `Database`
- [ ] Roundtrip tests: create workstream → get → verify fields match
- [ ] List tests: create multiple → list → verify count and contents
- [ ] Delete test: create → delete → get returns None
- [ ] Session-workstream relationship: list_for_workstream only returns sessions for that workstream
- [ ] Scratch sessions: list_scratch returns sessions with NULL workstream_id
- [ ] update_workstream_id: sets workstream_id on a scratch session (used by promotion)

## Test Cases **[CONDITIONAL: Testing Task]**

{Delete unless this is a testing task}

### Test Case 1: {Test Case Name}
- **Test ID**: TC-001
- **Preconditions**: {What must be true before testing}
- **Steps**: 
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

### Test Case 2: {Test Case Name}
- **Test ID**: TC-002
- **Preconditions**: {What must be true before testing}
- **Steps**: 
  1. {Step 1}
  2. {Step 2}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

## Documentation Sections **[CONDITIONAL: Documentation Task]**

{Delete unless this is a documentation task}

### User Guide Content
- **Feature Description**: {What this feature does and why it's useful}
- **Prerequisites**: {What users need before using this feature}
- **Step-by-Step Instructions**:
  1. {Step 1 with screenshots/examples}
  2. {Step 2 with screenshots/examples}
  3. {Step 3 with screenshots/examples}

### Troubleshooting Guide
- **Common Issue 1**: {Problem description and solution}
- **Common Issue 2**: {Problem description and solution}
- **Error Messages**: {List of error messages and what they mean}

### API Documentation **[CONDITIONAL: API Documentation]**
- **Endpoint**: {API endpoint description}
- **Parameters**: {Required and optional parameters}
- **Example Request**: {Code example}
- **Example Response**: {Expected response format}

## Implementation Notes
- `workstream_store.rs`, `session_store.rs` in `crates/arawn-storage/src/`
- Traits defined in `arawn-storage` (not `arawn-core`) — core stays persistence-agnostic
- Map between `arawn_core::Workstream`/`Session` and SQLite rows in the store impls
- UUIDs stored as TEXT in SQLite, timestamps as ISO 8601 TEXT
- All tests use `Database::in_memory()`
- Depends on: ARAWN-T-0009 (crate scaffolding + Database)

## Status Updates
- **2026-03-31**: Complete. WorkstreamStore with create/get/find_by_name/list/delete. SessionStore with create/get/list_for_workstream/list_scratch/update_workstream_id. SessionMeta type for DB rows (no messages). 13 new tests (19 total in storage). Note: implemented as concrete structs with &Database ref, not traits — can extract traits later if needed for mocking. Also noted Session::into_session() needs a from_parts constructor to preserve IDs across load — tracked as TODO.