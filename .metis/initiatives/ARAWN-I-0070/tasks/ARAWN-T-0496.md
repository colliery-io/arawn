---
id: gui-s2-action-item-inbox-surface
level: task
title: "GUI-S2: Action-item inbox surface (scan / dismiss / snooze / open)"
short_code: "ARAWN-T-0496"
created_at: 2026-06-13T16:02:54.454634+00:00
updated_at: 2026-06-13T16:02:54.454634+00:00
parent: ARAWN-I-0070
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0070
---

# GUI-S2: Action-item inbox surface (scan / dismiss / snooze / open)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0070]]

## Objective **[REQUIRED]**

The differentiating surface: an **inbox-style action-item triage** — scan a list, dismiss/snooze/open, keyboard-friendly. This is the UX that's a grind in ratatui and native in HTML, and the reason for the GUI per ADR A-0005.

### Type
- [x] Feature — GUI surface

### Priority
- [x] P2 - Medium

## Acceptance Criteria **[REQUIRED]**

- [ ] Inbox lists action items (`todos.list` + ceremony action items) as bulk-scannable rows.
- [ ] Dismiss / open / (snooze) actions wired to existing RPCs (`todos.done` / `undo` / `patch` / `archive`); rows update optimistically via fragments.
- [ ] Keyboard navigation for fast triage (e.g. j/k to move, x to dismiss, enter to open).
- [ ] Empty + large-list states handled.
- [ ] Test: list renders sample items; an action calls the correct RPC and updates the row. `angreal check all` green.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Consume `todos.*` (and ceremony item) reads; render rows; wire actions through the hypermedia runtime to the existing mutation RPCs with optimistic fragment swaps.

### Dependencies
GUI-F2.

### Risk Considerations
**Snooze** may have no backing RPC today — verify; if missing, file a protocol task (`todos.snooze` / a due-date patch) rather than faking it client-side. Optimistic updates must reconcile on server confirm/err.

## Status Updates **[REQUIRED]**

*To be added during implementation*