---
id: complete-arawnservice-trait-to
level: task
title: "Complete ArawnService trait to cover all RPC methods with typed responses"
short_code: "ARAWN-T-0154"
created_at: 2026-04-10T01:01:18.281366+00:00
updated_at: 2026-04-10T23:27:54.365438+00:00
parent: ARAWN-I-0024
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0024
---

# Complete ArawnService trait to cover all RPC methods with typed responses

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0024]]

## Objective
Expand `ArawnService` trait from 7 to all 16 RPC methods. Define typed response structs for methods currently returning `serde_json::Value`. Map `ServiceError` variants to distinct RPC error codes.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] All 16 RPC methods represented in `ArawnService` trait
- [ ] Typed response structs: `MemoryStoreResult`, `MemorySummary`, `CommandList`, `InventoryResult`, `PromotionResult`
- [ ] `ServiceError` variants mapped to distinct codes: `not_found`, `invalid_operation`, `engine_error`, `storage_error`, `internal_error`
- [ ] WS handler calls through trait for all methods (no direct LocalService access)
- [ ] Tests updated for new error codes

## Implementation Notes
- Files: `crates/arawn-service/src/lib.rs` (trait + types), `crates/arawn/src/local_service.rs` (impl), `crates/arawn/src/ws_server.rs` (handler)

## Status Updates
- **COMPLETE**: All 16 RPC methods now go through the `ArawnService` trait.
- Added 9 new trait methods: `promote_session`, `resolve_user_input`, `query_inventory`, `list_available_commands`, `list_workflows`, `remember_fact`, `memory_summary`, `forget_entity`, `get_permission_mode`, `set_permission_mode`
- Added typed response structs in `arawn-service/src/types.rs`: `MemoryStoreResult`, `MemorySummary`, `MemoryStoreSummary`, `MemoryTypeCount`, `ForgetResult`, `ForgetCandidate`, `InventoryItem`, `CommandInfo`, `PromotionResult`, `WorkflowInfo`, `PermissionModeInfo`
- Added `ServiceError::error_code()` mapping each variant to a distinct RPC error code string
- WS handler now calls all methods through the trait and uses typed error codes instead of hardcoded `"service_error"`
- Updated `local_service.rs` test for new `PromotionResult` return type
- All tests pass (0 failures)