---
id: tui-does-not-support-text
level: task
title: "TUI does not support text selection or copy-to-clipboard"
short_code: "ARAWN-T-0113"
created_at: 2026-04-06T10:02:28.366117+00:00
updated_at: 2026-04-06T11:07:10.281031+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#bug"
  - "#phase/active"


exit_criteria_met: false
initiative_id: NULL
---

# TUI does not support text selection or copy-to-clipboard

## Objective

Users cannot select or copy text from the TUI chat area. When the agent produces a report, code snippet, or URL, there's no way to copy it to the clipboard without going to the JSONL files on disk.

## Backlog Item Details

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P1 - High (important for user experience)

### Expected vs Actual
- **Expected**: Mouse drag selects text, Cmd+C copies to system clipboard. Or a keyboard shortcut copies the last message / selected message.
- **Actual**: Mouse events are captured by the TUI (crossterm) for scrolling/clicking. No text selection or clipboard support.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] User can copy text from chat messages to system clipboard
- [ ] At minimum: a command or keybinding to copy the last assistant message (e.g., `/copy` or Cmd+C)
- [ ] Ideally: mouse-based text selection in the chat area with clipboard copy

## Implementation Notes

### Approaches
1. **Copy last message command** — `/copy` puts the last assistant message content on the clipboard via `arboard` or `clipboard` crate. Simplest.
2. **Mouse text selection** — track mouse drag events in the chat area, highlight selected text, copy on Cmd+C. Complex — requires mapping screen coordinates back to text content.
3. **Export command** — `/export` writes the last message (or full session) to a file. Workaround, not a real fix.

### Key consideration
The TUI uses `EnableMouseCapture` which intercepts all mouse events. Most terminal emulators support holding Option/Shift to bypass TUI mouse capture and use native terminal selection — this might already work as a workaround. Worth documenting even if we add a proper solution.

### Reference
Claude Code's terminal output is plain text (not a TUI), so copy-paste works natively. For TUI apps, `arboard` crate provides cross-platform clipboard access.

## Status Updates

*To be added during implementation*