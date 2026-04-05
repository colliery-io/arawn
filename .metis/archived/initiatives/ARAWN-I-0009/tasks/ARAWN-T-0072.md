---
id: permission-rule-types-and-pattern
level: task
title: "Permission rule types and pattern matcher — Allow/Deny/Ask with tool name + content globs"
short_code: "ARAWN-T-0072"
created_at: 2026-04-03T02:48:44.695940+00:00
updated_at: 2026-04-03T10:14:10.652749+00:00
parent: ARAWN-I-0009
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0009
---

# Permission rule types and pattern matcher — Allow/Deny/Ask with tool name + content globs

## Parent Initiative

[[ARAWN-I-0009]]

## Objective

Define the core permission rule types (Allow, Deny, Ask) and build a pattern matcher that can match tool names with optional content globs. This is the foundation everything else in the permission system builds on.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `PermissionRule` enum with Allow, Deny, Ask variants
- [ ] Each rule holds a tool name pattern (exact match or glob, e.g., `"Bash"`, `"file_*"`)
- [ ] Optional content pattern for matching tool input (e.g., `Bash(git *)` matches Bash calls starting with "git")
- [ ] `PermissionRule::matches(tool_name, tool_input) -> bool` method
- [ ] `PermissionDecision` enum: Allowed, Denied, Ask — returned by matching
- [ ] Rule list evaluated in order: first deny match → Denied, first allow match → Allowed, first ask match → Ask, no match → fallback
- [ ] Unit tests for exact match, glob match, content pattern match, and priority ordering

## Implementation Notes

### Technical Approach
- New `crates/arawn-permissions/` crate or module within `arawn-core`
- Use `glob` or `globset` crate for pattern matching
- Content patterns parsed from `ToolName(pattern)` syntax — split on first `(`, trim trailing `)`
- Keep it simple: no regex, just glob patterns. Can extend later if needed.

### Dependencies
- None — this is the foundation task with no upstream deps

## Status Updates

- Created `crates/arawn-engine/src/permissions/` module with `mod.rs` and `rules.rs`
- `PermissionRule` struct with kind (Allow/Deny/Ask), tool_pattern, optional content_pattern
- `PermissionRule::parse()` for compact string format: `"Bash"` or `"Bash(git *)"` 
- `PermissionRule::matches()` with custom glob matcher (*, ? support)
- `PermissionDecision` enum: Allowed, Denied, Ask, NoMatch
- `RuleMatcher::evaluate()` with priority: deny > allow > ask > no match
- 18 unit tests all passing