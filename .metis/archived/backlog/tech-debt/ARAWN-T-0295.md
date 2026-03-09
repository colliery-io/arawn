---
id: add-websocket-command-dispatch
level: task
title: "Add WebSocket command dispatch tests"
short_code: "ARAWN-T-0295"
created_at: 2026-03-08T20:21:17.699985+00:00
updated_at: 2026-03-09T00:44:08.752263+00:00
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

# Add WebSocket command dispatch tests

## Objective

WebSocket tests cover connect/subscribe but not the full command dispatch path — chat messages, tool calls, and streaming responses flowing through the WS handler. Add tests that verify commands sent over WebSocket are dispatched correctly and produce expected responses.

## Backlog Item Details

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P2 - Medium (nice to have)

### Technical Debt Impact
- **Current Problems**: WS handler dispatch logic is only tested at the unit level. No test sends a chat message through WS and verifies the response stream.
- **Benefits of Fixing**: Confidence that the WS command dispatch path works end-to-end.
- **Risk Assessment**: Medium — WS is the primary real-time communication channel.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Test sending a chat message via WS and receiving streamed response
- [ ] Test sending an invalid/unknown command and receiving error response
- [ ] Test that tool call results are forwarded back through WS correctly
- [ ] Test that session-scoped messages only go to the correct session's subscribers
- [ ] `cargo test -p arawn-server` passes
- [ ] `angreal check clippy` clean

## Implementation Notes

### Technical Approach
- Use `axum::extract::ws` test utilities or `tokio-tungstenite` for WS client
- Spin up test server with mock LLM backend
- Send WS messages and assert on received frames

### Files
- `crates/arawn-server/src/routes/ws/handlers.rs`
- `crates/arawn-server/tests/` (new integration test file)

## Status Updates

### Session 1
- Added `CommandProgress` and `CommandResult` variants to `WsServerMessage` in `arawn-test-utils/src/ws_client.rs`
- Added `command()` and `collect_until_command_done()` helpers to `TestWsClient`
- Added 7 new integration tests in `websocket_integration.rs`:
  - `test_ws_command_unknown` — unknown command returns CommandResult with success=false
  - `test_ws_command_requires_auth` — command without auth returns unauthorized error
  - `test_ws_command_compact_with_progress` — valid compact command produces progress + result
  - `test_ws_command_compact_invalid_session` — compact with bad session_id fails gracefully
  - `test_ws_chat_tool_call_events_forwarded` — StreamingMockBackend with tool_then_text verifies ToolStart/ToolEnd flow through WS
  - `test_ws_session_scoped_isolation` — two WS clients get different sessions, chunks reference correct session_ids
- All 19 tests pass, clippy clean