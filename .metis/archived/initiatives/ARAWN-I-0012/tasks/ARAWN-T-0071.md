---
id: permission-prompt-modal-centered
level: task
title: "Permission prompt modal — centered dialog for tool permission requests"
short_code: "ARAWN-T-0071"
created_at: 2026-04-03T02:38:32.402938+00:00
updated_at: 2026-04-03T18:04:21.108298+00:00
parent: ARAWN-I-0012
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0012
---

# Interactive modal — unified primitive for permission prompts and AskUser tool

*Absorbs T-0060 (AskUser interactive rendering).*

## Parent Initiative

[[ARAWN-I-0012]]

## Objective

Build a single `InteractiveModal` widget that handles all user-facing interactive prompts: permission requests (Allow/Deny), AskUser tool questions, and any future tool that needs user input. Inspired by Claude Code's layered approach: visual chrome (Dialog) → selection list (Select) → state machine → overlay key capture.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `InteractiveModal` widget renders as a centered overlay on the chat area
- [ ] Configurable: title, subtitle, options list, optional free-text input, border color
- [ ] Arrow keys navigate options, Enter selects, Escape cancels
- [ ] When modal is active, ALL key events route to modal (overlay pattern)
- [ ] Result returned via async channel (tool execution awaits response)
- [ ] Permission prompt use case: title="Permission Required", options=[Allow Once, Always, Deny], amber border
- [ ] AskUser use case: title=question, options=choices OR free text input, cyan border
- [ ] Chat content visible but dimmed behind modal
- [ ] Graceful degradation on small terminals (min width/height check)
- [ ] `TuiPermissionPrompt` implements the `PermissionPrompt` trait from I-0009 using this modal
- [ ] Unit tests for modal state (navigation, selection, cancel)

## Architecture

### Layers (following Claude Code pattern)

1. **ModalChrome** — centered Block with rounded border, title, subtitle, footer hints ("Enter to confirm / Esc to cancel")
2. **ModalSelect** — list of options with focused highlight, arrow key navigation
3. **ModalState** — focused_index, options, optional text_input buffer, result channel
4. **Overlay capture** — when `app.active_modal.is_some()`, event.rs routes all keys to modal handler

### Data flow

```
Tool calls PermissionPrompt::prompt(request)
    → sends ModalRequest through channel to TUI event loop
    → TUI shows InteractiveModal, captures keys
    → user selects option
    → result sent back through oneshot channel
    → PermissionPrompt::prompt() returns PermissionResponse
```

### App state

```rust
pub struct App {
    // ...
    pub active_modal: Option<ModalState>,
}

pub struct ModalState {
    pub title: String,
    pub subtitle: Option<String>,
    pub options: Vec<ModalOption>,
    pub focused_index: usize,
    pub border_color: Color,
    pub result_tx: oneshot::Sender<usize>,  // sends selected index
}

pub struct ModalOption {
    pub label: String,
    pub description: Option<String>,
}
```

## Dependencies
- I-0009 permission system (completed) — provides PermissionPrompt trait
- T-0066 unified layout (completed) — rendering context

## Status Updates

- Replaced `PermissionPrompt` with generic `ModalPrompt` — tools build their own config
- `ModalRequest { title, subtitle, options }` → `Option<usize>`
- PermissionChecker builds [Allow Once, Always, Deny], interprets index
- InteractiveModal: centered overlay, border, arrow-key nav, Enter/Escape
- ModalState with oneshot channel for async result
- TuiModalPrompt bridges trait → TUI via mpsc
- CliModalPrompt (stdin), MockModalPrompt (tests)
- Overlay key capture when modal active
- Absorbed T-0060
- 124 tests passing