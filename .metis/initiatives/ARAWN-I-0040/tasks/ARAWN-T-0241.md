---
id: memory-test-suite-longmemeval
level: task
title: "Memory test suite + LongMemEval bench green on graphqlite backend"
short_code: "ARAWN-T-0241"
created_at: 2026-05-12T01:33:04.344914+00:00
updated_at: 2026-05-12T01:33:04.344914+00:00
parent: ARAWN-I-0040
blocked_by: ["ARAWN-T-0240"]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0040
---

# Memory test suite + LongMemEval bench green on graphqlite backend

## Parent Initiative

[[ARAWN-I-0040]]

## Objective

Quality gate for Phase 1. The full arawn-memory test suite passes on the graphqlite backend, and the LongMemEval bench scores at parity (or better) with the pre-migration baseline. Engine-level integration tests that exercise memory through tools (`memory_store`, `memory_search`, auto-memory) pass unchanged.

Phase 1 only exits when this task is green.

## Scope

- Run the full `crates/arawn-memory/tests/*.rs` suite. Fix any regressions surfaced by the migration. Most should be storage-implementation-detail issues (e.g. SQL assertions in tests that need to become Cypher assertions, or schema-shape assertions that need updating).
- Run `crates/arawn-memory/tests/longmemeval_bench.rs`. Compare the score to the pre-migration baseline. Document any delta (positive or negative). Investigate any regression > 2%.
- Run `crates/arawn-memory/tests/recall_eval.rs`. Same parity expectation.
- Run engine-level tests that exercise memory: `crates/arawn-tests/tests/memory_tools.rs` and `crates/arawn-tests/tests/memory_stack.rs`. These should pass without modification — the public API stayed stable.
- `angreal test all` clean (full workspace test run).
- `angreal check workspace` + `angreal check clippy` clean.

### Baseline capture

Before this task begins (or as its first step), capture the current LongMemEval + recall_eval scores on the pre-migration code. Store as a one-line baseline note in this task's status updates. Without that baseline, "parity" is unverifiable.

### Behavior parity, not implementation parity

Tests that assert the *storage shape* (e.g. "there's a row in the `entities` table with these columns") need to be rewritten as behavioral assertions ("after inserting this entity, `get_entity(id)` returns it with these fields"). The point of the public API stability is that behavioral tests are the right level; implementation-detail tests should never have existed and won't survive the migration.

### What's deferred

- Phase 2 (projections) and beyond.
- Engine-level / TUI smoke tests that aren't directly memory-related.

## Acceptance Criteria

- [ ] LongMemEval bench score recorded for both pre-migration and post-migration runs in the status updates. Delta < 2% (or, if better, that's fine).
- [ ] `recall_eval.rs` passes; delta documented if any.
- [ ] All unit + integration tests in `crates/arawn-memory/tests/` pass.
- [ ] `crates/arawn-tests/tests/memory_*.rs` pass without source changes (the public API is stable).
- [ ] `angreal test all` clean.
- [ ] `angreal check workspace` + `angreal check clippy` clean.
- [ ] Phase 1 marked complete on I-0040; the initiative is unblocked to move into Phase 2 (projections).

## Implementation Notes

### Technical approach

1. Capture baseline on the pre-migration commit (or by `git stash`-ing the in-progress migration and running the bench).
2. Run the full memory test suite. Categorize each failure: implementation-detail assertion (fix the test), real behavior regression (fix the code), or test-infrastructure issue (e.g. fixture relies on a SQL schema that doesn't exist anymore).
3. Run LongMemEval; compare scores. If a regression > 2%, investigate top-failing scenarios and fix in T-0239 or T-0240 before closing this task.
4. Run engine-level tests; they should pass unchanged.

### Dependencies

- T-0240 (search + dedup paths working on the new backend).

### Risk considerations

- **Hidden bench-mode regressions.** LongMemEval exercises long-horizon retrieval; a small change to the scoring path or rerank logic can shift scores in ways unit tests don't catch. Mitigation: capture the baseline before *any* migration work touches scoring code; compare carefully; spend real time on a regression before declaring done.
- **Flaky bench.** If the bench is non-deterministic (e.g. uses an LLM), run multiple times and compare distributions, not single scores.

## Status Updates

*To be added during implementation*
