---
id: compactor-summarization-logic
level: task
title: "Compactor — summarization logic, split old/recent, LLM call"
short_code: "ARAWN-T-0022"
created_at: 2026-04-01T03:28:13.738293+00:00
updated_at: 2026-04-01T04:09:24.540258+00:00
parent: ARAWN-I-0004
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0004
---

# Compactor — summarization logic, split old/recent, LLM call

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0004]]

## Objective

Implement the `Compactor` that orchestrates the compaction flow: check if compaction is needed, split messages into old/recent, call LLM with compaction prompt, format the result, and mutate the session.

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

- [ ] `Compactor::new(llm, keep_recent)` — takes an LlmClient (can be a different/cheaper model than the main one) and number of recent messages to preserve (default 6)
- [ ] `Compactor::should_compact(session, limits, tool_tokens) → bool` — uses TokenEstimator to check threshold
- [ ] `Compactor::compact(session, limits) → Result<CompactionResult>` — orchestrates the full flow
- [ ] CompactionResult: `messages_summarized: usize`, `tokens_before: u32`, `tokens_after: u32`
- [ ] Compaction sends old messages to LLM with compaction prompt (no tools in request)
- [ ] Collects streamed response (non-streaming also fine — single text response expected)
- [ ] Calls `format_compact_summary` to strip `<analysis>`, extract `<summary>`
- [ ] Calls `Session::compact` to replace old messages with Summary
- [ ] If session has fewer messages than `keep_recent`, no compaction (nothing old to summarize)
- [ ] If session already starts with a Summary, uses partial compaction prompt
- [ ] Unit tests with MockLlmClient: compaction produces Summary, message count reduces, recent messages preserved

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

- `compactor.rs` in `crates/arawn-engine/src/`
- The compaction LLM call uses `ChatRequest` with no tools (tool definitions omitted to prevent tool calls during summarization)
- System prompt for compaction = the compaction prompt. Messages = the old messages to summarize.
- The LLM response is plain text containing `<analysis>` and `<summary>` blocks
- Use same `stream` + collect pattern as the engine, but simpler (no tool execution)
- Depends on: T-0020 (TokenEstimator, ModelLimits), T-0021 (Message::Summary, compact_prompt)

## Status Updates
- **2026-04-01**: Complete. Compactor with should_compact (token threshold + message count guard) and compact (split old/recent, choose full/partial prompt based on prior Summary, LLM call, format, Session::compact). CompactionResult tracks messages_summarized + token counts. 6 unit tests with MockLlm: threshold checks, produces Summary, preserves recent, noop on few messages. 155 total workspace tests, clippy clean.