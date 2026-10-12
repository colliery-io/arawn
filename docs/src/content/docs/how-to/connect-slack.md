---
title: "Connect Slack"
---
*How-to. ~10 minutes. Slack's OAuth model needs both bot and user tokens.*

By the end of this guide arawn will have OAuth tokens for one or more Slack workspaces, and the agent will be able to list channels, read history, post messages, react with emoji, list users, and open DMs in each of them.

If you've never set up an OAuth integration, skim the [OAuth primer](../../explanation/oauth-primer/) first.

## Prerequisites

- arawn server running ([first chat session](../../tutorials/first-chat/)).
- A Slack workspace where you can create and install apps. Slack restricts both — non-admins can usually create an app, but only workspace admins (or members with the "App Manager" permission) can install one. If your admin disallows custom apps you'll need to ask.
- Port 8080 free on your machine. Slack's redirect-URI allowlist is exact-match — no wildcard ports — so arawn pins to 8080 (configurable via `SlackProviderConfig::redirect_port` if needed).
- About 10 minutes in front of <https://api.slack.com/apps>.

## 1. Create a Slack app

<!-- VERIFY: 2026-05-20 — api.slack.com app-creation UI. -->

Go to <https://api.slack.com/apps> → **Create New App → From scratch**.

- **Name:** anything (`arawn-personal` works).
- **Workspace:** the first workspace you want arawn to operate in.

