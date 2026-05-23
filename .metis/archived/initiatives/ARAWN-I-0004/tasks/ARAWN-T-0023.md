---
id: wire-compaction-into-engine-loop
level: task
title: "Wire compaction into engine loop + persistence integration"
short_code: "ARAWN-T-0023"
created_at: 2026-04-01T03:28:15.159261+00:00
updated_at: 2026-04-01T04:12:37.872600+00:00
parent: ARAWN-I-0004
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0004
---

# Wire compaction into engine loop + persistence integration

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0004]]

## Objective

Wire the Compactor into the QueryEngine loop and integrate with the persistence layer so compaction events are durably recorded.

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

- [ ] `QueryEngine` accepts an optional `Compactor` (via config or constructor)
- [ ] At the top of each engine turn: if compactor present, call `should_compact()` → `compact()` if needed
- [ ] Compaction happens between turns, before `build_request` — transparent to the rest of the loop
- [ ] After compaction, the Summary message is appended to JSONL via the Store
- [ ] JSONL remains append-only — original messages stay, Summary appended after them
- [ ] Session load from JSONL: when a Summary message is found, use it as the starting point and skip prior messages for the in-memory session
- [ ] `Session::load_compacted(messages)` — builds session using the last Summary as base + messages after it
- [ ] Binary wired: creates Compactor with the same LLM client, passes to QueryEngine
- [ ] ModelLimits configured from GROQ_MODEL env (or defaults)
- [ ] Engine still works with no Compactor (backward compatible — compaction is optional)

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

## Implementation Notes

- QueryEngine changes: add `Option<Compactor>` field, check at top of `run` loop iteration
- The compactor needs access to the ToolRegistry (to estimate tool definition tokens) — pass as param or store ref
- Persistence: the binary's message-append loop already handles new messages. After compaction, the Summary message is in the session and gets appended like any other message.
- Session load: update `Session::from_parts` or add `load_compacted` that finds the last Summary and builds from there
- The JSONL file grows monotonically — compaction doesn't shrink it. The in-memory view is what gets compacted.
- Depends on: T-0022 (Compactor), T-0020 (ModelLimits in config)

## Status Updates
- **2026-04-01**: Complete. QueryEngine gains Option<Compactor> field + with_compactor() builder. Compaction check at top of each loop iteration: estimates tokens for tools + system prompt, calls should_compact, calls compact if needed. Failure is logged and skipped (graceful). Store::load_session uses Session::load_compacted for compaction-aware loading. Binary creates Compactor with same LLM client, wires via with_compactor(). Backward compatible — engine works with no compactor (all existing tests pass unchanged). 155 workspace tests, clippy clean.