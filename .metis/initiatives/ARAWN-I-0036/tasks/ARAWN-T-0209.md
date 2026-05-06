---
id: phase-2-tool-call-collapsed
level: task
title: "Phase 2 — Tool-call collapsed rendering (single-line by default)"
short_code: "ARAWN-T-0209"
created_at: 2026-05-06T11:21:51.757665+00:00
updated_at: 2026-05-06T11:21:51.757665+00:00
parent: ARAWN-I-0036
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0036
---

# Phase 2 — Tool-call collapsed rendering (single-line by default)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0036]]

## Objective

Replace the current 4-6-line bordered tool-call cards with a single-line collapsed renderer by default. Bordered card stays available on error or via Ctrl+E expand. This is I-0036 Phase 2 — the single most visible change in the visual coherence pass; with 10 tool calls in a row the chat goes from 50+ lines of mostly chrome to ~10 lines of compact content.

Decision (locked in I-0036): collapsed throughout, including while streaming. Single-line `⏵ shell · ls -la · 1.2s` (or `· running 1.2s` while in-flight). Simpler than expanded-while-streaming + collapse-on-completion; consistent with "chat content dominates, chrome recedes."

## Type / Priority

- Feature
- P1 — biggest visible change in the visual coherence pass.

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

- [ ] `ChatRole::ToolCall` rendering branches on a per-message expansion flag (already tracked in `App::expanded_tool_results: HashSet<usize>`). Default render is collapsed; expanded path is the existing bordered-card code.
- [ ] **Collapsed render shape:** single line.
  - In-flight: `  ⏵ {tool_name}{spinner_frame} · {summary} · running {elapsed}s`
  - Success: `  ⏵ {tool_name} · {summary} · {elapsed}s`
  - Error: **always expanded** (force the bordered card; an error needs the body visible). Error is the one case where the user shouldn't have to press Ctrl+E to see what went wrong.
- [ ] Spinner glyph: same `SPINNER_FRAMES` already used elsewhere; one frame per render-tick while in-flight.
- [ ] Summary truncation in collapsed mode: hard-cap at 60 display columns (use `unicode-width` per Phase 4's pattern; OK to land before T-0211 since the cap is small enough that drift is invisible — note this in T-0211's acceptance for cleanup).
- [ ] **Tool result rendering** stays as-is for now (it's already collapsed-by-default with Ctrl+E to expand). Sanity check: a successful tool call followed by its result reads as two related single lines, not two boxes.
- [ ] **Ctrl+E** (existing binding): toggles expanded state for the focused / hovered tool call. Today it expands tool *results*; extend coverage to tool *calls* via the same action. `Action::ToggleToolResult(idx)` becomes `Action::ToggleToolEntry(idx)` (rename) and works for both `ChatRole::ToolCall` and `ChatRole::ToolResult` indices.
- [ ] **Snapshot tests** updated and re-baselined. Add at least one new snapshot for "10 tool calls in a row" so future regressions are caught.
- [ ] **Performance:** confirm the new path doesn't re-render frame more often than the existing one (spinner ticks were already firing every 100ms; per-frame work shouldn't increase).

## Implementation Notes

- **Where to branch:** `crates/arawn-tui/src/render.rs` around line 332-396 (the `ChatRole::ToolCall` arm). Today it builds the bordered card unconditionally; new logic checks `app.expanded_tool_results.contains(&msg_idx)` first.
- **`expanded_tool_results` semantics:** today this set tracks which *results* have been Ctrl+E-expanded. Extend to also cover tool *calls* by indexing on the message's position in `app.messages`. The existing `ToggleToolResult(idx)` action and `ToggleAllToolResults` action both keep working — they now affect both calls and results.
- **In-flight detection:** today's `is_running = !next_is_result && app.is_generating` logic stays; the only change is the layout the result feeds into (single-line spans vs the bordered card).
- **Error branch:** the existing `next_is_error` flag (computed in `tool_call_flags`) determines whether to force-expand. If `next_is_error`, render the bordered card unconditionally regardless of `expanded_tool_results`.
- **Minimal example of the collapsed Line:**
  ```
  Line::from(vec![
      Span::raw("  "),
      Span::styled("⏵ ", chrome),
      Span::styled(name, theme::TOOL_NAME),  // lavender per Phase 1 mapping
      Span::styled(" · ", chrome),
      Span::styled(summary_truncated, theme::RESULT_LABEL),
      Span::styled(format!(" · {elapsed_str}"), chrome),
  ])
  ```
- **Don't touch** the `unicode-width` measurement of the existing bordered card — Phase 4 / T-0211 owns that. Just use `chars().count()` for the 60-column cap in collapsed mode (it's a soft cap; off-by-one for a CJK char doesn't break anything).

## Out of Scope (defer)

- Display-width measurement for the collapsed line — covered by T-0211's broader sweep.
- Renaming `ToggleAllToolResults` → `ToggleAllToolEntries` — the existing name still works because the underlying semantics are now broader; rename is cosmetic and can wait.
- Inline expansion of result bodies on Ctrl+E — today Ctrl+E on a result expands it; today on a call doesn't do anything. After this task, Ctrl+E on a call expands it. No new affordance.

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