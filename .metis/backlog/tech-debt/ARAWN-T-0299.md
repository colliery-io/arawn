---
id: add-tui-reconnection-resilience
level: task
title: "Add TUI reconnection resilience tests"
short_code: "ARAWN-T-0299"
created_at: 2026-03-08T20:21:21.095305+00:00
updated_at: 2026-03-08T20:21:21.095305+00:00
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

*To be added during implementation*