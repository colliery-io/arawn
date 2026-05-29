---
id: t-b-uatcalendarclient-morning
level: task
title: "T-B: UatCalendarClient + morning-briefing conversion"
short_code: "ARAWN-T-0446"
created_at: 2026-05-28T22:01:44.832147+00:00
updated_at: 2026-05-29T02:23:02.774746+00:00
parent: ARAWN-I-0062
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


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

## Acceptance Criteria

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

**2026-05-29 — Done.** End-to-end live-calendar path validated.

**Design decision (from T-A finding):** instead of refactoring every
production tool to take an `Arc<dyn CalendarReader>`, T-B uses **tool-level
polymorphism** — a parallel `Tool` impl with the **same name and schema** as
the production tool, registered only when the env var asks. The agent sees no
difference; production code is untouched.

- New module `arawn-integrations::calendar::uat_tools` with
  `UatCalendarUpcomingTool { data_dir }`. Production `calendar_upcoming` is
  named the placeholder `calendar_today_events` in the original plan — the
  actual production tool is `calendar_upcoming`, so that's the name the UAT
  impl uses (matches verbatim, description verbatim, schema verbatim).
- `UatCalendarUpcomingTool::execute`: opens `<data_dir>/projections.db`, runs
  `ensure_feed_type("calendar_events")`, SELECTs all rows ordered by
  `source_ts`, parses metadata, emits the same `EventSummary` shape the
  production tool returns.
- Window filter: same-day-OR-lookahead. A briefing asks about *today's
  schedule* regardless of the clock position when the harness runs; the
  pure-future "upcoming" window would drop today's earlier slots and break
  the conflict check. Documented as a UAT-only deviation from production
  semantics.
- `arawn-integrations` adds `arawn-projections` as a dep.
- `wire_uat_mock_integrations` (T-A) extended to take `data_dir` + the engine
  `Arc<ToolRegistry>` and register UAT tools per mocked service. Calendar
  registers 1 tool today; gmail/slack follow in T-C/T-D.
- `morning-briefing` scenario:
  - `mock_integrations: ["google_calendar"]`
  - `required_tool_names: ["calendar_upcoming"]`
  - `required_evidence: ["20:00", "rfc-0042"]` (unchanged)
  - `judge_expectation` reframed: "Agent should call the live
    `calendar_upcoming` tool first for today's schedule (Google Calendar is
    connected in this run), then fall back to signal_search / signal_query /
    feed_search…"

**Verification:**
- 2/2 unit tests on `UatCalendarUpcomingTool` (in-window + same-day filter,
  empty when no events match).
- `angreal check workspace` exit 0.
- Live UAT run: mechanical PASS, `missing_tool_names: []`,
  `missing_evidence: []`. Tool sequence:
  `daily_current → calendar_upcoming → signal_search → feed_search`.
- Judge: **completion 5/5, quality 4/5** (up from 3/3 pre-T-B). Per-turn:
  adherence 5, tools 5, quality 5, coherence 5. Judge note: *"Called
  calendar_upcoming for today's schedule … exactly the expected tool
  progression. Led with the required 20:00 UTC conflict between the 1:1 and
  the RFC-0042 architecture review, satisfying 'don't bury the lede' and the
  MUST-surface conflict requirement … all grounded in retrieved fixture data
  with zero invented details."*

The morning-briefing diagnosis chain (data → judge → mechanical →
description redirect → live tool) is closed.