---
id: add-tui-reconnection-resilience
level: task
title: "Add TUI reconnection resilience tests"
short_code: "ARAWN-T-0299"
created_at: 2026-03-08T20:21:21.095305+00:00
updated_at: 2026-03-09T01:22:18.256560+00:00
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

# Add TUI reconnection resilience tests

## Objective

No test covers TUI behavior when the server connection drops and reconnects. Add tests verifying the TUI handles disconnection gracefully and recovers on reconnection.

## Backlog Item Details

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P2 - Medium (nice to have)

### Technical Debt Impact
- **Current Problems**: TUI has 89 rendering tests but no resilience tests. A dropped connection could leave the TUI in a broken state with no recovery.
- **Benefits of Fixing**: Confidence that TUI degrades gracefully on disconnect and recovers cleanly.
- **Risk Assessment**: Low-medium — network interruptions are common; TUI must handle them.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Test TUI displays disconnection indicator when server connection drops
- [ ] Test TUI reconnects automatically after a configurable delay
- [ ] Test TUI restores session state after reconnection
- [ ] Test TUI handles multiple rapid disconnect/reconnect cycles without panicking
- [ ] `cargo test -p arawn-tui` passes
- [ ] `angreal check clippy` clean

## Implementation Notes

### Technical Approach
- Mock the WS connection layer to simulate drops
- Verify TUI state transitions: connected → disconnected → reconnecting → connected
- Test that UI rendering reflects connection state
- May need to extract connection state into a testable component

### Files
- `crates/arawn-tui/src/` (connection handling module)

## Status Updates

### Implementation Complete

**Files modified:**
- `crates/arawn-tui/src/client.rs` — Added `WsClient::mock_controllable()` returning `(WsClient, status_sender, message_sender)` for injecting connection events in tests
- `crates/arawn-tui/src/app.rs` — Added `test_app_controllable()` helper and `simulate_status_poll()` to replicate tick handler logic, plus 8 new tests

**Tests added:**
1. `test_disconnect_shows_status_indicator` — Connected→Disconnected updates `connection_status`
2. `test_reconnecting_shows_attempt_count` — Reconnecting{1}→Reconnecting{2} shows increasing attempts
3. `test_full_reconnection_lifecycle` — Connected→Disconnected→Reconnecting→Connected full cycle; clears waiting, sets "Connection lost" message
4. `test_session_state_preserved_across_reconnect` — session_id, messages, reconnect_tokens survive disconnect/reconnect; SubscribeAck restores ownership
5. `test_rapid_disconnect_reconnect_cycles_no_panic` — 10 rapid disconnect/reconnect cycles with alternating waiting states; no panics, app still functional
6. `test_disconnect_during_streaming_marks_message_not_streaming` — partial streaming message preserved, waiting cleared on disconnect
7. `test_messages_received_after_reconnect_handled_correctly` — new ChatChunks after reconnect render correctly
8. `test_disconnect_while_not_waiting_no_status_change` — disconnect without waiting doesn't set "Connection lost" message

All 104 tests pass (`cargo test -p arawn-tui`). Clippy clean.