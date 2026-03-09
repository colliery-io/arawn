---
id: add-memory-store-concurrency-tests
level: task
title: "Add memory store concurrency tests"
short_code: "ARAWN-T-0298"
created_at: 2026-03-08T20:21:20.342864+00:00
updated_at: 2026-03-09T01:22:18.001301+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Add memory store concurrency tests

## Objective

No concurrent read/write tests exist on the memory store. Add tests that verify thread-safety under concurrent access — multiple tasks reading and writing simultaneously.

## Backlog Item Details

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P2 - Medium (nice to have)

### Technical Debt Impact
- **Current Problems**: Memory store has 152 tests but all are sequential. Race conditions or deadlocks under concurrent access would be invisible.
- **Benefits of Fixing**: Confidence in thread-safety of the memory store under real-world concurrent access.
- **Risk Assessment**: Medium — memory store is accessed from multiple async tasks concurrently.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Test concurrent writes from multiple tasks don't lose data
- [ ] Test concurrent reads during writes return consistent state
- [ ] Test concurrent delete + read doesn't panic
- [ ] Test concurrent search operations return valid results
- [ ] `cargo test -p arawn-memory` passes
- [ ] `angreal check clippy` clean

## Implementation Notes

### Technical Approach
- Use `tokio::spawn` to create concurrent tasks hitting the memory store
- Use barriers/latches to synchronize starts for maximum contention
- Run each test multiple times (or use a loop) to increase chance of hitting races
- Verify final state is consistent after all tasks complete

### Files
- `crates/arawn-memory/src/` (add to existing test modules)

## Status Updates

### Implementation Complete

**File modified:** `crates/arawn-memory/src/store/mod.rs` — added `concurrency_tests` module with 6 tests

**Tests added:**
1. `test_concurrent_writes_no_data_loss` — 8 threads × 20 writes via barrier, verify total count = 160
2. `test_concurrent_reads_during_writes` — 4 writers + 4 readers simultaneously, readers always see ≥ seed data
3. `test_concurrent_delete_and_read_no_panic` — concurrent delete + get on same IDs, no panics, results are Ok
4. `test_concurrent_search_returns_valid_results` — 6 threads searching "alpha"/"beta" concurrently, results are correct
5. `test_concurrent_note_writes_no_data_loss` — 6 threads × 15 note inserts, verify total = 90
6. `test_concurrent_mixed_operations_no_deadlock` — 4 threads doing inserts/lists/stats simultaneously, no deadlock, final state consistent

All 148 tests pass (`cargo test -p arawn-memory`). Clippy clean.