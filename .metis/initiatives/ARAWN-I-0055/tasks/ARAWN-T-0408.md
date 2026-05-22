---
id: t-d-promote-workstream-memory-always-on
level: task
title: "T-D: Promote Workstream + Memory to always-on"
short_code: "ARAWN-T-0408"
created_at: 2026-05-22T16:36:03.000000+00:00
updated_at: 2026-05-22T16:36:03.000000+00:00
parent: ARAWN-I-0055
blocked_by: [ARAWN-T-0406]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0055
---

# T-D: Promote Workstream + Memory to always-on

## Parent Initiative

[[ARAWN-I-0055]]

## Objective

Move `ToolCategory::Workstream` and `ToolCategory::Memory` into the always-on set in `filter_tools_for_context` — same treatment as `Core` and `Utility`. The agent should never lose access to context-switching (workstream_*) or recall (memory_*) mid-turn.

## Acceptance Criteria

- [ ] `filter_tools_for_context` inserts `ToolCategory::Workstream` and `ToolCategory::Memory` into the active set unconditionally at the top of the function, alongside `Core` and `Utility`.
- [ ] The keyword branches that previously gated these categories are removed.
- [ ] Unit tests:
  - [ ] `workstream_tools_visible_with_empty_user_message`: build session with messages.len() > 2, user message = "x" (no keywords). Assert `workstream_switch` is in the filtered list.
  - [ ] `memory_tools_visible_with_empty_user_message`: same setup. Assert `memory_recall` is in the filtered list.
- [ ] `cargo test --workspace --lib` green.

## Implementation Notes

- Rationale: both categories surface "ambient" capabilities — small surface area, high frequency of legitimate use, no semantic reason to gate. The keyword filter for these categories has been a source of latent bugs (the agent intermittently can't switch workstream because the user didn't say "workstream").
- Token cost: workstream_* tools are ~5 tools, memory_* tools are ~3 tools. ~2K tokens added unconditionally. Acceptable on a 32K budget (still leaves 5-7K for selectable categories).

## Status Updates

*To be added during implementation*
