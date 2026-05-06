---
id: phase-1-catppuccin-mocha-palette
level: task
title: "Phase 1 — Catppuccin Mocha palette + theme.rs wiring"
short_code: "ARAWN-T-0208"
created_at: 2026-05-06T11:21:43.322005+00:00
updated_at: 2026-05-06T11:21:43.322005+00:00
parent: ARAWN-I-0036
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0036
---

# Phase 1 — Catppuccin Mocha palette + theme.rs wiring

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0036]]

## Objective

Wire `theme.rs` into actual call sites and replace the ad-hoc color landscape with a coherent **Catppuccin Mocha** palette. This is I-0036 Phase 1 — the foundational change that unlocks every later phase. Today `theme.rs` exists but is decorative; `render.rs` and `markdown.rs` hardcode `Color::Cyan`, `Color::Rgb(100,100,115)`, etc. as literals. Until callers import from the theme, "centralized theme" is a lie.

## Type / Priority

- Tech Debt + Feature
- P1 — blocks every later phase of I-0036; biggest single perception shift in the visual coherence pass.

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

- [ ] `crates/arawn-tui/src/theme.rs` rewritten to expose Catppuccin Mocha as a named palette:
  - 12-step gray scale (`base`, `mantle`, `crust`, `surface0..2`, `overlay0..2`, `subtext0..1`, `text`).
  - Accent + status colors (`mauve`, `blue`, `lavender`, `red`, `green`, `yellow`, `peach`, `teal`, `flamingo`, `pink`).
  - Semantic aliases mapped on top: `accent` → mauve, `success` → green, `warning` → yellow, `danger` → red, `border` → surface1, `border_focused` → mauve, `fg_muted` → overlay2, `fg` → text, `fg_strong` → text + bold modifier role, `bg` → base, `bg_muted` → mantle, `bg_subtle` → crust, `code_bg` → mantle, `code_fg` → text.
- [ ] All hard-RGB values are baked in once at the top of `theme.rs` (Catppuccin Mocha hexes converted to `Rgb(r,g,b)`); no runtime dep added.
- [ ] Every `Color::Rgb(...)`, `Color::DarkGray`, `Color::Cyan`, etc. literal in `render.rs`, `markdown.rs`, `modal.rs` is replaced with a `theme::*` reference. Audit: `grep -n "Color::" crates/arawn-tui/src/{render,markdown,modal}.rs` returns zero callsite hits (only re-exports / type definitions remain).
- [ ] Each existing semantic constant (`USER`, `ASSISTANT`, `TOOL_NAME`, `CHROME`, `RESULT_TEXT`, etc.) maps to a Catppuccin color or a Catppuccin-derived modifier:
  - `USER` (was Green) → `green`
  - `ASSISTANT` → `text`
  - `SYSTEM` → `mauve`
  - `ERROR` → `red`
  - `TOOL_NAME` → `lavender` (interactive-but-not-focused)
  - `GENERATING` → `yellow`
  - `SUCCESS` → `green`
  - `CHROME` → `surface1`
  - `RESULT_TEXT` → `subtext0`
  - `RESULT_LABEL` → `overlay2`
  - `INLINE_CODE_FG` / `CODE_FG` → `peach` / `text`
  - others map by feel; document the mapping in `theme.rs` doc comments.
- [ ] One worked example: `STATUS_BAR_BG` already wired in T-0207 stays consistent (`bg_muted` = mantle ≈ Rgb(24,24,37)).
- [ ] Snapshot tests re-baselined via `cargo insta accept`. Diffs reviewed once for sanity (no missing content; only color shifts).
- [ ] `accent` (mauve) is used **only** for `border_focused` and the input-focus indicator — no aliasing to tool names, headings, links, etc. (Heading color reassignments happen in Phase 3 / T-0210; in this task headings stay using their existing constants which now resolve through the new palette.)

## Implementation Notes

- **Catppuccin Mocha hexes** to bake in (sourced from `https://catppuccin.com/palette/`):
  - `base`: `#1e1e2e` (Rgb 30, 30, 46)
  - `mantle`: `#181825` (Rgb 24, 24, 37)
  - `crust`: `#11111b` (Rgb 17, 17, 27)
  - `surface0`: `#313244`, `surface1`: `#45475a`, `surface2`: `#585b70`
  - `overlay0`: `#6c7086`, `overlay1`: `#7f849c`, `overlay2`: `#9399b2`
  - `subtext0`: `#a6adc8`, `subtext1`: `#bac2de`
  - `text`: `#cdd6f4`
  - `lavender`: `#b4befe`, `blue`: `#89b4fa`, `sapphire`: `#74c7ec`, `sky`: `#89dceb`, `teal`: `#94e2d5`
  - `green`: `#a6e3a1`, `yellow`: `#f9e2af`, `peach`: `#fab387`, `maroon`: `#eba0ac`, `red`: `#f38ba8`, `mauve`: `#cba6f7`, `pink`: `#f5c2e7`, `flamingo`: `#f2cdcd`, `rosewater`: `#f5e0dc`
- **Semantic naming layer** sits on top of the palette so phase 3 + 4 can reassign meanings without touching the palette itself. E.g. `pub const ASSISTANT: Color = TEXT` rather than `pub const ASSISTANT: Color = Color::Rgb(0xCD, 0xD6, 0xF4)`.
- **Bg-muted vs bg** distinction matters once the dashboard surfaces from I-0035 land — sidebar/status/modal use `bg_muted`, chat content area uses `bg` (terminal default).
- **Don't touch** the `unicode-width` measurement work yet (that's Phase 4 / T-0211) or the tool-call collapse (Phase 2 / T-0209). This task is pure palette + wiring.
- **Snapshot rebaseline:** expect every styled snapshot to fail. The diff should be color-only, not content. Use `cargo insta review` (interactive) on the first pass to spot any actual content drift, then `cargo insta accept` once clean.
- **`syntect` code highlighting** uses its own theme (`base16-eighties.dark` currently, per `markdown.rs`). Don't try to retheme syntect to Catppuccin in this task — it has a Catppuccin theme available but switching it is its own concern; bookmark for a follow-up. Do verify the syntect bg matches our new `code_bg` (mantle) so syntax-highlighted blocks blend with arawn's chrome.

## Out of Scope (defer)

- Tool-call collapsed rendering — T-0209 (Phase 2).
- Heading hierarchy / inline code drop-the-backticks / sidebar tab strip / assistant gutter — T-0210 (Phase 3).
- Display-width measurement — T-0211 (Phase 4).
- Empty-state hero / OAuth heartbeat / thinner streaming cursor — T-0212 (Phase 5).
- Light theme (Catppuccin Latte) — explicitly deferred per I-0036 non-goals.
- syntect theme swap — separate small follow-up.

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