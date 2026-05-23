---
id: scaffold-arawn-integration-crate
level: task
title: "Scaffold arawn-integration crate — capability traits, shared types, IntegrationError"
short_code: "ARAWN-T-0179"
created_at: 2026-04-17T03:01:13.526793+00:00
updated_at: 2026-04-17T03:08:58.100446+00:00
parent: ARAWN-I-0029
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0029
---

# Scaffold arawn-integration crate — capability traits, shared types, IntegrationError

## Parent Initiative

[[ARAWN-I-0029]]

## Objective

Stand up the new `arawn-integration` crate with the four capability traits, shared domain types, and `IntegrationError`. No provider implementations land here — only the trait surface and types every provider will share. Compiles cleanly and is ready for OAuth/token work to layer on top.

Crate layout per the initiative:

```
arawn-integration/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── traits/{task_list,push,schedule,messaging}.rs
    ├── error.rs
    └── types/{task,notification,event,message}.rs
```

`IntegrationError` (thiserror): `AuthExpired`, `RateLimited { retry_after }`, `ApiError { status, body }`, `Network`, `MissingCapability`, `InvalidConfig`.

Estimated size: **S–M** (1–2 days).

### Priority
- [x] P2 - Medium (foundation; nothing else builds without it)

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

- [ ] New `crates/arawn-integration/` workspace member with `Cargo.toml` and `src/lib.rs`
- [ ] Four `#[async_trait]` traits defined: `TaskListProvider`, `PushProvider`, `ScheduleProvider`, `MessagingProvider`
- [ ] Shared types: `Task`, `NewTask`, `TaskUpdate`, `TaskFilter`, `Notification` (with urgency enum), `Event`, `NewEvent`, `EventUpdate`, `TimeRange`, `BusySlot`, `Message`, `MessageId`, `MessageFilter`, `Channel`
- [ ] `IntegrationError` enum (thiserror) with the variants listed in the objective
- [ ] All public types are `Send + Sync` and serde-serializable where it makes sense (`Task`, `Notification`, etc.)
- [ ] Compiles with `cargo check --workspace` clean
- [ ] No tests required beyond compile-time checks (next tasks add real tests)
- [ ] Re-exported from crate root: `pub use traits::*; pub use types::*; pub use error::IntegrationError;`

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

## Status Updates

- Created `crates/arawn-integration/` workspace member with `Cargo.toml`. Added to root `[workspace] members` and `[workspace.dependencies]`.
- Module layout: `lib.rs` (re-exports), `error.rs`, `types.rs`, `traits/{mod,task_list,push,schedule,messaging}.rs`. Used a single `types.rs` instead of a directory — only ~150 lines, splitting felt premature.
- `IntegrationError` (thiserror) variants: `AuthExpired`, `RateLimited{retry_after}`, `ApiError{status,body}`, `Network`, `MissingCapability{capability}`, `InvalidConfig`, `Decode`. Added `IntegrationError::missing(capability)` constructor.
- All four traits defined as `#[async_trait] + Send + Sync`. Domain types (Task/NewTask/TaskUpdate/TaskFilter, Notification + Urgency, Event/NewEvent/EventUpdate/TimeRange/BusySlot, Message/MessageId/MessageFilter/Channel) all serde-derived.
- `cargo check -p arawn-integration` clean. No tests yet — compile-time check only, per spec.