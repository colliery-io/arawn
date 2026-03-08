---
id: add-memory-store-concurrency-tests
level: task
title: "Add memory store concurrency tests"
short_code: "ARAWN-T-0298"
created_at: 2026-03-08T20:21:20.342864+00:00
updated_at: 2026-03-08T20:21:20.342864+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#tech-debt"


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

*To be added during implementation*