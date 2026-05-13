---
id: extractor-integration-tests
level: task
title: "Extractor integration tests — fixture projections + mock LLM"
short_code: "ARAWN-T-0254"
created_at: 2026-05-13T01:28:15.413201+00:00
updated_at: 2026-05-13T03:41:04.986612+00:00
parent: ARAWN-I-0040
blocked_by: [ARAWN-T-0252, ARAWN-T-0253]
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0040
---

# Extractor integration tests — fixture projections + mock LLM

## Parent Initiative

[[ARAWN-I-0040]]

## Objective

End-to-end tests for the Phase-4 extractor: feed fixture → projection rows → CoT chain (mock LLM) → workstream KB. Verifies the dispatch trigger fires, the chain stages compose correctly, link-by-name resolves through FTS, and backfill walks pre-existing rows.

A mock LLM client returns scripted responses keyed by prompt content. No real network calls in this test suite — that's UAT scope (Phase 7).

## Scope

### Mock LLM

A `MockLlm` in `crates/arawn-extractor/tests/mock_llm.rs` (or shared in `arawn-llm` if other suites need it). Records inbound prompts; returns scripted responses keyed by prompt content prefix:

```rust
let mock = MockLlm::new()
    .respond_to("classify", json!({"in_scope": true, "reason": "matches pat's 1:1 scope"}))
    .respond_to("extract", json!([{"entity_type": "decision", "title": "...", ...}]))
    .respond_to("link", json!([{"from": "...", "rel": "supersedes", "to_name": "..."}]));
```

Failures (no matching response) return a clear error so test diagnostics are obvious.

### Test scenarios

1. **Happy path — fresh row extracts into target workstream.**
   - Seed projection table with one gmail message.
   - Register workstream "pat" with description.
   - Fire dispatch hook (or call `run_for_workstream` directly).
   - Assert: entity + EXTRACTED_FROM relation written; cursor advanced past the row's source_ts.

2. **Out-of-scope skip.**
   - Same setup, but mock returns `in_scope: false`.
   - Assert: no entities written; cursor still advances (we processed the row, just didn't keep anything).

3. **Link-by-name resolves to existing entity.**
   - Pre-seed the workstream KB with an existing Fact `"open question: which auth library?"`.
   - Mock returns an extracted Decision + a link `{"rel": "supersedes", "to_name": "open question: which auth library?"}`.
   - Assert: SUPERSEDES edge from new Decision → existing Fact.

4. **Link-by-name with no match — dropped with warn.**
   - Same as #3 but the target name doesn't exist in the KB.
   - Assert: entity written, no edge created. No panic.

5. **Backfill walks existing rows on bind.**
   - Pre-seed projection table with 5 gmail messages.
   - Call `WorkstreamBindTool` to bind the feed to a workstream.
   - Assert: all 5 rows processed, entities written, cursor advanced past the most recent.

6. **Cursor idempotency on re-run.**
   - Run a workstream's extractor twice over the same projection set.
   - Assert: second run is a no-op (cursor already at the head).

7. **Two workstreams, one row.**
   - Two workstreams "pat" and "auth-migration". Same gmail message.
   - Mock returns `in_scope: true` for both.
   - Assert: entity lands in BOTH KBs independently.

### What's deferred

- UAT against real LLMs (Phase 7).
- Quality eval — does the extractor pull the *right* entities from realistic content? Needs a labeled dataset; out of scope for this phase.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `MockLlm` available; integration test suite uses it.
- [ ] All 7 scenarios above pass.
- [ ] No real network calls in the test suite.
- [ ] `cargo test -p arawn-extractor` green.
- [ ] `angreal check workspace` + `angreal check clippy` clean.

## Implementation Notes

### Where the tests live

`crates/arawn-extractor/tests/` (integration tests on the crate). The mock LLM stays in this crate's test-utils unless `arawn-llm`'s own tests benefit from it — defer the share until there's a second consumer.

### Determinism

The CoT chain's stages each map to one LLM call. With scripted mock responses, every stage is deterministic — no flakiness from model variation.

### Dependencies

- T-0252 (CotChain — the prompts the mock has to respond to).
- T-0253 (backfill — scenario #5 exercises it).

## Status Updates

### 2026-05-12 — Implementation complete

**Stage-keyed mock LLM (test-only, inline in `cot.rs`).**
Instead of consuming a strict response queue, `KeyedMockLlm` inspects the request's `system_prompt` for stage-distinguishing phrases ("You decide whether" → classify, "Pull typed knowledge entities" → extract, "Propose relations" → link) and dispatches to a per-stage queue with optional per-stage default. Defaults are the right ergonomic primitive for these tests: nearly every scenario wants the same classify+extract+link response regardless of which projection row is being processed, so setting a default is one line vs. pushing N copies.

**Test infrastructure: `Fixture`.**
- Opens a tempdir-backed `Store` (with scratch workstream) and `ProjectionStore`.
- Provides a `MemoryResolver` that caches `MemoryManager` handles per workstream so the test can read back the exact KB the chain wrote into (a naive resolver returns a fresh handle each call, defeating assertions).

**7 scenarios implemented as `#[tokio::test]`s in `cot::integration`:**

1. `happy_path_extracts_into_workstream` — single row, classify=true → one decision written; cursor advanced; entity searchable in workstream KB via FTS.
2. `out_of_scope_skips_but_advances_cursor` — classify=false skips, but the cursor still advances so the row isn't re-classified next pass.
3. `link_by_name_resolves_to_existing_kb_entity` — pre-seeds a Fact in the workstream KB; CoT emits a `supersedes` link by `to_name`; assertion checks `relations_written == 1`, proving the FTS-resolve path fired.
4. `link_to_missing_target_is_dropped_without_panic` — link's `to_name` isn't in the KB and isn't a sibling; entity written, edge dropped silently.
5. `backfill_walks_existing_rows` — 5 rows / batch_size 2 via `run_for_workstream_until_exhausted`; all 5 processed across multiple iterations.
6. `rerun_is_idempotent_via_cursor` — second run returns `processed = 0` because the cursor caught up.
7. `two_workstreams_each_get_the_entity` — same projection row, two workstreams; both classify=true; entity lands in both KBs independently.

**Validation:**
- `cargo test -p arawn-extractor` — 26 pass (19 prior + 7 new); 0 failures.
- `angreal test unit` — full workspace green.
- `angreal check clippy` — exit 0.

**Notes:**
- Tests stay inline in `cot.rs` rather than a separate `tests/` file per project convention (inline test modules preferred).
- No real network calls; UAT against real LLMs is Phase 7.
- The KeyedMockLlm wraps `arawn-llm`'s `LlmClient` trait directly rather than reusing the in-order `MockLlmClient`, because the CoT chain issues calls in a fixed-per-stage order that maps better to stage-keyed dispatch than to a flat queue.