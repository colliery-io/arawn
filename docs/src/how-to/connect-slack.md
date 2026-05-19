# Connect Slack

*How-to. ~10 minutes. Slack's OAuth model needs both bot and user tokens.*

By the end of this guide arawn will have OAuth tokens for one Slack workspace, and the agent will be able to list channels, read history, post messages, and search.

## Prerequisites

- arawn server running ([first chat session](../tutorials/first-chat.md)).
- A Slack workspace where you can create apps.
- Port 8080 free on your machine. (Slack's redirect-URI allowlist is exact-match — no wildcards.)
- About 10 minutes in front of <https://api.slack.com/apps>.

## 1. Create a Slack app

Go to <https://api.slack.com/apps> → **Create New App → From scratch**.

- **Name:** anything (`arawn-personal` works).
- **Workspace:** the workspace you want arawn to operate in.

> **Note:** Multi-workspace support is on the roadmap (see ARAWN-I-0034). Each app and its tokens are scoped to a single workspace today.

## 2. Add OAuth scopes

In the app's settings, **OAuth & Permissions → Scopes**.

Add these **Bot Token Scopes**:

```
channels:history
channels:read
chat:write
chat:write.public
files:read
groups:history
groups:read
im:history
im:read
im:write
mpim:history
mpim:read
mpim:write
users:read
users:read.email
```

Add these **User Token Scopes** as well — Slack's dual-token model means some tools need user-level access to private channels you've joined:

```
channels:history
channels:read
groups:history
groups:read
im:history
im:read
mpim:history
mpim:read
search:read
```

## 3. Set the redirect URI

In **OAuth & Permissions → Redirect URLs**, click **Add New Redirect URL** and enter exactly:

```
http://localhost:8080/oauth/callback
```

> **Note:** use `localhost`, not `127.0.0.1`. Slack does string comparison and rejects `127.0.0.1` even though it resolves to the same address. The port is fixed at 8080 because Slack's allowlist is exact-match. Make sure port 8080 is free when you `/connect`.

Save URLs.

## 4. Install the app to your workspace

In the app's settings, **Install App → Install to `<Workspace>`**. Slack shows a consent screen with the scopes you asked for. Approve.

After install, the bot/user tokens are visible on this page. arawn doesn't use them directly — it does its own OAuth dance via `/connect slack` to mint its own tokens. The install step is what registers your app with the workspace so OAuth will succeed.

## 5. Get the client_id and client_secret

In the app's settings, **Basic Information → App Credentials**. Copy:

- **Client ID** (looks like `2130966322213.11049839823699`)
- **Client Secret** (32-char hex string)

## 6. Paste into arawn.toml

```toml
[integrations.slack]
client_id = "2130966322213.11049839823699"
client_secret = "262f1cf7e5773131e68c7b61df992a1b"
```

## 7. Restart and connect

```sh
arawn serve   # restart
```

In the TUI:

```
/connect slack
```

Your browser opens; you re-approve (this time it's arawn's own OAuth dance, not the install flow). On success the TUI shows `ℹ [integration] connected: slack`.

> **Note:** if you change scopes later, you must **re-install the app** in step 4 *and* run `/disconnect slack` then `/connect slack` again. Slack's tokens are scope-locked at issue time.

## 8. Verify

```
/integrations
```

Slack should appear as `connected (6 tools)`. Then:

```
list my Slack channels
```

The agent should return a list of channels.

## Troubleshooting

If anything failed, see [debug OAuth failures](./debug-oauth-failures.md). The most common Slack-specific issues:

- `Error 400: redirect_uri_mismatch` — your redirect URL doesn't match step 3 exactly. Use `localhost` (not `127.0.0.1`), port `8080` (not anything else).
- `Address already in use (port 8080)` — something else is listening on 8080. Stop it, or wait for the bound socket to leave TIME_WAIT.

## What's next

- See every Slack tool the agent has, including `slack_list_channels`, `slack_history`, `slack_post`, `slack_react`, `slack_users_list`, `slack_open_dm`: [integrations reference](../reference/integrations.md).
- Bind a Slack channel to a workstream: [bind a workstream to a feed](./bind-a-workstream-to-a-feed.md).
- Watch a channel via `/watch slack/channel-archive`: [create a feed](./create-a-feed.md).
