---
id: e2e-tests-for-routes-commands-rs-0
level: task
title: "E2E tests for routes/commands.rs (0% E2E coverage)"
short_code: "ARAWN-T-0306"
created_at: 2026-03-09T15:43:28.934426+00:00
updated_at: 2026-03-10T00:55:54.873655+00:00
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

# E2E tests for routes/commands.rs (0% E2E coverage)

## Objective

Add E2E tests for `routes/commands.rs` which has 0% E2E coverage (71% from unit tests). Commands are registered server-side and executed via REST API — E2E tests would verify the full request/response cycle including command discovery, execution, and error handling.

## Backlog Item Details

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [x] P2 - Medium (unit tests provide decent coverage)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] E2E tests for listing available commands
- [ ] E2E tests for executing a command
- [ ] E2E tests for command not found / invalid input errors
- [ ] Coverage for `routes/commands.rs` reaches at least 50% from E2E

## Implementation Notes

### Key Files
- `crates/arawn-server/src/routes/commands.rs` (319/348 lines, 91.7% unit; 0% E2E)
- May need to register mock commands in TestServerBuilder

## Status Updates

### Session 1 - Complete
- Created `crates/arawn-server/tests/e2e_commands.rs` with 10 E2E tests
- Tests cover all 3 handlers: list_commands, compact_command, compact_command_stream
- List commands: verifies compact command is present with description
- Compact (non-streaming): invalid session ID (400), session not found (404), not needed (200/compacted=false), force with 6 turns (200/compacted=true), natural threshold
- Compact stream (SSE): invalid session ID (400), session not found (404), not needed (completed event with compacted=false), with 6 turns (started → summarizing → completed events)
- Used ScriptedMockBackend with enough invocations for multi-turn sessions + compaction summary
- Custom `parse_compact_sse()` helper for parsing `data:`-only SSE format (no `event:` lines)
- **Coverage result: routes/commands.rs 85.4% E2E-only (152/178 lines)** — far exceeds 50% target
- All tests pass, clippy clean