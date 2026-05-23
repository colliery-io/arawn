---
id: tool-call-display-status
level: task
title: "Tool call display — status indicators, compact inline format, expandable results"
short_code: "ARAWN-T-0068"
created_at: 2026-04-03T02:38:28.998490+00:00
updated_at: 2026-04-03T12:44:53.302911+00:00
parent: ARAWN-I-0012
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0012
---

# Tool call display — status indicators, compact inline format, expandable results

## Parent Initiative

[[ARAWN-I-0012]]

## Objective

Replace the current plain-text `[tool: name]` prefix rendering with compact, visually distinct tool call indicators. Running tools show progress, completed tools show success/failure, and results are truncated by default with expand-on-scroll.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Tool calls render with a status icon: `⏺` yellow/blinking while running, green on success, red on error
- [ ] Tool name displayed inline (e.g., `⏺ Bash`, `⏺ Read`)
- [ ] Tool input shown as compact summary (command for Bash, path for Read, pattern for Grep)
- [ ] Tool results truncated to ~5 lines by default
- [ ] Results expandable when user scrolls to them (or keybind to expand/collapse)
- [ ] Errors show red indicator + error text prominently
- [ ] Clear visual distinction between tool call (request) and tool result (response)

## Implementation Notes

### Technical Approach
- New `ToolCallWidget` in `crates/arawn-tui/src/widgets/` that encapsulates the rendering
- Each tool call in the message gets its own widget instance with state (collapsed/expanded)
- Status derived from the tool use block's state (pending/streaming/complete/error)
- Collapse state tracked per-tool-call in the chat panel's state

### Dependencies
- Depends on T-0066 (unified layout) — renders within the new chat panel

## Status Updates

- Tool calls: `⏳ ToolName args` (yellow while generating) / `✓ ToolName args` (green when done)
- Tool results: collapsed by default with `▸ name result (N lines)`, first 5 lines shown
- Expanded results: `▾ name result` with all lines
- Errors: `✗ name error` in red with content always visible (up to 10 lines)
- `ToggleToolResult(idx)` action + `expanded_tool_results` HashSet on App
- Tool input args threaded from engine: added `input: Value` to EngineEvent::ToolCallStart
- `format_tool_input()` extracts key args per tool (command for shell, path for file ops, pattern for grep)
- Decided against separate widget — inline rendering in render_chat is simpler
- 70 total tests passing, full workspace green