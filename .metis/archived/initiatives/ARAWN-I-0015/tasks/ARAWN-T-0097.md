---
id: memory-store-tool-store-entities
level: task
title: "memory_store tool — store entities with search-before-create, confidence scoring"
short_code: "ARAWN-T-0097"
created_at: 2026-04-05T14:41:51.246385+00:00
updated_at: 2026-04-05T15:36:52.591574+00:00
parent: ARAWN-I-0015
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0015
---

# memory_store tool — store entities with search-before-create, confidence scoring

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0015]]

## Objective **[REQUIRED]**

Engine tool that lets the agent store knowledge in the KB. Implements search-before-create: searches both tiers for existing matches before inserting, reinforces existing entities if found, supersedes if contradicted. Routes to global or workstream store based on entity type or explicit scope.

### Type: Feature | Priority: P1 | Effort: M

## Depends On: T-0094 (arawn-memory), T-0095 (vectors), T-0096 (two-tier init)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `MemoryStoreTool` implements `Tool` trait, registered in engine
- [ ] Parameters: title (required), entity_type (required), content, tags, scope
- [ ] Scope inference: preference/person → global, decision/convention/note → workstream, fact → workstream (overridable)
- [ ] Search-before-create: FTS + semantic search across both tiers before inserting
- [ ] Returns Inserted (new entity ID), Reinforced (existing ID + new count), or Superseded (old ID + new ID)
- [ ] Confidence set automatically: Stated (1.0) when from `/remember`, Inferred (0.5) when from extraction
- [ ] Entity embedded on creation (if embedder available)
- [ ] `extracted_from` relation auto-created linking entity to source session
- [ ] Unit tests: insert path, reinforce path, supersede path, scope routing

## Implementation Notes

- Holds `Arc<MemoryManager>` and optional `Arc<dyn Embedder>`
- Search-before-create uses semantic similarity (if embedder available) + FTS to find candidates
- Candidate matching: same entity_type + similarity above threshold (0.85) → reinforce; conflicting content → supersede
- The tool description should guide the LLM on when to use each entity_type

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