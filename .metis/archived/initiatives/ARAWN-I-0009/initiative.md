---
id: permission-system-allow-deny-ask
level: initiative
title: "Permission system — allow/deny/ask rules with tool-level access control"
short_code: "ARAWN-I-0009"
created_at: 2026-04-03T01:59:38.742751+00:00
updated_at: 2026-04-04T02:06:33.928757+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: L
initiative_id: permission-system-allow-deny-ask
---

# Permission system — allow/deny/ask rules with tool-level access control Initiative

## Context

We have a basic OS sandbox (fidius) but no application-level permission rules. Every tool call either succeeds or hits the sandbox. No way for users to say "always allow file_read but ask before shell commands." Claude Code has a layered permission model with allow/deny/ask rules per tool, 5 permission modes, and user prompting for dangerous operations.

## Goals & Non-Goals

**Goals:**
- Permission rules: allow, deny, ask — per tool with optional content patterns (e.g. `"Bash(git *)"`)
- Permission modes: default (ask for writes), acceptEdits (auto-approve file changes), bypassPermissions
- Rule sources with priority: user (~/.arawn/settings.json) > project (.arawn/settings.json) > session
- User prompting via TUI/CLI when a tool call requires "ask" permission
- Session-scoped permission grants ("allow this tool for the rest of the session")
- Integration with existing OS sandbox as enforcement backend

**Non-Goals:**
- Enterprise/managed policy settings
- Auto-classifier for safe command detection
- Per-subcommand Bash permission splitting

## Architecture

### Core Components
1. **PermissionRule** — enum of Allow/Deny/Ask with tool name + optional content pattern
2. **PermissionConfig** — loaded from user/project settings files, merged with priority
3. **PermissionChecker** — the check() entry point called by the engine before each tool execution
4. **PermissionMode** — enum (Default, AcceptEdits, BypassPermissions) controlling fallback behavior
5. **PermissionPrompt** — trait for prompting the user (TUI implements it, tests mock it)
6. **SessionGrants** — in-memory store for "Allow Always" grants within a session

### Integration Points
- Engine calls `PermissionChecker::check()` before `ToolExecutor::execute()`
- TUI implements `PermissionPrompt` trait for interactive prompting (T-0071 in I-0012)
- Settings files parsed from existing config infrastructure
- OS sandbox (fidius) remains as the enforcement backend — permissions are the policy layer above it

## Detailed Design

### Permission Rule Format
```json
{ "allow": ["file_read", "grep", "glob", "think"],
  "deny": ["Bash(rm -rf *)"],
  "ask": ["shell", "file_edit", "file_write"] }
```

### Permission Modes
- `default` — read-only tools auto-allowed, write tools ask
- `acceptEdits` — file_edit/file_write auto-allowed, shell still asks
- `bypassPermissions` — everything auto-allowed (power user)

### Check Flow
1. Engine calls `permission_checker.check(tool_name, tool_input)` before execution
2. Match against deny rules first (highest priority) → block
3. Match against allow rules → permit
4. Match against ask rules → prompt user via TUI/WebSocket
5. No match → fall back to permission mode default

### User Prompt (TUI)
When "ask" triggers, TUI shows: tool name, input preview, options (Allow Once / Allow Always / Deny). "Allow Always" adds a session-scoped allow rule.

### Decomposition
1. Permission rule types + matcher (reuse hooks matcher from I-0008)
2. Permission config loading (user + project + session)
3. PermissionChecker with check() method
4. Engine integration (call check before tool execution)
5. TUI/WebSocket user prompt flow
6. Session-scoped rule persistence
7. Permission modes
8. Tests

## Testing Strategy

Unit tests for rule matching, config merging, and the check flow. Mock the PermissionPrompt trait for testing ask behavior. Integration test with a real engine loop to verify tools are blocked/allowed correctly.

## Alternatives Considered

- **Sandbox-only approach (no application-level permissions)**: Rejected — OS sandbox is too coarse. Can't distinguish "allow git but not rm" or let users customize per-project.
- **Capability-based model**: More theoretically pure but harder to configure. Rule-based allow/deny/ask maps directly to what users expect from Claude Code.
- **Auto-classifying commands as safe/dangerous**: Too error-prone and opaque. Explicit rules are predictable.

## Implementation Plan

**Phase 1 (core):** Permission rule types, matcher, config loading, PermissionChecker with check() — pure logic, no UI.
**Phase 2 (integration):** Wire PermissionChecker into the engine loop. Permission modes. Session grants.
**Phase 3 (prompt):** PermissionPrompt trait + TUI implementation (connects to T-0071 in I-0012).
**Phase 4 (polish):** Settings file format, CLI for managing rules, documentation.

Dependency graph:
```
rules + matcher ──> config loading ──> PermissionChecker
                                           │
                                    engine integration
                                           │
                              ┌────────────┼────────────┐
                              ▼            ▼            ▼
                      permission modes  session grants  PermissionPrompt trait
                                                            │
                                                    TUI modal (T-0071)
```