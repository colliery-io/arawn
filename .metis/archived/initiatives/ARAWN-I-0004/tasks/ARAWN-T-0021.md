---
id: message-summary-variant-compaction
level: task
title: "Message::Summary variant + compaction prompt"
short_code: "ARAWN-T-0021"
created_at: 2026-04-01T03:28:12.394446+00:00
updated_at: 2026-04-01T03:58:13.409725+00:00
parent: ARAWN-I-0004
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0004
---

# Message::Summary variant + compaction prompt

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0004]]

## Objective

Add `Message::Summary` variant to arawn-core and implement the compaction prompt module with the structured 9-section format from Claude Code's proven design.

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

- [ ] `Message::Summary` variant: `content: String`, `original_count: usize`, `estimated_tokens_saved: u32`
- [ ] Summary serializes/deserializes to JSONL with `"role": "summary"` tag
- [ ] `Session::compact(&mut self, summary, keep_recent)` — replaces messages before keep window with a single Summary message
- [ ] `compact_prompt.rs` module with `get_compact_prompt()` and `get_partial_compact_prompt()` returning the structured prompt
- [ ] Prompt includes NO TOOLS preamble, `<analysis>` scratchpad instruction, 9-section `<summary>` template
- [ ] `format_compact_summary(raw)` — strips `<analysis>` block, extracts `<summary>` content
- [ ] `get_compact_user_summary_message(summary)` — wraps formatted summary with "This session is being continued..." framing
- [ ] QueryEngine's `build_request` handles `Message::Summary` (maps to user role with summary content)
- [ ] Unit tests: Summary serialization roundtrip, Session::compact reduces message count, prompt format functions

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

- `Message::Summary` in `crates/arawn-core/src/message.rs` — new variant with serde tag `"summary"`
- `Session::compact` in `crates/arawn-core/src/session.rs` — mutates messages in place
- `compact_prompt.rs` in `crates/arawn-engine/src/` — prompt templates adapted from Claude Code's `prompt.ts`
- Prompt reference: `claude-code/src/services/compact/prompt.ts`
- Depends on: nothing (extends existing types + new module)

## Status Updates
- **2026-04-01**: Complete. Message::Summary variant (added in T-0020, fleshed out here). Session::compact(summary, keep_recent) replaces old messages with Summary + preserves recent. Session::load_compacted finds last Summary and skips prior. compact_prompt.rs with full/partial prompts (9-section structured format from Claude Code), format_compact_summary (strips analysis, extracts summary), get_compact_user_summary_message (continuation framing). QueryEngine::build_request maps Summary to user role. 10 new tests (4 session compact, 6 prompt format). 149 total, clippy clean.