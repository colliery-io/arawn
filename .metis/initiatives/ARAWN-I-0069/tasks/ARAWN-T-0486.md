---
id: p3-3-clear-stale-modal-state-on
level: task
title: "P3-3: Clear stale modal state on disconnect (TUI)"
short_code: "ARAWN-T-0486"
created_at: 2026-06-13T14:43:28.607262+00:00
updated_at: 2026-06-13T14:59:07.394655+00:00
parent: ARAWN-I-0069
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0069
---

# P3-3: Clear stale modal state on disconnect (TUI)

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0069]] — implements P3-3 (MEDIUM, TUI correctness).

## Objective **[REQUIRED]**

Clear pending-modal state when the WS connection drops, so a stale pending-response (oneshot) from before a disconnect can't leak into the next connection and desync the UI.

**The defect** (`crates/arawn-tui/src/event_loop/mod.rs:1109-1125`): pending modal oneshot channels aren't cleaned up on disconnect — a `UserInputRequest` left unanswered when the socket closes survives, and the next connection can resolve against it.

### Type
- [x] Bug — TUI state correctness across reconnects

### Priority
- [x] P2 - Medium

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] On `WsEvent::Closed` (disconnect), all pending modal state (oneshot senders / pending request map) is cleared.
- [ ] A reconnect starts with no carried-over pending modal; a fresh `UserInputRequest` after reconnect works normally.
- [ ] Inline test: a disconnect mid-modal (pending request outstanding) leaves no pending state afterward.
- [ ] `angreal check all` + `angreal test unit` green.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
In the TUI event loop's disconnect handler, drop/drain the pending-modal collection (the oneshot map keyed by request_id). Dropping the senders also unblocks any awaiting resolver with a cancelled result.

### Dependencies
Pure TUI-crate change.

### Risk Considerations
Ensure dropping a pending sender doesn't panic an awaiting task — a cancelled oneshot should resolve to a clean "cancelled" path, not unwrap.

## Status Updates **[REQUIRED]**

**2026-06-13 — Implemented & complete.**
- Added `App::clear_pending_modal() -> bool` (`app/mod.rs`): drops `active_modal` (which owns the result oneshot *sender* → any awaiter resolves cleanly) and `pending_modal_response` (the `(request_id, oneshot::Receiver)` consumed by the post-close handler at `event_loop/mod.rs:290`). Returns whether anything was cleared.
- Wired it into both disconnect paths in the event loop: `WsEvent::Closed` and `WsEvent::Error` now call `clear_pending_modal()` before breaking, with a debug log when state was actually dropped.
- Inline test `clear_pending_modal_drops_modal_and_oneshot`: opens the branch modal (double-Esc sets both `active_modal` + `pending_modal_response`), clears, asserts both gone + the bool, and asserts idempotent no-op on a second call.
- `cargo test -p arawn-tui --lib`: 260 passed. Gate clippy (`cargo clippy -p arawn-tui -- -D warnings`, no `--all-targets`) clean; fmt clean.

**Architectural finding:** `run_tui` (`event_loop/mod.rs:88`) connects once and the event loop `break`s straight to terminal-restore + `Ok(())` on disconnect — there is **no in-process reconnect today**, so the leak the ticket guards against can't actually fire in the current build. This change is the correct cleanup regardless: it unblocks any awaiting modal oneshot immediately, and it makes the state safe if/when an in-loop reconnect is added (which the GUI/daily-driver work may well introduce). Kept the fix minimal + forward-safe rather than building a reconnect loop that isn't in scope.