---
id: agent-autonomy-controls-soft-turn
level: initiative
title: "Agent autonomy controls — soft turn limits, permission mode cycling, and accept commands"
short_code: "ARAWN-I-0017"
created_at: 2026-04-05T19:04:10.458281+00:00
updated_at: 2026-04-09T16:13:17.821983+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: M
initiative_id: agent-autonomy-controls-soft-turn
---

# Agent autonomy controls — soft turn limits, permission mode cycling, and accept commands Initiative

## Context

Arawn's agent loop currently has a hard `max_iterations` cap (now 40) that kills the engine with `EngineError::MaxIterations` when reached. This creates a poor UX for complex tasks like codebase analysis, multi-repo audits, or large refactors — the agent just stops mid-work with an error. There's no way for the user to say "keep going" without restarting.

Claude Code (reference implementation) takes a fundamentally different approach: the turn limit is optional/absent, and the real control lever is **permission modes** that determine how much autonomy the agent has. Users cycle between modes (`default → acceptEdits → bypassPermissions`) to control the agent's freedom, not its lifespan.

Arawn already has the permission mode infrastructure (`PermissionMode::Default`, `AcceptEdits`, `BypassPermissions`, `Plan`), `PermissionChecker` with `MockModalPrompt` for testing, and the `AskUserTool` for human-in-the-loop prompts. What's missing is:

1. **Soft turn limits** — check-in with the user instead of hard-stopping
2. **Runtime mode switching** — user commands to change permission mode mid-session
3. **Accept mode UX** — a simple `/accept on` command that lets the agent run autonomously

### Reference: Claude Code permission modes

| Mode | Behavior |
|------|----------|
| `default` | Prompts for every uncertain action |
| `acceptEdits` | Auto-allows file writes, prompts for shell |
| `bypassPermissions` | Auto-allows everything (except safety-critical paths) |
| `plan` | Read-only, side-effect-free tools only |
| `dontAsk` | Converts prompts to denials (strict safety) |

Modes cycle via hotkey (Shift+Tab). No turn limit is enforced by default — the agent runs until it's done or the user cancels.

## Goals & Non-Goals

**Goals:**
- Raise `max_iterations` to a high value (200+) so the agent doesn't die mid-task on complex work
- Enforce session state dumping so a co-ordinating caller can resume from a saved session if the agent dies or hits the cap
- User can toggle permission modes at runtime via TUI commands (`/accept on`, `/accept off`, `/plan`) as convenience over editing arawn.toml
- Permission mode displayed in TUI status bar so the user always knows the current autonomy level

**Non-Goals:**
- Soft check-in / AskUser interrupts mid-loop (over-engineered — high cap + resumable sessions is simpler)
- AI-based permission classifier (Claude Code's "auto" mode)
- Token budget limits (separate concern)
- Remote/headless mode (handled by workflow initiative)

## Use Cases

### Use Case 1: Extended codebase analysis
- **Actor**: User via TUI
- **Scenario**: User asks agent to "analyze this codebase for security issues." Agent starts working. At turn 30, soft limit fires — AskUser: "I've been working for 30 turns. Continue?" User selects "Keep going." Agent continues. Finishes at turn 52 with a full report.
- **Expected Outcome**: Agent completes the task without hard-stopping.

### Use Case 2: Accept mode for batch operations
- **Actor**: User via TUI
- **Scenario**: User types `/accept on`. Status bar shows "BYPASS" mode. User says "refactor all error handling in src/ to use thiserror." Agent works through dozens of files without prompting. When done, user types `/accept off` to restore default mode.
- **Expected Outcome**: Agent runs autonomously until the task is complete, no turn check-ins.

### Use Case 3: Plan mode for safe exploration
- **Actor**: User via TUI
- **Scenario**: User types `/plan`. Agent enters plan mode — can read files, search, think, but cannot write or execute. User reviews the plan, then types `/accept on` to execute it.
- **Expected Outcome**: Safe exploration followed by autonomous execution.

## Detailed Design

### 1. Raise max_iterations + session dump on cap

Raise `max_iterations` default from 40 to 200 in `QueryEngineConfig`. When the cap is hit, instead of just returning `EngineError::MaxIterations`, ensure the session is fully persisted (it already should be — JSONL is appended per-message) and include a resumption hint in the error so a co-ordinating caller knows the session ID and can restart.

```toml
# arawn.toml
[engine]
max_iterations = 200   # raised from 40
```

The session JSONL already captures every message, tool call, and tool result. A caller resuming from a saved session just needs to send a new user message like "Continue where you left off" to the same session ID.

### 2. Permission mode TUI commands

Add TUI commands that modify the `PermissionChecker`'s mode. Permission rules are already behind `Arc<RwLock<>>` and hot-reloadable via `ConfigWatcher`. These commands are convenience sugar:

| Command | Action |
|---------|--------|
| `/accept on` | Set mode to `BypassPermissions` |
| `/accept off` | Set mode to `Default` |
| `/accept edits` | Set mode to `AcceptEdits` |
| `/plan` | Set mode to `Plan` (already exists) |

### 3. TUI status bar

Display current permission mode in the TUI footer:
- `DEFAULT` — normal mode
- `ACCEPT EDITS` — file writes auto-approved
- `BYPASS` — full autonomy (highlighted to be visible)
- `PLAN` — read-only mode

## Alternatives Considered

- **Soft check-in / AskUser interrupts**: Over-engineered. High cap + resumable sessions is simpler and covers the same case. The agent doesn't need to ask "should I keep going?" — if it dies, the caller restarts it.
- **Remove all turn limits**: Too dangerous. A bug in the tool loop could burn tokens indefinitely. Hard cap is a safety net.
- **Claude Code's auto mode (AI classifier)**: Interesting but requires a secondary model call per tool use. Not in scope.

## Implementation Plan

**Phase 1**: Raise `max_iterations` default to 200. Ensure session dump on cap hit includes resumption metadata (session ID, turn count, last tool state).

**Phase 2**: `/accept on|off|edits` TUI commands wired to permission mode switching.

**Phase 3**: TUI status bar showing current permission mode.

**Phase 4**: Integration tests — cap hit persists session, mode commands change behavior, status bar reflects mode.