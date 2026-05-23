---
id: tui-interactive-rendering-for
level: task
title: "TUI interactive rendering for AskUser tool — selectable options with keyboard navigation"
short_code: "ARAWN-T-0060"
created_at: 2026-04-03T01:34:13.423658+00:00
updated_at: 2026-04-05T16:15:45.144589+00:00
parent: ARAWN-I-0012
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# TUI interactive rendering for AskUser tool — selectable options with keyboard navigation

## Objective

When the `ask_user` tool fires, the TUI should render the questions as interactive selectable options (arrow keys to navigate, Enter to select) instead of plain text that requires the user to type a number. Multi-select questions should support toggling options with Space.

Currently the ask_user tool returns formatted text and the user has to type "1", "2", etc. in the input field. This works but feels clunky — interactive selection is the expected UX.

### Type: Feature | Priority: P1

- **User Value**: Natural, keyboard-driven selection instead of typing numbers. Matches the Claude Code AskUserQuestion rendering.
- **Effort Estimate**: M — touches WebSocket protocol, TUI app state, rendering, and input handling.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] TUI detects `ask_user` tool results and renders them as a selection widget
- [ ] Arrow keys (Up/Down) navigate between options
- [ ] Enter selects the highlighted option
- [ ] For `multiSelect` questions: Space toggles selection, Enter confirms
- [ ] Selected answer sent back as a user message to the server
- [ ] Falls back gracefully to text rendering for CLI mode (no TUI)
- [ ] Header and question text displayed above options

## Status

**Absorbed into T-0071** — the unified InteractiveModal primitive handles both permission prompts and AskUser questions with the same widget.

## Implementation Notes (historical)

### Protocol changes
- WebSocket `EngineEvent` needs a new variant (e.g., `AskUser { questions: Vec<Question> }`) so the TUI can distinguish ask_user results from regular tool results
- Server-side: when the engine produces an ask_user tool result, emit the structured questions rather than the formatted text
- Response: TUI sends the user's selections back as a `send_message` with the answer text

### TUI changes
- New `Focus` state: `AskUser` — entered when ask_user event arrives
- Render questions as a list widget with highlight cursor
- Input handling: arrow keys move cursor, Enter/Space select
- On confirm: format answer as text ("React" or "Jest, Vitest") and send as user message
- Exit AskUser focus and return to normal Chat/Input flow

### Dependencies
- WebSocket protocol (arawn-service) for structured event
- TUI app state (arawn-tui) for new focus mode
- Render module for selection widget

### Reference
- Claude Code's `AskUserQuestionTool.tsx` renders options as a vertical list with selection indicators
- Preview feature (optional): side-by-side layout with markdown preview pane per option