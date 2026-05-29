---
id: t-d-uatslackclient-mention-scan
level: task
title: "T-D: UatSlackClient + mention-scan conversion"
short_code: "ARAWN-T-0448"
created_at: 2026-05-28T22:01:44.832147+00:00
updated_at: 2026-05-29T16:39:10.209320+00:00
parent: ARAWN-I-0062
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


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

## Acceptance Criteria

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

**2026-05-29 — Done.** Slack live path validated end-to-end.

**Surface correction:** the original task assumed a `slack_search` UAT
tool, but production has no `slack_search` (slack-morphism doesn't
typed-expose `search.messages`). The real workflow is
`slack_list_channels` → `slack_history` per channel + filter in prose, so
the UAT impls mirror that.

- New `arawn-integrations::slack::uat_tools`:
  - `UatSlackListChannelsTool` returns distinct `channel_id` values from
    `slack_messages` via `json_extract(metadata, '$.channel_id')`, mapped
    to `ChannelSummary { id, name, kind, … }` with `kind` inferred from
    the id prefix (C/G/D/M).
  - `UatSlackHistoryTool` returns messages for one channel, newest first,
    in the production `MessageSummary` shape (`ts`, `user`, `text`,
    `thread_ts`, `reply_count`, `reactions`).
  - Inherent `list_channels` / `channel_history` methods for unit tests.
- `slack/mod.rs` re-exports; `wire_uat_mock_integrations` gains a
  `"slack"` arm registering the 2 read tools.
- `mention-scan` scenario converted: `mock_integrations: ["slack"]`,
  `required_tool_names: ["slack_history"]`, `required_evidence: ["@pat"]`,
  judge_expectation reframed for the discover→read workflow.

**Verification:** 2 unit tests green; live UAT mechanical PASS with all
required fields empty, tool errors 0. Tool sequence:
`slack_list_channels → slack_history × 3` (one per channel discovered).
Judge: **completion 5/5, quality 5/5**, per-turn 5/5/5/5. Notes: *"Used
the live targeted Slack tools (slack_list_channels then slack_history per
channel) rather than free-form search. Returned EXACTLY the two literal
@pat mentions … with both messages quoted verbatim … Correctly excluded
David's 'Pat?' … Meets every rubric requirement with no inflation or
omission."*