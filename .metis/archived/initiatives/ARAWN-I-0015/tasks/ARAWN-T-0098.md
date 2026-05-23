---
id: memory-search-tool-composite
level: task
title: "memory_search tool — composite retrieval (semantic + FTS + tags + graph expansion)"
short_code: "ARAWN-T-0098"
created_at: 2026-04-05T14:41:52.475056+00:00
updated_at: 2026-04-05T15:40:25.295102+00:00
parent: ARAWN-I-0015
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0015
---

# memory_search tool — composite retrieval (semantic + FTS + tags + graph expansion)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0015]]

## Objective **[REQUIRED]**

Engine tool that lets the agent query the KB using multiple retrieval strategies. Combines semantic similarity, FTS5 text search, tag filtering, and optional 1-hop graph expansion. Results are deduplicated and ranked by composite score.

### Type: Feature | Priority: P1 | Effort: M

## Depends On: T-0093 (arawn-embed), T-0094 (arawn-memory), T-0095 (vectors)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `MemorySearchTool` implements `Tool` trait, registered in engine
- [ ] Parameters: query (required), entity_type (optional filter), tags (optional filter), scope (global/workstream/both, default both), limit (default 10), include_related (default false)
- [ ] Semantic search: embed query via `Embedder`, search sqlite-vec for similar entities
- [ ] FTS5 search: query against entities_fts for keyword matches
- [ ] Tag filter: intersect with specified tags when provided
- [ ] Composite scoring: `0.4 * semantic + 0.3 * fts_rank + 0.3 * confidence`
- [ ] Deduplication: entity appearing in both semantic + FTS results uses highest score
- [ ] Graph expansion: when `include_related`, follow relations 1 hop and append related entities with relation type
- [ ] Searches both global and workstream stores, merges results
- [ ] Skips superseded entities
- [ ] Output: ranked list with entity title, type, score, confidence, reinforcement count, content snippet, tags, related entities
- [ ] Unit tests: FTS-only search, semantic-only search, combined scoring, tag filtering, graph expansion

## Implementation Notes

- Port recall logic from backup `arawn-memory/src/store/recall.rs` — simplified (no time range filter initially)
- Semantic search gracefully degrades if no embedder configured (FTS + confidence only)
- Graph expansion uses `get_relations()` on each matched entity, fetches 1-hop neighbors
- Content snippet: first 200 chars of entity content for display

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