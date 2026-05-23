---
id: token-counting-and-cost-tracking
level: task
title: "Token counting and cost tracking — per-turn usage reporting"
short_code: "ARAWN-T-0040"
created_at: 2026-04-01T11:02:03.875836+00:00
updated_at: 2026-04-02T12:51:36.518080+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Token counting and cost tracking — per-turn usage reporting

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective

Track actual token usage from LLM API responses and accumulate per-session cost. Currently the `Usage` struct exists in `ChatChunk::Done` but is ignored after parsing — never accumulated or reported. Users have no visibility into how many tokens a conversation consumed or what it cost. Essential for budgeting, especially with paid APIs like Anthropic.

Claude Code tracks input/output tokens per turn, accumulates total cost, and displays it in the UI. Also supports budget limits (max_budget_usd) to cap spending.

### Priority
- P2 — important for cost awareness, especially once Anthropic provider is added

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `SessionStats` struct: `total_input_tokens: u64`, `total_output_tokens: u64`, `total_turns: u32`, `total_tool_calls: u32`
- [ ] Engine accumulates `Usage` from each `ChatChunk::Done` into `SessionStats`
- [ ] `SessionStats` exposed on `Session` (persisted in SQLite session metadata, not just in-memory)
- [ ] Per-model cost rates: configurable map of model → cost_per_1k_input / cost_per_1k_output
- [ ] `SessionStats::estimated_cost_usd()` computes cost from token counts + rates
- [ ] Compaction LLM calls also counted (separate from main conversation — flagged as compaction tokens)
- [ ] CLI displays session stats at end: "Session used X input + Y output tokens (~$Z.ZZ)"
- [ ] `arawn --session <id>` shows stats for a resumed session (cumulative across runs)
- [ ] Optional budget limit: `max_budget_usd` in config — engine stops with error if exceeded
- [ ] Test: stats accumulate across multiple engine turns
- [ ] Test: cost calculation with known rates

## Implementation Notes

- `Usage` already parsed from Groq's `ChatChunk::Done` — just need to accumulate it
- `SessionStats` could live in `arawn-core` (domain type) or `arawn-engine` (engine concern)
- SQLite migration V2 to add `input_tokens`, `output_tokens` columns to sessions table
- Cost rates: start with hardcoded map (Groq is free tier, Anthropic has known pricing), make configurable later via arawn.toml
- Budget enforcement: check after each LLM call, before starting next turn
- Depends on: nothing (enhancement to existing engine + storage)

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