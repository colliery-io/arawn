---
id: p2-7-extraction-debuggability
level: task
title: "P2-7: Extraction debuggability — confidence floor, run IDs, explain/rerun/dismiss tooling"
short_code: "ARAWN-T-0482"
created_at: 2026-06-12T12:02:17.697049+00:00
updated_at: 2026-06-13T12:43:11.010412+00:00
parent: ARAWN-I-0068
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0068
---

# P2-7: Extraction debuggability — confidence floor, run IDs, explain/rerun/dismiss tooling

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0068]] — implements P2-7 (MEDIUM).

## Objective **[REQUIRED]**

Make extraction debuggable and idempotent: wire the unused confidence floor, stamp per-run provenance IDs so backfills don't duplicate, distinguish "extracted empty" from "skipped", and add user tooling to see why a signal exists / re-run / dismiss it.

**The defect** (`arawn-extractor/src/cot.rs:255-259, 451, 494-501`, `runner.rs:210-264`): `link_by_name` takes the first FTS hit and the `_floor: f32` parameter is unused — typos link signals to the wrong entity. EXTRACTED_FROM edges carry no run-id, so backfill re-runs duplicate entities. Valid-but-empty LLM output advances the cursor (non-deterministic re-runs). The `len() >= 3` token filter drops short tokens ("AI") from global-fact context. There's no way to see why a row was classified in/out of scope, re-run one row, or mark a signal garbage (recourse today: manual memory deletion).

### Type
- [x] Bug / Feature — extraction determinism + debuggability

### Priority
- [x] P2 - Medium

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] The confidence floor is wired into `link_by_name` — low-confidence FTS hits are rejected rather than silently accepted.
- [ ] EXTRACTED_FROM edges carry a per-run provenance ID; a re-run/backfill doesn't duplicate entities.
- [ ] "Extracted empty" is distinguished from "skipped" so re-running a lens over the same feed is deterministic.
- [ ] User-facing tooling: `signal_explain` (why a row was classified), `extract_rerun <row>`, `signal_dismiss` (mark garbage, don't re-extract).
- [ ] Inline tests for floor enforcement + idempotent re-run.
- [ ] `angreal check all` and `angreal test unit` green.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
In `arawn-extractor`: use the `floor` parameter in `link_by_name`; add a run-id to the extraction run and stamp it on EXTRACTED_FROM; mark rows extracted-empty vs unprocessed distinctly so the cursor logic is deterministic. Add the three agent tools (explain/rerun/dismiss). Consider lowering the token-length filter or making it configurable.

### Dependencies
Cursor positions surface via [[ARAWN-T-0476]]. Largely independent of the storage/ceremony tasks.

### Risk Considerations
Determinism vs the model's nondeterminism — the cursor/empty-marker change is the key to reproducible re-runs. The floor threshold needs a sensible default (too high drops real links).

## Status Updates **[REQUIRED]**

### 2026-06-13 — COMPLETE (determinism core) ✅ · tooling spun out to [[ARAWN-T-0484]]
**Shipped — the determinism/data-integrity half:**
- **Confidence floor wired**: the unused `_floor` is now live. Added `MemoryStore::search_scored` (returns each hit's `-bm25` relevance score; ≥0 for any match) + `fts_search_scored`. `resolve_by_fts`/`scored_fts_hit` reject a top FTS hit whose score is below the floor — a typo'd name no longer links a signal to a barely-related entity. Default floor 0.0 = accept-all (historical behavior).
- **Token filter widened**: `relevant_global_facts` ≥3 → ≥2, so meaningful short tokens ("AI"/"ML"/"QA") reach global-fact context instead of being silently dropped.
- **Re-run idempotency / no-duplicate**: already satisfied — the chain extracts via `store_fact`, which dedups by title (T-0481 hardened it to a single-lock compound op), and EXTRACTED_FROM targets the stable `projection_id_to_uuid`. The existing `rerun_is_idempotent_via_cursor` + `out_of_scope_skips_but_advances_cursor` tests prove deterministic re-runs. So the "per-run id to avoid duplicates" concern is moot (entities reinforce, not duplicate); a per-run *annotation* is low-value and moved to T-0484.

**Tests:** `high_confidence_floor_rejects_fts_link` (entity still extracted, sub-floor link dropped); existing 30 extractor tests green incl. idempotency. fmt + clippy -D warnings + memory(9)/extractor(31) green.

**Spun out to [[ARAWN-T-0484]] (P3):** the three operator tools (`signal_explain`/`extract_rerun`/`signal_dismiss`) + the "extracted-empty vs skipped" per-row marker. These need a NEW `extraction_log` table (per-row classify verdict/reason + dismissed flag) that the determinism fixes didn't require — a separable sub-project, decomposed out so the data-integrity core ships clean. The corruption-prevention value of P2-7 (floor + idempotency) is delivered here; T-0484 is the quality-of-life tooling.