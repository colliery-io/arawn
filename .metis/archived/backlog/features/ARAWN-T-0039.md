---
id: parallel-tool-execution-concurrent
level: task
title: "Parallel tool execution — concurrent tool calls when safe"
short_code: "ARAWN-T-0039"
created_at: 2026-04-01T11:02:02.957130+00:00
updated_at: 2026-04-02T20:07:33.206193+00:00
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

# Parallel tool execution — concurrent tool calls when safe

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective

When the LLM returns multiple tool_use blocks in a single response, execute them concurrently instead of sequentially. Currently the engine loops through tool calls one at a time — if the LLM requests `file_read("a.rs")` and `file_read("b.rs")` in the same turn, they run serially. Parallel execution halves the latency for independent reads/commands.

Claude Code supports parallel execution for independent tools. Safety consideration: some tool combinations shouldn't parallelize (e.g., two shell commands that modify the same file). For v1, parallelize read-only tools (file_read, grep, think) and serialize write tools (file_write, file_edit, shell).

### Priority
- P3 — performance optimization, not blocking. Serial execution works correctly.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] When response contains multiple tool_use blocks, group by safety class: read-only (parallelizable) vs write (serial)
- [ ] Read-only tools execute concurrently via `tokio::join!` or `futures::join_all`
- [ ] Write tools execute serially (same as today) after all parallel reads complete
- [ ] Tool results appended to session in the same order as the original tool_use blocks (deterministic)
- [ ] Each tool declares a `is_read_only() → bool` method on the `Tool` trait (default: false)
- [ ] FileReadTool, GrepTool, ThinkTool return `true` for `is_read_only`
- [ ] ShellTool, FileWriteTool, FileEditTool return `false`
- [ ] Mixed batches: parallel reads first, then serial writes
- [ ] Test: two file_reads in same turn complete faster than serial (timing test)
- [ ] Test: write tools still execute serially
- [ ] Test: tool results ordered correctly regardless of execution order

## Implementation Notes

- Modify `QueryEngine`'s tool execution loop: partition tool_calls into read/write groups, `join_all` the reads, then loop the writes
- The `Tool` trait gains `fn is_read_only(&self) -> bool { false }` with default impl
- `ToolContext` is `Clone` so each concurrent task can have its own reference
- Plugin tools (via fides) default to `is_read_only = false` (conservative — plugins can't declare this yet)
- Depends on: nothing (enhancement to existing engine loop)

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