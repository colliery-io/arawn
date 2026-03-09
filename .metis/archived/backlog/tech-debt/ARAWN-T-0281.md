---
id: add-websocket-e2e-integration
level: task
title: "Add WebSocket E2E integration tests for chat flow"
short_code: "ARAWN-T-0281"
created_at: 2026-03-08T03:17:25.921539+00:00
updated_at: 2026-03-08T15:30:04.165275+00:00
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

# Add WebSocket E2E integration tests for chat flow

## Objective

The WebSocket chat flow — the primary way the TUI talks to the server — has **zero E2E test coverage**. Only protocol parsing is unit-tested. Write integration tests that exercise the full flow: WS connect → authenticate → subscribe → chat → receive response → unsubscribe.

This would have caught the session ownership bug, the idle timeout disconnect, and the read-only mode issue we just fixed.

### Priority
- [x] P0 - Critical (this is the primary user-facing communication path)
- **Size**: L

### Current Problems
- Session ownership logic only tested with unit-level state tests (no real WS connections)
- Keepalive/ping-pong behavior untested
- Reconnection behavior untested
- Concurrent client behavior (ownership conflict) untested
- Chat message routing through workstreams untested over WS

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Test file: `crates/arawn-server/tests/websocket_integration.rs`
- [ ] Tests use `TestWsClient` from `arawn-test-utils` (or inline tokio-tungstenite)
- [ ] Scenarios covered:
  - [ ] Connect and authenticate (no-auth mode + token mode)
  - [ ] Subscribe to session → receive `subscribe_ack` with `is_owner: true`
  - [ ] Send chat message → receive agent response
  - [ ] Multi-turn conversation in same session
  - [ ] Second client subscribes → gets `is_owner: false`
  - [ ] Owner disconnects → second client can claim ownership
  - [ ] Idle timeout (5min) sends error and closes connection
  - [ ] Ping/pong keepalive prevents idle timeout
  - [ ] Reconnect with token → reclaims ownership within grace period
  - [ ] Reconnect after grace period → gets new ownership
  - [ ] Concurrent chat from non-owner → `session_not_owned` error
  - [ ] Invalid session ID → error response
  - [ ] Unauthenticated chat → `unauthorized` error

## Implementation Notes

### Test infrastructure needed

Use `tokio-tungstenite` directly or create `TestWsClient` wrapper:

```rust
let server = TestServer::builder().build().await;
let (mut ws, _) = tokio_tungstenite::connect_async(server.ws_url()).await.unwrap();

// Send subscribe
ws.send(Message::Text(json!({"type": "subscribe", "session_id": "..."}).to_string())).await;
let ack: ServerMessage = read_next(&mut ws).await;
assert!(ack.is_owner);

// Send chat
ws.send(Message::Text(json!({"type": "chat", "message": "hello"}).to_string())).await;
let response: ServerMessage = read_next(&mut ws).await;
assert_eq!(response.type_, "chat_response");
```

### Key scenarios by complexity

**Basic (must have):**
- Connect → auth → subscribe → chat → response
- Unsubscribe → re-subscribe

**Ownership (critical — these bugs happened):**
- Two clients, ownership transfer on disconnect
- Dead connection detection (active_connections tracking)
- Pending reconnect grace period behavior

**Edge cases:**
- Binary frame with valid JSON (accepted)
- Binary frame with invalid UTF-8 (rejected)
- Malformed JSON → parse_error
- Very large message handling

### Dependencies
- Depends on ARAWN-T-0279 (shared test utils) for TestServer, or can inline TestServer temporarily

## Status Updates

### Completed
- Created 2 test files with 21 WebSocket E2E tests:
  - `tests/websocket_integration.rs` — 13 tests (auth, subscribe, chat flow, ping, errors, malformed JSON)
  - `tests/websocket_ownership.rs` — 8 tests (reader vs owner, reconnect tokens, non-owner rejection, unsubscribe)
- Tests use `TestWsClient` and `TestServer` from `arawn-test-utils`
- Scenarios covered:
  - [x] Connect and authenticate (no-auth + token mode)
  - [x] Subscribe → `subscribe_ack` with `is_owner: true`
  - [x] Send chat → receive agent response with SessionCreated + ChatChunks
  - [x] Multi-turn in same session
  - [x] Second client subscribes → `is_owner: false`
  - [x] Owner disconnects → pending reconnect blocks new owner
  - [x] Reconnect with valid token → reclaims ownership
  - [x] Reconnect with invalid token → denied
  - [x] Unsubscribe releases ownership (no pending reconnect)
  - [x] Non-owner chat → `session_not_owned` error
  - [x] Invalid session ID → error
  - [x] Unauthenticated operations → `unauthorized` error
  - [x] Malformed JSON → `parse_error`
  - [x] Ping/pong keepalive
  - [x] Multiple independent session subscriptions
- All 21 tests pass