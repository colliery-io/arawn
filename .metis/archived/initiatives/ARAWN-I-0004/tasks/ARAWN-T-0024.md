---
id: compaction-tests-mock-llm
level: task
title: "Compaction tests — mock LLM, threshold detection, session load with Summary"
short_code: "ARAWN-T-0024"
created_at: 2026-04-01T03:28:17.415807+00:00
updated_at: 2026-04-01T04:16:41.277278+00:00
parent: ARAWN-I-0004
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0004
---

# Compaction tests — mock LLM, threshold detection, session load with Summary

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0004]]

## Objective

Comprehensive tests for compaction: unit tests for components, functional tests for the full compaction flow via TestHarness, and persistence integration tests for session load with Summary messages.

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

- [ ] Test: `should_compact` returns false when under threshold
- [ ] Test: `should_compact` returns true when over threshold
- [ ] Test: compaction with MockLlm — session gets Summary, old messages removed, recent preserved
- [ ] Test: partial compaction — session starting with Summary gets re-summarized correctly
- [ ] Test: `format_compact_summary` strips `<analysis>`, extracts `<summary>` content
- [ ] Test: `format_compact_summary` handles missing tags gracefully
- [ ] Test: engine loop with compactor — over-threshold session compacts mid-conversation
- [ ] Test: engine loop without compactor — backward compatible, no compaction
- [ ] Persistence test: append messages + Summary to JSONL, load session, verify compacted view
- [ ] Persistence test: session resume after compaction loads Summary + recent messages only
- [ ] Test: compaction doesn't fire when session has fewer messages than keep_recent

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

- Unit tests inline in each module (token_estimator, compact_prompt, compactor)
- Functional tests in `crates/arawn-tests/tests/compaction.rs` using TestHarness + MockLlm
- Persistence tests in `crates/arawn-tests/tests/compaction.rs` using Store + tempdir
- MockLlm for compaction: script a response that includes `<analysis>...</analysis><summary>...</summary>` blocks
- To test threshold: create a session with many large messages, verify should_compact triggers
- Depends on: T-0023 (everything wired)

## Status Updates
- **2026-04-01**: Complete. 6 functional tests in arawn-tests/tests/compaction.rs: engine compacts over threshold (verifies Summary inserted + message count reduced), engine without compactor (backward compat), engine under threshold (no compaction), persistence with Summary (load skips pre-summary messages), persistence without Summary (loads all), resume after compaction (Summary + recent loaded, engine continues). Combined with inline unit tests: 9 in token_estimator, 6 in compactor, 6 in compact_prompt, 4 in session compact. 161 total workspace tests, clippy clean.