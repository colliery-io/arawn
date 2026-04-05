---
id: hooks-system-lifecycle-event
level: initiative
title: "Hooks system — lifecycle event interception and automation"
short_code: "ARAWN-I-0008"
created_at: 2026-04-03T01:58:07.890170+00:00
updated_at: 2026-04-04T03:00:32.467378+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: L
initiative_id: hooks-system-lifecycle-event
---

# Hooks system — lifecycle event interception and automation Initiative

## Context

Arawn has no way to intercept tool execution or react to lifecycle events. In Claude Code, hooks are the mechanism for enforcing project conventions ("always run lint after editing src/"), safety guardrails ("block rm -rf"), CI integration, and custom automation. Claude Code's full system has 25 hook events, 5 hook types, async execution, and plugin-scoped hook namespacing — but we can ship a useful MVP with a fraction of that surface area.

This initiative covers the full hooks infrastructure: all 25 hook event types matching Claude Code's surface area, command hooks executed as shell subprocesses, matcher-based filtering, and integration points throughout the engine. Events for features we haven't built yet (worktrees, subagents, etc.) are defined in the enum but won't fire until those features land.

## Goals & Non-Goals

**Goals:**
- Event system with all 25 Claude Code hook events (PreToolUse, PostToolUse, PostToolUseFailure, PermissionRequest, PermissionDenied, SessionStart, SessionEnd, Setup, Stop, StopFailure, UserPromptSubmit, SubagentStart, SubagentStop, PreCompact, PostCompact, Notification, WorktreeCreate, WorktreeRemove, CwdChanged, FileChanged, TeammateIdle, TaskCreated, TaskCompleted, Elicitation, ElicitationResult)
- Command hooks — run shell commands on events
- Matcher syntax — filter hooks by tool name and content patterns (e.g. `"Bash(git *)"`)
- PreToolUse hooks can block/allow/modify tool calls
- Configuration in settings.json (user-level and project-level, with priority merging)

**Non-Goals:**
- Prompt hooks (LLM-evaluated) — defer
- HTTP hooks (POST to endpoints) — defer
- Agent hooks (verifier agents) — defer
- Enterprise policy lockdown

## Architecture

### Overview

The hooks system adds a lifecycle event layer between the query engine and tool execution. Before and after each tool call, the engine emits events that are matched against user-configured hook rules. Command hooks run shell subprocesses and interpret exit codes to allow, block, or warn.

### Components

