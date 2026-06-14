---
id: gui-s4-signals-memory-extraction
level: task
title: "GUI-S4: Signals / memory / extraction-provenance surface"
short_code: "ARAWN-T-0497"
created_at: 2026-06-13T16:02:54.560571+00:00
updated_at: 2026-06-14T00:37:42.699941+00:00
parent: ARAWN-I-0070
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0070
---

# GUI-S4: Signals / memory / extraction-provenance surface

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0070]]

## Objective **[REQUIRED]**

Render the deeper inspection surfaces: **signals across lenses, memory inspection, and extraction provenance**. The richest surface and the one most likely to expose protocol gaps — capture any missing reads as new RPC tasks rather than client-side hacks.

### Type
- [x] Feature — GUI surface

### Priority
- [x] P3 - Low

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] Signals-across-lenses view (read path confirmed or a new RPC filed if missing).
- [ ] Memory inspection rendered (`get_memory_summary` + memory search).
- [ ] Extraction provenance surfaced (signal-explain / extraction-log from T-0484).
- [ ] Any protocol gap discovered is filed as a follow-up RPC task, not worked around in the client.
- [ ] Test: each sub-view renders sample data. `angreal check all` green.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Consume `get_memory_summary` + memory search and the T-0484 extraction tooling (signal explain / extraction log). For signals-across-lenses, confirm the read RPC exists; if not, this task's first output is filing it.

### Dependencies
GUI-F2. Reads memory RPCs + T-0484 extraction surface. Likely surfaces protocol gaps (own follow-ups).

### Risk Considerations
Highest chance of revealing missing RPCs — keep protocol-first discipline; file gaps, don't hack the client.

## Status Updates **[REQUIRED]**

**2026-06-13 — Implemented & complete (renders available data; gaps filed, not faked).**
- New `GET /signals` (`gui::signals_page`) renders three panels:
  - **Memory** — `memory_summary()` (total entities + by-type table). Available.
  - **Extraction provenance** — extraction **cursors** from `status().extraction.cursors` (lens/feed_type/cursor). Available.
  - **Signals across lenses** — placeholder naming the gap.
- **Protocol gaps discovered + filed (the predicted outcome of this surface):**
  - **[[ARAWN-T-0498]] (GUI-P1)** — no signals-across-lenses read RPC exists (`query_inventory` is tools/feeds only). Filed P2.
  - **[[ARAWN-T-0499]] (GUI-P2)** — no extraction-log read (T-0484 `ExtractionLogStore` has no RPC) and no non-destructive memory-search read. Filed P3.
  - The panels **name these tasks inline** rather than hacking a client-side DB peek — protocol-first discipline held.
- **Tests (2 new, 24 total in `ws_server::gui`):** memory + cursors render with the gap tasks named; degraded path (no memory / no cursors) still renders + still names gaps.
- **Verified end-to-end** (restarted `arawn serve` pid 40158): `GET /signals` → 200 with Memory (Total entities: 0), Extraction provenance, Signals panel, T-0498/T-0499 references, active nav. **All five surfaces** (`/`, `/health`, `/brief`, `/inbox`, `/signals`) → 200. Gate clippy + fmt clean.

This completes the **v1 surface set** for I-0070. Remaining initiative children are the two filed protocol follow-ups (T-0498/T-0499) — deferred backlog, not v1 scope.