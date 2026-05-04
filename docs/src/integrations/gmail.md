# Gmail

The Gmail integration lets the agent read your inbox, search messages, send mail, and mark messages read. Five tools land in the engine when Gmail is configured: `gmail_inbox_read`, `gmail_search`, `gmail_get_message`, `gmail_send`, `gmail_mark_read`.

## Setup

### 1. Create a Google Cloud project + OAuth client

You need a Google Cloud project with the Gmail API enabled and a Desktop OAuth 2.0 client. arawn ships without baked-in OAuth credentials so each user owns their own quota.

1. Visit the [Google Cloud Console](https://console.cloud.google.com/) and create a new project (or pick an existing one).
2. **APIs & Services → Library** — search for "Gmail API" and enable it.
3. **APIs & Services → OAuth consent screen** — pick "External", give it a name (e.g. "arawn"), and add your Google account as a test user. Don't worry about app verification; arawn is single-user.
4. **APIs & Services → Credentials → Create Credentials → OAuth client ID** — pick **Desktop app**. After creation, download the JSON or copy the client ID and client secret.

### 2. Set environment variables

Before running `arawn serve`:

```sh
export ARAWN_GMAIL_CLIENT_ID="...your client id..."
export ARAWN_GMAIL_CLIENT_SECRET="...your client secret..."
```

If both are present at startup, the server registers the Gmail integration and its five tools. If either is missing, Gmail is silently skipped and the rest of arawn still works.

### 3. Connect your account

In the TUI:

```
/connect gmail
```

The TUI:

1. Asks the server to start the OAuth flow.
2. Opens the Google authorization URL in your browser (or prints it for copy/paste).
3. You sign in, grant the three scopes (read, send, modify), and Google redirects back to `http://127.0.0.1:<port>/oauth/callback` where the server captures the code.
4. The server exchanges the code for refresh + access tokens, stores them encrypted under `~/.arawn/tokens/`, and broadcasts a `ServerNotice` you'll see as `ℹ [integration] gmail connected`.

### 4. Verify

```
/integrations
```

Should show:

```
| Service | Connected |
|---------|-----------|
| gmail   | ✓         |
```

Then ask the agent something concrete:

> What's in my inbox? Top 5 only.

The agent will call `gmail_inbox_read` with `limit=5`.

## What the tools do

| Tool | What it returns | Permission category |
|---|---|---|
| `gmail_inbox_read` | Last N messages with sender/subject/date/snippet. Body always truncated. | ReadOnly |
| `gmail_search` | Same shape; takes a Gmail search query (`from:alice has:attachment newer_than:7d`). | ReadOnly |
| `gmail_get_message` | Full plain-text body for one message id (decodes multipart/alternative for you). | ReadOnly |
| `gmail_send` | Sends a plain-text message; returns the new message id. HTML is not yet supported. | Other (mode default: ask) |
| `gmail_mark_read` | Strips the `UNREAD` label. | FileWrite |

## Disconnecting

```
/disconnect gmail
```

Removes the stored token. The integration stays registered — you can `/connect gmail` again later. To prevent the integration from registering at all, unset the `ARAWN_GMAIL_*` env vars before starting `arawn serve`.

## Token refresh

Access tokens expire after an hour. The integration refreshes them transparently — when an API call sees the token is expired, it hits Google's `/token` endpoint with the stored refresh token and persists the new access token before retrying. You don't need to re-`/connect` unless you revoke the grant from your Google account or the refresh token expires (Google's standard 6-month idle window).

## Caveats

- **Quota.** The Gmail API has per-user quotas (roughly 250 quota units/sec, with reads ≈ 5 units and sends ≈ 100 units). arawn surfaces 429 responses verbatim through the engine error chain — the agent will see the rate-limit error and back off.
- **HTML send is not supported in v1.** `gmail_send` constructs `text/plain` only. HTML, attachments, and CC/BCC are follow-ups.
- **Single-account.** One Gmail account per arawn install. Multi-account is a future redesign.
- **OAuth consent screen "test mode" expires after 7 days for unverified apps.** If you've left the OAuth consent screen as "External + Testing", Google will invalidate refresh tokens after a week and you'll have to `/connect gmail` again. To avoid this, either submit your app for verification (overkill for personal use) or switch the consent screen to "Internal" if you're on Google Workspace.

## Troubleshooting

### `gmail connection FAILED: HTTP 403`

Most often: the Gmail API isn't enabled on the project, or your account isn't in the test-user list. Re-check both in the Cloud Console.

### `gmail connection FAILED: invalid_grant`

The refresh token is no longer valid (revoked or expired). Run `/disconnect gmail` to clear the stored token, then `/connect gmail` to acquire a new one.

### Browser doesn't open automatically

The TUI tries `open` (macOS), `xdg-open` (Linux), or `cmd /c start` (Windows). If none works, the auth URL is always printed in the TUI — copy it into a browser manually. If you're on a remote SSH session, copy the URL to a browser on your local machine; the localhost callback works as long as the local browser can reach the remote machine on the redirect port (which it does via the public DNS or via SSH port forwarding if you've set one up).

### `Gmail integration skipped` in server log

`ARAWN_GMAIL_CLIENT_ID` and/or `ARAWN_GMAIL_CLIENT_SECRET` aren't set. Set them and restart `arawn serve`.
