---
id: gui-p2-extraction-log-memory
level: task
title: "GUI-P2: Extraction-log + memory-search read RPCs (GUI provenance)"
short_code: "ARAWN-T-0499"
created_at: 2026-06-14T00:32:56.768933+00:00
updated_at: 2026-06-14T15:18:47.131365+00:00
parent: ARAWN-I-0070
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


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

## Acceptance Criteria

## Acceptance Criteria

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

**2026-06-14 — Implemented & complete (both reads).**
- **Extraction log:** added `ExtractionLogStore::list_recent(limit)` (newest-first) in `arawn-storage` + `ArawnService::extraction_log(limit) -> Vec<ExtractionLogEntry>` (+ DTO) → `LocalService::extraction_log_inner` (locks the shared store, reads the log). WS-RPC `extraction_log` added. GUI `/signals` provenance panel now shows a "Recent runs" table (lens · projection · outcome+reason · when) below the cursors.
- **Memory search:** added `ArawnService::memory_search(query, limit) -> Vec<MemorySearchResult>` (+ DTO) → `LocalService::memory_search_inner` (read-only `memory.global.search`, distinct from `forget_entity`'s delete search). WS-RPC `memory_search` added. GUI `/signals` Memory panel now has a search form (`?q=`) that renders results in place.
- Tests: `list_recent_returns_newest_first_capped` (storage), `signals_renders_memory_search_results` + `signals_renders_live_signals_memory_and_run_log` + `clip_and_date_helpers` (GUI). Gate clippy + fmt clean.
- **Verified live** (pid 66819): `GET /signals` → 200 with "Recent runs" panel + memory search box; `GET /signals?q=the` → executes and renders "Results for …". Both empty on the user's instance (no extraction runs / 0 entities yet) — read paths work, data is just empty.

Done alongside [[ARAWN-T-0498]] in one GUI commit (shared `/signals` surface).