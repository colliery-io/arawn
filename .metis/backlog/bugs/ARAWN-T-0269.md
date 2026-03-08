---
id: bug-user-messages-stored-twice-in
level: task
title: "Bug: User messages stored twice in session log"
short_code: "ARAWN-T-0269"
created_at: 2026-03-06T03:14:14.909494+00:00
updated_at: 2026-03-07T21:24:52.473880+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Bug: User messages stored twice in session log

## Objective

Every user message sent via the TUI is stored twice in the workstream messages.jsonl file, approximately 0.5-6 seconds apart. This inflates conversation history and may cause the LLM to see duplicate context.

## Backlog Item Details

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [ ] P1 - High (important for user experience)

### Impact Assessment
- **Affected Users**: All TUI users
- **Reproduction Steps**:
  1. Start the arawn server
  2. Open the TUI and connect to a workstream
  3. Send any message (e.g., "Hello Arawn")
  4. Inspect `~/.config/arawn/workstreams/workstreams/<id>/messages.jsonl`
- **Expected vs Actual**:
  - Expected: One JSONL entry per user message
  - Actual: Two entries with identical content, different IDs and timestamps ~0.5-6s apart

### Evidence

From `scratch/messages.jsonl`:
```
Line 5: {"role":"user","content":"Hello Arawn","timestamp":"2026-03-06T03:03:16.601287Z"}
Line 6: {"role":"user","content":"Hello Arawn","timestamp":"2026-03-06T03:03:17.171626Z"}
```

Every user message in both workstreams exhibits this pattern.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Each user message produces exactly one entry in messages.jsonl
- [ ] Existing conversation replay is not broken by the fix

## Implementation Notes

### Likely Areas
- `crates/arawn-server/src/routes/` — HTTP/WebSocket handler may be firing twice
- `crates/arawn-workstream/` — message persistence layer
- TUI client may be double-submitting (less likely since server logs show single requests)

## Status Updates

### Investigation Result: Already Fixed

**Root Cause (confirmed):** The WS handler in `handlers.rs` was calling `ws_manager.send_message()` to store the user message immediately when received, AND then `save_turn()` was storing it again when the agent completed. Two writes per user message.

**Fix (already applied in commit `19907b8`):** Removed the immediate `send_message()` call. Only `save_turn()` now persists user messages, along with tool calls, results, and assistant response — all in one atomic write.

**Validation:**
- Session `9dec55cc` (before fix, `03:03 UTC`): Every user message duplicated, deltas match agent processing time (0.5–6s)
- Session `772df0cf` (after fix, `13:38 UTC`): No duplicates — one user + one assistant per turn
- The existing data still contains old duplicates but new messages are clean

**No code changes needed.** The bug was fixed in the same commit that added the explanatory comment at line 371-373 of `handlers.rs`.