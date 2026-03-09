---
id: bug-tool-arguments-and-results
level: task
title: "Bug: Tool arguments and results stored as null/empty in session log"
short_code: "ARAWN-T-0270"
created_at: 2026-03-06T03:14:15.589393+00:00
updated_at: 2026-03-08T01:39:01.641393+00:00
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

# Bug: Tool arguments and results stored as null/empty in session log

## Objective

Tool call arguments are stored as `null` and tool results have empty content in the workstream messages.jsonl. This means session history loses all tool interaction detail, making replay and debugging impossible.

## Backlog Item Details

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P1 - High (important for user experience)

### Impact Assessment
- **Affected Users**: All users — tool history is lost for every session
- **Reproduction Steps**:
  1. Start the server and send a message that triggers tool use
  2. Inspect `~/.config/arawn/workstreams/workstreams/<id>/messages.jsonl`
  3. Look at `tool_use` and `tool_result` entries
- **Expected vs Actual**:
  - Expected: `tool_use` entries contain arguments JSON; `tool_result` entries contain output content
  - Actual: Arguments are `null`, result content is empty `""`

### Evidence

From workstream `76ea4fdc`:
```json
// tool_use — arguments null
{"role":"tool_use","content":"","metadata":"{\"tool_id\":\"fc_10f20ee1...\",\"name\":\"shell\",\"arguments\":null}"}

// tool_result — content empty
{"role":"tool_result","content":"","metadata":"{\"tool_call_id\":\"fc_10f20ee1...\",\"success\":false}"}
```

The `shell` tool was called with null arguments (causing failure), and `web_fetch` results returned `success:true` but with no content stored.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `tool_use` entries in messages.jsonl include the full arguments JSON
- [ ] `tool_result` entries include the tool output content
- [ ] Existing session replay works with populated fields

## Implementation Notes

### Likely Areas
- `crates/arawn-workstream/` — message serialization when persisting tool calls
- `crates/arawn-agent/src/agent.rs` — how tool calls/results are passed to the workstream store
- The `metadata` field is a stringified JSON — arguments may be lost during serialization

## Status Updates

### Investigation Result: Already Fixed

**Root Cause (confirmed):** In the WS handler's `handle_chat` function (handlers.rs), the pre-fix code:
- `StreamChunk::ToolStart { id, name }` — destructured WITHOUT `arguments`, hardcoded `arguments: serde_json::Value::Null`
- `StreamChunk::ToolEnd { id, success, .. }` — destructured WITHOUT `content`, used only streaming output (empty when no streaming)

**Fix (already applied in commit `19907b8`):**
- `ToolStart` now captures `arguments` from the stream chunk
- `ToolEnd` now falls back to the chunk's `content` when no streaming output was accumulated

**Data Validation:**
- All existing tool data in messages.jsonl is from March 6 (before server was rebuilt with fix)
- The server hasn't processed any new tool-using messages since the fix was deployed
- Code review confirms the fix is correct — `StreamChunk::ToolStart` carries `tool_use.input.clone()` from the LLM response, and `ToolEnd` carries the tool execution output

**Note:** There is a separate design concern where `stream.rs` makes a redundant sync API call after streaming to get tool uses (double cost). This is a separate optimization issue, not related to the persistence bug.