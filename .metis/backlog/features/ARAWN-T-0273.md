---
id: tui-mouse-scroll-support-and
level: task
title: "TUI: Mouse scroll support and scroll position indicator"
short_code: "ARAWN-T-0273"
created_at: 2026-03-06T13:50:01.257795+00:00
updated_at: 2026-03-08T01:50:30.393872+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# TUI: Mouse scroll support and scroll position indicator

## Objective

Enable mouse scroll wheel navigation in the TUI chat view and add a visual scroll position indicator (scrollbar or percentage) so users can orient themselves in long conversations.

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [x] P1 - High (important for user experience)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Mouse scroll wheel scrolls chat content up/down in the TUI
- [ ] A visual indicator (scrollbar, percentage, or position marker) shows current scroll position relative to total content
- [ ] Scroll behavior feels natural (appropriate speed, smooth)

## Status Updates

### Implementation Complete
- Added `Mouse(MouseEvent)` variant to `Event` enum in `events.rs`, forwarding `CrosstermEvent::Mouse` events
- Added `PanelAreas` struct to `app.rs` storing cached `Rect` for chat, tool_pane, logs, sidebar
- Layout stores panel areas during render for mouse hit-testing
- Added `handle_mouse()` method routing `ScrollUp`/`ScrollDown` to correct panel based on mouse cursor position
- Added `panel_at(col, row)` hit-test method checking sidebar → logs → tool_pane → chat
- Scroll lines per wheel tick: 3
- Added scroll position indicator (percentage + ↓) at top-right of chat area when not at bottom
- All checks pass: `cargo check`, `angreal check clippy`, `angreal check fmt`

*To be added during implementation*