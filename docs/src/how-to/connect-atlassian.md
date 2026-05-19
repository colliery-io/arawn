# Connect Atlassian (Jira + Confluence)

*How-to. ~10 minutes. One OAuth 2.0 (3LO) app covers both Jira and Confluence.*

By the end of this guide arawn will have an OAuth token for one Atlassian cloud site (your Jira and Confluence instance), and the agent will be able to search issues, transition them, read Confluence pages, and write content.

## Prerequisites

- arawn server running ([first chat session](../tutorials/first-chat.md)).
- An Atlassian cloud account with Jira and/or Confluence enabled.
- Port 8080 free on your machine. (Same fixed-port rule as Slack — Atlassian's redirect-URI allowlist is exact-match.)

## 1. Create the OAuth 2.0 (3LO) integration

Go to <https://developer.atlassian.com/console/myapps/> → **Create → OAuth 2.0 integration**.

- **Name:** anything (`arawn-personal` works).

## 2. Add APIs and scopes

In the app's settings, **Permissions** tab. For each API you want to use, click **Add** then **Configure**.

**Jira API** — add scopes:

```
read:jira-user
read:jira-work
write:jira-work
```

**Confluence API** — add scopes:

```
read:confluence-content.all
write:confluence-content
read:confluence-space.summary
```

> **Note:** Atlassian has both **classic** and **granular** scope versions. The scopes above are the classic set, which is what arawn uses. If a scope name doesn't appear in the picker, look in the "Classic scopes" section.

## 3. Set the callback URL

**Authorization** tab → **Callback URL**:

```
http://localhost:8080/oauth/callback
```

Same fixed-port rule as Slack — port 8080 must be free when you `/connect`. Save.

## 4. Get the client_id and client_secret

**Settings** tab → **Authentication details**. Copy:

- **Client ID**
- **Secret**

## 5. Paste into arawn.toml

```toml
[integrations.atlassian]
client_id = "your-client-id"
client_secret = "your-client-secret"
```

## 6. Restart and connect

```sh
arawn serve   # restart
```

In the TUI:

```
/connect atlassian
```

Atlassian's consent screen asks which **site** (your Jira/Confluence cloud instance) to grant access to. Pick the one you want. arawn discovers the underlying `cloud_id` automatically after consent and stores it alongside the token.

## 7. Verify

```
/integrations
```

Atlassian should appear as `connected (11 tools)` — 6 Jira tools + 5 Confluence tools. Then:

```
show me my open Jira issues
```

The agent calls `jira_search` (which filters by `assignee = currentUser()` when no other filter is given) and returns your open issues.

## Troubleshooting

If anything failed, see [debug OAuth failures](./debug-oauth-failures.md). The most common Atlassian-specific issues:

- The consent screen lists multiple sites and you picked the wrong one. Run `/disconnect atlassian` then `/connect atlassian` to redo the selection.
- `Address already in use (port 8080)` — same fix as Slack.

## What's next

- See every Jira and Confluence tool: [integrations reference](../reference/integrations.md).
- Bind a Jira project to a workstream: [bind a workstream to a feed](./bind-a-workstream-to-a-feed.md).
- Mirror a Jira project locally via `/watch jira/project-tracker`: [create a feed](./create-a-feed.md).
