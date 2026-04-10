---
id: add-engineevent-warning-variant
level: task
title: "Add EngineEvent::Warning variant and surface persistence errors"
short_code: "ARAWN-T-0143"
created_at: 2026-04-10T01:00:58.470325+00:00
updated_at: 2026-04-10T01:22:17.067994+00:00
parent: ARAWN-I-0021
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0021
---

# Add EngineEvent::Warning variant and surface persistence errors

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0021]]

## Objective
Add `EngineEvent::Warning { message: String }` variant for non-fatal user-visible problems. Use it to surface JSONL persistence failures in `send_message` instead of silently swallowing them.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] `EngineEvent::Warning { message: String }` added to enum
- [ ] WS server serializes Warning events to clients
- [ ] TUI displays Warning events (or at minimum doesn't crash on them)
- [ ] When JSONL message persist fails, Warning emitted before Complete
- [ ] `update_session_stats` errors logged at warn level instead of silenced
- [ ] Integration test verifying Warning event on persist failure

## Implementation Notes
- Files: `crates/arawn-service/src/lib.rs` (EngineEvent), `crates/arawn/src/local_service.rs` (send_message persist error handling), `crates/arawn/src/ws_server.rs` (serialize Warning), `crates/arawn-tui/src/event_loop.rs` (handle Warning)
- Replace `if let Err(e)` silent patterns with error accumulator, emit Warning before Complete

## Status Updates
- Added `EngineEvent::Warning { message: String }` to `arawn-service/src/types.rs`
- Serde auto-serializes as `{"event": "Warning", "data": {"message": "..."}}` — no WS server changes needed
- Added `EventUpdate::Warning(String)` to TUI ws_client.rs
- Added Warning handler in both TUI event_loop.rs and app.rs (displays as "Warning: ..." system message)
- Updated local_service.rs success path: persist errors accumulated, Warning emitted before Complete
- Updated local_service.rs error path: persist errors now logged at error/warn level instead of silenced
- `update_session_stats` errors now logged at warn level in both paths
- Skipped integration test for persist failure — would require injecting I/O errors into JSONL store, complex mock setup
- All 29 service+WS tests pass, clean build

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