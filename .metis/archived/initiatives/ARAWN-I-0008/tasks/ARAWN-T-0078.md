---
id: hook-core-types-hookevent
level: task
title: "Hook core types — HookEvent, HookMatcher, HookResult, HookConfig structs"
short_code: "ARAWN-T-0078"
created_at: 2026-04-04T02:16:22.354523+00:00
updated_at: 2026-04-04T02:29:43.636997+00:00
parent: ARAWN-I-0008
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0008
---

# Hook core types — HookEvent, HookMatcher, HookResult, HookConfig structs

## Objective

Create the foundational types for the hooks system in a new `arawn_hooks` crate (or module within the engine). Pure types, no I/O — everything else builds on these.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `HookEvent` enum with all 25 variants matching Claude Code: PreToolUse, PostToolUse, PostToolUseFailure, PermissionRequest, PermissionDenied, SessionStart, SessionEnd, Setup, Stop, StopFailure, UserPromptSubmit, SubagentStart, SubagentStop, PreCompact, PostCompact, Notification, WorktreeCreate, WorktreeRemove, CwdChanged, FileChanged, TeammateIdle, TaskCreated, TaskCompleted, Elicitation, ElicitationResult
- [ ] `HookInput` enum or struct per event type with appropriate fields (tool events get tool_name/tool_input, session events get session metadata, permission events get rule/tool info, etc.)
- [ ] `HookEventMetadata` for each variant: summary description, matcher field name (e.g. `tool_name` for tool events, `source` for SessionStart, `notification_type` for Notification)
- [ ] `HookMatcher` struct: tool name (exact, pipe-separated, or regex) + optional content pattern (glob)
- [ ] `HookMatcher::matches(tool_name, tool_input) -> bool` implementation
- [ ] `HookResult` enum: `Allow`, `Block { reason: String, stderr: String }`, `Warn { message: String }`
- [ ] `HookConfig` struct: deserialized from JSON, maps `HookEvent` → Vec of hook rule entries
- [ ] `CommandHookDef` struct: `command: String`, `timeout: Option<u64>`, `matcher: Option<HookMatcher>`
- [ ] Serde Deserialize for all config types matching the settings.json format from I-0008
- [ ] Unit tests for HookMatcher: exact match, pipe-separated, content pattern, no-match cases

## Implementation Notes

- Reuse glob matching from the permission system's pattern matcher where possible
- The settings.json format nests hooks under event keys: `hooks.PreToolUse[].hooks[]`
- Content patterns use the `ToolName(pattern)` syntax — parse into tool name + glob pattern

## Status Updates

*To be added during implementation*