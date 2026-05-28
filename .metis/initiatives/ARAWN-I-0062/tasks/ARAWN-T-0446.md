---
id: t-b-uatcalendarclient-morning
level: task
title: "T-B: UatCalendarClient + morning-briefing conversion"
short_code: "ARAWN-T-0446"
created_at: 2026-05-28T22:01:44.832147+00:00
updated_at: 2026-05-28T22:01:44.832147+00:00
parent: ARAWN-I-0062
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0062
---

# T-B: UatCalendarClient + morning-briefing conversion

## Parent Initiative
[[ARAWN-I-0062]]

## Objective
First service end-to-end. Stand up a projection-backed `UatCalendarClient`,
wire it into the calendar integration, and convert the `morning-briefing`
scenario so the agent answers "today's schedule" via `calendar_today_events`
rather than the corpus fallback.

## Technical Approach
- Build `ProjectionFixtureBackend` (`crates/arawn-tests/tests/uat_mocks/
  backend.rs`) that opens the seeded projection store and exposes typed
  per-feed-type accessors.
- Implement `UatCalendarClient` against whatever calendar-client trait the
  production `calendar_*` tools dispatch through. Read-side surface first:
  list events for today (filter on `source_ts`), get one event by id.
  Return the projection rows as the calendar API shape — fields the real
  tool impls actually read (id, summary, start.dateTime, end.dateTime,
  attendees if present).
- Wire `register_mock_integrations(&["google_calendar"])` into the
  morning-briefing scenario.
- Update the user_message + judge_expectation to expect
  `calendar_today_events` as the first call.
- Set `required_tool_names: ["calendar_today_events"]` and
  `required_evidence: ["20:00", "rfc-0042"]` so the harness asserts both the
  call AND the data.

## Acceptance Criteria
- [ ] `UatCalendarClient` returns deterministic calendar data from the
      fixture's `calendar_events` projection rows.
- [ ] `calendar_today_events` (and one other read tool) succeeds in UAT
      against the mock client.
- [ ] `morning-briefing` scenario: agent calls `calendar_today_events`
      (verified by `required_tool_names`); mechanical PASS; judge completion
      ≥ 4/5; the 20:00 UTC conflict is surfaced.
- [ ] `angreal check workspace` + `arawn-tests` green.

## Dependencies
Depends on [[ARAWN-T-0445]].

## Status Updates
*To be added during implementation*
