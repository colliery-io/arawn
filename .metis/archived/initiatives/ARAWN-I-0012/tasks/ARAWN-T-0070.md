---
id: progress-indicators-spinners-for
level: task
title: "Progress indicators — spinners for tool execution, streaming indicator, elapsed time"
short_code: "ARAWN-T-0070"
created_at: 2026-04-03T02:38:31.159554+00:00
updated_at: 2026-04-03T17:45:32.806130+00:00
parent: ARAWN-I-0012
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0012
---

# Progress indicators — spinners for tool execution, streaming indicator, elapsed time

## Parent Initiative

[[ARAWN-I-0012]]

## Objective

Add animated progress indicators so users can see when tools are executing and when the assistant is generating. Replace the current static state with visual feedback that something is happening.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Animated spinner shown next to tool calls while they execute (unicode braille or dots)
- [ ] Tool execution shows tool name + elapsed time while running (e.g., `⠋ Bash 3.2s`)
- [ ] Streaming indicator: pulsing cursor or `...` animation while waiting for first token
- [ ] Generating indicator in status bar while assistant is producing output
- [ ] Spinners stop and show final status (checkmark/X) when tool completes
- [ ] Animation driven by TUI tick event — no extra threads

## Implementation Notes

### Technical Approach
- Create a `Spinner` widget (or use ratatui's if available) that cycles through frames on tick
- Spinner state tracked per-tool-call — start time recorded when tool begins, elapsed calculated on each render
- Streaming indicator lives in the chat area at the cursor position
- Status bar generating indicator is a separate spinner instance
- All animation driven by the existing TUI tick interval (likely 100-250ms)

### Dependencies
- Closely related to T-0068 (tool call display) — spinners are part of tool call rendering
- Depends on T-0066 (unified layout)

## Status Updates

- Tool call spinner: braille animation on the *running* tool call only (not all)
- Elapsed time: `⠋ shell cargo test 3.2s` — uses `created_at: Instant` on ChatMessage
- Tool call icons: ✓ (success), ✗ (error from next result), braille spinner (running), ⏳ (pending)
- Pre-computed `tool_call_flags` vec to avoid borrow conflicts in render loop
- Thinking indicator: `⠋ thinking... 2.3s` when generating with no content yet
- `generation_started: Option<Instant>` on App, set on Submit, cleared on Complete/Error
- Status bar spinner already done in T-0069
- All driven by existing 100ms tick interval
- 73 tests passing