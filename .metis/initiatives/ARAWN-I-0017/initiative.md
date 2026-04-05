---
id: agent-autonomy-controls-soft-turn
level: initiative
title: "Agent autonomy controls — soft turn limits, permission mode cycling, and accept commands"
short_code: "ARAWN-I-0017"
created_at: 2026-04-05T19:04:10.458281+00:00
updated_at: 2026-04-05T19:04:10.458281+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


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
- Replace hard `max_iterations` cap with a soft check-in system (AskUser prompt at configurable intervals)
- Hard safety cap stays as a last resort (much higher — 200) to prevent truly runaway loops
- User can toggle permission modes at runtime via TUI commands (`/accept on`, `/accept off`, `/plan`)
- When `BypassPermissions` is active, soft check-ins are skipped — the agent runs until done
- `AcceptEdits` mode skips prompts for file operations but still checks in on shell commands
- Permission mode displayed in TUI status bar so user always knows current autonomy level
- Mode persists for the session (not across sessions unless configured in arawn.toml)

**Non-Goals:**
- AI-based permission classifier (Claude Code's "auto" mode) — too complex for now
- Per-tool granular allow/deny rules at runtime (already exists in permission config)
- Token budget limits (separate concern from turn limits)
- Remote/headless mode (agent running without TUI) — handle in workflow initiative

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

### Soft turn limit mechanism

Replace the current hard cap in `QueryEngine::run()`:

**Current:**
```rust
for iteration in 0..self.config.max_iterations {
    // ... engine loop ...
}
Err(EngineError::MaxIterations(self.config.max_iterations))
```

**Proposed:**
```rust
let mut iteration = 0;
loop {
    iteration += 1;
    
    // Hard safety cap (200) — unconditional stop
    if iteration > self.config.hard_max_iterations {
        return Err(EngineError::MaxIterations(iteration));
    }
    
    // Soft check-in (every N turns, configurable)
    if iteration > 0 
        && iteration % self.config.soft_checkin_interval == 0
        && !self.is_bypass_mode()
    {
        // Inject AskUser prompt asking whether to continue
        let should_continue = self.checkin_with_user(session, iteration).await?;
        if !should_continue {
            return Ok("Stopped at user request.".into());
        }
    }
    
    // ... normal engine loop ...
}
```

### Config additions

```rust
pub struct QueryEngineConfig {
    // ... existing fields ...
    /// Soft check-in interval (turns). 0 = disabled. Default: 30.
    pub soft_checkin_interval: usize,
    /// Hard safety cap. Default: 200.
    pub hard_max_iterations: usize,
}
```

```toml
# arawn.toml
[engine]
soft_checkin_interval = 30   # ask user every 30 turns (0 = never)
hard_max_iterations = 200    # absolute safety cap
```

### Permission mode runtime switching

Add TUI commands that modify the `PermissionChecker`'s mode via the existing `update_mode()` API:

| Command | Action |
|---------|--------|
| `/accept on` | Set mode to `BypassPermissions` |
| `/accept off` | Set mode to `Default` |
| `/accept edits` | Set mode to `AcceptEdits` |
| `/plan` | Set mode to `Plan` |

These commands call `permission_checker.update_mode(mode)` — the existing hot-reload API we tested in I-0013.

### TUI status bar

Display current permission mode in the TUI footer/status area:
- `DEFAULT` — normal mode
- `ACCEPT EDITS` — file writes auto-approved
- `BYPASS` — full autonomy (highlighted/colored to be visible)
- `PLAN` — read-only mode

### Interaction between modes and soft limits

| Mode | Soft check-in | Hard cap |
|------|--------------|----------|
| Default | Every N turns | 200 |
| AcceptEdits | Every N turns | 200 |
| BypassPermissions | **Skipped** | 200 |
| Plan | Every N turns | 200 |

In bypass mode, the user has explicitly opted into full autonomy — don't interrupt with check-ins. The hard cap still applies as a safety net.

## Alternatives Considered

- **Remove all turn limits**: Too dangerous. A bug in the LLM or tool loop could burn tokens indefinitely. Hard cap is necessary as a safety net.
- **Only permission modes, no soft limits**: Doesn't help in default mode where long tasks still hit the cap. Soft limits solve the "I'm working on something big" case without requiring bypass mode.
- **Per-task turn budgets**: Over-engineered for now. A global soft interval + bypass toggle covers the 90% case.
- **Claude Code's auto mode (AI classifier)**: Interesting but complex. Requires a secondary model call per tool use to evaluate safety. Worth exploring later but not in scope here.

## Implementation Plan

**Phase 1: Soft turn limits** — Replace hard cap with soft check-in + higher hard cap in QueryEngine. Add config fields. Update tests.

**Phase 2: Permission mode commands** — `/accept on|off|edits`, `/plan` commands that call `update_mode()`. Wire through TUI command handler.

**Phase 3: TUI status bar** — Display current permission mode in the TUI. Update on mode change.

**Phase 4: Integration testing** — Test soft check-in fires, bypass skips it, mode commands work end-to-end.