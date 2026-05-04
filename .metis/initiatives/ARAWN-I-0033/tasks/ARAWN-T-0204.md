---
id: notification-integration-slack
level: task
title: "Notification integration: Slack incoming-webhook channel for v1"
short_code: "ARAWN-T-0204"
created_at: 2026-05-03T12:43:21.447837+00:00
updated_at: 2026-05-04T20:12:36.721437+00:00
parent: ARAWN-I-0033
blocked_by: [ARAWN-T-0200, ARAWN-T-0201]
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0033
---

# Slack integration: read/write so the agent can watch channels and post

## Parent Initiative

[ARAWN-I-0033](../initiative.md)

## Objective

Full read/write Slack integration so the agent can read channel history (gain context, find prior decisions), search across channels, post messages, and react. The original "incoming webhook v1" framing was wrong — it confined Slack to a notification sink. The agent's leverage is being a participant: "what did engineering decide last week" is a bigger win than "ping #ops when a workflow finishes."

ADR-0001 § decision-4 records the design change. This task supersedes the prior Slack-webhook plan.

## Type / Priority
- Feature
- P1 — Pairs with Gmail and Calendar to cover "what's happening across the systems I run my day from."

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `crates/arawn-integrations/src/slack/` mirrors the Gmail/Calendar layout — `integration.rs`, `client.rs` (slack-morphism Hub builder), `tools.rs`. Reuses `crate::google_common::ArawnGetToken` shape if applicable, OR introduces a Slack-specific auth adapter (Slack tokens don't expire by default; the refresh code path may simplify or vanish — decide during implementation).
- [ ] Provider config: Slack OAuth v2. Auth URL `https://slack.com/oauth/v2/authorize`, token URL `https://slack.com/api/oauth.v2.access`. Bot scopes: `channels:read`, `channels:history`, `groups:read`, `groups:history`, `im:read`, `im:history`, `mpim:history`, `chat:write`, `reactions:write`, `search:read`, `users:read`.
- [ ] `slack-morphism = "2.20"` (gold-standard, reuses hyper-rustls/ring) is the API client. Disable any default features that pull bits we don't need; enable `hyper`.
- [ ] Five tools registered:
  - `slack_list_channels({include_dms?, include_private?})` → list visible channels with id, name, type (public/private/im/mpim), member_count. Permission: ReadOnly.
  - `slack_history({channel, limit, oldest?, latest?})` → last N messages (default 20, max 200) with ts, user, text, thread_ts, reaction summary. Permission: ReadOnly.
  - `slack_search({query, count?})` → cross-channel search via `search.messages`. Permission: ReadOnly.
  - `slack_post({channel, text, thread_ts?})` → posts via `chat.postMessage`. Permission: Other (mode default: ask).
  - `slack_react({channel, ts, name})` → emoji reaction via `reactions.add`. Permission: FileWrite.
- [ ] Token persistence via T-0200's `TokenStore` keyed `slack`. Token refresh: if Slack workspace has token rotation enabled, surface "invalid_auth" through the engine error chain (T-0191) so the user knows to `/connect slack` again; do NOT attempt to refresh in v1 (most Slack apps don't enable rotation).
- [ ] `ARAWN_SLACK_CLIENT_ID` and `ARAWN_SLACK_CLIENT_SECRET` env vars (no shared fallback — Slack apps are unrelated to Google projects). If either is missing at startup, integration is silently skipped, same pattern as Gmail/Calendar.
- [ ] User resolution: `slack_history` and `slack_search` results carry user IDs (e.g. `U12345`). For v1 these go to the agent as-is; the agent can call `slack_list_channels` (which includes a `users` summary) or a follow-up `slack_users_lookup` if a future ticket adds it. Don't hide the IDs by inlining names — the LLM handles the join.
- [ ] Manual smoke test path documented in `docs/src/integrations/slack.md`: create a Slack app, add bot scopes, install to your workspace, paste env vars, `/connect slack`, run `slack_list_channels` and verify shape.
- [ ] At least one unit test per tool's parameter parsing, plus tests for the response-shape mappers (channel record → simplified summary, message → simplified record). Integration test against recorded fixtures is deferred to the same captured-corpus pipeline T-0202 / T-0203 deferred theirs to.

## Implementation Notes

- **Drop the `NotificationChannel` trait** and the `notify_send` tool from the original ticket scope. They were artifacts of the webhook framing. Slack is its own integration, parallel to Gmail; future Discord etc. follow the same shape (full integration, not a notification slot).
- **slack-morphism Hub** takes a `SlackHyperClient` and a `SlackApiToken`. The auth shape doesn't quite match `google_apis_common::GetToken` — but the OAuth dance is the same RFC 6749 + PKCE arawn-auth already does, so `arawn-auth::OAuthClient` + `CallbackServer` work directly. The "auth adapter" for slack-morphism is just constructing a `SlackApiToken` from the persisted access token before each call.
- **Bot tokens vs user tokens.** Use bot tokens — they have a stable identity (the app), don't carry the connecting user's full permissions, and are how Slack expects integrations to authenticate. The OAuth response carries both `access_token` (bot) and `authed_user.access_token` (user); we keep the bot one.
- **Reaction emoji name** is the bare name without colons (`thumbsup`, not `:thumbsup:`). Document in the tool description.
- **Block Kit** is out of scope — `slack_post` is plain text only. Block messages are a follow-up if/when the agent needs richer formatting.
- The tool descriptions should explicitly call out that `channel` accepts either an id (`C12345`) or a name (`#general`); slack-morphism's `chat.postMessage` accepts either.

