---
id: slack-integration-messaging-push
level: initiative
title: "Slack Integration — Messaging, Push, Nag Loop, and Channel Monitoring"
short_code: "ARAWN-I-0031"
created_at: 2026-04-17T02:46:58.210079+00:00
updated_at: 2026-04-17T02:46:58.210079+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: M
initiative_id: slack-integration-messaging-push
---

# Slack Integration — Messaging, Push, Nag Loop, and Channel Monitoring Initiative

## Context

Slack is the user's "always-on" channel: where arawn reaches out for attention (push) and where it monitors threads it's been told to watch (messaging). It serves dual duty as both a `MessagingProvider` and the first `PushProvider`, both backed by the same Slack client and OAuth token.

This initiative also lands the **nag workflow** (cron-based: arawn looks at the `task_reference` memories from I-0030, finds overdue items, pushes a reminder) and a **channel monitoring workflow template** (cron-based: arawn polls a Slack channel since the last check, filters for relevant messages, surfaces them into a session or push).

Monitoring is workflow-driven on purpose — no always-on Slack Socket Mode connection. This keeps arawn idle when the user isn't asking it to do anything.

## Goals & Non-Goals

**Goals:**
- `providers/slack/messaging.rs` implementing `MessagingProvider` (send, read, list channels).
- `providers/slack/push.rs` implementing `PushProvider`, sharing the underlying Slack client.
- `providers/slack/auth.rs` with Slack OAuth scopes (`chat:write`, `channels:read`, `channels:history`, `groups:read`, `groups:history`, `im:write`).
- Engine tools: `message_send`, `message_read`, `push_notify`.
- Nag workflow (Cloacina cron task) wired against both `memory_search` (for `task_reference` overdue items) and `PushProvider`. Configurable cadence (default: every 30 min).
- Channel monitoring workflow template (`workflow_create`-installable). Polls a target channel since last check, filters for matches against a user-supplied predicate, pushes summary into a session.
- UAT scenarios: agent sends a message, agent reads recent channel history, push notification arrives in Slack, nag workflow surfaces an overdue Google task as a Slack push.

**Non-Goals:**
- Slack Socket Mode / always-on event listening.
- Multi-workspace Slack (single workspace per config).
- Slash commands / interactive Slack components — arawn reaches out, Slack does not reach in.
- Reply-threading semantics beyond writing into an existing thread by `thread_ts`.
- Discord / Telegram / SMS providers.

## Requirements **[CONDITIONAL: Requirements-Heavy Initiative]**

{Delete if not a requirements-focused initiative}

### User Requirements
- **User Characteristics**: {Technical background, experience level, etc.}
- **System Functionality**: {What users expect the system to do}
- **User Interfaces**: {How users will interact with the system}

### System Requirements
- **Functional Requirements**: {What the system should do - use unique identifiers}
  - REQ-001: {Functional requirement 1}
  - REQ-002: {Functional requirement 2}
- **Non-Functional Requirements**: {How the system should behave}
  - NFR-001: {Performance requirement}
  - NFR-002: {Security requirement}

## Use Cases **[CONDITIONAL: User-Facing Initiative]**

{Delete if not user-facing}

### Use Case 1: {Use Case Name}
- **Actor**: {Who performs this action}
- **Scenario**: {Step-by-step interaction}
- **Expected Outcome**: {What should happen}

### Use Case 2: {Use Case Name}
- **Actor**: {Who performs this action}
- **Scenario**: {Step-by-step interaction}
- **Expected Outcome**: {What should happen}

## Architecture **[CONDITIONAL: Technically Complex Initiative]**

{Delete if not technically complex}

### Overview
{High-level architectural approach}

### Component Diagrams
{Describe or link to component diagrams}

### Class Diagrams
{Describe or link to class diagrams - for OOP systems}

### Sequence Diagrams
{Describe or link to sequence diagrams - for interaction flows}

### Deployment Diagrams
{Describe or link to deployment diagrams - for infrastructure}

## Architecture

### Layout

```
arawn-integration/src/providers/slack/
├── mod.rs        — re-exports + SlackClient (shared http+token wrapper)
├── auth.rs       — Slack OAuth scope strings
├── messaging.rs  — MessagingProvider impl
├── push.rs       — PushProvider impl (delegates to messaging)
└── types.rs      — wire types

arawn-workflow/templates/
├── nag.toml             — nag cron workflow
└── channel-monitor.toml — channel monitor template (user fills in target)
```

### Shared `SlackClient`

Holds the Slack bot/user token (from the encrypted store), the `reqwest::Client`, and rate-limit state. `messaging.rs` and `push.rs` both borrow it. `push.rs::send_notification` is essentially `messaging.rs::send_message` to the configured `default_channel` with notification-formatted text — code reuse, not duplication.

### Nag workflow

Implemented as a Cloacina cron task that on each tick:
1. Calls `memory_search` for `task_reference` entities where `due_at < now` and not yet pushed (state tracked in workflow context).
2. For each, calls `TaskListProvider::get_task` to confirm the task isn't already complete.
3. For each still-open overdue task, calls `PushProvider::send_notification` with a short summary.
4. Records the push timestamp on the memory entity to avoid spamming.

Cadence: default 30 min, configurable via the workflow's `[schedule]` block.

### Channel monitor template

A template the user installs via `workflow_create monitor-slack-<name>`. Parameters:
- `channel`: `#name` or channel ID
- `cadence`: cron expression (default `*/5 * * * *` — every 5 min)
- `predicate`: a free-text description of what to surface (interpreted by an LLM call)
- `action`: one of `"push"` (send a Slack DM summary) or `"session"` (open a new arawn session with the matched messages as context)

