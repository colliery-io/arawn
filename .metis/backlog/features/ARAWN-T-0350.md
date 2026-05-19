---
id: background-sub-agent-task
level: task
title: "Background sub-agent task enumerator (or rename session-todo task_*)"
short_code: "ARAWN-T-0350"
created_at: 2026-05-19T12:07:47.659218+00:00
updated_at: 2026-05-19T17:51:57.405578+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Background sub-agent task enumerator (or rename session-todo task_*)

## Objective

Resolve the naming collision between **per-session todos** (the existing `task_create` / `task_update` / `task_list` / `task_get` family in `tools/task_list.rs`) and **background sub-agent management** (the existing `task_output` / `task_stop` against `BackgroundTaskManager`). Today there is no tool that enumerates running background sub-agents, and the `task_*` name space conflates the two concepts.

## Impact

- **Severity:** P2 — capability gap + naming confusion.
- **Affected users:** anyone spawning multiple `run_in_background: true` sub-agents. The agent has no way to ask "what background tasks have I started in this session and are any still running?" — it has to remember the ids manually.
- **Naming confusion:** the docs originally claimed `task_list` listed background sub-agents (it doesn't). The disambiguation just landed in the triple-check (commit 7037763).

## Proposed design

Two options:

### Option A — rename the existing session-todo family

- `task_create` → `todo_create`, `task_update` → `todo_update`, etc.
- Add a new `task_list` (and `task_get`) that operates on `BackgroundTaskManager`.
- Cost: behavior change visible to any agent / skill / plugin using the old names. Need a deprecation period.

### Option B — add a new `bg_*` family

- `bg_list`, `bg_get`, `bg_output` (alias for `task_output`), `bg_stop` (alias for `task_stop`).
- Keep `task_*` as-is.
- Cost: redundant tool names; users see two families that look similar.

Recommend **Option A**. Confusion-reduction wins, and the existing `task_*` for session todos is barely used (we have `todo_*` for the user-facing todo system anyway).

## Implementation notes

- Session-todo tools: `crates/arawn-engine/src/tools/task_list.rs` (currently 4 tools: `task_create`, `task_update`, `task_list`, `task_get`).
- Background sub-agent management: `crates/arawn-engine/src/{background.rs, tools/{task_output, task_stop}.rs}`.
- `BackgroundTaskManager` exposes the data needed for `bg_list` / `task_list` (status, started_at, finished_at) — see `background.rs::BackgroundTaskStatus`.

## Acceptance criteria

- [ ] Decision recorded between Option A and Option B.
- [ ] Background sub-agent enumeration tool exists (`task_list` after rename, or `bg_list`).
- [ ] Per-id getter exists.
- [ ] Docs in `sub-agents.md` and `agent-tools.md` updated to reflect the new shape.

Surfaced during ARAWN-I-0051 doc triple-check.

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

*To be added during implementation*