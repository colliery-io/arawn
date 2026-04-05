---
id: notebookedit-tool-jupyter-notebook
level: task
title: "NotebookEdit tool — Jupyter notebook cell editing"
short_code: "ARAWN-T-0056"
created_at: 2026-04-03T01:01:38.553713+00:00
updated_at: 2026-04-03T01:01:38.553713+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


exit_criteria_met: false
initiative_id: NULL
---

# NotebookEdit tool — Jupyter notebook cell editing

## Objective

Add a tool for editing Jupyter notebook (.ipynb) cells — replace cell contents, insert new cells, delete cells. Notebooks are JSON files with a specific schema; this tool provides structured cell-level editing rather than raw JSON manipulation.

### Type: Feature | Priority: P3

- **User Value**: Data scientists and ML engineers can work with notebooks without leaving the agent.
- **Effort Estimate**: S

## Acceptance Criteria

## Acceptance Criteria

- [ ] Replace cell contents by cell_number (0-indexed)
- [ ] Insert new cell at position (edit_mode=insert)
- [ ] Delete cell at position (edit_mode=delete)
- [ ] Preserves notebook metadata and output cells
- [ ] notebook_path must be absolute

## Implementation Notes

- Parse .ipynb as JSON, manipulate `cells` array, write back
- Preserve `metadata`, `nbformat`, `nbformat_minor` fields
- Cell types: code, markdown, raw
- Reference: Claude Code's `NotebookEditTool/`