---
id: t-b-cadence-check-tool-who-am-i
level: task
title: "T-B: cadence_check tool — who am I overdue with"
short_code: "ARAWN-T-0462"
created_at: 2026-05-31T15:44:54.775913+00:00
updated_at: 2026-06-01T02:12:44.439159+00:00
parent: ARAWN-I-0065
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0065
---

# T-B: cadence_check tool — who am I overdue with

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0065]]

## Objective

Build `cadence_check` — answers *"who am I overdue with?"* by joining `PersonProfile.relation_to_user` (the I-0064 org substrate) against the `calendar_events` projection. Returns sorted list of directs / managers / peers past the overdue threshold, oldest first; "no recorded meeting" bubbles to the top.

## Status Updates

**2026-05-31 — shipped.**

- `crates/arawn-engine/src/tools/cadence_check.rs` (new): `CadenceCheckTool` taking `MemoryHandle` + `Arc<ProjectionStore>`. Params: `role` (default `directs`), `overdue_days` (default 14). Pulls matching PersonProfiles, loads the last 90 days of calendar events once, matches each person against attendees via token-substring (lowercased ≥2-char tokens), renders markdown with rel label per person. "🎉" message when nothing overdue.
- `crates/arawn-engine/src/tools/mod.rs` + `lib.rs`: module + re-export
- `crates/arawn/src/main.rs:528-545`: registered inside the projections block (needs both prerequisites)
- 11 new tests: no-directs message, never-met flagged, recent-meeting excluded, old-meeting flagged, sort order, role filter, unknown role, threshold respected, helpers

Verification: `cargo test -p arawn-engine --lib tools::cadence_check` — 11/11 pass; `angreal check workspace` — clean.

## Known follow-ups (deferred)

- `role: "skips"` — needs a quarterly-cadence marker on PersonProfile
- Attendee aliasing — token substring is best-effort; Person.email or alias table needed for collisions
- Per-relation overdue defaults — directs probably want 14d, quarterly skip-levels want 90d

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