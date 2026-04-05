---
id: session-scoped-permission-grants
level: task
title: "Session-scoped permission grants — in-memory Allow Always store"
short_code: "ARAWN-T-0076"
created_at: 2026-04-03T02:48:49.586641+00:00
updated_at: 2026-04-03T10:20:42.786735+00:00
parent: ARAWN-I-0009
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0009
---

# Session-scoped permission grants — in-memory Allow Always store

## Parent Initiative

[[ARAWN-I-0009]]

## Objective

When a user selects "Allow Always" on a permission prompt, store that grant for the rest of the session so they aren't prompted again for the same tool. Grants are in-memory only — they don't persist across sessions.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `SessionGrants` struct stores a set of granted tool name patterns
- [ ] "Allow Always" from a permission prompt adds the tool to session grants
- [ ] Session grants are checked before config rules in the check flow (short-circuit)
- [ ] Grants are scoped to exact tool name or tool+content pattern (matching what was prompted)
- [ ] Grants are cleared when the session ends (in-memory only, no file persistence)
- [ ] Optional: `clear_grants()` method to reset mid-session
- [ ] Unit tests for grant storage, lookup, and session scoping

## Implementation Notes

### Technical Approach
- `SessionGrants` is a `HashSet<PermissionRule>` or similar — stores allow rules added during the session
- Owned by `PermissionChecker`, checked as the first step in `check()`
- When the prompt returns "Allow Always", the checker inserts a new Allow rule into session grants
- "Allow Once" just returns Allowed without storing anything

### Dependencies
- Depends on T-0074 (PermissionChecker) — grants live inside the checker
- Depends on T-0077 (PermissionPrompt) — prompt returns the user's choice including "Allow Always"

## Status Updates

- Already implemented as part of T-0074 (PermissionChecker). All criteria met:
- `SessionGrants` struct with `HashSet<String>` + grant/is_granted/clear methods
- "Allow Always" response from prompter adds tool to grants via `prompt_user()`
- Session grants are checked first in `check()` — short-circuits before rule evaluation
- Grants scoped to exact tool name (string match)
- In-memory only (`Mutex<SessionGrants>` on `PermissionChecker`)
- `clear_grants()` method exposed on checker
- Tests: `session_grant_short_circuits`, `ask_with_allow_always_grants_session`, `clear_grants_resets`