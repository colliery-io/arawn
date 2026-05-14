---
id: palaces-documentation-docs-src
level: task
title: "Palaces documentation — docs/src/palaces/ + SUMMARY + getting-started update"
short_code: "ARAWN-T-0267"
created_at: 2026-05-14T20:48:18.439226+00:00
updated_at: 2026-05-14T20:59:03.564236+00:00
parent: ARAWN-I-0040
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0040
---

# Palaces documentation — docs/src/palaces/ + SUMMARY + getting-started update

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0040]]

## Objective **[REQUIRED]**

{Clear statement of what this task accomplishes}

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

## Status Updates

### 2026-05-14 — Complete

Five new docs under `docs/src/palaces/` plus a SUMMARY entry and a new
"Workstream palaces (optional)" section in getting-started.md:

- **`palaces/index.md`** — three-layer mental model (feeds → projections → palaces), what lives in a palace, the workstream lifecycle, when palaces make sense vs when to stay at projections, pointers to ADR-0002/0003/0004.
- **`palaces/projections.md`** — the middle layer; per-feed-type tables; what fields each carries; how rows get written by the dispatcher; the embedding pass; when to read projections vs the palace.
- **`palaces/extraction.md`** — the 4-stage CoT chain (classify → extract → link → write); the two-field tag model with the empirical justification for why we abandoned free-form-only; link-by-name; provenance; cursor + idempotency.
- **`palaces/steward.md`** — the four subroutines + dust; the journal contract; the Extract→Suggest→Add cycle as the canonical example; blast-radius caps; per-subroutine apply/rollback dispatch table.
- **`palaces/agent-read-patterns.md`** — recipes for every tool the agent reaches for: signal_search / signal_query / signal_timeline / workstream_show for reading; workstream_dust for curation; workstream_journal / workstream_refine for review; workstream_apply / workstream_rollback for commit; workstream_tag for manual ontology CRUD. Ends with a "typical session" walkthrough.

Plus:
- `docs/src/SUMMARY.md` gains the palaces subtree under Reference.
- `docs/src/getting-started.md` gains section 8 "Workstream palaces (optional)" with the create flow + bind + query commands.

`angreal docs build` succeeds; all five pages render under `docs/book/palaces/`.

This task completes the I-0040 spec's documentation deliverables. UAT broadening for the tag-promoter cycle is T-0268.