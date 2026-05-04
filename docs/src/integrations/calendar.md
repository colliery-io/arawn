# Google Calendar

Lets the agent answer "what's on my calendar tomorrow," schedule new events, and check for conflicts before suggesting times. Three tools land when Calendar is configured: `calendar_upcoming`, `calendar_create_event`, `calendar_find_conflicts`.

## Setup

### 1. Reuse or create a Google Cloud OAuth client

If you've already set up Gmail (see [Gmail integration](./gmail.md)), you can use the same OAuth client by setting the shared env vars below — both integrations will hit the same Google Cloud project. If not, follow the same Cloud Console walkthrough as Gmail and just add the Calendar API:

1. [Google Cloud Console](https://console.cloud.google.com/) → your project.
2. **APIs & Services → Library** → enable **Google Calendar API**.
3. **Credentials** → use your existing OAuth client ID, OR create a new Desktop OAuth client if you want isolation.

### 2. Set environment variables

Two patterns:

**Shared with Gmail** (one OAuth project for all Google integrations):

```sh
export ARAWN_GOOGLE_CLIENT_ID="..."
export ARAWN_GOOGLE_CLIENT_SECRET="..."
```

`ARAWN_GMAIL_CLIENT_ID` / `_SECRET` and `ARAWN_GCAL_CLIENT_ID` / `_SECRET` both fall back to these when unset.

**Per-service** (separate Google projects per integration):

```sh
export ARAWN_GCAL_CLIENT_ID="..."
export ARAWN_GCAL_CLIENT_SECRET="..."
```

Per-service overrides shared values when both are set.

### 3. Connect

```
/connect google_calendar
```

OAuth flow opens in your browser, you grant the `calendar.events` scope, the server captures the callback and persists tokens. You'll see `ℹ [integration] google_calendar connected` in the TUI.

### 4. Verify

```
/integrations
```

Should show:

```
| Service          | Connected |
|------------------|-----------|
| gmail            | ✓ (if also configured) |
| google_calendar  | ✓ |
```

Try it:

> What's on my calendar in the next 24 hours?

The agent calls `calendar_upcoming({lookahead_hours: 24})`.

## What the tools do

| Tool | Returns | Permission category |
|---|---|---|
| `calendar_upcoming({lookahead_hours, calendar_id?})` | Events ordered by start time with id, title, start/end (RFC3339), location, attendees, html_link. | ReadOnly |
| `calendar_create_event({title, start, end, attendees?, description?, location?, calendar_id?})` | New event id + html_link. start/end must be RFC3339 with timezone (`2026-05-08T10:00:00-04:00`). | Other (mode default: ask) |
| `calendar_find_conflicts({start, end, calendar_id?})` | Busy intervals overlapping the window, plus `any_conflicts: bool`. Uses the freebusy API for fast precomputed busy blocks. | ReadOnly |

`calendar_id` defaults to `"primary"` everywhere. Pass an explicit calendar id (`a@b.com` or a calendar resource id) to operate on a different calendar.

### Time format conventions

All time inputs and outputs are wire-format **RFC 3339** (`YYYY-MM-DDTHH:MM:SS+ZZ:ZZ` or `Z` for UTC). The tools don't do timezone math — the model handles "tomorrow at 3pm Pacific" → RFC3339 reasoning in its response. This keeps the tool layer dumb and the model's reasoning auditable.

For all-day events read by `calendar_upcoming`, the `start` / `end` fields fall back to `YYYY-MM-DD` strings (Google's all-day representation). The model should treat date-only values as inclusive of the whole day.

## Disconnecting

```
/disconnect google_calendar
```

Removes the stored token. The integration stays registered.

## Caveats

- **Recurring events.** `calendar_upcoming` requests `singleEvents=true` so the API expands recurrences into individual instances within the lookahead window. Long lookaheads on calendars with frequent recurrences can return a lot of rows.
- **Attendee response state** (accepted / tentative / declined) is not surfaced in v1. If you need it, ask the agent to use the underlying `events.get` directly — that's a follow-up.
- **No timezone normalization.** If your calendars span multiple timezones, the agent sees raw RFC3339 with whatever offset Google returns. Reason about that when you ask "is this event tomorrow."
- **Single-account.** One Google account per arawn install. Multi-account is a future redesign.

## Troubleshooting

### `connection FAILED: HTTP 403 — Calendar API has not been used in project N before or it is disabled`

The Calendar API isn't enabled on your Cloud project. Visit **APIs & Services → Library** and enable it.

### `calendar_create_event 'start' must be RFC3339`

The start/end strings must include a timezone offset (`-04:00`) or `Z`. `2026-05-08T10:00:00` (no zone) is rejected — calendars without timezone are ambiguous.

### `Google Calendar integration skipped` in server log

None of the four env vars (`ARAWN_GCAL_CLIENT_ID`, `ARAWN_GCAL_CLIENT_SECRET`, `ARAWN_GOOGLE_CLIENT_ID`, `ARAWN_GOOGLE_CLIENT_SECRET`) are set. Set at least one pair and restart `arawn serve`.

### Browser doesn't open / stuck on "Waiting for browser authorization"

Same as Gmail. The auth URL is always printed; copy/paste into a browser if `open` / `xdg-open` / `cmd /c start` didn't fire automatically.
