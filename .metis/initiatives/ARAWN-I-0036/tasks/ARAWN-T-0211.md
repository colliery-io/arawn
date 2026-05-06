---
id: phase-4-chrome-correctness-display
level: task
title: "Phase 4 — Chrome correctness (display-width measurement everywhere)"
short_code: "ARAWN-T-0211"
created_at: 2026-05-06T11:22:03.216534+00:00
updated_at: 2026-05-06T12:09:10.481532+00:00
parent: ARAWN-I-0036
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0036
---

# Phase 4 — Chrome correctness (display-width measurement everywhere)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0036]]

## Objective

Sweep all chrome / box / line-fill code in `arawn-tui` so width measurements use **display columns**, not byte length or naive char count. T-0207 already fixed the worst offender (the tool-call top/bottom border drift), pulling in `unicode-width` along the way. This task extends the same pattern to every other place where chrome width matters — input area cursor positioning, status bar truncation, autocomplete dropdown sizing, modal width, table column widths, etc. — to prevent recurrence.

## Type / Priority

- Tech Debt
- P2 — incremental polish; today's surface mostly works because tool names rarely contain CJK, but the bug class will keep recurring without a sweep.

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

- [ ] **Audit pass:** `grep -rn '\.len()' crates/arawn-tui/src/{render,markdown,modal}.rs` and review every hit. Anything used to compute pixel/cell column positions or padding gets flagged.
- [ ] Replace **`.len()`** with `unicode_width::UnicodeWidthStr::width(...)` everywhere the value drives rendering geometry (column position, fill count, truncation cap, alignment padding).
- [ ] **Locations to fix** (specific from the audit):
  - `render.rs` — input area cursor X position (currently uses `cursor_pos` which is a byte offset into `input_buffer`; the rendered cursor column needs to be display-width of the prefix).
  - `render.rs` — `truncate_for_display` and `truncate_to` already use char count (T-0207 partial fix); upgrade to display width for CJK accuracy.
  - `render.rs` — autocomplete dropdown row truncation (`truncate_to(&cmd.description, dropdown_area.width as usize - 18)` — 18 is a fudge that breaks under unicode names).
  - `render.rs` — sidebar list item rendering (workstream / session names with unicode shouldn't push the right edge).
  - `markdown.rs` — table column-width calculation (`col_widths[i] = col_widths[i].max(cell.chars().count())` — char count is wrong for CJK; CJK chars are 2 columns wide).
  - `modal.rs` — option label width / wrap (currently relies on `Paragraph` wrapping; verify rendered widths align for unicode option labels).
  - `wrap.rs` — verify the existing wrap module already uses display widths (it should, per the wrap-pipeline rewrite). If not, fix.
- [ ] T-0209's collapsed tool-call summary uses display-width cap instead of `chars().count()` (note from T-0209 acceptance).
- [ ] **Test:** add a snapshot covering: tool name with emoji, tool name with CJK, summary with mixed unicode + ASCII. All chrome aligned regardless.
- [ ] **No new dep:** `unicode-width` is already in arawn-tui's `Cargo.toml` from T-0207. No additional crates.

## Implementation Notes

- **Helper to add:** `theme.rs` (or a new `width.rs` helper) gets a small inline fn:
  ```rust
  pub fn width(s: &str) -> usize {
      unicode_width::UnicodeWidthStr::width(s)
  }
  ```
  Imports stay local to call sites; a single `use crate::width::display_width;` per file is enough.
- **Cursor position bug** in input area is the highest-value fix here. Today the cursor lands at the wrong column when the buffer contains any wide char. Snapshot test: `cargo test -p arawn-tui input_buffer` snapshots, but they don't currently exercise unicode buffers.
- **Table column widths** in `markdown.rs` will see real changes for any markdown table containing CJK or emoji cells. Existing snapshots may fail; review for content correctness then accept.
- **Wrap module** already does display-width measurement (see `wrap.rs:88` `unicode_width::UnicodeWidthStr::width`). Audit just to confirm no `.len()` slipped in.
- **Don't touch** the visual hierarchy work (T-0210) or palette (T-0208). Pure correctness pass.

## Out of Scope (defer)

- Grapheme cluster handling (e.g. ZWJ-joined emoji) — `unicode-width` handles single code points well; complex graphemes are an edge case worth its own task if it surfaces.
- Right-to-left text — different beast entirely; not in I-0036's scope.
- Wrapping behavior changes — wrap module is already correct; this task is about chrome width only.

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

### 2026-05-06 — Implementation complete

- Added `crates/arawn-tui/src/width.rs` with `display_width(s)` and `truncate_display(s, max)` — both delegate to `unicode_width::UnicodeWidthStr` and treat the cap as terminal cells, not bytes or chars. The `…` is one cell.
- Replaced the legacy `truncate_for_display` and `truncate_to` bodies in `render.rs` with thin wrappers around `width::truncate_display` so a single source of truth governs every truncation in the TUI.
- T-0209 follow-up: collapsed tool-call summary cap now uses `truncate_display` (display cells) instead of `chars().count()`.
- Tool result preview: `any_line_too_wide` and per-line truncation switched to `display_width` / `truncate_display`.
- Markdown table column widths and word-min-widths now measure in display cells.
- Wrap module's tokenizer (`wrap.rs`) records each token's width as `display_width(&buf)`, not `chars().count()`.
- Input-area cursor X is now derived from `display_width(&input_buffer[..cursor_byte])` in both the scrolled and unscrolled branches. Previously the byte offset was treated as a column count, which placed the cursor in the wrong cell after any non-ASCII char.
- New snapshot test `snapshot_unicode_chrome_alignment` captures a tool call + result with `🔥` in the name and `日本語` in the path. Chrome bars and gutters stay aligned despite the 2-cell-wide chars.
- Updated `truncate_for_display_handles_utf8_at_boundary` test to assert the new `…` (1 cell) and display-width semantics.
- `angreal check workspace`, `angreal check clippy`, and full unit suite all green.

Phase 4 complete.