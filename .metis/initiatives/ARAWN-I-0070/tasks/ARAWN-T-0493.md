---
id: gui-f2-hypermedia-runtime-sse-ws
level: task
title: "GUI-F2: Hypermedia runtime + SSE/WS push bridge + base layout shell"
short_code: "ARAWN-T-0493"
created_at: 2026-06-13T16:02:47.892215+00:00
updated_at: 2026-06-13T16:02:47.892215+00:00
parent: ARAWN-I-0070
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0070
---

# GUI-F2: Hypermedia runtime + SSE/WS push bridge + base layout shell

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0070]]

## Objective **[REQUIRED]**

Wire the server-rendered hypermedia runtime + an SSE/WS push bridge + a base layout shell. After this, server-originated updates (`ServerNotice` incl. `briefing_ready`, engine events) drive live fragment/signal updates with no full reload, and the four surfaces have a nav shell to live in.

### Type
- [x] Feature — GUI foundation

### Priority
- [x] P1 - High

## Acceptance Criteria **[REQUIRED]**

- [ ] Hypermedia library chosen (datastar vs htmx+SSE) and embedded; the choice is recorded with rationale.
- [ ] Server-side templating chosen (askama / maud / minijinja) and a reusable fragment-render helper exists.
- [ ] A base layout/shell renders with navigation to the four v1 surfaces (health, brief, inbox, signals).
- [ ] A push endpoint (SSE or WS) streams server-originated updates; a demo fragment updates on a `briefing_ready` notice without a page reload.
- [ ] Tests: shell renders; the push endpoint emits a fragment when a notice is broadcast. `angreal check all` green.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Bridge the existing `notice_tx` broadcast (and engine-event stream) into an SSE/WS endpoint that emits rendered HTML fragments / hypermedia signals. Keep the client runtime tiny and stateless — server is the source of truth (no client store).

### Dependencies
GUI-F1 (serve route), GUI-G2 (browser-safe transport).

### Risk Considerations
Don't reintroduce client-side state. Keep the runtime small to preserve the thin-client property. Ensure the push bridge survives reconnects gracefully.

## Status Updates **[REQUIRED]**

*To be added during implementation*