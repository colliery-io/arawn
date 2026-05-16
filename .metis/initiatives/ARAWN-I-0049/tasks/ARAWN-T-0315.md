---
id: rationale-preservation-on-weekly
level: task
title: "Rationale preservation on weekly confirm"
short_code: "ARAWN-T-0315"
created_at: 2026-05-16T22:52:36.907863+00:00
updated_at: 2026-05-16T22:52:36.907863+00:00
parent: ARAWN-I-0049
blocked_by: [ARAWN-T-0313]
effort: S
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0049
---

# Rationale preservation on weekly confirm

## Parent Initiative

[[ARAWN-I-0049]]

## Objective

The weekly ceremony's confirm flow currently discards the LLM
rationale that accompanied each candidate priority. With
`todos.rationale` now available, preserve it.

## Acceptance Criteria

- [ ] `weekly_confirm_priority` tool (post-[[ARAWN-T-0313]]
      wrapper) copies the candidate item's rationale into
      `todos.rationale` rather than writing `""`.
- [ ] Source of the rationale: the `ceremony_items` row that the
      candidate priority was promoted from. Read its
      `rationale` column (or equivalent metadata field) and
      propagate.
- [ ] If a candidate has no rationale (manual add via
      `weekly_add_priority`), leave `todos.rationale` NULL —
      don't synthesise.
- [ ] `/week` render shows the rationale on confirmed priorities
      (extends [[ARAWN-T-0307]]'s renderer).
- [ ] Unit test: confirm a candidate with non-empty rationale →
      `todos.rationale` is the same string.

## Implementation Notes

### Technical Approach

- Source field today: weekly plugin emits items with
  `kind='pattern'` and stashes the rationale on the item row.
  Wrapper reads the source `ceremony_items` row, then calls
  `TodoService::create` with the rationale populated.

### Dependencies

- Blocked by [[ARAWN-T-0313]] (wrapper must exist before it
  can preserve anything).

### Risk Considerations

- Backfilled rows: [[ARAWN-T-0311]]'s backfill copies whatever
  was in `ceremony_priorities.rationale` (often "") into
  `todos.rationale`. This task fixes the forward path; historical
  empty rationales stay empty.

## Status Updates

*To be added during implementation*
