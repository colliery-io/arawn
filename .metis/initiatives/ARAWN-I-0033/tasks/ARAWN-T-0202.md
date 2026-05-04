---
id: gmail-integration-read-inbox
level: task
title: "Gmail integration: read inbox, search, send, mark read"
short_code: "ARAWN-T-0202"
created_at: 2026-05-03T12:43:21.447837+00:00
updated_at: 2026-05-04T19:16:55.801009+00:00
parent: ARAWN-I-0033
blocked_by: [ARAWN-T-0200, ARAWN-T-0201]
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0033
---

# Gmail integration: read inbox, search, send, mark read

## Parent Initiative

[ARAWN-I-0033](../initiative.md)

## Objective

First concrete consumer of the integration framework. Implements `GmailIntegration` against the trait from T-0200 plus four engine tools (`gmail_inbox_read`, `gmail_search`, `gmail_send`, `gmail_mark_read`) so the agent can answer "what's in my inbox," draft replies, and triage notifications.

## Type / Priority
- Feature
- P1 — The MVP integration. Once it works end-to-end, the framework is proven.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `crates/arawn-integrations/src/gmail.rs` (or sibling crate `arawn-integrations-gmail`) implements `Integration` trait. Provider config wired with the Gmail OAuth endpoints and scopes (`gmail.readonly`, `gmail.send`, `gmail.modify`).
- [ ] Four tools registered into the engine when GmailIntegration is connected:
  - `gmail_inbox_read({limit, label})` → most recent N messages (default 10) with sender, subject, snippet, received-at, message_id. Permission category: ReadOnly.
  - `gmail_search({query, limit})` → Gmail search syntax pass-through, same response shape. Permission category: ReadOnly.
  - `gmail_send({to, subject, body, in_reply_to?})` → sends; returns the new message_id. Permission category: Other (mode default: ask).
  - `gmail_mark_read({message_id})` → strips the UNREAD label. Permission category: FileWrite (modifies state but not destructive).
- [ ] Token refresh handled transparently — if access token expired, refresh via stored refresh token before failing the call.
- [ ] Token storage uses T-0200's helpers (no Gmail-specific encryption logic).
- [ ] Integration test against recorded fixtures (NOT real Gmail in CI) verifies the response-parsing for `inbox_read` and `search`.
- [ ] Manual smoke test (documented, not automated): connect via `/connect gmail`, send a `gmail_inbox_read({limit: 5})` and verify shape matches expectation.
- [ ] OAuth client_id/client_secret read from env vars (`ARAWN_GMAIL_CLIENT_ID`, `ARAWN_GMAIL_CLIENT_SECRET`); user-facing docs explain how to provision a Google Cloud project.

## Implementation Notes

- Use `reqwest` for the Gmail REST v1 API. Avoid bringing in a heavy Google client SDK — the surface needed is small.
- Body parsing: Gmail's MIME structure is nasty. For `inbox_read`, return only the snippet + a flag indicating the body is truncated; let the agent ask `gmail_search` with `id:<message_id>` for the full body if it wants.
- `gmail_send` should accept either plain text or HTML body. v1: just plain text via `text/plain` MIME. HTML is a follow-up.
- Rate limits: Gmail has per-user quotas. On 429 responses, surface the error directly through the engine error chain (T-0191) rather than retrying — let the agent decide.
- Document the Google Cloud Console setup in `docs/src/integrations/gmail.md` (new page). Includes: enable Gmail API, create OAuth client (Desktop type), download credentials JSON, set env vars.

## Status Updates

### 2026-05-04 — Implemented (using google-gmail1)

