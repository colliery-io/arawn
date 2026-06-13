---
id: p3-2-display-width-aware-text
level: task
title: "P3-2: Display-width-aware text wrapping + narrow-terminal clamps (TUI)"
short_code: "ARAWN-T-0485"
created_at: 2026-06-13T14:43:27.220448+00:00
updated_at: 2026-06-13T14:55:33.599912+00:00
parent: ARAWN-I-0069
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0069
---

# P3-2: Display-width-aware text wrapping + narrow-terminal clamps (TUI)

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0069]] — implements P3-2 (MEDIUM, TUI correctness).

## Objective **[REQUIRED]**

Make TUI text wrapping measure **display cells**, not codepoints, so CJK/emoji (2-cell glyphs) wrap correctly instead of overflowing or breaking mid-glyph — the same width-vs-codepoint bug class as the already-fixed table wrapping, which didn't reach these paths. Also clamp the autocomplete dropdown to the terminal width.

**The defect** (`crates/arawn-tui/src/markdown.rs:573-615` esp. 583/594; `render/input.rs:87-96`): `wrap_text()` uses `chars().count()` for word width and `take(width)` for hard breaks; the `/`-command dropdown clamps to a 20-cell minimum and overflows terminals narrower than that.

### Type
- [x] Bug — TUI rendering correctness

### Priority
- [x] P2 - Medium

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] `wrap_text()` (word-wrap + hard-break paths) measures width via `unicode-width` display cells, not `chars().count()`/`take()` — a 2-cell glyph never splits and a wrapped line never exceeds the target width.
- [ ] The autocomplete dropdown clamps to the actual terminal width (no overflow on narrow terminals).
- [ ] Table-driven inline tests over ASCII / CJK / emoji / mixed content (wrap width respected; no mid-glyph break).
- [ ] `angreal check all` + `angreal test unit` green; existing TUI snapshot tests still pass (or updated intentionally).

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
`unicode-width` is already a dependency (the table-wrap fix used it — see `width.rs`/`wrap.rs`). Route `wrap_text` and the input/dropdown width math through the same display-cell measurement helper. Reuse existing width utilities rather than re-deriving.

### Dependencies
None beyond the existing `unicode-width` helpers. Pure TUI-crate change.

### Risk Considerations
Snapshot tests may shift — verify diffs are correct (better wrapping), not regressions. Keep the helper consistent with the existing table-wrap path so behavior is uniform.

## Status Updates **[REQUIRED]**

**2026-06-13 — Implemented & complete.**
- Added `width::char_width(char) -> usize` (via `unicode-width`'s `UnicodeWidthChar`) — the per-glyph counterpart to the existing `display_width(&str)`.
- Rewrote `markdown::wrap_text()` to measure words via `display_width()` and extracted a `hard_break_word()` helper that chunks by display cells, so a 2-cell CJK/emoji glyph never straddles a wrap boundary and no wrapped line exceeds `width` cells. Replaced the old `chars().count()` / `take(width)` codepoint logic in both the word-wrap and hard-break paths.
- Fixed the autocomplete dropdown (`render/input.rs`): the old `input_area.width.clamp(20, 50)` forced a 20-cell *minimum* that overflowed terminals narrower than ~22 cols. Now `avail = (width - x_offset).max(1)` then `.min(50)` — never exceeds the terminal. Also guarded the description-truncation math with `saturating_sub(18)` (was raw `- 18`, an underflow panic risk on narrow terminals).
- Tests: table-driven `wrap_text_respects_display_width_across_scripts` (ASCII/CJK/emoji/mixed/oversize-word), `wrap_text_never_splits_a_two_cell_glyph` (width=1 → one whole glyph per line, no drops/dupes), `wrap_text_zero_width_is_single_empty_line`.
- `cargo test -p arawn-tui --lib`: 259 passed (incl. existing snapshot tests — no shift). `cargo clippy -p arawn-tui -- -D warnings` (the gate's invocation, no `--all-targets`): clean. fmt clean.
- Note: 4 pre-existing `--all-targets` clippy lints exist in unrelated test-only code (`snapshot_tests.rs:10`, `command.rs:1382`, `render/mod.rs:293/314`) — toolchain drift, not in scope for this task and not hit by `angreal check all`.