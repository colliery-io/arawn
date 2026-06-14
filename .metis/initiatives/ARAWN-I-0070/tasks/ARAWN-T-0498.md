---
id: gui-p1-signals-across-lenses-read
level: task
title: "GUI-P1: Signals-across-lenses read RPC (GUI prerequisite)"
short_code: "ARAWN-T-0498"
created_at: 2026-06-14T00:32:56.003521+00:00
updated_at: 2026-06-14T15:18:46.410266+00:00
parent: ARAWN-I-0070
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0070
---

# GUI-P1: Signals-across-lenses read RPC (GUI prerequisite)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0070]]

## Objective **[REQUIRED]**

Add a **signals-across-lenses read RPC** to `ArawnService`. Filed from GUI-S4 (ARAWN-T-0497): extracted signals live in the projection store but there is **no read path on the service trait** (`query_inventory` covers tools/feeds, not signals). The GUI signals panel can't light up without it. Protocol-first: the read must be an RPC, not a client-side DB peek.

### Type
- [x] Feature — protocol (GUI-serving read)

### Priority
- [x] P2 - Medium

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] A new `ArawnService` method (e.g. `list_signals(lens: Option<..>, limit)`) returns recent extracted signals across lenses as a typed DTO (id, lens, type, summary/body, source citation, ts).
- [ ] Exposed as a WS-RPC method in the `RPC_METHODS` allowlist + handler, returning structured JSON (not markdown).
- [ ] GUI-S4 signals panel consumes it (replaces the "no read RPC yet" placeholder).
- [ ] Tests for the service method + RPC round-trip.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Read from the projection/signal store (the extractor writes signals there). Mirror the shape of the existing structured reads (`ceremonies.*`, `status`). Keep it cheap (recent N, optional lens filter).

### Dependencies
Parent [[ARAWN-I-0070]]; consumed by [[ARAWN-T-0497]] (GUI-S4).

### Risk Considerations
Signals can be high-volume — cap the result and paginate/filter by lens; don't dump the whole store.

## Status Updates **[REQUIRED]**

**2026-06-14 — Implemented & complete.**
- New `ArawnService::list_signals(limit) -> Vec<SignalDto>` (+ `SignalDto` DTO in `arawn-service`). `LocalService::list_signals_inner` (new `local_service/inspect.rs`) enumerates `<data_dir>/lenses/*`, opens each lens's `memory.db` read-only via `MemoryStore::open` (no embedder — uses `list_all_ranked`, a ranked list not a vector search), labels each entity with its source lens, merges newest-first, caps to `limit`. Lens KBs that fail to open are skipped.
- WS-RPC `list_signals` added to the allowlist + handler (default limit 50).
- GUI `/signals` "Signals across lenses" panel now renders real rows (lens · type · signal+summary · updated) — the placeholder/gap note is gone.
- Tests: `signals_renders_live_signals_memory_and_run_log` + the storage/empty-state tests; gate clippy + fmt clean.
- **Verified live** (`arawn serve` pid 66819, real instance): `GET /signals` → 200, signals panel renders (empty state "No signals extracted yet" since the user's lens KBs have no extractions yet — the read path works, data is just empty).

Done alongside [[ARAWN-T-0499]] in one GUI commit (shared `/signals` surface).