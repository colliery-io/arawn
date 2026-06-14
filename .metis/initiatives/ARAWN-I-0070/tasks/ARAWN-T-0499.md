---
id: gui-p2-extraction-log-memory
level: task
title: "GUI-P2: Extraction-log + memory-search read RPCs (GUI provenance)"
short_code: "ARAWN-T-0499"
created_at: 2026-06-14T00:32:56.768933+00:00
updated_at: 2026-06-14T00:32:56.768933+00:00
parent: ARAWN-I-0070
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0070
---

# GUI-P2: Extraction-log + memory-search read RPCs (GUI provenance)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0070]]

## Objective **[REQUIRED]**

Add read RPCs for **extraction provenance** and **memory search**, filed from GUI-S4 (ARAWN-T-0497). The extraction *log* (per-run outcomes / signal-explain history from T-0484's `ExtractionLogStore`) and arbitrary **memory search** both lack a read path on `ArawnService` — only `status().extraction.cursors` (positions) and `memory_summary()` (counts) are exposed. The GUI provenance panel can show cursors + counts but not the run log or search results without these.

### Type
- [x] Feature — protocol (GUI-serving read)

### Priority
- [x] P3 - Low

## Acceptance Criteria **[REQUIRED]**

- [ ] `ArawnService` read for the extraction log (recent runs: lens, feed_type, outcome ok/empty/skipped, reason, ts) from `ExtractionLogStore` (T-0484), exposed as a WS-RPC.
- [ ] `ArawnService` read for memory search (query → matching entities) — distinct from `forget_entity`'s delete-oriented search — exposed as a WS-RPC returning structured results.
- [ ] GUI-S4 consumes both (replaces the "needs a read RPC" notes in the extraction + memory panels).
- [ ] Tests for each method + RPC round-trip.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Extraction log: thin read over `ExtractionLogStore::record`/`get` (T-0484). Memory search: surface the read half of the existing graphqlite search the extractor/`forget` already use, as a non-destructive query RPC.

### Dependencies
Parent [[ARAWN-I-0070]]; consumed by [[ARAWN-T-0497]] (GUI-S4). Builds on T-0484 (extraction log).

### Risk Considerations
Cap result sizes. Memory search must be read-only (no reinforcement/dedup side effects).

## Status Updates **[REQUIRED]**

*Filed 2026-06-13 from GUI-S4 (ARAWN-T-0497) as discovered protocol gaps. Not yet started.*