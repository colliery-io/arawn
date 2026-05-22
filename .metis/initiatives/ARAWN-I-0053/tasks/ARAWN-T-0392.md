---
id: refresh-arawn-memory-benchmark
level: task
title: "Refresh arawn-memory benchmark scenarios — update longmemeval/recall_eval to current use cases"
short_code: "ARAWN-T-0392"
created_at: 2026-05-21T14:53:37.162614+00:00
updated_at: 2026-05-21T14:53:37.162614+00:00
parent: ARAWN-I-0053
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


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

*To be added during implementation*
