---
id: extraction-on-compaction-extract
level: task
title: "Extraction on compaction — extract key facts from messages before context is lost"
short_code: "ARAWN-T-0100"
created_at: 2026-04-05T14:41:55.139469+00:00
updated_at: 2026-04-05T15:46:33.454218+00:00
parent: ARAWN-I-0015
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0015
---

# Extraction on compaction — extract key facts from messages before context is lost

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0015]]

## Objective **[REQUIRED]**

When the compactor runs, scan the messages being summarized for extractable knowledge before they're lost to compaction. Use the LLM to identify facts, decisions, conventions, and preferences, then store them in the KB via search-before-create. This is the automatic memory accumulation path — the agent learns without being told to remember.

### Type: Feature | Priority: P2 | Effort: L

## Depends On: T-0097 (memory_store tool)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Hook into the compaction pipeline: after messages are selected for summarization but before they're replaced
- [ ] Use the LLM (compaction model) to extract structured signals from the messages: facts, decisions, conventions, preferences mentioned
- [ ] Extraction prompt produces JSON array of `{entity_type, title, content, tags}` candidates
- [ ] Each candidate passed through `memory_store` with search-before-create (reinforces or creates)
- [ ] All extracted entities get `extracted_from` relation to the session
- [ ] Confidence set to `Inferred` (0.5) for auto-extracted entities
- [ ] Extraction is best-effort — failure doesn't block compaction
- [ ] Configurable: extraction can be disabled in arawn.toml `[memory] extract_on_compact = true`
- [ ] Integration test: mock session with known facts → compact → verify entities in KB

## Implementation Notes

- Add extraction step in `Compactor::compact()` — run BEFORE replacing messages with summary
- Extraction prompt: "Given these conversation messages, extract any reusable knowledge as structured entities. Focus on: user preferences, project decisions, coding conventions, people mentioned. Return JSON array."
- Use same LLM as compaction (already available in Compactor)
- Rate limit: max 10 entities per compaction to avoid noise
- This is the equivalent of Clotho's debrief-processor but triggered automatically, not by ceremony

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