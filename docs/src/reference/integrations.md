# Integrations

*Reference. Per-provider matrix — auth model, scopes, registered tools, feed templates, permission behavior.*

Source: `crates/arawn-integrations/src/<svc>/`.

## At-a-glance

| Provider | Auth | Agent tools | Feed templates |
|---|---|---|---|
| Gmail | OAuth 2.0 | 5 | `gmail/inbox-archive`, `gmail/label-archive`, `gmail/sender-filter` |
| Google Calendar | OAuth 2.0 | 3 | `calendar/upcoming-archive` |
| Google Drive | OAuth 2.0 | 7 | `drive/folder-sync`, `drive/recent` |
| Slack | OAuth 2.0 (bot+user dual-token) | 6 | `slack/channel-archive`, `slack/dm-archive`, `slack/my-mentions` |
| Atlassian (Jira+Confluence) | OAuth 2.0 (3LO) | 11 | `jira/project-tracker`, `jira/assignee-tracker`, `confluence/space-archive` |
| GitHub | GitHub App (read-only) | 0 (feed-only) | `github/notifications`, `github/issues-and-prs`, `github/review-queue`, `github/repo-mirror` |

How to connect: [Google](../how-to/connect-google.md), [Slack](../how-to/connect-slack.md), [Atlassian](../how-to/connect-atlassian.md), [GitHub](../how-to/connect-github.md).

## Gmail

**Auth:** OAuth 2.0 via the shared Google OAuth client. Setup: [connect Google](../how-to/connect-google.md).

**Scopes:**

```
https://www.googleapis.com/auth/gmail.readonly
https://www.googleapis.com/auth/gmail.send
https://www.googleapis.com/auth/gmail.modify
```

**Tools:**

| Tool | Permission-prompt behavior |
|---|---|
| `gmail_inbox_read` | Read — auto-allow |
| `gmail_search` | Read — auto-allow |
| `gmail_get_message` | Read — auto-allow |
| `gmail_send` | Other — asks in `default` mode |
| `gmail_mark_read` | Other — asks in `default` mode |

**Feed templates:** `gmail/inbox-archive`, `gmail/label-archive`, `gmail/sender-filter`. See [feed templates reference](./feed-templates.md).

## Google Calendar

**Auth:** shared Google OAuth client.

**Scopes:**

```
https://www.googleapis.com/auth/calendar.events
```

**Tools:**

| Tool | Permission-prompt behavior |
|---|---|
| `calendar_upcoming` | Read — auto-allow |
| `calendar_create_event` | Other — asks |
| `calendar_find_conflicts` | Read — auto-allow |

**Feed templates:** `calendar/upcoming-archive`.

## Google Drive

**Auth:** shared Google OAuth client.

**Scopes:**

```
https://www.googleapis.com/auth/drive
```

Full read+write (not `.readonly`), so write tools work. If you only want read access, omit the upload/update/delete tools at the agent level.

**Tools:**