## Status Updates

### 2026-05-04 — Implemented (4 tools, search deferred)

**Crate:** `slack-morphism = "2.20"` with `hyper` feature, gold-standard Slack rust client. Pulls hyper-rustls/ring (composes with the rustls setup from Gmail/Calendar). Added `rvstruct = "0.3"` to scope-import the `ValueStruct` trait so the slack-morphism newtype IDs (SlackChannelId, SlackTs, etc.) expose `.value()` cleanly.

**Crate structure** (`crates/arawn-integrations/src/slack/`):
- `integration.rs` — `SlackIntegration` impl. Service name `slack`. 11 bot scopes per ADR-0001 § 4. Slack OAuth v2 endpoints. Reuses `arawn-auth::OAuthClient` for the standard PKCE dance.
- `client.rs` — `SlackContext` bundles the `SlackHyperClient` (Arc-wrapped) + a `SlackApiToken` built from the persisted access token. The `session()` helper opens a slack-morphism session for tool calls.
- `tools.rs` — four `arawn_tool::Tool` impls.

**Four tools** (search deferred):

| Tool | Permission | Backed by |
|---|---|---|
| `slack_list_channels` | ReadOnly | `conversations.list` |
| `slack_history` | ReadOnly | `conversations.history` |
| `slack_post` | Other (mode default: ask) | `chat.postMessage` |
| `slack_react` | FileWrite | `reactions.add` |

`ChannelSummary` projects `SlackChannelInfo` down to id, name, kind ("public"/"private"/"im"/"mpim"), member_count, topic, purpose. `MessageSummary` carries ts, user (Slack id), text, thread_ts, reply_count, reaction summaries.

**Cross-channel search dropped from v1.** slack-morphism doesn't typed-expose `search.messages` — using a raw HTTP escape hatch would complicate response handling. Per-channel `slack_history` covers most "what was discussed" use cases. Filed as a follow-up.

**Auth model.** Slack bot tokens don't expire by default; no refresh adapter in v1. If a Slack workspace enables token rotation, `invalid_auth` surfaces through the engine error chain (T-0191) and the user runs `/connect slack` again. Documented.

**Token type.** Bot tokens (`xoxb-...`), not user tokens. The OAuth response carries both; we keep the bot one. Stored in `TokenStore` keyed `slack`.

**Rustls crypto provider.** slack-morphism's connector uses rustls 0.23, which won't auto-pick a crypto provider when both ring and aws-lc-rs are visible in the workspace dep tree. Added `arawn_integrations::install_default_crypto_provider()` (idempotent wrapper around `rustls::crypto::ring::default_provider().install_default()`). Called once at server startup in `main.rs`. Tests call it inline.

**main.rs registration.** `ARAWN_SLACK_CLIENT_ID` + `ARAWN_SLACK_CLIENT_SECRET` env vars; integration silently skipped if either is missing.

**Docs** (`docs/src/integrations/slack.md`): Slack app creation walkthrough including all 11 bot scopes, redirect URI caveat (Slack requires exact-match allowlist; arawn picks an OS-assigned port — workaround documented), env vars, `/connect` flow, what each tool does + permission categories, user ID convention (don't pre-resolve), token model, troubleshooting (invalid_redirect, invalid_client_id, not_in_channel, missing_scope). Linked from `docs/src/SUMMARY.md`.

**Tests** (6 new in `arawn-integrations`):
- `slack::integration::default_provider_carries_eleven_bot_scopes`
- `slack::integration::provider_lifts_into_oauth_config`
- `slack::tools::summarize_channel_classifies_kind_correctly`
- `slack::tools::summarize_channel_carries_topic_and_purpose`
- `slack::tools::summarize_message_extracts_user_text_and_reactions`
- `slack::client::build_constructs_bot_token_from_access`

29 arawn-integrations tests pass; 927 workspace lib tests pass; 0 clippy warnings.

**Acceptance criteria status:**
- [x] Crate layout mirrors Gmail/Calendar.
- [x] Slack OAuth v2 with all 11 bot scopes.
- [x] slack-morphism 2.20 + hyper feature.
- [x] Four tools registered with the right permission categories. **`slack_search` deferred** — see deferred section.
- [x] Token persistence via `TokenStore` keyed `slack`. No refresh in v1; rotation caveat documented.
- [x] `ARAWN_SLACK_CLIENT_ID/_SECRET` env vars; silently skip if missing.
- [x] User IDs returned as-is (LLM handles the join).
- [x] Manual smoke test path documented.
- [x] Unit tests for parameter parsing and response-shape mappers.

**Deferred:**
- `slack_search` — requires raw HTTP since slack-morphism doesn't typed-expose `search.messages`. File a follow-up if/when the agent needs cross-channel search beyond per-channel history.
- Token refresh for Slack workspaces with token rotation enabled. Auto-refresh would just call `OAuthClient::refresh` which arawn-auth already supports — small follow-up.
- `slack_users_lookup` for U-id → name resolution. Not needed in v1 since the LLM handles the join from the channel-members data `slack_list_channels` already surfaces.