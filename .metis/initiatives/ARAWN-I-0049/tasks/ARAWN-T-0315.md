---
id: rationale-preservation-on-weekly
level: task
title: "Rationale preservation on weekly confirm"
short_code: "ARAWN-T-0315"
created_at: 2026-05-16T22:52:36.907863+00:00
updated_at: 2026-05-17T12:42:33.605883+00:00
parent: ARAWN-I-0049
blocked_by: [ARAWN-T-0313]
archived: false

tags:
  - "#task"
  - "#phase/completed"


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

- [x] `confirm_priority` extracts `body.rationale` (JSON-embedded
      on the candidate ceremony_items row) and writes it to
      `todos.rationale`. Empty / whitespace / missing rationale
      coerces to NULL — canonical "no rationale" state.
- [x] Source of the rationale: the candidate `ceremony_items` row's
      `body` JSON, which now carries `{text, rationale}` per the
      updated weekly compose SYSTEM_PROMPT.
- [x] `add_priority` (user-write path) accepts an explicit rationale
      arg and writes it to `todos.rationale`; no synthesis.
- [x] `/week` already JOINs ceremony_priorities → todos for
      list_priorities — `PriorityDto.rationale` now reflects the
      preserved value end-to-end.
- [x] Two new unit tests: one confirms body.rationale propagates
      into `todos.rationale`; one confirms a body without rationale
      leaves the column NULL (not "").

## Status Updates

### 2026-05-17 — shipped

- Updated weekly SYSTEM_PROMPT to require `body: {text, rationale}`
  on priority candidates. Non-priority sections remain optional.
- `confirm_priority` now parses body_str as JSON, extracts the
  `rationale` field as `Option<String>`, filters out empty/
  whitespace, and writes it via `params![..., &rationale, ...]`.
  rusqlite's `Option<&str>` binding produces NULL on `None`.
- `PriorityDto.rationale` populated from the extracted value
  (`unwrap_or_default()` keeps the public type stable as `String`).
- Workspace 1806/0 — two new confirm_priority tests added.

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