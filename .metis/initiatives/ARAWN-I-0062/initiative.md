---
id: mocked-integrations-in-uat
level: initiative
title: "Mocked integrations in UAT — validate the live-agent tool path, not just the corpus fallback"
short_code: "ARAWN-I-0062"
created_at: 2026-05-28T22:01:44.832147+00:00
updated_at: 2026-05-28T22:01:44.832147+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: L
initiative_id: mocked-integrations-in-uat
---

# Mocked integrations in UAT

## Context

UAT today seeds projections (`personal-day.json`, etc.) into the corpus but
registers **zero live integrations**. The category-visibility filter at
`crates/arawn-engine/src/query_engine.rs:1176-1184` gates `calendar_*` /
`gmail_*` / `slack_*` / `drive_*` / `atlassian_*` / `github_*` behind
`is_connected("<service>")`. In UAT nothing is connected → those tools are
**not even visible to the agent** → the agent is forced down the
`signal_*`/`feed_search`/`daily_*` path.

That path is the *retrospective corpus* path. In production, "what's on my
calendar today?" should resolve via the **live calendar tool**, not by reading
extracted entities or top-10 FTS projection rows. The system prompt already
says exactly this: *"Prefer specialized integration tools over `feed_search` for
interactive queries. Feeds are NOT a source of immediate truth."* But UAT can't
exercise that preference because the live tools aren't there.

The morning-briefing scenario surfaced the gap concretely: the agent reached
for `signal_search` with an FTS-unfriendly query and got zero results — the
right answer (`calendar_today_events`) wasn't an option because no integration
was connected. ARAWN-I-0061 closed the related model + harness gaps
(`required_evidence` mechanical check, synthesis directive, description
redirects); this initiative closes the remaining **fidelity** gap.

## Goals & Non-Goals

**Goals:**
- UAT scenarios can exercise the **production tool path**: live integration
  tool first, fall back to corpus only when the integration genuinely doesn't
  have the data.
- Per-service `Uat*Client` stubs read from the projection store (the same
  fixture the corpus path uses), so the live answer is deterministic and
  faithful to the seeded scenario.
- The category-visibility filter sees the mocked services as connected.
- The existing calendar/gmail/slack scenarios (morning-briefing, inbox-summary,
  mention-scan, draft- / schedule-with-confirmation) get converted to the
  live-tool path with sharpened `judge_expectation`s + `required_evidence`.

**Non-Goals:**
- No real OAuth / network in UAT. Stubs are projection-backed; nothing touches
  external services.
- Not building stubs for tools no UAT scenario currently exercises — start
  with what the existing scenarios need, add more as scenarios grow.
- No changes to production integration code beyond what the mock interfaces
  require.

## Detailed Design

The harness needs two layers:

### Connection layer

A `UatMockIntegration` (one per service) that implements `Integration` with
`is_connected() == true` and registers in the existing `integration_registry`.
Once registered, the existing `connected_services` fn in
`crates/arawn/src/local_service/mod.rs:468-485` reports the service as
connected, and `query_engine::filter_tools_for_context` lets the tools through.
This part is mechanical (~20 lines of plumbing per service + one harness call
to register the set the scenario wants).

### Behavior layer

Each integration tool calls into a provider client (`gmail_client.get_messages`,
etc.) via the integration's exposed API. For UAT those clients need to return
projection-backed canned data — the same fixture rows the corpus path reads,
exposed via the integration's natural API shape.

Per service:

- **`UatCalendarClient`** — backs `calendar_today_events`,
  `calendar_event_get`, `calendar_event_create` (read-side first). Reads
  `calendar_events` projection rows for the seeded fixture and returns them as
  the calendar API would (RFC3339 timestamps, event ids).
- **`UatGmailClient`** — backs `gmail_inbox`, `gmail_message_get`,
  `gmail_thread_get`, `gmail_search`. Reads `gmail_messages` rows; threads
  reconstruct from `thread_id`.
- **`UatSlackClient`** — backs `slack_channel_history`, `slack_dm`,
  `slack_search`. Reads `slack_messages` rows.
- **`UatDriveClient`** — backs `drive_search`, `drive_file_get`. Reads
  `drive_*` rows.
- Atlassian + GitHub left out of the initial pass unless a scenario needs
  them.

The stubs share a single `ProjectionFixtureBackend` that's wired with the
scenario's seed and exposes per-feed-type helpers, so each `Uat*Client` is a
thin facade.

### Where the stubs live

`crates/arawn-tests/tests/uat_mocks/` — UAT-only, never compiled into the
production binary. Inside `uat.rs`'s `Harness::start_server`, a new
`register_mock_integrations(&self, scenario)` call wires the registry before
the engine boots.

### Side-effect tools

`calendar_event_create`, `gmail_send`, `slack_send` are write-side. The
draft-/schedule-with-confirmation scenarios need them to "stage" a draft and
await confirmation. The mock clients record the staged request to an in-memory
ledger the harness inspects via `required_evidence` / a new mechanical check.

## Implementation Plan

- **T-A — Connection plumbing.** `UatMockIntegration` (one per service) that
  reports `is_connected() == true`; harness hook to register a chosen set per
  scenario; baseline test confirming integration tools become visible.
- **T-B — `UatCalendarClient` + morning-briefing conversion.** First service
  end-to-end. `morning-briefing` keeps the corpus seed, registers Calendar,
  and the `required_evidence` says the agent must call `calendar_today_events`
  (new substrate — see Q1).
- **T-C — `UatGmailClient` + inbox-summary conversion.**
- **T-D — `UatSlackClient` + mention-scan conversion.**
- **T-E — Side-effect ledger + draft- / schedule-with-confirmation.** Staged-
  draft mechanism + scenarios that assert the draft was prepared but never
  "sent" (confirmation flow).
- **T-F — `UatDriveClient` + any remaining converts.** Drive last because no
  current scenario needs it; spike it for completeness.
- **T-G — Sharpen `required_evidence`** across the converted scenarios so the
  new mechanical check actually exercises live-tool retrieval (add
  `required_tool_name` if helpful — see Q1).

## Open Questions

- **Q1 — Evidence substrate.** Today `required_evidence` greps tool result
  *content*. For live-tool assertion we may also want `required_tool_name:
  Vec<String>` (the tool must be called by name at least once). Extend the
  mechanical struct or fold into `required_evidence` via a `tool::<name>`
  pseudo-substring? Decide in T-A.
- **Q2 — Stub fidelity.** How closely do `Uat*Client` return shapes need to
  mimic the real API? For UAT we need just enough that the agent-side tool
  impl returns deterministic output. Probably "fields the production tool
  actually reads" — verify per tool.
- **Q3 — Side-effect cancellation.** When the (mocked) user cancels a draft,
  what's the right transcript shape? Resolve in T-E.

## Alternatives Considered

- **Live integrations with OAuth in UAT.** Rejected — non-deterministic,
  brittle, slow, requires real accounts.
- **Inline tool stubs that bypass `IntegrationClient`.** Cheaper, but forks
  the test path from production and won't catch IntegrationClient regressions.
  Rejected.
- **Keep UAT corpus-only forever, mark it as such.** Rejected — the live path
  is half of what makes ARAWN useful and currently isn't tested at all.

## Status Updates

**2026-05-28 — Created.** Spun out as a follow-up after the morning-briefing
3/3 quality finding revealed UAT doesn't exercise the live integration path.
ARAWN-I-0061 closed the model + harness gaps the analysis surfaced; this
initiative closes the remaining fidelity gap.
