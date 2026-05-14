---
id: tag-promoter-steward-subroutine
level: task
title: "tag-promoter steward subroutine — propose discovered tag promotion (Suggest)"
short_code: "ARAWN-T-0265"
created_at: 2026-05-14T13:43:22.696577+00:00
updated_at: 2026-05-14T20:41:49.190869+00:00
parent: ARAWN-I-0040
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0040
---

# tag-promoter steward subroutine — propose discovered tag promotion (Suggest)

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

**`arawn-steward::tag_promoter` module.** Pure-stats subroutine (no LLM call) — counts `tags_discovered` frequencies across active entities and proposes promotion of recurring tags into the workstream's declared ontology. Suggest stage of the ADR-0004 Extract→Suggest→Add cycle.

- `TagPromoterConfig`: `min_count` (default 3 — matches dust's "appeared in 3 independent contexts" intuition), `sample_entities` (5, samples returned in proposal payload), `max_entities_scanned` (2000, soft cap).
- Tally normalizes tags (lowercase + trim) so `Falcon` / `falcon ` / `FALCON` collapse to one bucket.
- Filters: empty tags dropped; `steward:*` markers dropped (internal metadata, not vocabulary); tags already in the workstream's ontology dropped (no point re-promoting); tags with pending `tag-promoter` proposals dropped (no duplicate proposals across passes).
- Sort by `(count desc, tag asc)` so the strongest candidates surface first when the per-pass cap bites.
- Proposal payload: `{tag, count, sample_entity_ids}`. Journaled with `applied=false`.

**JournalGate read-side accessor.** Added `JournalGate::inner_journal()` so subroutines can run read-side queries (like `pending_proposals`) against the underlying journal without needing a second handle. Write contract on the gate stays intact.

**Wiring (`main.rs`).** `TagPromoterSubroutine::default()` registered alongside reshelve / map / doorwatch in the steward runner. Runs every steward pass.

**Tests (inline, 7):**
- `promotes_tag_at_threshold` — happy path
- `below_threshold_no_proposal`
- `skips_tags_already_in_ontology`
- `skips_steward_internal_markers`
- `dedupes_against_pending_proposals` — two consecutive runs only emit one proposal
- `cap_caps_proposals_per_pass`
- `normalizes_case_and_whitespace_during_tally`

`cargo test -p arawn-steward` — 45/45 (8 new for tag-promoter + the accept/rollback expansions that landed alongside in T-0266). Clippy clean.

**Coupling:** This is the Suggest half; the Add half (committing the proposal into the ontology + the manual `workstream_tag` CRUD tool) lives in T-0266 and shipped in the same commit.