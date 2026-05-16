---
id: tui-todo-slash-command-done-toggle
level: task
title: "TUI /todo slash command + done-toggle integration"
short_code: "ARAWN-T-0314"
created_at: 2026-05-16T22:52:35.918379+00:00
updated_at: 2026-05-16T22:52:35.918379+00:00
parent: ARAWN-I-0049
blocked_by: [ARAWN-T-0313]
effort: M
archived: false

tags:
  - "#task"
  - "#phase/todo"


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

- [ ] `/todo` slash command opens a modal listing open todos
      (kind in {user, weekly_priority, rollover}, `done_at IS
      NULL`). Sorted by due_at NULLS LAST, then created_at desc.
- [ ] `space` toggles done via `todos.done` / `todos.undo`. `a`
      opens inline text input → `todos.create` with kind=`user`.
      `d` archives via `todos.archive`. `q`/`esc` exits.
- [ ] `/today` and `/week` modals' priority/todo done-toggle
      paths route through `todos.done` instead of any
      ceremony-specific mutation.
- [ ] WS auto-refresh: on `TodoEvent::*` notices, the active
      `/todo` modal re-fetches and re-renders.
- [ ] Modal-state unit tests for each keybind; one integration
      test driving the modal through create + done + archive.

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

## Status Updates

*To be added during implementation*
