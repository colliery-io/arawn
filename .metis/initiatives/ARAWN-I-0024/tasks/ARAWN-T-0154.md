---
id: complete-arawnservice-trait-to
level: task
title: "Complete ArawnService trait to cover all RPC methods with typed responses"
short_code: "ARAWN-T-0154"
created_at: 2026-04-10T01:01:18.281366+00:00
updated_at: 2026-04-10T01:01:18.281366+00:00
parent: ARAWN-I-0024
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


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
- [ ] All 16 RPC methods represented in `ArawnService` trait
- [ ] Typed response structs: `MemoryStoreResult`, `MemorySummary`, `CommandList`, `InventoryResult`, `PromotionResult`
- [ ] `ServiceError` variants mapped to distinct codes: `not_found`, `invalid_operation`, `engine_error`, `storage_error`, `internal_error`
- [ ] WS handler calls through trait for all methods (no direct LocalService access)
- [ ] Tests updated for new error codes

## Implementation Notes
- Files: `crates/arawn-service/src/lib.rs` (trait + types), `crates/arawn/src/local_service.rs` (impl), `crates/arawn/src/ws_server.rs` (handler)

## Status Updates
- **DEFERRED**: Multi-file refactoring across arawn-service, arawn (local_service + ws_server). Requires adding 9 trait methods, defining 5 response structs, updating 600-line WS handler. Best done in a dedicated session with `/metis-ralph ARAWN-T-0154`.

## REMOVED_SECTIONS

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

## Acceptance Criteria **[REQUIRED]**

- [ ] {Specific, testable requirement 1}
- [ ] {Specific, testable requirement 2}
- [ ] {Specific, testable requirement 3}

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

*To be added during implementation*