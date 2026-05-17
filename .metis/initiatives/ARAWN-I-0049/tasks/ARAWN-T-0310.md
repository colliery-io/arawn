---
id: todoevent-broadcast-todos-ws-rpc
level: task
title: "TodoEvent broadcast + todos.* WS-RPC methods"
short_code: "ARAWN-T-0310"
created_at: 2026-05-16T22:52:29.858042+00:00
updated_at: 2026-05-17T01:28:23.681386+00:00
parent: ARAWN-I-0049
blocked_by: [ARAWN-T-0309]
archived: false

tags:
  - "#task"
  - "#phase/completed"


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

- [x] `TodoEvent::{Created, Completed, Updated, Archived}` variants
      emitted by every mutating `TodoService` method via the
      `with_events` builder.
- [x] WS-RPC methods registered: `todos.create / list / get /
      done / undo / patch / archive / search`.
- [x] Each method maps `StorageError` → `todo_error` with
      `details.kind` discriminator (`not_found`, `invalid_operation`,
      `database`, `io`, `json`, `migration`).
- [x] `RPC_METHODS` array in `ws_server.rs` updated; `todos.*`
      prefix matcher dispatches through `handle_todo_rpc`.
- [x] Service-level integration test (`todos_rpc_round_trip` in
      arawn-tests/websocket.rs) drives create → list → get → done
      → undo → patch → search → archive end-to-end. Second test
      verifies the error envelope on a missing id.

## Status Updates

### 2026-05-17 — shipped

- `TodoEvent` tagged enum with 4 variants + `todo_event_channel`
  helper. Payloads stay tight (id + kind, plus `done_at` on
  Completed) so the broadcast stays responsive.
- `TodoService::with_events(sender)` builder — opt-in event
  emission. Mutations emit only on real state transitions
  (idempotent paths stay quiet).
- `LocalService.todo_event_tx` + `todo_event_sender()` /
  `subscribe_todo_events()` accessors. RPC handlers clone the
  sender when constructing a `TodoService`.
- Forwarder task in main.rs wraps `TodoEvent` into
  `ServerNotice { category: "todo_event", message: serde_json }`
  on the existing notice broadcast — same shape the TUI's
  category-dispatcher already understands.
- 2 new storage unit tests (event ordering across all 5 ops;
  idempotent paths skip emission) + 2 new WS integration tests.
- arawn-storage 70/70, arawn-tests/websocket 16/16, full
  workspace build clean.

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