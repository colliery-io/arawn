---
id: move-tool-category-onto-tool-trait
level: task
title: "Move tool_category onto Tool trait — delete string dispatch in permissions/checker.rs"
short_code: "ARAWN-T-0188"
created_at: 2026-04-18T14:13:33.004458+00:00
updated_at: 2026-04-18T14:56:59.782406+00:00
parent: ARAWN-I-0032
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0032
---

# Move tool_category onto Tool trait — delete string dispatch in permissions/checker.rs

## Parent Initiative

[[ARAWN-I-0032]]

## Objective

`permissions::checker::tool_category(name: &str) -> ToolCategory` is a string switch hard-coding every tool name. The `Tool` trait already has `fn category(&self) -> ToolCategory` with a `Core` default. Let each tool declare its own category; delete the centralized switch.

Permission checker callers pass `&dyn Tool` (or the category directly) instead of a string name.

Estimated size: **S** (~0.5 day).

### Priority
- [x] P2 - Medium (cleanup)

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

- [ ] Each built-in tool that currently appears in the `tool_category` string switch overrides `Tool::category()` to return its correct category
- [ ] `permissions::checker::tool_category(name: &str)` function deleted; callers updated
- [ ] Permission checker's `check(name, ...)` either keeps the string name param (looking up via registry) or switches to accepting a `&dyn Tool` / `ToolCategory` — documented in commit
- [ ] Check the 4 `permissions::ToolCategory` (ReadOnly/FileWrite/Shell/Other) vs `tool::ToolCategory` (Core/Task/Agent/Web/etc.) distinction is preserved — they're semantically different categories; pick the right one (likely `tool::ToolCategory`) and rationalize
- [ ] Existing permission tests pass (especially rules that gate on category)
- [ ] `cargo check --workspace` clean; `angreal test unit` green
- [ ] Single focused commit

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

**2026-04-18 — Completed**

Moved permission-risk classification from the string-switch in `permissions/checker.rs` onto the `Tool` trait.

Key changes:
- New `arawn_tool::PermissionCategory` enum (ReadOnly/FileWrite/Shell/Other), re-exported from `arawn_tool`.
- `Tool::permission_category()` trait method with default derived from `is_read_only()`:
  - `is_read_only() == true` → `ReadOnly`
  - Otherwise → `Other`
- Explicit overrides on write/shell tools: `FileEditTool`, `FileWriteTool`, `ShellTool`.
- `PermissionMode::fallback` now takes `(category, tool_name)` — `tool_name` still used for the plan-mode `enter_plan_mode`/`exit_plan_mode` carve-out.
- `PermissionChecker::check(tool_name, tool_input, category)` — caller passes the declared category. Engine looks it up via `registry.get(name).map(|t| t.permission_category()).unwrap_or(Other)`.
- Deleted: `permissions::checker::ToolCategory` enum, `permissions::checker::tool_category(&str) -> ToolCategory` function, corresponding tests, and re-exports in `permissions/mod.rs` and `engine/lib.rs`.

Distinction preserved: `tool::ToolCategory` (Core/Task/Agent/Web/etc.) remains unchanged — it's the *feature-area* grouping used for context filtering. The new `PermissionCategory` is the *risk class* used for permission-mode fallback. Two orthogonal enums, same shape conceptually but different purposes — documented via the new type's doc comment.

As a side benefit, this also fixes a latent bug: the old `tool_category()` used CamelCase tool names (`"Read"`, `"Edit"`, `"Bash"`) but production tools use snake_case (`"file_read"`, `"file_edit"`, `"shell"`). The old switch silently returned `Other` for every real tool; now the category is tied to the tool itself so it's always correct.

Verification: `angreal check workspace` clean, `angreal test unit` green (all suites including permissions, hot_reload, testing harness).