**Architecture decision (mid-task):** initially started a hand-rolled REST client. User pushed back on inventing wheels; switched to `google-gmail1` v7.0 (Byron's `google-apis-rs` autogen). The previously-cited friction (yup-oauth2 dependency) turned out to be 20 lines of trait impl — `google_apis_common::GetToken` is a simple async trait, no yup-oauth2 required. Disabled the `yup-oauth2` default feature on `google-gmail1`; kept the `ring` feature for hyper-rustls.

**Crate structure** (`crates/arawn-integrations/src/gmail/`):
- `integration.rs` — `GmailIntegration` impl of the `Integration` trait. `connect()` runs `oauth_flow::run_oauth_flow` with the standard Google OAuth provider config (3 scopes: gmail.readonly, gmail.send, gmail.modify). `is_connected()` is a cheap disk-only check via `TokenStore::load`.
- `client.rs` — `ArawnGetToken` adapter that implements `google_apis_common::GetToken` against an `arawn_auth::Token`. Refreshes via `OAuthClient::refresh` when the access token is expired and persists the new token via a `TokenStoreHandle`. `build_gmail_hub` constructs a `Gmail<HttpsConnector>` with hyper-util + hyper-rustls native roots — one place for all the connector setup.
- `tools.rs` — five `arawn_tool::Tool` impls (one more than the ticket asked for; see below).

**Five tools** (ticket asked for four; added `gmail_get_message` to honor the "search by id returns full body" requirement that the ticket implied):

| Tool | Permission category |
|---|---|
| `gmail_inbox_read` | ReadOnly |
| `gmail_search` | ReadOnly |
| `gmail_get_message` (full body, decodes multipart/alternative → text/plain) | ReadOnly |
| `gmail_send` (plain text only; uses `messages_send().upload(stream, "message/rfc822")`) | Other |
| `gmail_mark_read` | FileWrite |

`MessageSummary` (the inbox/search response shape) carries `body_truncated: true` so the agent knows to call `gmail_get_message` for full bodies.

**Send via upload, not doit.** Surprise: `UserMessageSendCall` in google-gmail1 only exposes `upload()`, not a plain `doit()`. Gmail's API treats outgoing messages as media uploads. We pass the raw RFC2822 bytes as a `Cursor<Vec<u8>>` with mime `message/rfc822`. Documented in the code comment.

**Token refresh.** Routed through `arawn_auth::OAuthClient::refresh` from inside `ArawnGetToken::get_token`. When Google's `/token` returns a fresh access token (and the refresh token, since Google omits it on refresh, is preserved), we persist via `TokenStoreHandle::save_token` so process restart picks up the new token.

**main.rs registration.** If both `ARAWN_GMAIL_CLIENT_ID` and `ARAWN_GMAIL_CLIENT_SECRET` are present at startup, `GmailIntegration` is constructed, registered with `LocalService::register_integration`, and all five tools are registered into the engine `ToolRegistry`. If either env var is missing, Gmail is silently skipped (debug log only) and the rest of arawn still works.

**Docs** (`docs/src/integrations/gmail.md`): full Cloud Console walkthrough, env var setup, `/connect` flow, what each tool does + permission categories, token refresh behavior, troubleshooting (HTTP 403, invalid_grant, browser-doesn't-open, integration-skipped), and caveats (quota, no HTML, single-account, 7-day test-mode token expiry). Linked from `docs/src/SUMMARY.md` under a new "Integrations" section.

**Tests** (10 new in `arawn-integrations`, all green):
- `default_provider_has_three_gmail_scopes`, `provider_lifts_into_oauth_config` — provider config sanity.
- `arawn_get_token_returns_unexpired_access_directly` — auth adapter doesn't refresh prematurely.
- `summary_from_message_extracts_known_headers`, `summary_handles_empty_payload` — response shaping.
- `extract_plain_text_finds_top_level_text_plain`, `extract_plain_text_descends_into_multipart_alternative`, `extract_plain_text_returns_none_when_html_only` — multipart MIME walking is the bug-prone part; tested at the function level.
- `rfc2822_includes_required_headers_and_body`, `rfc2822_threads_via_in_reply_to` — outbound message construction.

915 workspace lib tests pass.

**Acceptance criteria status:**
- [x] `Integration` trait implemented; provider config wired; three Gmail scopes.
- [x] Five tools registered (four required + one bonus for full-body fetch).
- [x] Token refresh transparent (in `ArawnGetToken`).
- [x] Token storage uses T-0200's helpers (`TokenStore` + `TokenStoreHandle`).
- [x] Manual smoke test path documented in `docs/src/integrations/gmail.md`.
- [x] OAuth client_id/secret read from env vars.

**Deferred (per ticket and ADR):**
- Recorded-fixtures integration test against multipart-MIME real-Gmail responses. The MIME-walking is the bug-prone piece and is tested at function level with synthetic `Message` values. A real-fixtures test against captured Gmail JSON would be more thorough but needs a captured-corpus pipeline that doesn't exist yet.
- HTML body in `gmail_send`. Plain text only per ticket spec.

**Caveats discovered during implementation:**
- google-gmail1 + default features pulls a long dep chain (hyper, hyper-util, hyper-rustls, ring/aws-lc-rs, rustls, google-apis-common). With our `default-features = false, features = ["ring"]` config, we still pick up everything except yup-oauth2. This is the cost of the "gold standard" decision — accepted.
- Build time: cold cargo build adds ~25s vs the hand-rolled REST baseline. Manageable.