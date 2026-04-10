---
id: add-per-session-lock-to-prevent
level: task
title: "Add per-session lock to prevent concurrent send_message corruption"
short_code: "ARAWN-T-0141"
created_at: 2026-04-10T01:00:55.651119+00:00
updated_at: 2026-04-10T01:15:42.890366+00:00
parent: ARAWN-I-0021
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0021
---

# Add per-session lock to prevent concurrent send_message corruption

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0021]]

## Objective
Add per-session locking in `LocalService` to prevent concurrent `send_message` calls from interleaving JSONL writes and corrupting session history.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] `DashMap<Uuid, Arc<Mutex<()>>>` (or similar) added to `LocalService`
- [ ] Lock acquired before session load in `send_message`, released after persist
- [ ] Concurrent `send_message` to same session returns `ServiceError::InvalidOperation`
- [ ] Test verifying concurrent send is rejected

## Implementation Notes
- File: `crates/arawn/src/local_service.rs`
- Add `session_locks: Arc<DashMap<Uuid, Arc<Mutex<()>>>>` field
- In `send_message`, `try_lock()` on the session entry — if already held, return error
- This DashMap can later be reused for cancellation tokens (ARAWN-T-0149)

## Status Updates
- Used `Arc<Mutex<HashSet<Uuid>>>` instead of DashMap to avoid new dependency
- `active_sessions` field added to `LocalService`, initialized in `new()`
- At start of `send_message`: `insert(session_id)` — if already present, return `ServiceError::InvalidOperation`
- At end of spawned engine task: `remove(&session_id)` — releases lock regardless of success/error
- Cloned `active_sessions` into the spawned task for cleanup
- All 15 local_service tests pass, clean build

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