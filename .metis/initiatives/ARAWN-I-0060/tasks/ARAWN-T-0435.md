---
id: t-f-tui-file-into-x-framing-cross
level: task
title: "T-F: TUI — file-into-X framing + cross-lens hit provenance"
short_code: "ARAWN-T-0435"
created_at: 2026-05-27T02:35:59.477701+00:00
updated_at: 2026-05-27T02:35:59.477701+00:00
parent: ARAWN-I-0060
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0060
---

# T-F: TUI — file-into-X framing + cross-lens hit provenance

## Parent Initiative

[[ARAWN-I-0060]]

## Objective

Update the TUI so the sidebar/status reads as "filing into <lens>" (the write
target) rather than "scoped to <lens>", and so search results surface which lens
each hit came from.

## Type
Feature — `arawn-tui` (`app/mod.rs` `current_lens`, sidebar render, status line,
wherever search hits render).

## Scope

- Relabel the active-lens indicator: `current_lens` → present as the **write
  target** ("filing into work"), not the read scope.
- Where cross-lens search results render, show each hit's source-lens label
  (provenance) so the user sees the chat is reading across lenses.
- Reflect the reframed `/lens switch` (write target) in any inline help/toasts.

## Acceptance Criteria

- [ ] Sidebar/status shows the write-target framing, not "scoped to".
- [ ] Search-result rendering attributes hits to their source lens.
- [ ] `arawn-tui` snapshot tests updated to the new labels; `angreal check
      workspace` + tui tests pass.

## Dependencies
Depends on [[ARAWN-T-0431]] (hits carry source lens) and [[ARAWN-T-0432]]
(write-target semantics). Last task of the initiative.

## Status Updates

*To be added during implementation*
