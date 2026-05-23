---
id: tui-has-no-clear-working-idle
level: task
title: "TUI has no clear working/idle state indicator — impossible to tell if agent is thinking or waiting for input"
short_code: "ARAWN-T-0114"
created_at: 2026-04-06T10:05:10.980984+00:00
updated_at: 2026-04-06T11:23:28.181546+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# TUI has no clear working/idle state indicator — impossible to tell if agent is thinking or waiting for input

## Objective

After tool calls execute, the TUI gives no indication of whether the agent is still working (waiting for next LLM response) or has finished and is waiting for user input. The chat area just stops at the last tool result with no visual boundary. Users can't tell if they should type or wait.

Screenshot from 2026-04-06 shows: `file_write` error → `file_read` success → then nothing. Is the agent thinking? Crashed? Waiting for me?

## Backlog Item Details

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P0 - Critical (blocks users/revenue)

### Expected vs Actual
- **Expected**: Clear visual states — spinner/animation when agent is thinking, distinct "ready for input" state when done
- **Actual**: No distinction between "agent is processing" and "agent is done, your turn". Chat just stops.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Visible spinner/animation in status bar or chat area while agent is working (LLM call in flight, tools executing)
- [ ] Clear "ready" state when agent finishes — input area activates, status changes, or visual separator appears
- [ ] During long LLM calls (local models, 10-30s), the spinner keeps animating so user knows it's not frozen
- [ ] After the final assistant message, a visual boundary or prompt indicator shows it's the user's turn

## Implementation Notes

### What exists
- `app.is_generating` boolean tracks whether the engine is running
- Spinner frame animates on a tick interval when `is_generating` is true
- `Complete` event sets `is_generating = false`

### What's likely broken
The live progress events (T-0111 fix) send `ToolCallStart`/`ToolCallResult` during the loop, but the `Complete` event only fires after `engine.run()` returns. Between tool calls — while the next LLM call is in flight — there may be no events at all, so the spinner might not be visible or the state might be wrong.

### Needed
1. **Status bar**: Show clear states — "Thinking...", "Running [tool_name]...", "Ready"
2. **Spinner**: Must keep animating between tool calls (during LLM round-trips)
3. **End-of-turn marker**: Visual separator or input focus change when agent is done
4. **Input area**: Visually disabled/grayed when agent is working, active when ready

### Key files
- `crates/arawn-tui/src/event_loop.rs` — tick interval, spinner, is_generating
- `crates/arawn-tui/src/render.rs` — status bar rendering
- `crates/arawn-tui/src/app.rs` — is_generating state management

### Additional symptom: missed flush on final text response
User reported 2026-04-06: agent asked "What specific repos should I prioritize?" but the text was NOT visible on screen until the user typed their next message ("do it"). The message was in memory but the screen hadn't been redrawn. This means the `Complete` event's flush isn't reliably triggering `terminal.draw()`, or the final assistant text arrives in a way that doesn't trigger a redraw.

This is a **data loss UX bug** — the user responded to a question they couldn't see, which means they missed the context and gave a blind answer.

## Status Updates

*To be added during implementation*