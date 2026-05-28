---
id: t-c-uatgmailclient-inbox-summary
level: task
title: "T-C: UatGmailClient + inbox-summary conversion"
short_code: "ARAWN-T-0447"
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

# T-C: UatGmailClient + inbox-summary conversion

## Parent Initiative
[[ARAWN-I-0062]]

## Objective
Add Gmail to the mocked-integration surface. Convert `inbox-summary` so the
agent reads the inbox via `gmail_inbox` (or `gmail_search`) rather than
`feed_search` against `gmail_messages`.

## Technical Approach
- Mirror T-B for Gmail. Read-side: list messages (optionally filtered by
  label / since), get one message by id, get a thread.
- Thread reconstruction: group rows by `thread_id` and order by
  `internal_date` / `source_ts`.
- Return shapes match what the production `gmail_*` tools actually consume.
- Wire `register_mock_integrations(&["google_calendar", "gmail"])` to
  morning-briefing as well (so the agent has both) and to inbox-summary
  exclusively for the inbox path.
- `required_tool_names: ["gmail_inbox"]` (or whichever read tool fits) on
  inbox-summary; `required_evidence` keeps its marketing-noise filter focus.

## Acceptance Criteria
- [ ] `UatGmailClient` returns deterministic inbox/thread data from the
      fixture's `gmail_messages` rows.
- [ ] `gmail_inbox`, `gmail_thread_get` (or the actual read tools the agent
      uses) succeed in UAT against the mock client.
- [ ] `inbox-summary` scenario: agent calls a `gmail_*` read tool first
      (verified); marketing rows are still correctly filtered out; mechanical
      PASS; judge completion ≥ 4/5.
- [ ] `angreal check workspace` + `arawn-tests` green.

## Dependencies
Depends on [[ARAWN-T-0445]]; benefits from [[ARAWN-T-0446]] sharing the
backend.

## Status Updates
*To be added during implementation*
