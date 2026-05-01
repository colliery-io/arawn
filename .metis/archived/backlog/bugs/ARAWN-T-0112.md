---
id: markdown-table-column-widths
level: task
title: "Markdown table column widths ignore content — narrow first column causes character-level word wrapping"
short_code: "ARAWN-T-0112"
created_at: 2026-04-05T19:15:06.489675+00:00
updated_at: 2026-04-05T21:09:31.828069+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Markdown table column widths ignore content — narrow first column causes character-level word wrapping

## Objective

Markdown tables in the TUI render with incorrect column widths. The first column gets ~8 characters wide regardless of content, causing words like "Buffer overflow risk" to wrap character-by-character across 6 lines. The table is unreadable.

## Backlog Item Details

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P1 - High (important for user experience)

### Reproduction
- Session `79668a6f`, message 91 — Safety/Security table with 3 columns (Issue, Details, Mitigation)
- Screenshot captured 2026-04-05 showing "strcpy / unsafe string functions" wrapped to ~6 lines in the Issue column
- Any LLM response with a 3-column markdown table triggers this

### Expected vs Actual
- **Expected**: Column widths proportional to content. Short headers get enough room, long content columns flex.
- **Actual**: First column is ~8 chars wide. Content wraps at character boundaries, not word boundaries. Table takes 5x the vertical space it should.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Table column widths calculated based on content (scan header + row content to determine proportions)
- [ ] Minimum column width respects longest word in the column (no mid-word breaks)
- [ ] Available terminal width distributed proportionally across columns
- [ ] 3-column tables with short/long/medium content render readably
- [ ] Snapshot tests updated for table rendering

## Implementation Notes

### Key file
`crates/arawn-tui/src/markdown.rs` — markdown-to-ratatui rendering

### Approach
1. Parse table rows and measure max content width per column
2. Calculate proportional widths based on content, with a minimum width per column
3. Distribute remaining terminal width to columns with the most content
4. Word-wrap within cells at word boundaries, not character boundaries

### Related
Previously noted in user feedback memory (`feedback_table_wrapping.md`): "Markdown table rendering breaks on wide tables, cells need width-aware wrapping"

## Status Updates

### 2026-04-05 — Complete
- Replaced proportional scaling algorithm with shrink-to-fit: repeatedly caps the widest column until total fits, preserving short columns at natural width
- Minimum column widths now based on longest word per column (not a fixed 4), preventing mid-word breaks
- Min widths capped at `available / ncols` to prevent a single long word from dominating
- Added test `table_wide_content_preserves_short_columns` reproducing the exact bug scenario
- All 94 TUI tests pass including existing snapshots