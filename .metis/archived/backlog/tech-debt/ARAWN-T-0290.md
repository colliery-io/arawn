---
id: add-comprehensive-domain-model
level: task
title: "Add comprehensive domain model tests"
short_code: "ARAWN-T-0290"
created_at: 2026-03-08T20:21:12.329533+00:00
updated_at: 2026-03-08T23:44:45.727486+00:00
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

# Add comprehensive domain model tests

## Objective

The `arawn-domain` crate has only 8 tests — critically low for core domain logic that underpins the entire system. Add comprehensive tests covering all domain model types, their invariants, serialization, and edge cases.

## Backlog Item Details

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P1 - High (important for user experience)

### Technical Debt Impact
- **Current Problems**: Only 8 tests for the entire domain model layer. Regressions in domain types could silently break multiple downstream crates.
- **Benefits of Fixing**: Confidence in domain model invariants, safe refactoring of core types.
- **Risk Assessment**: High risk — domain is foundational; bugs here propagate everywhere.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] All domain model structs have construction/validation tests
- [ ] Serialization round-trip tests for all types used in API/storage
- [ ] Edge case tests (empty strings, boundary values, invalid states)
- [ ] `cargo test -p arawn-domain` passes with 30+ tests
- [ ] `angreal check clippy` clean

## Implementation Notes

### Technical Approach
- Read all types in `crates/arawn-domain/src/`
- Add inline `#[cfg(test)]` modules for each source file
- Focus on: construction, validation, Display/Debug impls, serde round-trips, PartialEq edge cases

### Files
- `crates/arawn-domain/src/**/*.rs`

## Status Updates

### Completed
- Added 61 new tests (6 → 67 total), exceeding the 30+ target
- **error.rs** (10 tests): Display for all 5 variants, Debug impl, empty strings, special chars, Result type alias
- **services/chat.rs** (30 tests): ChatResponse fields/clone/debug/truncated/tool_calls, ToolCallSummary fields/clone/debug, TurnOptions default/clone/debug, ChatService accessors/clone/agent, session_to_messages empty/multiple/incomplete, messages_as_refs empty, multi-turn chat with MockBackend
- **services/mcp.rs** (24 tests): McpService enabled/disabled/clone, all disabled-error paths (list/connect/shutdown/add/remove), add+list server, add+remove server, connected check for unknown, McpServerInfo/McpToolInfo fields/clone/debug
- **services/memory.rs** (4 tests): enabled/disabled, clone with shared Arc, clone disabled
- **services/mod.rs** (4 new tests): DomainServices clone, mcp disabled/enabled, chat accessor, agent accessor
- All tests pass, clippy clean, fmt clean