---
id: context-file-support-arawn-md
level: task
title: "Context file support — .arawn.md equivalent of CLAUDE.md for project context"
short_code: "ARAWN-T-0037"
created_at: 2026-04-01T11:02:00.804062+00:00
updated_at: 2026-04-02T20:22:35.260535+00:00
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

# Context file support — .arawn.md equivalent of CLAUDE.md for project context

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective

Support `.arawn.md` files as project-level context that gets injected into the system prompt automatically. Equivalent of Claude Code's `CLAUDE.md`. Users place an `.arawn.md` in their workstream root (or home dir for global context), and its contents are appended to the system prompt for every LLM request in that workstream. Enables project-specific instructions ("this is a Rust project, use cargo for builds", "database is PostgreSQL", etc.) without repeating them each session.

### Priority
- P2 — significantly improves context quality for repeated use in the same project

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] On session start, check for `.arawn.md` in workstream root directory
- [ ] Also check for `~/.arawn/context.md` as global fallback (applied to all workstreams)
- [ ] If found, read contents and append to system prompt as a `# Project Context` section
- [ ] Workstream-level `.arawn.md` takes precedence; global is appended after it (both can coexist)
- [ ] File is read once at session start and cached — changes require new session (or explicit reload command)
- [ ] If file doesn't exist, no error — silently skip
- [ ] Large files truncated with warning (max 10k chars — roughly 2.5k tokens)
- [ ] System prompt builder becomes a function: `build_system_prompt(base, workstream, context_files) → String`
- [ ] Test: session with `.arawn.md` includes its content in system prompt
- [ ] Test: session without `.arawn.md` works unchanged
- [ ] Test: truncation kicks in for large files

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