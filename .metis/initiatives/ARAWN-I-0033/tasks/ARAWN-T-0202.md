---
id: gmail-integration-read-inbox
level: task
title: "Gmail integration: read inbox, search, send, mark read"
short_code: "ARAWN-T-0202"
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

# Gmail integration: read inbox, search, send, mark read

## Parent Initiative

[ARAWN-I-0033](../initiative.md)

## Objective

First concrete consumer of the integration framework. Implements `GmailIntegration` against the trait from T-0200 plus four engine tools (`gmail_inbox_read`, `gmail_search`, `gmail_send`, `gmail_mark_read`) so the agent can answer "what's in my inbox," draft replies, and triage notifications.

## Type / Priority
- Feature
- P1 — The MVP integration. Once it works end-to-end, the framework is proven.

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

*To be added during implementation*
