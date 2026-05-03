---
id: notification-integration-slack
level: task
title: "Notification integration: Slack incoming-webhook channel for v1"
short_code: "ARAWN-T-0204"
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

# Notification integration: Slack incoming-webhook channel for v1

## Parent Initiative

[ARAWN-I-0033](../initiative.md)

## Objective

Outbound-only notification channel so the agent (and workflows from T-0198) can post messages to a Slack channel. Per ARAWN-A-0001's resolved Slack-auth question: incoming webhook for v1, not full OAuth — the goal is "agent can ping me," not "manage Slack."

## Type / Priority
- Feature
- P2 — Important for workflow / scheduled-job notifications, but lower than Gmail/Calendar for interactive use.

## Acceptance Criteria

- [ ] `NotificationChannel` trait abstraction (in `arawn-integrations`) so a future Discord/email/etc. channel can plug in alongside Slack:
  ```rust
  #[async_trait]
  pub trait NotificationChannel: Send + Sync {
      fn name(&self) -> &str;
      async fn send(&self, msg: &Notification) -> Result<(), IntegrationError>;
  }
  ```
- [ ] `SlackWebhookChannel` impl. Webhook URL stored via T-0200's credential helpers (encrypted at rest under `~/.arawn/integrations/slack/credentials.age`), NOT in `arawn.toml` or env vars (it's a secret).
- [ ] `/connect slack` flow is a small variant of T-0201's OAuth: instead of a browser callback, it prompts in-TUI for the webhook URL, validates with a probe POST, and stores. (Modify T-0201 if needed to accept this "manual paste" credential mode.)
- [ ] One engine tool: `notify_send({channel, message, severity?})` — `channel` is the integration name ("slack"; "discord" later), `severity` defaults to "info". Permission category: Other.
- [ ] `Notification` type carries `text: String` plus optional fields (`title`, `severity`, `link`). Slack-specific formatting (block kit, etc.) is converted internally — the agent never sees it.
- [ ] Workflow-friendly: workflows from T-0198 can `use arawn_integrations::notify` for the same shape. Document in `docs/src/integrations/notifications.md`.
- [ ] Integration test against a mock HTTP server.

## Implementation Notes

- Webhook is dead simple: `POST <webhook_url> {"text": "..."}`. Don't get clever for v1.
- The "manual paste" credential mode for `/connect slack` is the wedge for future non-OAuth integrations. Keep its UX consistent with the OAuth flow (same `/connect <service>` entry point, just a different inner flow).
- A future "Slack OAuth" path (multi-channel send, channel discovery, ephemeral messages) would be a separate `SlackOAuthChannel` impl alongside `SlackWebhookChannel` — not a rewrite. Document this in the trait docs.
- For workflow use: a workflow that wants to notify on completion shouldn't have to authenticate separately. The integration registry on `LocalService` is shared; workflows inherit it.

## Status Updates

*To be added during implementation*
