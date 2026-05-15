---
---
id: gather-compose-write-pipeline-with
level: task
title: "Gather→compose→write pipeline with two-write-path citation enforcement"
short_code: "ARAWN-T-0282"
created_at: 2026-05-15T23:45:06.978670+00:00
updated_at: 2026-05-15T23:45:06.978670+00:00
parent: ARAWN-I-0043
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0043
---

# Gather→compose→write pipeline with citation enforcement

## Goal
The pipeline that drives one ceremony run: ask the plugin for facts, optionally detect patterns, ask the plugin to compose items, transactionally write tablet + sections + items. Citation enforcement is the load-bearing part.

## Reference
I-0043 Design Decisions §4 (two write paths) + Compose Chain section.

## Acceptance
- `CeremonyCtx` exposes:
  - `write_composed_item(item: ComposedItem) -> Result<()>` — strict path, requires `citation_id`; refuses with `CeremonyError::MissingCitation` if absent.
  - `write_user_item(item: UserItem) -> Result<()>` — permissive path, no citation required. Used for `kind=freeform` diary entries and user-added todos.
  - `write_pattern_row(p: DetectedPattern) -> Result<String>` — returns the pattern row id callers pass as `citation_id` on dependent composed items.
- Whole-run transaction: tablet + sections + items + patterns commit together or roll back together.
- `arawn_llm::gate::acquire_local()` wrap around the plugin's compose phase.
- Token usage records emitted with `call_site = "ceremony.<kind>.compose"`.
- Tests:
  - composed item missing citation → returns error, no row written.
  - composed item with citation → row written.
  - user item without citation → row written.
  - mid-run failure rolls back the whole tablet.

## Out of scope
The pattern detector framework itself (T-0286) — this task wires the call site, the framework is implemented separately.

## Notes
This is the spec for the engine's contract with plugins. Get the API right; everything downstream depends on it.
