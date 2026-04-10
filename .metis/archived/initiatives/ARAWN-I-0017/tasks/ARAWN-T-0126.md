---
id: permission-mode-rpc-set-permission
level: task
title: "Permission mode RPC — set_permission_mode and get_permission_mode server methods"
short_code: "ARAWN-T-0126"
created_at: 2026-04-09T16:03:02.959402+00:00
updated_at: 2026-04-09T16:13:15.377028+00:00
parent: ARAWN-I-0017
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0017
---

# Permission mode RPC — set_permission_mode and get_permission_mode server methods

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0017]]

## Objective

Add WS server RPC methods so the TUI can read and change the permission mode at runtime. The `PermissionChecker` already has `update_mode()` and `mode()` — this task exposes them over the wire.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `set_permission_mode` RPC method accepts `{"mode": "default"|"accept_edits"|"bypass"|"plan"}` and calls `permission_checker.update_mode()`
- [ ] `get_permission_mode` RPC method returns `{"mode": "default"|"accept_edits"|"bypass"|"plan"}`
- [ ] `LocalService` exposes `shared_permission_checker()` returning `Arc<PermissionChecker>` (similar to existing `shared_permission_rules()`)
- [ ] WS client gets `set_permission_mode()` and `get_permission_mode()` helper methods
- [ ] Invalid mode strings return a clear error

### Key files
- `crates/arawn/src/ws_server.rs` — add RPC dispatch cases
- `crates/arawn/src/local_service.rs` — expose permission checker
- `crates/arawn-tui/src/ws_client.rs` — client helper methods

### Dependencies
- None (PermissionChecker infrastructure exists)

## Status Updates

*To be added during implementation*