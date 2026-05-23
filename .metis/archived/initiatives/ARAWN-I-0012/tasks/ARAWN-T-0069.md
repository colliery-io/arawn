l---
id: status-bar-redesign-model-tokens
level: task
title: "Status bar redesign — model, tokens, session info, generating indicator"
short_code: "ARAWN-T-0069"
created_at: 2026-04-03T02:38:30.329023+00:00
updated_at: 2026-04-03T17:04:22.685336+00:00
parent: ARAWN-I-0012
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0012
---

# Status bar redesign — model, tokens, session info, generating indicator

## Parent Initiative

[[ARAWN-I-0012]]

## Objective

Redesign the status bar to show useful session info at a glance: model name, token usage, session identifier, and a generating indicator when the assistant is streaming.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Status bar positioned at the bottom of the screen (above the input area)
- [ ] Shows current model name (e.g., "claude-sonnet-4-20250514")
- [ ] Shows token count: input tokens + output tokens (e.g., "1.2k / 340")
- [ ] Shows session short-id or workstream name if available
- [ ] Animated generating indicator when assistant is streaming (spinner or pulsing dot)
- [ ] Cost display if cost data is available from the API response
- [ ] Status bar doesn't steal vertical space from chat when not needed — 1 line max

## Implementation Notes

### Technical Approach
- Rework the existing status bar widget in `crates/arawn-tui/src/ui/`
- Pull token counts from the API response metadata (usage field)
- Model name from the session/config
- Spinner uses frame-based unicode animation (braille or dots pattern), ticked on the TUI's tick event

### Dependencies
- Depends on T-0066 (unified layout) — status bar position changes with the new layout

## Status Updates

- Status bar moved from top to bottom (below input, above nothing)
- Layout: chat(flex) + separator(1) + input(1) + status(1)
- Shows: model │ tokens │ workstream │ session_id(8) │ spinner
- Token display: format_tokens() — 500, 1.2k, 12.3k, 1.5M
- Braille spinner: ⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ cycle at 100ms via tokio tick interval
- Added EngineEvent::Usage { input_tokens, output_tokens } emitted before Complete
- Model name passed through run_tui() from config.engine_llm().model
- Dark bg (30,30,40) for status bar instead of DarkGray
- Cost display deferred
- 73 tests passing, all snapshots updated