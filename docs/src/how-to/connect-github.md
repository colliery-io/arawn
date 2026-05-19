# Connect GitHub

*How-to. ~10 minutes. GitHub uses a different model than the other integrations — a GitHub App, not OAuth.*

By the end of this guide arawn will have a GitHub App installed for your account and the agent will be able to read your notifications inbox, your authored/assigned issues and PRs, and PRs where you're a requested reviewer. v1 is read-only.

## Prerequisites

- arawn server running ([first chat session](../tutorials/first-chat.md)).
- A GitHub account or org admin permissions if you want an org-wide install.
- About 10 minutes in front of <https://github.com/settings/apps/new>.

> **Why a GitHub App instead of OAuth?** Apps give finer scope control, better refresh semantics, and per-installation access tokens that mint on demand and expire in an hour. There's a one-time *operator* step to register the App, then a per-user *install* step. After that arawn signs short-lived JWTs in memory to mint installation tokens.

## Step 1 — Register the GitHub App (operator, once)

1. Open <https://github.com/settings/apps/new> (personal app) or your org's `Settings → Developer settings → GitHub Apps → New GitHub App` (org app).

2. Fill in:

   - **GitHub App name:** `arawn` or whatever you like — the slug in the URL is what arawn needs.
   - **Homepage URL:** anything (use your own homepage or the arawn repo).
   - **Setup URL:** `http://localhost/oauth/callback`. The port is dynamic. GitHub allows any 127.0.0.1 / localhost callback on apps.
   - **Webhook:** deactivate (arawn polls; no webhook ingestion in v1).
   - **Permissions (read-only):**
     - Repository → Contents: Read-only
     - Repository → Issues: Read-only
     - Repository → Pull requests: Read-only
     - Repository → Metadata: Read-only (mandatory)
     - Account → Email addresses: Read-only (optional, for attribution)
   - **Subscribe to events:** leave empty.
   - **Where can this app be installed?:** "Any account" if you want to share the App across orgs, "Only on this account" otherwise.

3. After **Create GitHub App**, on the resulting page:

   - Note the **App ID** (numeric, near the top).
   - Note the **slug** in the URL (`github.com/apps/<slug>` — visible on the public app page).
   - Click **Generate a private key** at the bottom — downloads a `.pem` file. Store it somewhere safe; arawn will read it at startup.

## Step 2 — Tell arawn about the App

Two options. Pick one.

**arawn.toml** (preferred for daemons):

```toml
[integrations.github]
app_id          = "123456"
app_slug        = "arawn"
private_key_path = "/secure/path/to/arawn-app.private-key.pem"
```

**Environment variables** (preferred for one-off / dev runs):

```sh
export ARAWN_GITHUB_APP_ID=123456
export ARAWN_GITHUB_APP_SLUG=arawn
export ARAWN_GITHUB_PRIVATE_KEY_PATH=/secure/path/to/arawn-app.private-key.pem
# or, if you'd rather inline the PEM body:
# export ARAWN_GITHUB_PRIVATE_KEY_PEM="$(cat /path/to/key.pem)"
```

Restart `arawn serve`. You should see `GitHub integration registered (read-only…)` in the startup logs.

## Step 3 — Install the App (per user)

In the TUI:

```
/connect github
```

arawn publishes the public install URL (`https://github.com/apps/<slug>/installations/new`); your browser opens, you pick which org/repos the App can see, and GitHub redirects to arawn's local callback. arawn captures the `installation_id`, encrypts it on disk under `<data_dir>/integrations/github/`, and reports success.

## 4. Verify

```
/integrations
```

GitHub should appear as `connected`. Then bind it to a workstream and the agent can use it via [bind a workstream to a feed](./bind-a-workstream-to-a-feed.md) and [create a feed](./create-a-feed.md):

```
/workstream bind <ws> github:repo:owner/name
/workstream bind <ws> github:org:owner
/watch github/notifications
```

## What's stored

| Location | Encrypted? | Lifetime |
|---|---|---|
| `<data_dir>/integrations/github/github.bin` | Yes (ChaCha20Poly1305) | Until `/disconnect github`. |
| In-memory installation access token | N/A (memory only) | 1 hour; auto-refreshed within 5 min of expiry. |

The App private key never moves — arawn reads it from disk at startup, signs JWTs in memory, and forwards those to GitHub to mint short-lived installation-access-tokens.

## Disconnect

```
/disconnect github
```

Removes the `installation_id` from disk. The App still appears in your GitHub installations list — uninstall it there to revoke arawn's access on GitHub's side as well.

## What you get

Four feed templates are shipped:

- `github/notifications` — your `/notifications` inbox.
- `github/issues-and-prs` — open + recently-closed issues and PRs you authored or are assigned to.
- `github/review-queue` — PRs where you're a requested reviewer.
- `github/repo-mirror` — full snapshot of a repo's open issues + PRs (used when you bind a workstream via `github:repo:` or `github:org:`).

Bind them to workstreams via `/workstream bind <ws> github:repo:owner/name` or `/workstream bind <ws> github:org:owner`. The org form expands to one `repo-mirror` feed per repo in the org and supersedes any per-repo binds in that workstream.

## What's next

- Bind a repo or org to a workstream: [bind a workstream to a feed](./bind-a-workstream-to-a-feed.md).
- All 4 GitHub feed templates with parameters: [feed templates reference](../reference/feed-templates.md).
- Why a GitHub App instead of OAuth: [integrations reference](../reference/integrations.md).
