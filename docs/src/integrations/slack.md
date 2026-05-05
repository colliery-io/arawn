# Slack

The Slack integration is **read/write**, not webhook-only. The agent can browse channel history, post, and react — gain context as a participant, not just a notification sink. ADR-0001 § 4 records the design decision.

Six tools land when Slack is configured: `slack_list_channels`, `slack_history`, `slack_post`, `slack_react`, `slack_users_list`, `slack_open_dm`. Cross-channel `slack_search` is deferred to a future ticket (slack-morphism doesn't typed-expose `search.messages`; per-channel history covers most "what was discussed" needs in v1).

## Setup

### 1. Create a Slack app

Each user owns their own Slack app — there's no shared arawn one.

1. Go to [api.slack.com/apps](https://api.slack.com/apps) → **Create New App** → **From scratch**.
2. Pick a name (`arawn` works) and the workspace you want it to act in.
3. **OAuth & Permissions → Scopes → Bot Token Scopes** — add all of:
   - `channels:read`, `channels:history` — public channels
   - `groups:read`, `groups:history` — private channels (only ones the bot is invited to)
   - `im:read`, `im:history`, `mpim:history` — DMs
   - `chat:write` — post messages
   - `reactions:write` — add reactions
   - `users:read` — resolve user IDs to names (optional, but the agent benefits from it)
4. **OAuth & Permissions → Redirect URLs** — add `http://127.0.0.1:0/oauth/callback`.
   - Slack will reject the literal `:0` placeholder. Workaround: add `http://127.0.0.1:8080/oauth/callback` and any other ports you might land on. arawn's callback server picks an OS-assigned port at flow-start time, so ideally Slack would accept a wildcard — it doesn't. **Pragmatic fix**: arawn-auth can be told to bind a specific port; for v1, accept that the callback URL must be added to Slack's allowlist for whichever port the OS picks. If this becomes painful, file a follow-up to add a fixed-port mode for Slack.
5. **Basic Information → App Credentials** — copy `Client ID` and `Client Secret`.
6. **Install to Workspace** — only available after scopes are added; this acquires the bot token. (You'll re-do this through arawn's `/connect` flow; the manual install is just to verify the app is reachable.)

### 2. Set environment variables

```sh
export ARAWN_SLACK_CLIENT_ID="..."
export ARAWN_SLACK_CLIENT_SECRET="..."
```

If both are present at server startup, the integration registers and the six tools land in the engine. If either is missing, Slack is silently skipped.

### 3. Connect

```
/connect slack
```

OAuth flow opens in your browser, you grant the bot scopes, the server captures the callback and stores the bot token. You'll see `ℹ [integration] slack connected` in the TUI.

### 4. Verify

```
/integrations
```

Should show:

```
| Service          | Connected |
|------------------|-----------|
| slack            | ✓ |
```

Try it:

> What's been happening in #engineering today?

The agent calls `slack_list_channels` to discover the channel id, then `slack_history` against it.

## What the tools do

| Tool | Returns | Permission category |
|---|---|---|
| `slack_list_channels({include_dms?, include_private?, limit?})` | Channels with id, name, kind (public/private/im/mpim), member_count, topic, purpose. | ReadOnly |
| `slack_history({channel, limit, oldest?, latest?})` | Last N messages with ts, user (Slack id), text, thread_ts, reply_count, reactions. Default 20, max 200. | ReadOnly |
| `slack_post({channel, text, thread_ts?})` | Posts plain text. `channel` accepts id or `#name`. Optional `thread_ts` makes it a thread reply. | Other (mode default: ask) |
| `slack_react({channel, ts, name})` | Adds an emoji reaction. `name` is bare (`thumbsup`, not `:thumbsup:`). | FileWrite |
| `slack_users_list({limit?, include_deleted?, include_bots?})` | Workspace directory: id, name (handle), real_name, display_name, email, title, is_bot, deleted. Default 200, max 1000. Bots and deactivated users excluded by default. | ReadOnly |
| `slack_open_dm({user_ids: [..]})` | Returns the DM channel id for the given user(s). Single id → 1:1 DM, multiple → mpim. Idempotent. | FileWrite |

### User IDs and DMs

`slack_history` returns Slack user IDs (e.g. `U12345`), not display names. To turn them into people:

1. **`slack_users_list`** — fetches the workspace directory once. The agent caches the id → name mapping for the session.
2. **`slack_open_dm`** — given a user id, returns the DM channel id. Pass that id to `slack_history` to read the conversation, or to `slack_post` to send a message.

Typical flow for "what did Alice say to me yesterday":

```
slack_users_list → find U12345 for "Alice"
slack_open_dm({user_ids: ["U12345"]}) → returns channel D67890
slack_history({channel: "D67890", oldest: <ts>}) → conversation
```

We don't pre-join names into history results — the LLM handles the join correctly and IDs are unambiguous.

### Channel arguments

`slack_list_channels` returns channel ids. Most other tools accept either id or `#name` — slack-morphism translates names internally for `chat.postMessage`. For `slack_history` you must use the id (Slack's `conversations.history` doesn't accept names).

## Token model

Slack bot tokens **don't expire** by default. arawn doesn't attempt to refresh them. If your Slack workspace enables "token rotation" (an opt-in app setting), the access token will eventually return `invalid_auth` — the engine error chain (T-0191) surfaces the message and the user runs `/connect slack` again. If token rotation becomes a real friction point, a future ticket can wire refresh through the `arawn_auth::OAuthClient::refresh` path Slack also supports.

## Disconnecting

```
/disconnect slack
```

Removes the stored token. The integration stays registered.

## Caveats

- **Private channels need the bot invited.** `groups:history` only returns history from private channels the bot is a member of. Invite `@arawn` (or whatever you named the app) into private channels you want it to see.
- **No cross-channel search in v1.** Use `slack_history` per channel. Filed as a follow-up.
- **Plain text only.** `slack_post` doesn't use Block Kit. mrkdwn formatting (`*bold*`, `_italic_`, `<@U12345>` mentions, etc.) does work since Slack parses message text by default.
- **Single-workspace.** One Slack workspace per arawn install.
- **Redirect URI port flexibility** — see the setup note above; arawn picks an OS-assigned port for the OAuth callback, but Slack's redirect-URI allowlist is exact-match. Add the relevant port(s) to your Slack app config. If this becomes annoying, file a fixed-port mode.

## Troubleshooting

### `connection FAILED: invalid_redirect`

The redirect URI arawn used (`http://127.0.0.1:<port>/oauth/callback`) isn't in your Slack app's allowlist. Add it under **OAuth & Permissions → Redirect URLs**.

### `connection FAILED: invalid_client_id`

The client_id env var doesn't match a real Slack app, or the app was deleted.

### `slack_history HTTP 403 not_in_channel`

The bot isn't a member of that private channel. Invite it from inside the channel (`/invite @arawn`).

### `slack_history HTTP 403 missing_scope`

You missed a scope when configuring the Slack app. Re-check the scope list above; you'll need to reinstall to the workspace and `/connect slack` again to acquire a token with the new scope.

### Browser doesn't open / stuck on "Waiting for browser authorization"

Same as Gmail/Calendar — the auth URL is always printed in the TUI; copy/paste into a browser if the auto-open didn't fire.
