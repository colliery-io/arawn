---
id: phase-3-hierarchy-fixes-monochrome
level: task
title: "Phase 3 — Hierarchy fixes (monochrome headings, inline code, sidebar tab, assistant gutter)"
short_code: "ARAWN-T-0210"
created_at: 2026-05-06T11:21:57.519168+00:00
updated_at: 2026-05-06T11:21:57.519168+00:00
parent: ARAWN-I-0036
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0036
---

# Phase 3 — Hierarchy fixes (monochrome headings, inline code, sidebar tab, assistant gutter)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0036]]

## Objective

Fix the visual hierarchy so the eye reads what it should:
- Headings: pure monochrome ladder (no accent in headings).
- Inline code: drop literal backticks (background already disambiguates).
- Sidebar tab strip: kill the `· ·` stipple noise; replace with a clean border + accent indicator.
- Assistant gutter: thin left rule so assistant prose has structural weight (today it's a 2-space indent that reads as plain terminal output).

This is I-0036 Phase 3. Depends on T-0208 (palette wiring) for the new theme constants.

## Type / Priority

- Feature + visual polish
- P1 — affects every chat message; combined with T-0209's tool-call collapse this is what makes the agent's prose dominate the screen.

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

### Headings — pure monochrome ladder

- [ ] In `theme.rs`: `HEADING_1 = text + bold`, `HEADING_2 = text + bold` (same color, only differing from H1 by future spacing/markers if needed), `HEADING_3 = subtext1 + bold`, `HEADING_4 = subtext0 + italic`.
  - Actually use distinct shades: H1 = `text` bold, H2 = `subtext1` bold, H3 = `subtext0` bold, H4 = `overlay2` italic. Gives a real luminance ladder.
- [ ] No accent (mauve) appears in any heading style.
- [ ] Update markdown.rs heading rendering to use the new constants (already wired via Phase 1, this is just reassignment in `theme.rs`).
- [ ] Snapshot `styled_snapshot_rich_markdown` re-baselined; verify visually that h1 is brightest, h4 is dimmest.

### Inline code — drop the backticks

- [ ] In `markdown.rs`'s inline-code rendering, do not emit the literal `` ` `` characters around the code text. The peach-on-mantle background already signals "this is code."
- [ ] Code blocks (fenced) keep their existing rendering — this only affects inline `` `code` ``.
- [ ] Snapshot diff shows `parse` rendered as 5 visible chars instead of 7.

### Sidebar tab strip — kill the stipple

- [ ] When the sidebar is collapsed, the tab strip currently renders alternating `· ` / `  ` rows in DarkGray on the sidebar bg. Snapshot shows 22 of 24 rows of stipple.
- [ ] Replace with: (a) a single right-border `│` in `surface1` (chrome) running the full height; (b) a single `▸` in accent (mauve) at vertical mid-point. Nothing else.
- [ ] Sidebar-collapsed snapshot re-baselined; verify the bulk of vertical space is empty / chat-bg-colored, not stippled.

### Assistant gutter

- [ ] Assistant messages render with a thin left rule `│` in `surface1` running the height of the message, with content offset 2 chars right of the rule.
- [ ] User messages keep the existing `❯ ` prefix in `green`; gutter rule does not apply (the prefix already differentiates).
- [ ] Tool calls / results keep the `  ⏵ ` / `  │ ` chrome from T-0209; gutter rule does not apply (they already have left-side chrome).
- [ ] System messages use a different left marker — say `│ ` in `mauve` — to differentiate "internal note" from "agent prose."
- [ ] Snapshot covering 1 user + 1 assistant + 1 tool-call+result + 1 system message round added; assert each speaker has a distinct gutter signal.

## Implementation Notes

- All four sub-tasks live in `crates/arawn-tui/src/render.rs` and `crates/arawn-tui/src/markdown.rs`. None require new types or actions.
- Order to implement (independent within the task; do them as one PR):
  1. Theme constant reassignments (cheap, sets the values)
  2. Inline code backtick drop (smallest code change)
  3. Sidebar tab strip (drop the loop, add 1-line border + 1-cell accent)
  4. Assistant gutter (modify the assistant arm in `render.rs:316-328`)
- **Don't change the heading content / structure** — only style assignments. The markdown parser already produces the right tag tree.
- **Sidebar tab strip:** the existing tab loop is in `render.rs` around line 90-104. Replace with a render call that draws a vertical border and a single accent indicator at `area.height / 2`.
- **Snapshot churn:** expect every styled snapshot to fail; review once for content drift, accept the rest.

## Out of Scope (defer)

- Heading prefix/numbering/anchoring — pure styling task.
- Block quote / horizontal rule restyling — not flagged in the design review; leave as-is.
- Sidebar list item polish (alternating bg, time formatting) — that's MP-12 from the review, separate task.
- Empty / loading state hero — T-0212 (Phase 5).

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