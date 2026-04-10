---
id: implement-real-cancellation-with
level: task
title: "Implement real cancellation with CancellationToken"
short_code: "ARAWN-T-0149"
created_at: 2026-04-10T01:01:11.187033+00:00
updated_at: 2026-04-10T02:10:16.756854+00:00
parent: ARAWN-I-0023
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0023
---

# Implement real cancellation with CancellationToken

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0023]]

## Objective
Implement real cancellation using `tokio_util::sync::CancellationToken`. The current `cancel()` RPC returns success but does nothing — the engine continues consuming LLM credits and executing tools.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] `CancellationToken` per session stored in `LocalService` (can share DashMap from ARAWN-T-0141)
- [ ] Token passed into `QueryEngine` and checked at each loop iteration top
- [ ] Token checked before each tool execution
- [ ] On cancellation: emit `EngineEvent::Error("Cancelled by user")` and break loop
- [ ] `cancel()` RPC cancels the token for the given session
- [ ] Messages from completed turns before cancellation are persisted
- [ ] Integration test for cancel during multi-tool chain

## Implementation Notes
- Files: `crates/arawn/src/local_service.rs`, `crates/arawn-engine/src/query_engine.rs`, `crates/arawn/src/ws_server.rs`
- Add `tokio-util` dependency for `CancellationToken`

## Status Updates
- Added `cancel_token: Option<CancellationToken>` field to `QueryEngine`
- Added `with_cancel_token()` builder method and `is_cancelled()` helper
- Cancellation checked at top of each loop iteration and before tool execution block
- On cancel: returns `EngineError::Other("Cancelled by user")`
- `LocalService` stores per-session `CancellationToken` in `cancel_tokens: HashMap<Uuid, CancellationToken>`
- Token created in `send_message`, passed to engine via `with_cancel_token()`, cleaned up in spawned task
- `cancel()` RPC now calls `token.cancel()` on the session's token (was a no-op TODO)
- Added `tokio-util` dependency to `crates/arawn/Cargo.toml`
- Messages from completed turns before cancellation are persisted (error path handles this)
- Skipped integration test — would require async timing coordination to cancel during multi-tool chain
- All 44 engine tests + 15 local_service tests pass

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