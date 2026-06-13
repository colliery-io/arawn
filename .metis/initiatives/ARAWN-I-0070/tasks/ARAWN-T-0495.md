---
id: gui-s1-brief-ceremony-tablet
level: task
title: "GUI-S1: Brief / ceremony-tablet surface"
short_code: "ARAWN-T-0495"
created_at: 2026-06-13T16:02:50.755483+00:00
updated_at: 2026-06-13T16:02:50.755483+00:00
parent: ARAWN-I-0070
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0070
---

# GUI-S1: Brief / ceremony-tablet surface

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0070]]

## Objective **[REQUIRED]**

Render the daily/weekly **brief and ceremony tablets** in the GUI, refreshing on the `briefing_ready` push — the ambient-brain centerpiece, in a medium that fits it (vs. markdown-in-a-chat-pane today).

### Type
- [x] Feature — GUI surface

### Priority
- [x] P2 - Medium

## Acceptance Criteria **[REQUIRED]**

- [ ] Brief/tablet view renders structured `ceremonies.get_by_period` / `list_items` / `list_priorities` / `get_diary` data.
- [ ] A `briefing_ready` push refreshes the view without a full reload.
- [ ] Priority confirm/reject + diary actions wired through the existing RPCs (`ceremonies.confirm_priority` / `reject_priority` / `upsert_diary`), OR v1 ships read-only with the write actions filed as a fast follow.
- [ ] Large tablets remain usable (scroll/sections).
- [ ] Test: renders a sample tablet; a `briefing_ready` notice triggers refresh. `angreal check all` green.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Consume the structured `ceremonies.*` reads; render via templating; subscribe the view to `briefing_ready` through the GUI-F2 push bridge.

### Dependencies
GUI-F2. Reads the existing ceremony RPCs + the `briefing_ready` notice (I-0035 Phase 4).

### Risk Considerations
Write actions (priority confirm/reject) mutate state — confirm RPC coverage; if any action lacks an RPC, file it rather than hacking client-side.

## Status Updates **[REQUIRED]**

*To be added during implementation*