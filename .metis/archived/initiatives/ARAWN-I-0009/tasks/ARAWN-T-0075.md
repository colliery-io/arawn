---
id: permission-modes-default
level: task
title: "Permission modes — Default, AcceptEdits, BypassPermissions fallback behavior"
short_code: "ARAWN-T-0075"
created_at: 2026-04-03T02:48:48.364558+00:00
updated_at: 2026-04-03T10:20:18.820280+00:00
parent: ARAWN-I-0009
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0009
---

# Permission modes — Default, AcceptEdits, BypassPermissions fallback behavior

## Parent Initiative

[[ARAWN-I-0009]]

## Objective

Implement permission modes that control the fallback behavior when no explicit rule matches a tool call. Modes provide sensible defaults so users don't need to write rules for every tool.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `PermissionMode` enum: `Default`, `AcceptEdits`, `BypassPermissions`
- [ ] `Default` mode: read-only tools (Read, Glob, Grep, Think) auto-allowed, write tools (Edit, Write, Bash) trigger Ask
- [ ] `AcceptEdits` mode: file tools (Read, Edit, Write, Glob, Grep) auto-allowed, Bash still triggers Ask
- [ ] `BypassPermissions` mode: everything auto-allowed (power user / CI mode)
- [ ] Mode is set in settings JSON: `{ "permission_mode": "default" }`
- [ ] Mode can be overridden via CLI flag (e.g., `--permission-mode bypass`)
- [ ] Tool categorization (read-only vs write vs shell) is data-driven, not hardcoded per-tool — new tools get a default category
- [ ] Unit tests for each mode's fallback behavior

## Implementation Notes

### Technical Approach
- `PermissionMode` is a field on `PermissionChecker`
- Each tool registers a category (ReadOnly, FileWrite, Shell, Other) — could be a method on the tool trait or a lookup table
- The mode's fallback is only consulted when no explicit allow/deny/ask rule matched
- `BypassPermissions` should log a warning at startup so users know they're running unguarded

### Dependencies
- Depends on T-0074 (PermissionChecker) — modes plug into the fallback path of check()

## Status Updates

- Added `PermissionMode` enum (Default, AcceptEdits, BypassPermissions) with serde support
- Added `ToolCategory` enum (ReadOnly, FileWrite, Shell, Other) + data-driven `tool_category()` lookup
- `PermissionMode::fallback()` returns the right decision per mode+category combo
- Integrated into `PermissionChecker`: `with_mode()` builder, NoMatch branch delegates to mode fallback
- Extracted `prompt_user()` helper to avoid duplication between Ask rules and mode fallback
- BypassPermissions logs info warning at construction
- 43 permission tests all passing — includes mode-specific tests for all 3 modes, explicit rules overriding mode, tool categories, serde round-trip
- CLI flag integration deferred — that's in the main arawn crate, not the engine