---
id: t-d-uatslackclient-mention-scan
level: task
title: "T-D: UatSlackClient + mention-scan conversion"
short_code: "ARAWN-T-0448"
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

# T-D: UatSlackClient + mention-scan conversion

## Parent Initiative
[[ARAWN-I-0062]]

## Objective
Mock Slack so the agent's `@mention` scan goes through `slack_search` /
`slack_channel_history` rather than `feed_search` against `slack_messages`.

## Technical Approach
- Mirror T-B/C for Slack. Read-side: channel history, DM history,
  message-text search.
- `@mention` semantics: `slack_search` returns messages whose text matches
  `@pat` as a token (already the scenario's filter criterion). The mock
  applies that filter on the projection rows.
- Wire mock-Slack into `mention-scan` and into morning-briefing (so Slack
  evidence is live there too).
- `required_tool_names: ["slack_search"]` (or the relevant tool) on
  mention-scan.

## Acceptance Criteria
- [ ] `UatSlackClient` returns deterministic channel/DM/search data from the
      fixture's `slack_messages` rows.
- [ ] `mention-scan` scenario: agent calls a `slack_*` read tool first
      (verified); only @-prefixed mentions surface; mechanical PASS; judge
      completion ≥ 4/5.
- [ ] `angreal check workspace` + `arawn-tests` green.

## Dependencies
Depends on [[ARAWN-T-0445]]; shares `ProjectionFixtureBackend` from
[[ARAWN-T-0446]].

## Status Updates
*To be added during implementation*
