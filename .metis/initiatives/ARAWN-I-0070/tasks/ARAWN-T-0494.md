---
id: gui-s3-health-observability
level: task
title: "GUI-S3: Health/observability dashboard surface"
short_code: "ARAWN-T-0494"
created_at: 2026-06-13T16:02:49.344437+00:00
updated_at: 2026-06-13T16:02:49.344437+00:00
parent: ARAWN-I-0070
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0070
---

# GUI-S3: Health/observability dashboard surface

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0070]]

## Objective **[REQUIRED]**

Render the versioned `status`/`health` surface as an observability dashboard (feeds, ceremonies, embedding/extraction, LLM, steward), auto-refreshing via the push bridge. **First surface to build** — its contract (`SystemStatus`, `SYSTEM_STATUS_VERSION`) is already stable and round-trip-tested, so it's the cheapest proof the surface stack works end-to-end.

### Type
- [x] Feature — GUI surface

### Priority
- [x] P2 - Medium

## Acceptance Criteria **[REQUIRED]**

- [ ] A dashboard page renders `SystemStatus` with a panel per subsystem (feeds, ceremonies, embedding/extraction, LLM connectivity, steward).
- [ ] Degraded/error states are visibly distinct (failing feed, errored embeddings, steward errors).
- [ ] The view updates live (push or short-poll) without a full reload.
- [ ] Version-aware: an older/newer `version` field renders defensively (unknown fields tolerated, no panic/blank).
- [ ] Test: renders a sample `SystemStatus`; tolerates a bumped version. `angreal check all` green.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Consume the existing `status` RPC; render the typed structs through the templating layer from GUI-F2. Reuse the same DTOs the TUI `/status` uses.

### Dependencies
GUI-F2 (runtime + shell). Reads the T-0476 status surface.

### Risk Considerations
Version drift — render unknown/absent fields defensively so a protocol bump never blanks the dashboard.

## Status Updates **[REQUIRED]**

*To be added during implementation*