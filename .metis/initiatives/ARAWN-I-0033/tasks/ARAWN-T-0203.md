---
id: google-calendar-integration-list
level: task
title: "Google Calendar integration: list upcoming, create event, find conflicts"
short_code: "ARAWN-T-0203"
created_at: 2026-05-03T12:43:21.447837+00:00
updated_at: 2026-05-03T12:43:21.447837+00:00
parent: ARAWN-I-0033
blocked_by: [ARAWN-T-0200, ARAWN-T-0201]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0033
---

# Google Calendar integration: list upcoming, create event, find conflicts

## Parent Initiative

[ARAWN-I-0033](../initiative.md)

## Objective

Second concrete consumer of the integration framework. Implements `GoogleCalendarIntegration` plus three engine tools so the agent can answer "what's on my calendar," schedule new events, and check for conflicts before suggesting times.

## Type / Priority
- Feature
- P1 — Pairs with Gmail to cover the core "what's happening today" question.

## Acceptance Criteria

- [ ] Integration impl in the same crate as Gmail (or sibling). Provider config: Google OAuth, scope `calendar.events`.
- [ ] Three tools registered:
  - `calendar_upcoming({lookahead_hours, calendar_id?})` → next events. Default calendar = "primary". Returns title, start, end, location, attendees. Permission category: ReadOnly.
  - `calendar_create_event({title, start, end, attendees?, description?, calendar_id?})` → returns the event id + a calendar URL. Permission category: Other (mode default: ask).
  - `calendar_find_conflicts({start, end, calendar_id?})` → list of overlapping events in the window. Permission category: ReadOnly.
- [ ] Token storage and refresh inherited from T-0200 helpers — same code path as Gmail.
- [ ] **OAuth scope-merging consideration:** if Gmail is already connected, the same Google OAuth flow can request both `gmail.*` and `calendar.events`. Decide in implementation: separate per-service connect flows (simpler, requires user to consent twice) vs combined "Google" connection (better UX, more code). Default to **separate per-service** for v1 — matches the framework's "one service one credential" model. Revisit if friction is real.
- [ ] Integration test against recorded fixtures.
- [ ] Manual smoke: connect, run `calendar_upcoming({lookahead_hours: 24})`, verify shape.
- [ ] Docs: `docs/src/integrations/calendar.md` (mirrors Gmail's docs page).

## Implementation Notes

- Calendar API (v3) is friendlier than Gmail's — JSON-native, no MIME parsing.
- All times in/out of these tools use ISO 8601 RFC3339 strings with timezone. Don't try to do timezone math; let the model handle it. Document the convention in tool descriptions.
- `find_conflicts` should be precise (overlap, not "same day"). The model often needs this to suggest meeting times.
- Don't bother with attendee response state for v1 (accepted/declined/tentative). Just list them.
- Recurring events: the API returns instances; pass them through. Don't try to expand recurrence rules client-side.

## Status Updates

*To be added during implementation*
