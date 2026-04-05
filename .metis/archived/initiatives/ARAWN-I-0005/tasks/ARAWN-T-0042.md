---
id: arawn-tui-crate-scaffolding-app
level: task
title: "arawn-tui crate scaffolding + App state + headless TestBackend"
short_code: "ARAWN-T-0042"
created_at: 2026-04-01T11:46:41.650042+00:00
updated_at: 2026-04-01T12:23:04.203149+00:00
parent: ARAWN-I-0005
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0005
---

# arawn-tui crate scaffolding + App state + headless TestBackend

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0005]]

## Objective

Create the `arawn-tui` crate with the foundational `App` state machine and headless `TestBackend` support. This is the skeleton everything else renders into — the App holds all mutable state, actions modify it, and render() draws it. Testable from day one via ratatui's `TestBackend`.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `crates/arawn-tui/` added to workspace
- [ ] Dependencies: `ratatui`, `crossterm`, `arawn-service` (trait only), `tokio`, `serde_json`
- [ ] `App` struct with state: `focus: Focus`, `input_buffer: String`, `cursor_pos: usize`, `messages: Vec<ChatMessage>`, `workstreams: Vec<WorkstreamInfo>`, `sessions: Vec<SessionInfo>`, `current_workstream: Option<WorkstreamInfo>`, `current_session: Option<SessionInfo>`, `is_generating: bool`, `streaming_text: String`
- [ ] `Focus` enum: `Input`, `Sidebar`, `Chat`
- [ ] `Action` enum: `TypeChar(char)`, `Backspace`, `Submit`, `Tab`, `Quit`, `ScrollUp`, `ScrollDown`, `SidebarUp`, `SidebarDown`, `SidebarSelect`, `NewSession`, `Cancel`
- [ ] `ChatMessage` display type: role (user/assistant/tool), content, tool_name for tool calls
- [ ] `App::new()` constructor with empty state
- [ ] `App::handle_action(action: Action)` — mutates state based on action
- [ ] `render(app: &App, frame: &mut Frame)` — pure function, placeholder layout (draws frame border)
- [ ] Headless test: create App → handle_action(TypeChar) → verify input_buffer updated
- [ ] Headless test: render with TestBackend → verify frame not empty
- [ ] Crate compiles, workspace passes

## Implementation Notes

- `lib.rs`, `app.rs`, `action.rs`, `render.rs` in `crates/arawn-tui/src/`
- `App` does NOT depend on any runtime — no WebSocket connection, no tokio. Pure state machine. The event loop (T-0045) bridges runtime events to actions.
- `ChatMessage` is a display-only type — not `arawn_core::Message`. It's what the TUI renders. Conversion from service types happens in the event loop.
- ratatui `TestBackend` is built-in: `Terminal::new(TestBackend::new(80, 24))`
- Keep render() minimal in this task — just the frame border + placeholder text. Real layout comes in T-0043.
- Depends on: nothing (new standalone crate)

## Status Updates
- **2026-04-01**: Complete. arawn-tui crate with ratatui 0.29 + crossterm 0.28. App state machine with full handle_action for all Action variants: typing, backspace, delete, cursor movement, submit, tab focus cycling, scroll, sidebar navigation, cancel, quit. ChatMessage display type with User/Assistant/ToolCall/ToolResult/System roles. render() with three-panel layout (status bar + sidebar/chat split + input bar), focus-dependent border colors, cursor positioning. pending_submit for event loop bridge. 13 tests: 10 App state tests + 3 headless render tests (80x24, 40x12, 120x40). 178 total workspace tests, clippy clean.