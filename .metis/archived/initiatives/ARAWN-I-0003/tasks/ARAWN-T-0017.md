---
id: built-in-tools-filewritetool
level: task
title: "Built-in tools — FileWriteTool, FileEditTool, GrepTool"
short_code: "ARAWN-T-0017"
created_at: 2026-04-01T01:16:45.631710+00:00
updated_at: 2026-04-01T02:43:12.543291+00:00
parent: ARAWN-I-0003
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0003
---

# Built-in tools — FileWriteTool, FileEditTool, GrepTool

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0003]]

## Objective

Implement three new built-in tools: FileWriteTool, FileEditTool, and GrepTool. These stay in arawn-engine (not plugins) because they need direct filesystem access scoped to the workstream.

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

- [ ] `FileWriteTool` — params: `path`, `content`. Creates parent dirs, path traversal protection, returns bytes written
- [ ] `FileEditTool` — params: `path`, `old_string`, `new_string`, `replace_all`. Reads file, replaces, writes back. Fails if old_string not found or ambiguous (unless replace_all)
- [ ] `GrepTool` — params: `pattern`, `path`, `glob`, `case_insensitive`. Shells to `rg`, falls back to `grep -r`. CWD = workstream root
- [ ] Each tool has valid JSON Schema from `parameters_schema()`
- [ ] Path traversal protection on FileWrite and FileEdit (same as FileRead)
- [ ] Unit tests: FileWrite creates file + creates parent dirs, FileEdit replaces text + fails on missing string, Grep finds matches
- [ ] TestHarness functional test: LLM calls FileWrite → FileRead to verify content
- [ ] All registered in binary crate alongside existing tools

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
- `tools/file_write.rs`, `tools/file_edit.rs`, `tools/grep.rs` in `crates/arawn-engine/src/`
- Follows same patterns as existing tools (Think, Shell, FileRead)
- FileWrite: `tokio::fs::write` after path validation
- FileEdit: read → string replace → write. Check uniqueness of old_string before replacing.
- Grep: `tokio::process::Command` with `rg` binary. Check `which rg` at startup, fall back to `grep -rn`.
- Independent of plugin work — can be done in parallel with T-0015/T-0016
- Depends on: nothing (extends existing tool infrastructure)

## Status Updates
- **2026-04-01**: Complete. FileWriteTool (create/overwrite, parent dir creation, path traversal via normalize+canonical root), FileEditTool (replacen/replace with ambiguity check, path traversal), GrepTool (rg with fallback to grep -rn, glob filter, case insensitive). 16 new unit tests (5 write, 6 edit, 5 grep). All registered in binary. 126 total workspace tests, clippy clean.