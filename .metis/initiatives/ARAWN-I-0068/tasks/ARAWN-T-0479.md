---
id: p2-3-ceremony-transactional
level: task
title: "P2-3: Ceremony transactional integrity + force-regenerate"
short_code: "ARAWN-T-0479"
created_at: 2026-06-12T12:02:12.986271+00:00
updated_at: 2026-06-13T11:57:01.659106+00:00
parent: ARAWN-I-0068
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


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

## Acceptance Criteria

## Acceptance Criteria

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

### 2026-06-13 — COMPLETE ✅
**Key constraint:** the per-item auto-commit existed *on purpose* — an earlier whole-pipeline `BEGIN IMMEDIATE…COMMIT` held the `arawn.db` write lock across the multi-second LLM compose and starved `create_session` (UAT busy_timeout). So the fix can't just wrap everything in a txn.

**Solution — transaction spans only the post-compose write phase:**
- `engine.rs run_pipeline`: gather (reads) → detect patterns (collect in memory, **pre-assign ids** for citation, DON'T write) → compose (LLM, no lock) → **ONE transaction** writing tablet + deferred pattern rows + all items, commit/rollback. Pattern events emitted only after commit. Returns `(tablet_id, item_count)`.
- Converted `insert_tablet`/`write_composed_item`/`write_user_item` to take `&Connection` (a `&Transaction` derefs) + new `write_pattern_row_tx`. Lock acquired once around the write phase, never across compose.
- A mid-write failure now rolls back the WHOLE dispatch — no half tablet, **no orphaned pattern rows** (the old bug: patterns were written before compose).

**force-regenerate:** new trait method `dispatch_with(kind, target, force)` (default delegates to `dispatch_for`); `dispatch`/`dispatch_for` route through it. On an `open` tablet + force → `delete_tablet` then regenerate. Reviewed/archived tablets are still never overwritten.
- **FK gotcha:** the connection doesn't enable `PRAGMA foreign_keys`, so the schema's `ON DELETE CASCADE` never fires. Rewrote `delete_tablet` to hand-cascade (items/priorities/diary/sections/tablet) in a txn. (`ceremony_todos_rolling` is a view since V9, not deleted.)

**Item counts:** `DispatchOutcome::Generated` gained `item_count: usize`. Updated all construct/match sites across arawn-ceremonies (engine/runner/backfill/tests) + arawn-engine tools (daily/weekly/ceremony) + arawn ws_server.

**Concurrent patch_item (TOCTOU):** addressed by construction — all ceremony writes share one `ConnHandle` Mutex, so a dispatch's write txn and a `patch_item` are serialized (never truly concurrent); a normal dispatch skips an existing open tablet (no clobber) and force deletes-then-recreates atomically.

**Tests:** `failed_dispatch_rolls_back_pattern_rows` (tablet+items+patterns all 0 after a mid-loop failure), `force_regenerates_open_tablet` (skip without force; regenerate to exactly 1 tablet/1 item with force). Existing `composed_item_missing_citation_rolls_back_whole_run` still green via the real txn.

**Gates:** fmt + clippy -D warnings clean · arawn-ceremonies lib (149) · arawn-engine lib (733) · retro_uat (2) all green. All acceptance criteria met.