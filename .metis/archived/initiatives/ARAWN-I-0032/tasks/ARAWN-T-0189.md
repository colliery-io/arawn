---
id: unify-error-conversions
level: task
title: "Unify error conversions — ServiceError carries typed sources, not strings"
short_code: "ARAWN-T-0189"
created_at: 2026-04-18T14:13:34.187103+00:00
updated_at: 2026-04-18T15:09:30.320200+00:00
parent: ARAWN-I-0032
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0032
---

# Unify error conversions — ServiceError carries typed sources, not strings

## Parent Initiative

[[ARAWN-I-0032]]

## Objective

`ServiceError::Engine(String)` / `ServiceError::Storage(String)` stringify their sources at the crate boundary, losing structured detail. Replace with typed sources:

```rust
enum ServiceError {
    Engine(#[from] EngineError),
    Storage(#[from] StorageError),
    // ...
}
```

Ripple fixes into WebSocket handler in arawn-bin, which `.to_string()`s typed errors for the wire payload — emit structured shapes instead.

Estimated size: **S** (~1 day).

### Priority
- [x] P2 - Medium (improves debuggability; foundation for future observability work)

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [ ] Bug - Production issue that needs fixing
- [ ] Feature - New functionality or enhancement  
- [ ] Tech Debt - Code improvement or refactoring
- [ ] Chore - Maintenance or setup work

### Priority
- [ ] P0 - Critical (blocks users/revenue)
- [ ] P1 - High (important for user experience)
- [ ] P2 - Medium (nice to have)
- [ ] P3 - Low (when time permits)

### Impact Assessment **[CONDITIONAL: Bug]**
- **Affected Users**: {Number/percentage of users affected}
- **Reproduction Steps**: 
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected vs Actual**: {What should happen vs what happens}

### Business Justification **[CONDITIONAL: Feature]**
- **User Value**: {Why users need this}
- **Business Value**: {Impact on metrics/revenue}
- **Effort Estimate**: {Rough size - S/M/L/XL}

### Technical Debt Impact **[CONDITIONAL: Tech Debt]**
- **Current Problems**: {What's difficult/slow/buggy now}
- **Benefits of Fixing**: {What improves after refactoring}
- **Risk Assessment**: {Risks of not addressing this}

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `ServiceError::Engine(String)` → `ServiceError::Engine(EngineError)` with `#[from]`
- [ ] `ServiceError::Storage(String)` → typed equivalent
- [ ] Any other `String`-valued `ServiceError` variants that wrap existing error types get typed
- [ ] Explicit `From<ToolError>` / `From<IntegrationError>` / etc. impls where they're missing and a boundary crossing exists
- [ ] WebSocket JSON-RPC error responses preserve structured detail (at minimum: error code, message, optional details object) rather than a flat stringified message
- [ ] Existing error-path tests still pass; add a regression test that a typed error surfaced from tool execution reaches the WebSocket client with code intact
- [ ] `cargo check --workspace` clean; `angreal test unit` green
- [ ] Single focused commit

## Test Cases **[CONDITIONAL: Testing Task]**

{Delete unless this is a testing task}

### Test Case 1: {Test Case Name}
- **Test ID**: TC-001
- **Preconditions**: {What must be true before testing}
- **Steps**: 
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

### Test Case 2: {Test Case Name}
- **Test ID**: TC-002
- **Preconditions**: {What must be true before testing}
- **Steps**: 
  1. {Step 1}
  2. {Step 2}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

## Documentation Sections **[CONDITIONAL: Documentation Task]**

{Delete unless this is a documentation task}

### User Guide Content
- **Feature Description**: {What this feature does and why it's useful}
- **Prerequisites**: {What users need before using this feature}
- **Step-by-Step Instructions**:
  1. {Step 1 with screenshots/examples}
  2. {Step 2 with screenshots/examples}
  3. {Step 3 with screenshots/examples}

### Troubleshooting Guide
- **Common Issue 1**: {Problem description and solution}
- **Common Issue 2**: {Problem description and solution}
- **Error Messages**: {List of error messages and what they mean}

### API Documentation **[CONDITIONAL: API Documentation]**
- **Endpoint**: {API endpoint description}
- **Parameters**: {Required and optional parameters}
- **Example Request**: {Code example}
- **Example Response**: {Expected response format}

## Implementation Notes **[CONDITIONAL: Technical Task]**

{Keep for technical tasks, delete for non-technical. Technical details, approach, or important considerations}

### Technical Approach
{How this will be implemented}

### Dependencies
{Other tasks or systems this depends on}

### Risk Considerations
{Technical risks and mitigation strategies}

## Status Updates **[REQUIRED]**

**2026-04-18 — Completed**

Converted `ServiceError` from string-only variants to typed sources, and propagated the structured detail through the WebSocket wire format.

Changes:
- `ServiceError::Engine(String)` → `Engine(#[from] arawn_engine::EngineError)`
- `ServiceError::Storage(String)` → `Storage(#[from] arawn_storage::StorageError)`
- Added `ServiceError::Memory(#[from] arawn_memory::MemoryError)` — the old stringified `Storage` was also catching memory errors; now they're properly distinct.
- `String`-payload variants kept for `NotFound`, `InvalidOperation`, `Internal` — those are user-facing messages that don't wrap typed sources.
- New `ServiceError::details() -> Option<serde_json::Value>` — emits `{ "kind": "..." }` tagging the inner variant (e.g. `tool_not_found`, `database`, `validation`).
- WebSocket `ErrorBody` now carries optional `details: Value`. New `Response::from_service_error(id, &e)` helper reads code + message + structured details off the typed error. String-only variants omit `details` in the serialized output (not `null`).
- `arawn-service` crate gains dep on `arawn-engine`, `arawn-storage`, `arawn-memory` (needed for the typed variants). No circular deps — the service crate is a trait boundary above all three.
- `local_service.rs`: deleted 18 `.map_err(|e| ServiceError::Storage(e.to_string()))` call sites; all now use `?` via the `#[from]` impls. One `tokio::fs::create_dir_all` call converts via `StorageError::from` (io::Error → StorageError::Io → ServiceError::Storage) to preserve the structural chain.

Tests: added three inline regression tests in `ws_server::tests`:
- `from_service_error_preserves_structured_detail_for_typed_variants` — StorageError::NotFound surfaces with code=`storage_error` and `details.kind=not_found`.
- `from_service_error_omits_details_for_string_only_variants` — `NotFound(String)` has no details field in the JSON (not emitted as null).
- `from_service_error_preserves_engine_error_kind` — EngineError::ToolNotFound surfaces as code=`engine_error`, `details.kind=tool_not_found`.

All tests green; `angreal check workspace` clean.