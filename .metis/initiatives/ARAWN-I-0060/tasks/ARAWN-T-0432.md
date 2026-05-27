---
id: t-c-demote-active-lens-shim-to
level: task
title: "T-C: Demote active-lens shim to write-target only"
short_code: "ARAWN-T-0432"
created_at: 2026-05-27T02:35:55.610358+00:00
updated_at: 2026-05-27T02:35:55.610358+00:00
parent: ARAWN-I-0060
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0060
---

# T-C: Demote active-lens shim to write-target only

## Parent Initiative

[[ARAWN-I-0060]]

## Objective

Keep a "current lens" but reframe it as the **write target only** — where new
learnings file — now that reads roam (T-B). Reads stop consulting it; the
extractor and memory writes use it.

## Type
Tech Debt / refactor — `arawn-engine` (`SessionLens`, `LensMemoryRouter`),
`arawn` service wiring, `arawn-extractor` write path.

## Technical Approach

- `SessionLens` survives as the **write-target** handle. Audit every reader of
  `LensMemoryRouter.current()` / `SessionLens.current()`:
  - **read sites** (search/query) → already moved off it in T-B; delete the
    dependency.
  - **write sites** (extractor entity writes, `memory` store-for-lens, promote)
    → keep, sourced from the write target.
- `/lens switch` + `lens_switch` set the write target (persisted on the session
  as today via `lens_name`); semantics reframed in T-E.
- Session still records its write-target lens (`Session.lens_name`/`lens_id`
  unchanged on disk) — no schema change.

## Acceptance Criteria

- [ ] No read/query path consults the active-lens shim; only write/extraction
      paths do.
- [ ] New learnings in a session file into the current write-target lens (default
      `scratch`), unchanged from today's *write* behavior.
- [ ] `/lens switch` changes the write target, not a read scope.
- [ ] `angreal check workspace` + tests pass; extraction-into-lens still works
      (existing extractor tests/fixtures green).

## Dependencies
Depends on [[ARAWN-T-0431]] (reads must be off the shim first). Pairs with
[[ARAWN-T-0433]] (persona) and [[ARAWN-T-0434]] (CLI reframe).

## Risk Considerations
- The shim is threaded widely — enumerate all callers and classify read vs write
  before deleting any.

## Status Updates

*To be added during implementation*
