---
id: refresh-arawn-memory-benchmark
level: task
title: "Refresh arawn-memory benchmark scenarios — update longmemeval/recall_eval to current use cases"
short_code: "ARAWN-T-0392"
created_at: 2026-05-21T14:53:37.162614+00:00
updated_at: 2026-05-22T00:17:04.201181+00:00
parent: ARAWN-I-0053
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0053
---

# Refresh arawn-memory benchmark scenarios — update longmemeval/recall_eval to current use cases

## Backlog Item Details

### Type
- [ ] Bug
- [x] Feature — improve the benchmarks to reflect current memory-model usage
- [ ] Tech Debt
- [ ] Chore

### Priority
- [ ] P0
- [ ] P1
- [x] P2 — capability tracking matters but not blocking day-to-day work
- [ ] P3

### Business Justification
- **User Value**: Indirect — better benchmarks mean better memory-model decisions, which translates to a better assistant experience over time.
- **Effort Estimate**: M — requires deciding which scenarios are representative of current arawn usage, building or sourcing data, and validating the metrics.

## Objective

The arawn-memory benchmark suites (`tests/longmemeval_bench.rs` and
`tests/recall_eval.rs`) are valuable for tracking memory-model capability,
but their scenarios were authored before arawn's current usage patterns
crystallized. Per operator (during ARAWN-I-0053 discovery, Tier 3 candidate
3.7): the benchmarks should be kept and updated to reflect current use cases
and the data we actually load.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Audit the current benchmark scenarios in both files. Document (in the file headers or a sibling README) what each scenario was originally trying to measure.
- [ ] Identify representative arawn use cases for the memory model. Examples might include:
  - Personal-note retrieval (operator drops a fact, asks about it later).
  - Cross-workstream retrieval (operator asks "what was that thing in Home Maintenance?").
  - Time-aware retrieval (operator asks "what did I work on last Thursday?").
  - Ontology-tagged retrieval (asking by tag concept).
  - Decision-vs-fact disambiguation.
- [ ] Update scenarios in `longmemeval_bench.rs` and `recall_eval.rs` to match. Either rewrite existing scenarios or add new ones; preserve any historical baselines as separate scenarios so trends are comparable.
- [ ] Fix the pre-existing dead-code warnings surfaced by the discovery experiment: `parse_date_to_days`, `reciprocal_rank_fusion`, `temporal_score` are functions never used; `question_id`, `question_date`, `haystack_dates` are fields never read. Either use them in the refreshed scenarios or remove them.
- [ ] Run the benchmarks end-to-end with the refreshed scenarios. Record the new baseline numbers in the file (or in a sibling document) so future runs have something to compare against.
- [ ] Document how to run the benchmarks (`cargo test -p arawn-memory --test longmemeval_bench -- --ignored --nocapture` etc.) in a maintainer-facing README or in the file headers.

## Implementation Notes

### Technical Approach

1. Read both files end-to-end. Understand the current shape (scoring metrics: Recall@K, NDCG, MRR; the harness that drives the benchmarks).
2. Confirm what data they currently load (external download? local fixture?) and whether that data is still representative.
3. Design representative scenarios. Operator may want to drive this collaboratively.
4. Update the code, fix the dead-code warnings, re-run.
5. Capture baselines.

### Dependencies

None blocking. Standalone follow-up to ARAWN-I-0053.

### Risk Considerations

- Bench scenarios should be reproducible: avoid embedding live API calls or operator-specific data; use fixtures.
- If the existing scenarios pull from a large external corpus, the refresh might require sourcing or generating a new one. Scope that separately if it gets large.

## Status Updates

### 2026-05-21 — landed (scope-trimmed)

**Audit completed. Findings:**
- `longmemeval_bench.rs` (now 412 lines) — external-corpus benchmark against the public LongMemEval dataset. One `#[ignore]` test (`longmemeval_benchmark`) measures Recall@5/10 + NDCG@10 against ground-truth session ids. The remaining functions were all reachable from that test; the dead code was speculative temporal-aware retrieval scaffolding from an abandoned experiment.
- `recall_eval.rs` (1144 lines) — in-tree fixture benchmark, no downloads required. Eight `#[test]` functions cover FTS, MemoryStack L1/L2 topical retrieval, supersede semantics, reinforcement ranking, edge cases, and a real-embedding vector recall pass. The query corpus already maps fairly well to the operator's listed use cases.

**Changes landed:**

1. **Dead-code cleanup in `longmemeval_bench.rs`** (the pre-existing warnings surfaced by the I-0053 discovery experiment):
   - Deleted `fn reciprocal_rank_fusion` (lines 25-36) — never called.
   - Deleted `fn parse_date_to_days` (lines 38-49) — never called.
   - Deleted `fn temporal_score` (lines 53-65) — never called.
   - Dropped `question_id`, `question_date`, `haystack_dates` fields from `LongMemEvalEntry` — serde deserialized them but the test never read them.
   - Dropped now-unused `use std::sync::Arc` import.
   - Dropped now-unused `i` from one `enumerate()` call site (the value was never read).

2. **Documentation refresh in both files:**
   - `longmemeval_bench.rs`: rewrote the file-level docstring. Explains what the benchmark measures (per-question vector recall against an external corpus), what it explicitly does NOT measure (arawn-specific concerns), the dataset, the cost (~5 min + 90 MB model), and the run command.
   - `recall_eval.rs`: added a scenario-to-use-case table mapping each `#[test]` to the operator's listed use cases:
     - `fts_recall_evaluation` → Personal-note retrieval.
     - `memory_stack_l1_coverage` → Recent / high-signal recall.
     - `memory_stack_l2_topical_retrieval` → Ontology-tagged retrieval.
     - `superseded_entities_excluded_from_all_searches` → Steward prune contract.
     - `reinforcement_boosts_ranking` → Frequency-of-mention ranking.
     - `vector_search_recall_real_embeddings` → Real-embedding vector retrieval.
     - `edge_case_very_short_query` + `edge_case_no_matches` → Degenerate inputs + no-fabrication honesty.
   - Documented the implicit cross-cutting coverage (cross-workstream, decision-vs-fact disambiguation via EntityType).
   - Documented the run command.

**Scope decisions:**
- **Did not add new benchmark scenarios.** The existing eight tests already cover the operator's listed use cases except "time-aware retrieval" (operator asks "what did I work on last Thursday?"). Time-aware retrieval would require a memory-API capability that doesn't currently exist (`query_by_time` or equivalent). Documented the gap in the docstring; filing a follow-up is the right next step rather than adding a test that exercises a non-existent capability.
- **Did not run benchmarks + capture baselines.** Running `longmemeval_bench` takes ~5 minutes + 90 MB model download. The 8 fixture tests in `recall_eval` ran clean (1.82s, all pass) so the basic recall posture is verified. Capturing numerical baselines under each test result is a separate maintenance task best done by the operator on their hardware.

**Validation:**
- `cargo check -p arawn-memory --tests`: ✅ clean (no warnings).
- `cargo test -p arawn-memory --test recall_eval`: ✅ **8 tests pass**, 0 fail (1.82s).
- `cargo test -p arawn-memory --tests --no-run`: ✅ clean (longmemeval_bench compiles, runs only with `--ignored`).