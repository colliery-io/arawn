---
id: todoevent-broadcast-todos-ws-rpc
level: task
title: "TodoEvent broadcast + todos.* WS-RPC methods"
short_code: "ARAWN-T-0310"
created_at: 2026-05-16T22:52:29.858042+00:00
updated_at: 2026-05-16T22:52:29.858042+00:00
parent: ARAWN-I-0049
blocked_by: [ARAWN-T-0309]
effort: S
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0049
---

# TodoEvent broadcast + todos.* WS-RPC methods

## Parent Initiative

[[ARAWN-I-0049]]

## Objective

Expose `TodoService` over the existing notice broadcast and WS-RPC
surface. After this lands the TUI and agent tools have a wire to
talk to.

## Acceptance Criteria

- [ ] `TodoEvent::{Created, Completed, Updated, Archived}` variants
      emitted by every mutating `TodoService` method onto the
      existing `tokio::sync::broadcast` notice channel.
- [ ] WS-RPC methods registered: `todos.create / list / get /
      done / undo / patch / archive / search`.
- [ ] Each method maps `TodoServiceError` → a stable error code
      (`todo_error`) with `details.kind` discriminator.
- [ ] `RPC_METHODS` array in `ws_server.rs` updated; `todos.*`
      prefix matcher added.
- [ ] Service-level integration test invokes create → list →
      done → undo round-trip via the RPC layer.

## Implementation Notes

### Technical Approach

- Mirror the pattern used by `ceremonies.*` RPC additions in
  `crates/arawn/src/ws_server.rs`.
- `ServerNotice` forwarding: spawn a forwarder task at startup
  (same pattern as the ceremony event forwarder from
  [[ARAWN-T-0308]]) that wraps `TodoEvent` into a
  `ServerNotice { category: "todo_event", payload: serde_json }`.
- Event-payload schema gets pinned now since downstream TUI
  refresh logic will pattern-match on it.

### Dependencies

- Blocked by [[ARAWN-T-0309]] (TodoService must exist).
- Consumed by [[ARAWN-T-0313]] (agent tools), [[ARAWN-T-0314]]
  (TUI `/todo` + WS auto-refresh).

### Risk Considerations

- Channel saturation: high-volume `Updated` events could spam
  the broadcast. Keep payload tight (id + kind + done_at) and
  let consumers re-fetch on demand if they need more detail.

## Status Updates

*To be added during implementation*