On each tick: read messages since last seen `ts`, filter via cheap LLM call, dispatch per `action`. State (last-seen `ts`) lives in the workflow's persistent context.

## Detailed Design

- **Token type**: prefer Bot tokens (`xoxb-...`) for v1. User tokens add complexity (per-user OAuth, per-user rate limits) without buying us much for the personal-assistant use case. Document the limitation: arawn posts as itself, not as the user.
- **Rate limiting**: Slack publishes per-method tier limits (Tier 1 = 1+ rpm, ..., Tier 4 = 100+ rpm). Wrap the client in a token-bucket per tier. On HTTP 429, honour `Retry-After`.
- **Push formatting**: `Notification { title, body, urgency }` → Slack message with markdown formatting; urgency `High` adds a `<!here>` mention and a 🔴 prefix.
- **Channel resolution**: accept both `#channel-name` and `Cxxxxxxx` IDs; resolve names to IDs via `conversations.list` cached for 5 min.
- **Read freshness**: `message_read` accepts a `since: Option<DateTime<Utc>>` — defaults to "since arawn last read this channel" tracked per-`(channel, session)` in memory.
- **Nag idempotency**: each `task_reference` memory gets a `last_nagged_at` field; nag workflow won't re-push within a configurable cooloff (default 4h).

## Use Cases

### UC-1: agent pushes a workflow result to Slack
- **Actor**: arawn (autonomous)
- **Scenario**: a long-running workflow finishes; agent calls `push_notify` with the result summary.
- **Expected**: user receives a Slack DM (or message in `default_channel`) with the summary.

### UC-2: nag for overdue task
- **Actor**: arawn (cron)
- **Scenario**: nag workflow ticks; finds a `task_reference` with `due_at` in the past and `last_nagged_at` > 4h ago; pushes "Reminder: still need to <task description>" to Slack.
- **Expected**: user sees one push per overdue task, no duplicates within cooloff.

### UC-3: channel monitor surfaces a relevant message
- **Actor**: arawn (cron)
- **Scenario**: user installs a monitor on `#oncall` with predicate "anything that mentions PagerDuty or production outage". Workflow ticks every 5 min; new matching message arrives; arawn pushes a summary.
- **Expected**: notification within one tick of the message appearing.

## UI/UX Design **[CONDITIONAL: Frontend Initiative]**

{Delete if no UI components}

### User Interface Mockups
{Describe or link to UI mockups}

### User Flows
{Describe key user interaction flows}

### Design System Integration
{How this fits with existing design patterns}

## Testing Strategy **[CONDITIONAL: Separate Testing Initiative]**

{Delete if covered by separate testing initiative}

### Unit Testing
- **Strategy**: {Approach to unit testing}
- **Coverage Target**: {Expected coverage percentage}
- **Tools**: {Testing frameworks and tools}

### Integration Testing
- **Strategy**: {Approach to integration testing}
- **Test Environment**: {Where integration tests run}
- **Data Management**: {Test data strategy}

### System Testing
- **Strategy**: {End-to-end testing approach}
- **User Acceptance**: {How UAT will be conducted}
- **Performance Testing**: {Load and stress testing}

### Test Selection
{Criteria for determining what to test}

### Bug Tracking
{How defects will be managed and prioritized}

## Alternatives Considered

**1. Email as PushProvider instead of Slack.**
Rejected for v1: SMTP/IMAP requires more setup, deliverability is fragile, and the user's stated channel of choice is Slack. Email is a future provider.

**2. Use Slack Socket Mode for monitoring.**
Rejected: requires an always-on connection and an Enterprise-style app config. Cron polling is sufficient for the personal-assistant use case and matches the rest of arawn's "idle until invoked" posture.

**3. Combine PushProvider into MessagingProvider as one trait.**
Rejected (in I-0029 already): they're conceptually distinct (push = "interrupt the user" with possibly different escalation policy; messaging = "read/write conversation"). Keeping them separate lets us swap providers independently — e.g., switch push to Pushover/PagerDuty later without touching messaging.

**4. Land nag and channel-monitor as a separate fourth initiative.**
Considered. Rejected because both depend on Push being live and have minimal additional design surface — folding them into Slack avoids a tiny initiative that would be 80% wiring.

## Implementation Plan

Depends on ARAWN-I-0029 and ARAWN-I-0030 being complete.

1. **SlackClient + auth scopes.**
   Shared `SlackClient` wrapping reqwest + token store + tier-based rate limiting. `auth.rs` defines scope set. Wire `arawn setup slack` to use it.

2. **MessagingProvider for Slack.**
   Implement `send_message`, `read_messages`, `list_channels`. Channel name → ID cache. Mock-server tests; live test gated on `SLACK_BOT_TOKEN`.

3. **PushProvider for Slack.**
   Thin wrapper over `MessagingProvider::send_message` posting to `default_channel`. Urgency formatting.

4. **Engine tools: message_send, message_read, push_notify.**
   Conditional registration on capability presence. Default permission rules requiring approval for first-write to a new channel (replays after).

5. **Nag workflow.**
   Cron task definition. Memory query for overdue `task_reference`. Idempotency via `last_nagged_at`. Configurable cadence.

6. **Channel monitor template.**
   `arawn-workflow/templates/channel-monitor.toml` with parameters. Last-seen-`ts` state. LLM-backed predicate matching using the cheap pool entry from ARAWN-I-0027 if present.

7. **UAT scenarios.**
   Live Slack workspace tests gated on env: send/read; push; nag end-to-end (create Google task with past due date → wait one tick → assert push arrived); channel monitor with a planted matching message.