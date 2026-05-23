---
id: microcompact-lightweight-tool
level: task
title: "Microcompact — lightweight tool result clearing without LLM call"
short_code: "ARAWN-T-0061"
created_at: 2026-04-03T02:03:49.034755+00:00
updated_at: 2026-04-03T22:20:26.284156+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Microcompact — lightweight tool result clearing without LLM call

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective

Replace old tool results in the message history with short stubs without making an LLM call. This runs before full compaction and prevents context bloat from large grep/shell/file_read outputs that are no longer relevant. Claude Code's microcompact targets shell, file_read, file_write, grep, glob, and web tools — replacing their content with "[result cleared]" after N turns.

### Type: Feature | Priority: P1 | Effort: M

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] After N turns (configurable, default 5), old ToolResult messages have content replaced with stub
- [ ] Targeted tools: shell, file_read, grep, glob, web_fetch, web_search (large-output tools)
- [ ] Think, task_*, ask_user results NOT cleared (small, semantically important)
- [ ] Runs each turn before the auto-compact threshold check — may prevent full compaction entirely
- [ ] Preserves tool_use_id so message structure stays valid
- [ ] Token savings logged for observability

## Implementation Notes

- Add `microcompact()` method to the engine, called at top of each turn in `QueryEngine::run()`
- Walk message history, find ToolResult messages older than N turns for targeted tools
- Replace content with `"[Previous {tool_name} result cleared — {original_len} chars]"`
- Keep is_error flag intact
- No LLM call needed — pure in-place message rewriting
- Reference: Claude Code's `microCompact.ts`

## Status Updates

- `Session::microcompact(keep_recent)` — pure in-place rewriting, no LLM call
- Targeted: shell, Bash, file_read, Read, grep, Grep, glob, Glob, web_fetch, web_search, file_write, file_edit
- Skips: results < 100 chars, errors, non-targeted tools (think, task_*)
- Stub format: `[Previous {tool_name} result cleared — {N} chars]`
- Called at top of each engine turn with keep_recent=6
- Builds tool_use_id → tool_name map from Assistant messages
- 5 tests: clears old, preserves recent, skips small, skips errors, skips non-targeted