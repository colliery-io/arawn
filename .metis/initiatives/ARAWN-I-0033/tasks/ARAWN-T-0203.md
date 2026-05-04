---
id: google-calendar-integration-list
level: task
title: "Google Calendar integration: list upcoming, create event, find conflicts"
short_code: "ARAWN-T-0203"
created_at: 2026-05-03T12:43:21.447837+00:00
updated_at: 2026-05-04T19:31:11.612059+00:00
parent: ARAWN-I-0033
blocked_by: [ARAWN-T-0200, ARAWN-T-0201]
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0033
---

# Google Calendar integration: list upcoming, create event, find conflicts

## Parent Initiative

[ARAWN-I-0033](../initiative.md)

## Objective

Second concrete consumer of the integration framework. Implements `GoogleCalendarIntegration` plus three engine tools so the agent can answer "what's on my calendar," schedule new events, and check for conflicts before suggesting times.

## Type / Priority
- Feature
- P1 — Pairs with Gmail to cover the core "what's happening today" question.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Integration impl in the same crate as Gmail (or sibling). Provider config: Google OAuth, scope `calendar.events`.
- [ ] Three tools registered:
  - `calendar_upcoming({lookahead_hours, calendar_id?})` → next events. Default calendar = "primary". Returns title, start, end, location, attendees. Permission category: ReadOnly.
  - `calendar_create_event({title, start, end, attendees?, description?, calendar_id?})` → returns the event id + a calendar URL. Permission category: Other (mode default: ask).
  - `calendar_find_conflicts({start, end, calendar_id?})` → list of overlapping events in the window. Permission category: ReadOnly.
- [ ] Token storage and refresh inherited from T-0200 helpers — same code path as Gmail.
- [ ] **OAuth scope-merging consideration:** if Gmail is already connected, the same Google OAuth flow can request both `gmail.*` and `calendar.events`. Decide in implementation: separate per-service connect flows (simpler, requires user to consent twice) vs combined "Google" connection (better UX, more code). Default to **separate per-service** for v1 — matches the framework's "one service one credential" model. Revisit if friction is real.
- [ ] Integration test against recorded fixtures.
- [ ] Manual smoke: connect, run `calendar_upcoming({lookahead_hours: 24})`, verify shape.
- [ ] Docs: `docs/src/integrations/calendar.md` (mirrors Gmail's docs page).

## Implementation Notes

- Calendar API (v3) is friendlier than Gmail's — JSON-native, no MIME parsing.
- All times in/out of these tools use ISO 8601 RFC3339 strings with timezone. Don't try to do timezone math; let the model handle it. Document the convention in tool descriptions.
- `find_conflicts` should be precise (overlap, not "same day"). The model often needs this to suggest meeting times.
- Don't bother with attendee response state for v1 (accepted/declined/tentative). Just list them.
- Recurring events: the API returns instances; pass them through. Don't try to expand recurrence rules client-side.

## Status Updates

### 2026-05-04 — Implemented

**Refactor first.** Pulled the Google-shared plumbing out of `gmail/client.rs` into a new `crate::google_common` module before adding Calendar:
- `ArawnGetToken` (the `google_apis_common::GetToken` impl backed by `arawn-auth`).
- `HttpsConnector` typedef + `build_https_client()` for the hyper-util/hyper-rustls setup.
- `TokenStoreHandle` carrying `(data_dir, service_name)` so refreshed tokens land under the right `TokenStore` key.

`gmail/client.rs` is now a 25-line wrapper that just builds `Gmail<HttpsConnector>` from the shared plumbing. Calendar's `client.rs` is symmetrical for `CalendarHub<HttpsConnector>`.

**Crate structure** (`crates/arawn-integrations/src/calendar/`): `integration.rs`, `client.rs`, `tools.rs`. Service name `google_calendar`. Single OAuth scope `https://www.googleapis.com/auth/calendar.events`.

**Three tools registered:**

| Tool | Permission | Backed by |
|---|---|---|
| `calendar_upcoming` | ReadOnly | `events.list` with `singleEvents=true&orderBy=startTime&timeMin&timeMax` |
| `calendar_create_event` | Other (mode default: ask) | `events.insert` |
| `calendar_find_conflicts` | ReadOnly | `freebusy.query` (faster + cleaner than walking events.list) |

`EventSummary` carries id, summary, description, location, start, end, attendees (just emails per ticket — no response state), html_link. `format_event_datetime` prefers `dateTime` (RFC3339 with offset) and falls back to `date` for all-day events.

**Time format convention** (per ticket): all I/O is wire-format RFC3339 with offset. The tools don't normalize timezones; the model handles "tomorrow at 3pm Pacific" → RFC3339. Documented in tool descriptions and the docs page.

**OAuth scope-merging decision** (per ADR-0001 and the ticket's "default to separate per-service"): Calendar reads `ARAWN_GCAL_CLIENT_ID/_SECRET` first, falls back to `ARAWN_GOOGLE_CLIENT_ID/_SECRET` so a user with one shared OAuth project can configure once. Each service still completes its own `/connect` flow with its own scope set — no "combined Google" UX, just shared credentials when the user wants them. Documented both patterns in `docs/src/integrations/calendar.md`.

**main.rs registration:** parallels Gmail. Reads the four env vars, constructs `GoogleCalendarIntegration`, registers via `service.register_integration`, registers the three tools.

**Docs** (`docs/src/integrations/calendar.md`): Cloud Console setup (or "reuse Gmail's project"), env var patterns, `/connect` flow, what each tool does, time format convention, recurring events caveat, troubleshooting (Calendar API not enabled, RFC3339 missing offset, integration skipped). Linked from `docs/src/SUMMARY.md`.

**Tests** (6 new in `arawn-integrations`):
- `calendar::integration::default_provider_has_calendar_events_scope`
- `calendar::integration::provider_lifts_into_oauth_config`
- `calendar::tools::format_event_datetime_prefers_datetime_over_date`
- `calendar::tools::format_event_datetime_falls_back_to_date_for_all_day`
- `calendar::tools::summary_from_event_extracts_attendee_emails`
- `calendar::tools::parse_rfc3339_accepts_offset_and_z`

Plus `google_common::tests::unexpired_token_returned_directly_no_refresh` migrated from gmail/client.rs (no behavior change).

23 arawn-integrations tests pass; 921 workspace lib tests pass; 0 clippy warnings.

**Acceptance criteria status:**
- [x] Integration impl in `arawn-integrations` (sibling module to Gmail).
- [x] Three tools registered with the right permission categories.
- [x] Token storage + refresh shared with Gmail via `google_common::ArawnGetToken`.
- [x] OAuth scope-merging: separate per-service flows; shared env-var fallback supports the one-OAuth-project case.
- [x] Manual smoke test path documented.
- [x] Docs page mirrors Gmail's.

**Deferred:**
- Recorded-fixtures integration test (same scope cut as T-0202; deferred to a follow-up that builds a captured-corpus pipeline).
- Attendee response state — explicitly out of scope per ticket.
- Recurring event expansion — `singleEvents=true` lets the API expand for us.