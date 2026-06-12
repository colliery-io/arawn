---
id: p2-3-ceremony-transactional
level: task
title: "P2-3: Ceremony transactional integrity + force-regenerate"
short_code: "ARAWN-T-0479"
created_at: 2026-06-12T12:02:12.986271+00:00
updated_at: 2026-06-12T12:02:12.986271+00:00
parent: ARAWN-I-0068
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0068
---

# P2-3: Ceremony transactional integrity + force-regenerate

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0068]] — implements P2-3 (HIGH).

## Objective **[REQUIRED]**

Make ceremony tablet writes transactional so a failure can't leave a half-written tablet or orphaned pattern rows, and add force-regenerate so a bad LLM output can be redone without manual row deletion.

**The defect** (`arawn-ceremonies/src/engine.rs:124-133, 154-182, 235-279`): the item-write loop auto-commits per item — if item N+1 fails, items 1..N remain and a partial tablet is presented as real. Pipeline error handling deletes the tablet row but not pattern rows written before the LLM compose call (orphaned cruft). No transaction guards against a concurrent user `patch_item()` (TOCTOU). An `open` tablet can't be force-regenerated.

### Type
- [x] Bug — data integrity in the ceremony pipeline

### Priority
- [x] P2 - Medium (HIGH finding; corrupts quietly over time)

## Acceptance Criteria **[REQUIRED]**

- [ ] detect→compose→write is wrapped in a transaction/savepoint — a failure rolls back ALL of that dispatch's rows (no partial tablet, no orphaned pattern rows).
- [ ] `dispatch` gains a `force` option: when set, an existing open tablet is deleted and regenerated.
- [ ] `DispatchOutcome` returns item counts so partial success is visible to the caller / `/status`.
- [ ] Concurrent `patch_item()` can't be clobbered by an in-flight dispatch (txn or conflict-resolution).
- [ ] Tests: an injected failure mid-item-loop rolls back cleanly; `force` regenerates an open tablet.
- [ ] `angreal check all` and `angreal test unit` green.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Wrap the dispatch write path in a SQLite transaction/savepoint in `arawn-ceremonies/engine.rs`; track inserted pattern-row IDs and roll them back on error (or defer pattern writes until compose succeeds). Add `force` to the dispatch method (delete open tablet first). Return counts in `DispatchOutcome`.

### Dependencies
Pairs with the integration tests in [[ARAWN-T-0483]] for failure-injection coverage. Surfaces partial-success via [[ARAWN-T-0476]].

### Risk Considerations
SQLite `busy_timeout` mitigates contention but not logical corruption — the transaction is what guarantees atomicity. `force` must not delete a tablet the user is mid-review on without intent.

## Status Updates **[REQUIRED]**

*To be added during implementation*