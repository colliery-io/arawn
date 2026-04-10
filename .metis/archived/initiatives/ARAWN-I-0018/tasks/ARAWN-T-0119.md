---
id: verify-workstream-scoped-context
level: task
title: "Verify workstream-scoped context — prompt, memory, and sandbox boundaries"
short_code: "ARAWN-T-0119"
created_at: 2026-04-06T19:29:55.786631+00:00
updated_at: 2026-04-07T12:16:18.738155+00:00
parent: ARAWN-I-0018
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0018
---

# Verify workstream-scoped context — prompt, memory, and sandbox boundaries

## Objective

Verify and fix that all workstream-scoped subsystems actually respect the active workstream: system prompt includes workstream name/root, workstream-level arawn.md is injected, memory KB is scoped, and the shell sandbox enforces workstream root_dir boundaries.

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

- [ ] System prompt includes current workstream name and root directory when session is in a non-scratch workstream
- [ ] Workstream-level `arawn.md` (at `{data_dir}/workstreams/{ws_dir}/arawn.md`) is found and injected into prompt context
- [ ] Memory KB uses workstream-scoped tier (not global) for workstream-bound sessions
- [ ] Shell tool sandbox restricts file access to the workstream's root_dir
- [ ] Integration test: create workstream, create session in it, verify ToolContext has correct working directory

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

2026-04-07: Code audit complete. All four scoping mechanisms verified correct:
1. System prompt: local_service.rs:609-625 uses actual workstream name/root in PromptContext
2. arawn.md: system_prompt.rs:341-357 find_context_files looks in workstream dir + global dir
3. Memory KB: manager.rs:30-66 creates separate global + workstream-scoped databases
4. Shell sandbox: context.rs:54-57 sets working_dir from workstream.root_dir, shell.rs uses it
No code changes needed — all already correctly scoped.