| Tool | Permission-prompt behavior |
|---|---|
| `drive_search` | Read — auto-allow |
| `drive_list` | Read — auto-allow |
| `drive_get_metadata` | Read — auto-allow |
| `drive_read` | Read — auto-allow |
| `drive_upload` | Other — asks |
| `drive_update` | Other — asks |
| `drive_delete` | Other — asks (delete moves to Drive's trash, not permadelete) |

**Read-content dispatch.** Google-native files are exported automatically:

| MIME type | Export |
|---|---|
| Docs | markdown |
| Sheets | CSV |
| Slides | plain text |
| Drawings | PNG |
| Forms / Sites / Scripts | none (open in browser) |
| Anything else | raw bytes — UTF-8 decoded for text-like MIME, base64 otherwise; capped at 1 MB |

**Feed templates:** `drive/folder-sync`, `drive/recent`.

## Slack

**Auth:** Slack OAuth 2.0 with **both** bot and user tokens (Slack's dual-token model). Setup: [connect Slack](../how-to/connect-slack.md).

**Scopes (bot):**

```
channels:history, channels:read, chat:write, chat:write.public,
files:read, groups:history, groups:read, im:history, im:read,
im:write, mpim:history, mpim:read, mpim:write, users:read,
users:read.email
```

**Scopes (user):**

```
channels:history, channels:read, groups:history, groups:read,
im:history, im:read, mpim:history, mpim:read, search:read
```

**Tools:**

| Tool | Permission-prompt behavior |
|---|---|
| `slack_list_channels` | Read — auto-allow |
| `slack_history` | Read — auto-allow |
| `slack_post` | Other — asks |
| `slack_react` | Other — asks |
| `slack_users_list` | Read — auto-allow |
| `slack_open_dm` | Other — asks |

> **Note:** `slack_search` is deferred (slack-morphism doesn't yet expose the typed API). Older docs that mention `slack_search` are out of date.

**Feed templates:** `slack/channel-archive`, `slack/dm-archive`, `slack/my-mentions`.

**Multi-workspace:** today one Slack app is bound to one workspace. Multi-workspace support is on the roadmap (ARAWN-I-0034).

## Atlassian (Jira + Confluence)

**Auth:** OAuth 2.0 3LO. Setup: [connect Atlassian](../how-to/connect-atlassian.md).

**Scopes (Jira):** `read:jira-user`, `read:jira-work`, `write:jira-work`.
**Scopes (Confluence):** `read:confluence-content.all`, `write:confluence-content`, `read:confluence-space.summary`.

arawn uses the **classic scope set** (not granular). If a scope above doesn't appear in the developer console picker, look under "Classic scopes".

**Cloud-ID auto-discovery.** Atlassian's API requires a `cloud_id` per request. arawn calls `/oauth/token/accessible-resources` after consent and caches the first cloud_id you granted access to. Multi-site selection happens on the consent screen.

**Tools — Jira (6):**

| Tool | Permission-prompt behavior |
|---|---|
| `jira_search` | Read — auto-allow |
| `jira_get_issue` | Read — auto-allow |
| `jira_create_issue` | Other — asks |
| `jira_update_issue` | Other — asks |
| `jira_add_comment` | Other — asks |
| `jira_transition_issue` | Other — asks |

> **Note:** older docs used `jira_search_issues`. The code-side name is `jira_search`.

**Tools — Confluence (5):**

| Tool | Permission-prompt behavior |
|---|---|
| `confluence_search` | Read — auto-allow |
| `confluence_get_page` | Read — auto-allow |
| `confluence_create_page` | Other — asks |
| `confluence_update_page` | Other — asks |
| `confluence_list_spaces` | Read — auto-allow |

**Feed templates:** `jira/project-tracker`, `jira/assignee-tracker`, `confluence/space-archive`.

## GitHub

**Auth:** GitHub App, not OAuth. The App is registered once (operator step), then each user installs it via `/integrations connect github`. Installation tokens are minted on demand from the App private key and expire in 1 hour (auto-refreshed within 5 min of expiry). Setup: [connect GitHub](../how-to/connect-github.md).

**Permissions (App-level, read-only):**

- Repository → Contents, Issues, Pull requests, Metadata.
- Account → Email addresses (optional).

**Agent tools:** none today. v1 is feed-only.

**Feed templates:** `github/notifications`, `github/issues-and-prs`, `github/review-queue`, `github/repo-mirror`.

**Workstream binding:** GitHub has two URI schemes for `workstream_bind`:

- `github:repo:owner/name` — single repository.
- `github:org:owner` — entire organization (expands to one `github/repo-mirror` feed per repo).

Org binds supersede per-repo binds in the same workstream. See [bind a workstream to a feed](../how-to/bind-a-workstream-to-a-feed.md).

## Storage

Encrypted-at-rest credential blobs land under:

```
<data_dir>/integrations/<service>/<service>.bin   # ChaCha20Poly1305
<data_dir>/tokens/                                # OAuth refresh tokens
```

See [data directory reference](./data-directory.md) for the full layout.

## Related

- Per-provider connect how-tos: [Google](../how-to/connect-google.md), [Slack](../how-to/connect-slack.md), [Atlassian](../how-to/connect-atlassian.md), [GitHub](../how-to/connect-github.md).
- All 17 feed templates with params: [feed templates reference](./feed-templates.md).
- Agent tools catalog: [agent tools reference](./agent-tools.md).
- Debugging connection failures: [debug OAuth failures](../how-to/debug-oauth-failures.md).
