---
id: two-tier-kb-initialization-global
level: task
title: "Two-tier KB initialization — global + workstream memory.db lifecycle"
short_code: "ARAWN-T-0096"
created_at: 2026-04-05T14:41:49.302083+00:00
updated_at: 2026-04-05T15:27:46.336730+00:00
parent: ARAWN-I-0015
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0015
---

# Two-tier KB initialization — global + workstream memory.db lifecycle

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0015]]

## Objective **[REQUIRED]**

Wire the two-tier memory database lifecycle into arawn's startup. Global KB at `~/.arawn/memory.db` (created once, shared across all workstreams). Workstream KB at `~/.arawn/workstreams/{ws}/memory.db` (created per-workstream). Both opened and passed to the engine/tools at session start.

### Type: Feature | Priority: P1 | Effort: S

## Depends On: T-0094 (arawn-memory)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Global memory.db created at `~/.arawn/memory.db` on first startup (with migrations)
- [ ] Workstream memory.db created at `~/.arawn/workstreams/{ws-dir}/memory.db` when workstream is first used
- [ ] `MemoryManager` struct holding both stores: `global: Arc<MemoryStore>`, `workstream: Arc<MemoryStore>`
- [ ] `MemoryManager` created in `main.rs` startup, passed to engine and tools
- [ ] Scope routing: Preference/Person entities default to global, Decision/Convention/Note default to workstream
- [ ] Graceful degradation: if memory.db can't be opened, log warning and continue without memory
- [ ] Unit tests: database creation, scope routing logic

## Implementation Notes

- Follow same pattern as existing `Store` initialization in `main.rs`
- `MemoryManager` is the single handle the rest of the system uses — abstracts the two-tier scoping
- Workstream directory path comes from existing `workstream_dir_name()` in arawn-storage
- Consider: should entities be queryable across both tiers? Yes — `memory_search` searches both by default

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

### 2026-04-05
- Created MemoryManager holding Arc<MemoryStore> for global + workstream
- Scope routing: Preference/Person → global, Decision/Convention/Fact/Note → workstream
- try_open_memory() for graceful degradation
- Vector init integrated — optionally enables sqlite-vec on both stores
- Wired into main.rs serve + CLI mode startup
- 41 tests passing (6 new manager tests)