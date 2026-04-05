---
id: tool-result-size-limiting-persist
level: task
title: "Tool result size limiting — persist large outputs to disk, send summary to LLM"
short_code: "ARAWN-T-0030"
created_at: 2026-04-01T10:53:41.956539+00:00
updated_at: 2026-04-01T22:59:52.539243+00:00
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

# Tool result size limiting — persist large outputs to disk, send summary to LLM

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective

When a tool produces output exceeding a size threshold, persist the full output to disk and send a truncated version to the LLM with a pointer to the file. The LLM can then use `file_read` to access the full content if needed. Prevents context window blowout from large shell commands (`ls -R`), file reads, or grep results.

### Priority
- P1 — currently causes hard crashes when tool output exceeds Groq's message size limit

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `DEFAULT_MAX_RESULT_SIZE_CHARS = 50_000` — per-tool result cap
- [ ] When tool output exceeds threshold: save full output to `~/.arawn/sessions/<id>/tool-results/<uuid>.txt`
- [ ] Replace tool result content with truncated preview + "Full output saved at <path>. Use file_read to access."
- [ ] Per-tool configurable threshold (some tools may want higher/lower limits)
- [ ] Engine passes the truncated version to the LLM, not the full output
- [ ] JSONL persistence stores the truncated version (full output is in the separate file)
- [ ] Test: large shell output gets persisted and truncated
- [ ] Test: small output passes through unchanged

## Implementation Notes

- Based on Claude Code's `toolResultStorage.ts` pattern (see `claude-code/src/utils/toolResultStorage.ts` and `claude-code/src/constants/toolLimits.ts`)
- Claude Code constants: 50k chars per result, 100k tokens max, 200k chars aggregate per message
- Implement as a middleware in the engine's tool execution path — after `Tool::execute` returns, check size and persist if needed
- Could be a `ToolOutput::truncate_if_needed(session_id, data_dir)` method or a separate `ToolResultStorage` service
- Affects: ShellTool (ls -R), FileReadTool (large files), GrepTool (many matches)

## Status Updates
- **2026-04-01**: Complete. tool_result_limiter.rs with limit_tool_result(output, session_id, data_dir, max_chars). Default 50k chars threshold. Exceeding outputs: full content persisted to ~/.arawn/sessions/<id>/tool-results/<uuid>.txt, replaced with 2k char preview + "Full output saved at <path>. Use file_read to access." Wired into QueryEngine tool execution loop via config.data_dir. Binary sets data_dir in both CLI and serve modes. 5 unit tests: pass-through small, truncate+persist large, preview contains header, error flag preserved, custom threshold. 236 total workspace tests, clippy clean.