---
id: p2-8-integration-test-suite-over
level: task
title: "P2-8: Integration test suite over the seams (arawn-tests)"
short_code: "ARAWN-T-0483"
created_at: 2026-06-12T12:02:18.573788+00:00
updated_at: 2026-06-12T12:02:18.573788+00:00
parent: ARAWN-I-0068
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0068
---

# P2-8: Integration test suite over the seams (arawn-tests)

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0068]] — implements P2-8 (HIGH). The structural test deliverable that pins every other Phase 2 fix.

## Objective **[REQUIRED]**

Stand up a real integration test suite over the seams in `crates/arawn-tests` (currently one comment, zero tests). Every Phase 2 bug class lives in seams the per-crate unit tests don't reach.

**The gap** (`crates/arawn-tests/src/lib.rs`): no end-to-end coverage of session lifecycle (create → append → load → compact → promote), store atomicity under injected failure (mkdir fails, JSONL write fails mid-promotion), startup ordering (feed runtime / ceremony engine absent), stream interruption mid-tool-call, or JSONL corruption handling (currently skip-with-warn, `jsonl.rs:88-99`).

### Type
- [x] Tech Debt — test infrastructure (pins the data-integrity fixes)

### Priority
- [x] P1 - High (without it, the other Phase 2 fixes regress invisibly)

## Acceptance Criteria **[REQUIRED]**

- [ ] 10–20 integration tests in `arawn-tests` covering: full session lifecycle incl. promotion; store atomicity under injected mkdir/JSONL-write failure; startup with feed-runtime/ceremony-engine absent; stream interruption mid-tool-call; JSONL corruption.
- [ ] Wired into `angreal test integration` (the `--ignored` integration target).
- [ ] Lands **alongside** the data-integrity tasks (T-0479 ceremony txn, T-0480 promotion, T-0481 embedding/memory) — not after — so each fix is pinned as it merges.
- [ ] `angreal check all` green; `angreal test integration` green.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Build the harness in `crates/arawn-tests/tests`: spin up a `Store` + service over a tempdir, drive the seam scenarios, and inject failures (e.g. make a path read-only to fail mkdir, truncate/corrupt a JSONL file, drop a stream mid-tool-call via the mock LLM). Mark slow ones `#[ignore]` under the integration target.

### Dependencies
Co-developed with [[ARAWN-T-0480]] (promotion), [[ARAWN-T-0479]] (ceremony txn), [[ARAWN-T-0481]] (embedding/memory) — their failure-injection tests live here. The mock-LLM stream-interruption path exercises the T-0468 work already landed.

### Risk Considerations
Failure injection is the hard part (portably making mkdir/JSONL-write fail). Keep tests deterministic and not dependent on the sandbox (the shell-tool sandbox race was just fixed; don't reintroduce flakiness).

## Status Updates **[REQUIRED]**

*To be added during implementation*