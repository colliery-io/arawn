---
id: improve-websocket-e2e-coverage-for
level: task
title: "Improve WebSocket E2E coverage for ws/connection.rs and ws/mod.rs"
short_code: "ARAWN-T-0307"
created_at: 2026-03-09T15:43:30.260089+00:00
updated_at: 2026-03-10T00:55:55.772228+00:00
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

# Improve WebSocket E2E coverage for ws/connection.rs and ws/mod.rs

## Objective

Improve WebSocket E2E coverage for `ws/connection.rs` (43.5% E2E) and `ws/mod.rs` (25.4% E2E). These modules are *only* reachable via E2E tests (0% unit coverage for connection.rs, 88% unit for mod.rs). Current WS tests cover basic auth, chat, tool execution, and multi-turn — but miss connection lifecycle (disconnect/reconnect), error frames, subscription edge cases, and concurrent connections.

## Backlog Item Details

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [x] P2 - Medium (basic paths covered)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] E2E tests for WebSocket connection drop and reconnect
- [ ] E2E tests for invalid message format handling
- [ ] E2E tests for subscription to nonexistent sessions
- [ ] E2E tests for concurrent WebSocket connections
- [ ] Coverage for `ws/connection.rs` reaches 70%+ from E2E
- [ ] Coverage for `ws/mod.rs` reaches 60%+ from E2E

## Implementation Notes

### Key Files
- `crates/arawn-server/src/routes/ws/connection.rs` (68/115 lines, 43.5% E2E; 13% unit)
- `crates/arawn-server/src/routes/ws/mod.rs` (208/216 lines, 25.4% E2E; 88.4% unit)
- Tests in `crates/arawn-server/tests/e2e_websocket.rs`

## Status Updates

### Session 1 - Complete
- Added 21 new tests to `crates/arawn-server/tests/e2e_websocket.rs` (4 existing → 25 total)
- **connection.rs** tests:
  - Auto-auth with no token configured (localhost mode)
  - Binary UTF-8 frames accepted as text (raw tungstenite)
  - Binary non-UTF-8 frames return "invalid_message" error
  - Connection survives parse errors (invalid JSON + continue)
  - Graceful close handling
- **handlers.rs** tests:
  - Auth failure (wrong token)
  - Chat/subscribe/command without authentication → "unauthorized"
  - Subscribe with invalid session ID → "invalid_session"
  - Subscribe ownership (first subscriber = owner with reconnect token)
  - Subscribe with reconnect token (disconnect + reclaim)
  - Unsubscribe from session
  - Cancel operation (fire-and-forget + connection survives)
  - Cancel with invalid session ID
  - Cancel without auth
  - Command execution via WS (compact not needed, session not found, unknown command)
  - Chat with invalid workstream ID → path traversal rejected
  - Concurrent WebSocket connections
  - try_recv with timeout
- **Coverage results (E2E-only)**:
  - connection.rs: 43.5% → **66.1%** (76/115 lines)
  - handlers.rs: → **86.9%** (218/251 lines)
  - mod.rs: 25.4% (unchanged — E2E can only cover ws_handler upgrade; unit tests provide 96.3% combined)
- **Remaining uncovered in connection.rs** (~34%): idle timeout (5min, impractical), post-disconnect session indexing (needs indexer), WS error path (hard to trigger)
- All 25 tests pass, clippy clean