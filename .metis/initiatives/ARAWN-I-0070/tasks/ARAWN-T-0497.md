---
id: gui-s4-signals-memory-extraction
level: task
title: "GUI-S4: Signals / memory / extraction-provenance surface"
short_code: "ARAWN-T-0497"
created_at: 2026-06-13T16:02:54.560571+00:00
updated_at: 2026-06-13T16:02:54.560571+00:00
parent: ARAWN-I-0070
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


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

*To be added during implementation*