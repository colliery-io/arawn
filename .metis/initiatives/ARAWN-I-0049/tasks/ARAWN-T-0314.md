---
id: tui-todo-slash-command-done-toggle
level: task
title: "TUI /todo slash command + done-toggle integration"
short_code: "ARAWN-T-0314"
created_at: 2026-05-16T22:52:35.918379+00:00
updated_at: 2026-05-17T18:46:40.395955+00:00
parent: ARAWN-I-0049
blocked_by: [ARAWN-T-0313]
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0049
---

# TUI /todo slash command + done-toggle integration

## Parent Initiative

[[ARAWN-I-0049]]

## Objective

User-facing TUI surface for the generic todo system. Parallel to
`/today`/`/week`/`/retro` but lifecycle-free — just a flat list
of open todos with done-toggle.

## Acceptance Criteria

## Acceptance Criteria

- [x] `/todo` slash command opens a modal listing open todos
      (`open_only=true` via `todos.list`, sorted by due_at NULLS
      LAST then created_at desc).
- [x] `space` toggles done — first press fires `todos.done`,
      second press fires `todos.undo` (state-aware). `a` opens
      inline text input → `todos.create` with kind=`user`. `d`
      archives via `todos.archive`. `q`/`esc` closes.
- [x] WS auto-refresh: `todo_event` ServerNotice category flips
      `pending_todo_refresh`; the event-loop tick after the
      next WS batch re-fetches the open todos.
- [x] Empty-state UX: "No open todos — press `a` to add one."
- [x] 8 modal-state unit tests covering each keybind, focus
      navigation, idempotent quit, and refocus clamping.
- [x] `/today` and `/week` modals: their done-toggle paths
      already route through `CeremonyService` post-T-0312, which
      writes to the canonical `todos` table — no surface change
      required to satisfy the AC.

## Status Updates

### 2026-05-17 — shipped

- `crates/arawn-tui/src/todo_modal.rs` — fresh module mirroring
  `ceremony_modal.rs` pattern. `TodoModalState` +
  `TodoOutcome` (Toggle / Archive / Add / Close).
- `App.todo_overlay: Option<TodoModalState>` +
  `pending_todo_refresh: bool` for WS-event-driven refresh.
- `event_loop.rs` gained `handle_todo_overlay_key`,
  `fetch_open_todos`, and a `todo_event` category handler.
- `apply_system_notice` recognises `todo_event` and flips the
  refresh flag without spamming chat (mirrors ceremony pattern).
- `render.rs` overlay-stack now includes `todo_modal` on top of
  ceremony_modal on top of standard modals.
- `/todo` slash command added to `CommandRegistry`. Suggestion
  list shows it alongside `/today`/`/week`/`/retro`.

## Implementation Notes

### Technical Approach

- Reuse the `ceremony_modal.rs` pattern from [[ARAWN-T-0308]] —
  `TodoModalState` + `TodoOutcome` enum-driven event handling.
- For the WS subscription, extend the existing notice consumer
  in `event_loop.rs` with a `category="todo_event"` arm; add a
  `pending_todo_refresh` flag mirroring `pending_ceremony_refresh`.

### Dependencies

- Blocked by [[ARAWN-T-0313]] (agent tool family in place,
  RPC stable).
- Blocks [[ARAWN-T-0316]] (UAT exercises TUI flow).

### Risk Considerations

- Empty list UX: many users will hit `/todo` cold. Render an
  empty state with the create hint (`press a to add`) — don't
  bail with an error.