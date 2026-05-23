---
id: non-tool-hook-event-wiring-session
level: task
title: "Non-tool hook event wiring — session, permission, compaction, and input hooks"
short_code: "ARAWN-T-0082"
created_at: 2026-04-04T02:16:26.970143+00:00
updated_at: 2026-04-04T02:36:36.958757+00:00
parent: ARAWN-I-0008
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0008
---

# Non-tool hook event wiring — fire hooks at session, permission, compaction, and input boundaries

## Objective

Wire HookRunner calls at all non-tool hook points throughout the engine. T-0081 covers PreToolUse/PostToolUse; this task covers everything else that we can fire today given our current feature set.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

**Session lifecycle:**
- [ ] `SessionStart` hooks fire on session init (input: session ID, working directory, timestamp, source: startup/resume/clear/compact)
- [ ] `SessionEnd` hooks fire on session shutdown (input: session metadata + reason)
- [ ] `Setup` hooks fire on first-time setup
- [ ] `Stop` hooks fire when the model produces a final response
- [ ] `StopFailure` hooks fire when the model's stop turn fails
- [ ] SessionEnd/Stop hooks have short timeout (default 2s) to avoid blocking shutdown

**User input:**
- [ ] `UserPromptSubmit` hooks fire when user submits a message (input: user message text)

**Permissions (integrates with I-0009):**
- [ ] `PermissionRequest` hooks fire when a tool triggers an ask permission prompt (input: tool name, tool input, rule)
- [ ] `PermissionDenied` hooks fire when user denies a permission prompt

**Compaction:**
- [ ] `PreCompact` hooks fire before context compaction
- [ ] `PostCompact` hooks fire after context compaction (input: compaction stats)

**Tool failure:**
- [ ] `PostToolUseFailure` hooks fire when tool execution throws an error (input: tool name, error)

**Notification:**
- [ ] `Notification` hooks fire on system notifications (matcher filters by notification_type)

**Deferred (defined but not wired — features don't exist yet):**
- SubagentStart, SubagentStop — wire when subagent system is built
- WorktreeCreate, WorktreeRemove — wire when worktree isolation is built
- CwdChanged, FileChanged — wire when file watching is built
- TeammateIdle, TaskCreated, TaskCompleted — wire when task system is built
- Elicitation, ElicitationResult — wire when elicitation is built

**General:**
- [ ] All non-tool hooks use the same HookRunner infrastructure from T-0081
- [ ] Hooks that fail at session boundaries log warnings but don't prevent session start/stop
- [ ] Integration test: SessionStart hook writes a marker file, verify it exists after init

## Implementation Notes

- Depends on T-0081 (HookRunner + engine integration)
- Claude Code uses a 1.5s timeout for SessionEnd hooks — we use 2s as default
- PermissionRequest/PermissionDenied wire into the permission system from I-0009
- PreCompact/PostCompact wire into the compaction pipeline
- UserPromptSubmit wires into the input handling path before the message is sent to the model

## Status Updates

*To be added during implementation*