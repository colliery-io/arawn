---
id: permissionprompt-trait-and-cli
level: task
title: "PermissionPrompt trait and CLI fallback implementation"
short_code: "ARAWN-T-0077"
created_at: 2026-04-03T02:48:50.571438+00:00
updated_at: 2026-04-03T10:22:10.568819+00:00
parent: ARAWN-I-0009
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0009
---

# PermissionPrompt trait and CLI fallback implementation

## Parent Initiative

[[ARAWN-I-0009]]

## Objective

Define the `PermissionPrompt` trait that abstracts how the user is asked for permission, and provide a basic CLI fallback implementation. The TUI modal (T-0071 in I-0012) will be the primary implementation, but this trait + CLI fallback makes the system testable and usable in non-TUI contexts.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `PermissionPrompt` async trait with `prompt(request: PermissionRequest) -> PermissionResponse`
- [ ] `PermissionRequest` struct: tool name, tool input summary, risk category
- [ ] `PermissionResponse` enum: AllowOnce, AllowAlways, Deny
- [ ] `CliPermissionPrompt` implementation: prints tool info to stderr, reads y/n/a from stdin
- [ ] `MockPermissionPrompt` for tests: configurable responses (always allow, always deny, etc.)
- [ ] Trait is object-safe so it can be stored as `Box<dyn PermissionPrompt>`
- [ ] CLI prompt works in non-interactive mode (defaults to Deny when stdin is not a TTY)

## Implementation Notes

### Technical Approach
- Trait defined in the permissions crate/module
- CLI implementation uses `crossterm` or raw stdin for reading — keep it simple, just a one-line prompt
- The TUI implementation (T-0071) will be a separate struct that sends the request through a channel to the TUI event loop and awaits the response
- Mock implementation is trivially a struct with a `VecDeque<PermissionResponse>` that pops answers

### Dependencies
- Depends on T-0074 (PermissionChecker calls the prompt trait)
- Unblocks T-0071 in I-0012 (TUI modal is just another implementation of this trait)

## Status Updates

- `PermissionPrompt` trait already created in T-0074 (async, object-safe, `Box<dyn PermissionPrompt>`)
- `PermissionRequest` / `PermissionResponse` types already in checker.rs
- Created `crates/arawn-engine/src/permissions/prompt.rs`:
  - `CliPermissionPrompt`: prints to stderr, reads y/yes/a/always/n from stdin
  - Non-interactive detection via `std::io::IsTerminal` — auto-denies when stdin isn't a TTY
  - `MockPermissionPrompt`: `always()` for fixed response, `with_responses()` for queued + default
- 3 prompt tests + 46 total permission tests all passing
- Workspace builds clean
- Note: risk_category not added to PermissionRequest — deferred until the risk classification system exists