1. **HookEvent** — enum of all 25 lifecycle event types matching Claude Code (events for unbuilt features are defined but won't fire yet)
2. **HookMatcher** — filters hooks by tool name and optional content pattern (reuses glob/regex matching from the permission system)
3. **HookConfig** — deserialized from settings files, contains event → hook rule mappings
4. **CommandHook** — executes a shell command, passes hook input as JSON on stdin, interprets exit codes
5. **HookRunner** — orchestrates matching, execution, and result aggregation for a given event
6. **HookResult** — enum: `Allow`, `Block(reason)`, `Warn(message)`, with optional modified tool input

### Integration Points

```
QueryEngine::execute_tool()
    │
    ├── hook_runner.run(PreToolUse, tool_name, tool_input)
    │       │
    │       ├── match hooks from config
    │       ├── execute matching hooks (parallel, with timeout)
    │       └── aggregate results → Allow / Block / Allow(modified_input)
    │
    ├── if blocked → return error to model
    ├── if modified → use modified input
    │
    ├── tool_executor.execute(tool_name, input)
    │
    └── hook_runner.run(PostToolUse, tool_name, tool_input, tool_output)
            └── (informational only — cannot block)
```

### Exit Code Semantics (matching Claude Code)

| Exit Code | Meaning | Behavior |
|-----------|---------|----------|
| 0 | Success | Allow; stdout suppressed unless relevant |
| 2 | Blocking error | Block tool execution; stderr shown to model |
| Other | Warning | Show stderr to user; don't block |

### Hook Input (JSON on stdin)

```json
{
  "hook_event": "PreToolUse",
  "tool_name": "Bash",
  "tool_input": { "command": "rm -rf /tmp/stuff" }
}
```

For `PostToolUse`, includes `tool_output`. For `SessionStart`/`Stop`, includes session metadata.

## Detailed Design

### Hook Events (full surface)

**Tool lifecycle:** PreToolUse (block/allow/modify), PostToolUse (react), PostToolUseFailure
**Permissions:** PermissionRequest, PermissionDenied
**Session:** SessionStart, SessionEnd, Setup
**Completion:** Stop, StopFailure
**User input:** UserPromptSubmit
**Subagents:** SubagentStart, SubagentStop
**Compaction:** PreCompact, PostCompact
**Notifications:** Notification
**Workspace:** WorktreeCreate, WorktreeRemove, CwdChanged, FileChanged
**Tasks/Agents:** TeammateIdle, TaskCreated, TaskCompleted
**Elicitation:** Elicitation, ElicitationResult

Events for features we don't have yet (worktrees, subagents, teammates, elicitation) are defined in the enum with their input schemas but won't be wired to fire until those features are built. This keeps the event surface stable and forward-compatible.

### Hook Type (MVP)
Command — execute shell command, exit 0 = allow, non-zero = block

### Matcher Syntax
`"Bash"` (any Bash), `"Bash(git *)"` (git commands), `"Edit(*.rs)"` (Rust files), `""` (all tools)

### Config (settings.json)

Hook rules live in `~/.arawn/settings.json` (user) and `.arawn/settings.json` (project), under a `hooks` key organized by event type. User settings take priority over project settings.

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "jq -r '.tool_input.command' | grep -q '^rm -rf' && exit 2",
            "timeout": 5
          }
        ]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "Edit",
        "hooks": [
          {
            "type": "command",
            "command": "echo 'File edited' >> /tmp/audit.log"
          }
        ]
      }
    ],
    "SessionStart": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "echo 'Session started at $(date)' >> /tmp/arawn.log"
          }
        ]
      }
    ]
  }
}
```

### Decomposition
1. Hook event types + HookMatcher + HookResult (pure types)
2. Command hook executor (subprocess, stdin, exit codes, timeout)
3. Config loading + settings merging
4. HookRunner (matching, parallel execution, result aggregation)
5. Engine integration (PreToolUse/PostToolUse around tool execution)
6. SessionStart/Stop hooks at session boundaries
7. Tests

## Testing Strategy

- **Unit tests** for HookMatcher (tool name matching, content pattern matching, edge cases)
- **Unit tests** for HookRunner (result aggregation, blocking precedence, timeout handling)
- **Integration tests** for CommandHook (real subprocess execution, exit code handling, stdin piping)
- **Engine integration test** verifying PreToolUse can block a tool call end-to-end
- **Mock-based tests** for hook config loading and merging (user > project priority)

## Alternatives Considered

- **Event bus / pub-sub pattern**: More flexible but overkill for MVP. Hooks are synchronous decision points (PreToolUse must block before execution), not fire-and-forget events. A simple runner with direct calls is sufficient.
- **WASM plugin hooks**: Maximum isolation and portability, but heavy implementation cost. Shell commands are what Claude Code uses and what users expect.
- **Trait-based hook interface (Rust-native)**: Would require compiled plugins. Shell commands are more accessible and match the Claude Code model. Can add Rust-native hooks later as an optimization.
- **Separate hook process (daemon)**: Long-running sidecar that receives events over IPC. Unnecessary complexity — spawning short-lived processes per event is fast enough and simpler to reason about.

## Implementation Plan

**Phase 1 — Core types and matcher:**
Hook event enum, HookMatcher (tool name + content pattern), HookResult enum. Pure types, no I/O.

**Phase 2 — Command hook executor:**
Spawn shell subprocess, pipe JSON input on stdin, read stdout/stderr, interpret exit codes. Timeout support via tokio.

**Phase 3 — Config loading:**
Parse hook rules from settings files (user > project priority). Deserialize into HookConfig.

**Phase 4 — HookRunner and engine integration:**
HookRunner matches config → executes hooks → aggregates results. Wire into QueryEngine before/after tool execution. SessionStart/Stop hooks at session boundaries.

**Phase 5 — Tests:**
Unit tests for matcher, executor, runner. Integration test for end-to-end PreToolUse blocking.

### Future work (not in scope)
- Prompt hooks (LLM-evaluated)
- HTTP hooks (POST to endpoints)
- Agent hooks (verifier agents)
- Async/background hooks
- Plugin-scoped hook namespacing