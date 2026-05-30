---
id: t-a-add-manages-reportsto-peerof
level: task
title: "T-A: Add Manages / ReportsTo / PeerOf to RelationType enum"
short_code: "ARAWN-T-0456"
created_at: 2026-05-30T20:36:33.139038+00:00
updated_at: 2026-05-30T20:39:00.127127+00:00
parent: ARAWN-I-0064
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0064
---

# T-A: Add Manages / ReportsTo / PeerOf to RelationType enum

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0064]]

## Objective

Foundation for I-0064: add `Manages`, `ReportsTo`, `PeerOf` to `RelationType` so the rest of the initiative has typed edges to write. Person↔Person org-graph edges (between two non-self Person entities). The user's own relations to people will live as a column on `PersonProfile` (T-B), not as graph edges, so no self-sentinel is needed here.

`OnTeam` and `BelongsToProject` deliberately deferred to I-0066 — they need `Team` and `Project` entity variants which don't exist yet.

## Acceptance Criteria

## Acceptance Criteria

- [x] `RelationType::{Manages, ReportsTo, PeerOf}` added to `arawn-memory/src/types.rs`
- [x] `as_str` and `from_str` cover the new variants; roundtrip test extended
- [x] `cypher_schema.rs` `relation_type_str` / `relation_type_from_str` mirror the new variants (`MANAGES`, `REPORTS_TO`, `PEER_OF`)
- [x] Cypher roundtrip test extended (was missing `Summarizes` too — fixed)
- [x] `cargo check` workspace clean
- [x] `cargo test -p arawn-memory` — 8/8 pass

## Status Updates

**2026-05-30 — shipped.** Edits:
- `arawn-memory/src/types.rs:79`: 3 variants added with doc comments explaining the I-0064 origin and the self-as-column decision
- `arawn-memory/src/types.rs:91-93`: `as_str` cases
- `arawn-memory/src/types.rs:106-108`: `from_str` cases
- `arawn-memory/src/types.rs:330-339`: roundtrip test extended (also picked up missing `Summarizes`)
- `arawn-memory/src/cypher_schema.rs:58-62`: `relation_type_str` cases (screaming-snake: `MANAGES`, `REPORTS_TO`, `PEER_OF`)
- `arawn-memory/src/cypher_schema.rs:72-76`: `relation_type_from_str` cases
- `arawn-memory/src/cypher_schema.rs:215-220`: cypher roundtrip extended (also picked up missing `Summarizes`)

Verification: `angreal check workspace` clean, `cargo test -p arawn-memory` 8/8 pass.

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