One app serves every workspace: you create it once, and [install it in more workspaces](#connect-another-workspace) later.

## 2. Add OAuth scopes

<!-- VERIFY: 2026-05-20 — OAuth & Permissions → Scopes section path. -->

In the app's settings, navigate to **OAuth & Permissions → Scopes**.

Slack runs a dual-token model. arawn uses **bot scopes** for writes (`slack_<alias>_post` posts as the bot identity) and **user scopes** for reads (so private channels you've joined are visible without inviting the bot to every one). Add both sets — the strings below are pulled verbatim from `crates/arawn-integrations/src/slack/integration.rs`.

**Bot Token Scopes (16):**

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
reactions:write
users:read
users:read.email
```

**User Token Scopes (10):**

```
channels:history
channels:read
groups:history
groups:read
im:history
im:read
mpim:history
mpim:read
users:read
search:read
```

> **Why two scope lists?** Slack's API distinguishes a "bot" identity (your app, which posts as itself) from a "user" identity (you, the installer, granting the app permission to act on your behalf). Bot tokens can only see private channels the bot is explicitly invited to; user tokens see every channel the user is in. So writes go through the bot token (messages appear as arawn) and reads go through the user token (full coverage without `/invite` everywhere). The [OAuth primer](../../explanation/oauth-primer/#why-every-provider-feels-different) has the wider context.

## 3. Set the redirect URI

<!-- VERIFY: 2026-05-20 — OAuth & Permissions → Redirect URLs section path. -->

In **OAuth & Permissions → Redirect URLs**, click **Add New Redirect URL** and enter **exactly**:

```
http://localhost:8080/oauth/callback
```

Save URLs.

> **`localhost`, not `127.0.0.1`.** Slack does string-comparison on the redirect URI and rejects `127.0.0.1:8080` even though it resolves to the same address. arawn's callback server emits the `localhost` form deliberately (see `crates/arawn-auth/src/server.rs:69-73`). The port is fixed at 8080 because Slack's allowlist is exact-match.

## 4. Install the app to your workspace

<!-- VERIFY: 2026-05-20 — Install App page path. -->

In the app's settings, navigate to **Install App → Install to `<Workspace>`**. Slack shows a consent screen with the scopes you asked for. Approve.

After install, Slack shows the bot/user tokens on this page. arawn doesn't read them directly — it does its own OAuth dance via `/connect slack` to mint its own tokens. The install step is what registers your app with the workspace so OAuth will succeed.

> **Workspace admin needed.** Non-admins typically can't approve the install. If the button is greyed out or you see "this requires approval", a workspace admin needs to either install on your behalf or grant you the "App Manager" permission.

## 5. Get the client_id and client_secret

<!-- VERIFY: 2026-05-20 — Basic Information → App Credentials section path. -->

In the app's settings, **Basic Information → App Credentials**. Copy:

- **Client ID** (looks like `2130966322213.11049839823699`)
- **Client Secret** (32-char hex string)

## 6. Paste into arawn.toml (or set env vars)

```toml
# ~/.arawn/arawn.toml
[integrations.slack]
client_id     = "2130966322213.11049839823699"
client_secret = "262f1cf7e5773131e68c7b61df992a1b"
```

Or, to keep credentials out of `arawn.toml`:

```sh
export ARAWN_SLACK_CLIENT_ID="2130966322213.11049839823699"
export ARAWN_SLACK_CLIENT_SECRET="262f1cf7e5773131e68c7b61df992a1b"
```

Env vars override TOML when both are set. See the [integrations config reference](../../reference/integrations-config/) for the full resolution chain.

## 7. Restart and connect

```sh
arawn serve   # restart
```

In the TUI:

```
/connect slack
```

Your browser opens; you re-approve (this time it's arawn's own OAuth dance, not the install flow). If you are in more than one workspace, pick the one to connect in Slack's page. On success the TUI shows `ℹ [integration] slack@acme connected`.

arawn names each workspace `slack@<alias>`. With plain `/connect slack` the alias comes from the workspace's name: lowercase, other characters turned into `-`, at most 32 characters (`Acme Corp` becomes `acme-corp`). A name with non-ASCII letters gives the team id instead (`t0123abcd`). If the alias is taken by another workspace, arawn adds `-2`, `-3`, and so on. To pick the alias yourself:

```
/connect slack@work
```

An alias is 1 to 32 of `a-z`, `0-9` and `-`.

From the CLI, `arawn connect slack` does the same and prints the name it connected as.

> **Scope changes after first install.** Slack issues tokens scoped to the install at time-of-install. If you go back and add or remove scopes in step 2, you must **re-install the app** in step 4 *and* run `/connect slack@<alias>` again. The new scopes only take effect on a fresh token.

## 8. Verify

```
/integrations
```

Each workspace has its own row, with the number of scopes its token holds:

```
| slack@acme | ✓ (16 scopes) |
```

Then:

```
list my Slack channels on acme
```

The agent should return a list of channels from `slack_acme_list_channels`.

## Connect another workspace

<!-- VERIFY: 2026-10-11 — Manage Distribution page path; the live two-workspace check is the owner's step. -->

1. Install the same app in the second workspace. An app can be installed outside the workspace it was created in only after **Manage Distribution → Activate Public Distribution**; then open the **Sharable URL** while signed in to the second workspace and approve. The app, its scopes, its redirect URL and its client id and secret stay the same: `[integrations.slack]` does not change.
2. Run `/connect slack` again (or `arawn connect slack`). In Slack's page, pick the second workspace.
3. `/integrations` now shows both, for example `slack@acme` and `slack@personal`.

No restart is needed: the second workspace's tools appear at once. Connecting a workspace that is already connected (the same Slack team) keeps its alias and refreshes its token.

## Tools and permission rules

Each workspace has its own six tools, named `slack_<alias>_<op>`, with any `-` in the alias as `_`:

| Workspace | Tools |
|---|---|
| `slack@acme` | `slack_acme_list_channels`, `slack_acme_history`, `slack_acme_post`, `slack_acme_react`, `slack_acme_users_list`, `slack_acme_open_dm` |
| `slack@acme-corp` | `slack_acme_corp_post`, … |

The names have the alias even with one workspace, so they do not change when you add a second. The agent's system prompt lists each connected workspace with its scopes.

Permission rules match tool names with `*`, so one rule covers every workspace:

```toml
[permissions]
ask  = ["slack_*_post"]          # ask before posting in any workspace
deny = ["slack_personal_post"]   # never post in personal
```

See [permission rules](../../reference/permissions/).

## Disconnect one workspace

```
/disconnect slack@personal
```

or `arawn disconnect slack@personal`. arawn deletes that workspace's token and drops its tools; the other workspaces stay connected. Its feeds stop working until you connect it again.

With only one workspace connected, `/disconnect slack` is enough. With more than one, arawn refuses and lists the aliases, so you name the one to drop.

## Upgrading from one workspace

An install from before workspaces were named is moved once, the first time the new `arawn serve` starts:

- The token moves from `slack` to `slack@<alias>`. The alias comes from the team stored with the token; if none is stored arawn asks Slack (`auth.test`), else it uses `default`. You do not reconnect.
- Every `slack/*` feed gets `workspace = "<alias>"`, and its stored messages get the workspace.
- In `arawn.toml`, permission rules that name an old tool (`slack_post`, `slack_history(...)`) are rewritten to the new name (`slack_acme_post`), and each `[[feeds]]` Slack entry gets `workspace` in its params. The file is backed up first as `arawn.toml.pre-slack-workspaces`. This is the only change arawn makes outside its data directory. Rules with `*` (`slack_*`) still match and are left alone.
- The server log and an `[integration]` notice say what changed.

The move is recorded in `<data_dir>/slack-migration.json`. If the server stops part-way, the next start finishes it with the same alias. Once done it does not run again.

## Troubleshooting

See [debug OAuth failures](../debug-oauth-failures/) for the symptom → cause → fix table. The most common Slack-specific issues:

- **`redirect_uri_mismatch`** — the redirect URL you saved in step 3 doesn't exactly match what arawn presents. Use `localhost` (not `127.0.0.1`), port `8080`, path `/oauth/callback`. No trailing slash.
- **`Address already in use (port 8080)`** — something else is bound to 8080. Stop it, or wait for the previous socket to leave `TIME_WAIT`. `lsof -iTCP:8080 -sTCP:LISTEN` will tell you who.
- **`invalid_permissions` / "Scope not in allowlist"** — workspace admin has restricted which scopes are installable. Ask the admin to allow the bot/user scopes from step 2.

## What's next

- [Integrations reference](../../reference/integrations/) — every Slack tool the agent has.
- [Bind a lens to a feed](../bind-a-lens-to-a-feed/) — turn a Slack channel into a local archive.
- [Create a feed](../create-a-feed/) — `/watch slack/channel-archive` to continually mirror a channel. Every Slack feed names its workspace; see [feed templates](../../reference/feed-templates/#slack).
