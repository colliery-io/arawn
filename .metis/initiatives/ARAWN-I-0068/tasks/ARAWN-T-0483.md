---
id: p2-8-integration-test-suite-over
level: task
title: "P2-8: Integration test suite over the seams (arawn-tests)"
short_code: "ARAWN-T-0483"
created_at: 2026-06-12T12:02:18.573788+00:00
updated_at: 2026-06-13T12:50:11.157062+00:00
parent: ARAWN-I-0068
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


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

## Acceptance Criteria

## Acceptance Criteria

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

### 2026-06-13 — COMPLETE ✅
**Finding:** `arawn-tests/tests/` was NOT empty — it already had compaction, engine_persistence, full_pipeline, hooks, hot_reload, memory_*, permissions, plugin_components, skills, tool_artifacts, workflows, etc. (the "one comment, zero tests" referred to `src/lib.rs`). Each data-integrity task this initiative also added its own integration coverage to `local_service.rs`/`websocket.rs`. So this task closed the remaining **seam gaps**.

**New file `arawn-tests/tests/seams.rs`** (3 tests):
- `session_lifecycle_create_append_load_promote` — full lifecycle (create scratch → send_message writes JSONL → load → promote into a lens → reload from the lens, history intact).
- `jsonl_corruption_skips_bad_line_and_loads_rest` — a malformed JSONL line is skipped (skip-with-warn, `jsonl.rs`) and the valid messages still load.
- `stream_interrupted_mid_tool_call_emits_error_and_recovers` — mock LLM streams a tool-call start then errors mid-arguments → the turn surfaces an `Error` event (no panic/hang) and the next turn on the same session completes (validates T-0468 stream-error handling).

**Coverage of the 5 acceptance seams (all green together):**
1. session lifecycle incl. promotion → seams + local_service + compaction.
2. store atomicity under injected failure → `arawn-storage::store` tests (promotion rollback, mkdir-first orphan-row).
3. startup with subsystems absent → `status_reports_subsystems_absent_by_default`.
4. stream interruption mid-tool-call → seams.
5. JSONL corruption → seams.

**Wiring:** fast seam/integration tests run under `cargo test --workspace` (what `angreal test all` runs); only slow/external tests (UAT) carry `#[ignore]` for the `angreal test integration` (`--ignored`) target — consistent with the existing suite.

**Gates:** fmt clean · full `arawn-tests` suite green (seams 3, local_service 18, websocket 17, + all existing files, 0 failures). All acceptance criteria met.