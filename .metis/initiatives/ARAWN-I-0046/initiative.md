---
id: linear-integration-personal-track
level: initiative
title: "Linear integration — personal-track issue surface for non-Atlassian work"
short_code: "ARAWN-I-0046"
created_at: 2026-05-15T15:09:14.411550+00:00
updated_at: 2026-05-15T15:09:14.411550+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: S
initiative_id: linear-integration-personal-track
---


## Context **[REQUIRED]**

Linear is the issue tracker most non-Atlassian small teams and many individual developers use for personal/side-project work. Arawn has Atlassian (Jira/Confluence) but nothing for the Linear-shaped half of the market. The vision points at "personal" use cases, and personal-track work disproportionately lives in Linear.

This is the smallest of the three connector follow-ons surfaced by the openhuman comparative dive (ARAWN-I-0044). Linear's API is GraphQL-only, well-documented, and uses simple personal API keys for the read-mostly case — meaningfully less moving parts than GitHub or Outlook.

## Goals & Non-Goals **[REQUIRED]**

**Goals:**
- Pull the user's assigned + authored Linear issues into a shape arawn can surface (morning brief, workstream routing).
- Agent tools for: comment, update status, assign, change priority.
- API-key auth path first (simpler, no OAuth dance, sufficient for personal use). OAuth path later if a multi-workspace story emerges.
- Threat-model parity with the rest of `arawn-integrations`: API key encrypted at rest per ARAWN-A-0001.

**Non-Goals:**
- Linear workflow automation, custom views, triage rules. Surface the data; the user manages the workflow in Linear's UI.
- Project / cycle / roadmap modelling. Stay at the issue level.
- Webhook ingestion. Poll on cron like everything else.

## Requirements **[CONDITIONAL: Requirements-Heavy Initiative]**

### Functional sketch
- REQ-001: Personal API key auth, stored encrypted, loaded at daemon start.
- REQ-002: GraphQL client (a thin wrapper; we do not need a full Linear schema codegen — three or four queries cover the use case).
- REQ-003: Pull assigned + authored issues with state, priority, due date, comments.
- REQ-004: Agent tools: `linear.comment`, `linear.set_state`, `linear.set_assignee`, `linear.set_priority`.
- REQ-005: Rate-limit awareness (Linear publishes per-key limits in response headers).

### Non-functional
- NFR-001: API key storage matches ARAWN-A-0001.
- NFR-002: GraphQL queries kept small + named so they can be diffed when Linear's schema evolves.

## Detailed Design **[REQUIRED]**

### Pre-design open questions
- Single workspace vs multi-workspace. API keys are per-workspace. Decide whether arawn supports a list of keys (and a default) or one key only. Likely list — mirrors the Slack multi-workspace work in ARAWN-I-0034.
- GraphQL client choice. We can hand-roll `reqwest` + `serde_json` for the half-dozen queries we need, or pull a GraphQL crate. Hand-roll is probably right for the scale.

### Existing pattern to copy
`crates/arawn-integrations/src/atlassian/` is the closest parallel — different protocol (REST vs GraphQL) but the same shape: encrypted credential, periodic fetch, typed models, agent tools that mutate.

## Alternatives Considered **[REQUIRED]**

- **Composio-proxied (openhuman's path).** Rejected per ARAWN-S-0004 §F.
- **OAuth-first instead of API-key-first.** Rejected for now: OAuth requires more setup (callback server, client registration) for a workflow most personal users would happily authenticate with a copied API key. OAuth lands later if multi-workspace usage demands per-user identity.

## Implementation Plan **[REQUIRED]**

Discovery-phase deliverables, not decomposed yet:

1. Decide single-key vs key-list (workspace handling).
2. Decide GraphQL client approach (hand-roll vs crate).
3. Decompose into tasks (~4 expected): credential store + auth; issue fetch + ingestion; agent tools (comment/state/assignee/priority); rate-limit + caching.

## Exit Criteria

- Linear issues assigned to the user surface in the morning brief alongside GitHub / Jira items.
- Agent can comment, set state, assign, and set priority on a target issue end-to-end, against a real workspace.
- API key survives a daemon restart and stays encrypted on disk.
