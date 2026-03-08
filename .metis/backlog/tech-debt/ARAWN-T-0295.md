---
id: add-websocket-command-dispatch
level: task
title: "Add WebSocket command dispatch tests"
short_code: "ARAWN-T-0295"
created_at: 2026-03-08T20:21:17.699985+00:00
updated_at: 2026-03-08T20:21:17.699985+00:00
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

*To be added during implementation*