---
id: input-handling-focus-cycling
level: task
title: "Input handling + focus cycling + keyboard shortcuts"
short_code: "ARAWN-T-0044"
created_at: 2026-04-01T11:46:43.273060+00:00
updated_at: 2026-04-01T12:27:44.893922+00:00
parent: ARAWN-I-0005
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0005
---

# Input handling + focus cycling + keyboard shortcuts

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0005]]

## Objective

Map crossterm key events to Actions, implement the full `App::handle_action` logic for text input, focus cycling, scrolling, and keyboard shortcuts. The bridge between terminal events and the App state machine.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `event.rs`: maps `crossterm::event::KeyEvent` → `Action`. Context-dependent: same key produces different actions depending on current focus.
- [ ] **Input focus**: printable chars → `TypeChar`, Backspace → `Backspace`, Enter → `Submit`, Left/Right → cursor movement, Home/End → cursor jump
- [ ] **Chat focus**: Up/Down → `ScrollUp`/`ScrollDown`, PgUp/PgDn → scroll page
- [ ] **Sidebar focus**: Up/Down → `SidebarUp`/`SidebarDown`, Enter → `SidebarSelect`, `n` → `NewSession`
- [ ] **Global**: Tab → `Tab` (cycles Input→Sidebar→Chat→Input), Ctrl-C → `Quit`, Esc → `Cancel` (if generating)
- [ ] `App::handle_action` fully implemented for all Action variants:
  - `TypeChar`/`Backspace`: update input_buffer + cursor_pos
  - `Submit`: move input_buffer into messages as user message, clear buffer, set `is_generating = true`
  - `Tab`: cycle focus
  - `ScrollUp`/`ScrollDown`: update scroll offset (clamped to message count)
  - `SidebarUp`/`SidebarDown`: update sidebar selection index
  - `Cancel`: set `is_generating = false` (actual cancellation sent by event loop)
- [ ] Submit blocked when `is_generating` or input_buffer is empty
- [ ] Test: TypeChar sequence → input_buffer matches
- [ ] Test: Submit → message added, buffer cleared, is_generating true
- [ ] Test: Tab cycles through all focus states
- [ ] Test: Submit blocked while generating

## Implementation Notes

- `event.rs` in `crates/arawn-tui/src/` — the mapping function is pure (KeyEvent → Option<Action>)
- The actual crossterm event reading loop (async EventStream) comes in T-0045 — this task just defines the mapping and App mutation
- Cursor position tracking: `cursor_pos` is a byte offset into `input_buffer`. Left/Right move by one char (handle multi-byte carefully).
- Scroll offset: `app.scroll_offset` tracks how far from the bottom. 0 = at bottom (auto-scroll). Scrolling up increases offset.
- Depends on: T-0042 (App state + Action enum), T-0043 (render pipeline to visualize)

## Status Updates
- **2026-04-01**: Complete. event.rs with map_key_event(KeyEvent, Focus, is_generating) → Option<Action>. Context-dependent: Input focus maps chars/backspace/enter/cursor/home/end, Chat maps arrows/pgup/pgdn to scroll, Sidebar maps arrows/enter/n. Global: Ctrl-C=Quit, Esc=Cancel (when generating), Tab cycles focus. App::handle_action already complete from T-0042. 7 event mapping tests + 10 App state tests = 23 total TUI tests, clippy clean.