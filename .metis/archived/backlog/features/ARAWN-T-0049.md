---
id: tui-mouse-support-click-to-focus
level: task
title: "TUI mouse support — click to focus, click sidebar items, scroll wheel"
short_code: "ARAWN-T-0049"
created_at: 2026-04-01T16:23:10.906379+00:00
updated_at: 2026-04-02T14:00:36.749554+00:00
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

# TUI mouse support — click to focus, click sidebar items, scroll wheel

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective

Add mouse event support to the TUI. crossterm + ratatui fully support mouse capture — clicks, scroll wheel, and drag. Enables: click to focus panels, click sidebar items to select, scroll wheel to scroll chat, click input to place cursor.

### Priority
- P3 — UX polish, not blocking. Keyboard-only works fine.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Enable `EnableMouseCapture` on terminal setup, `DisableMouseCapture` on teardown
- [ ] Click in sidebar area → Focus::Sidebar, click on specific workstream/session → select it
- [ ] Click in chat area → Focus::Chat
- [ ] Click in input area → Focus::Input, click position → set cursor_pos
- [ ] Scroll wheel in chat area → ScrollUp/ScrollDown
- [ ] Scroll wheel in sidebar → SidebarUp/SidebarDown
- [ ] Map `crossterm::event::Event::Mouse(MouseEvent)` → Action in event.rs
- [ ] Need to track panel regions from last render to determine which panel a click targets
- [ ] Test: mouse click coordinates map to correct focus panel
- [ ] Test: scroll wheel in chat maps to scroll actions

## Implementation Notes

- `crossterm::event::EnableMouseCapture` / `DisableMouseCapture` in event_loop.rs setup/teardown
- `MouseEvent` has `column`, `row`, `kind` (Down/Up/Drag/Moved/ScrollDown/ScrollUp)
- Need to store the layout regions (sidebar_rect, chat_rect, input_rect) from the last render and use them for hit-testing. Can store on App or pass through a separate struct.
- For click-to-position in input: map click column to cursor_pos (account for border + padding)
- For sidebar item selection: map click row to sidebar index (account for border + header rows)
- Depends on: existing TUI (I-0005)

## Status Updates
*To be added during implementation*