# GitHub

Arawn observes your GitHub activity as a first-class personal signal
stream: notifications, assigned issues, authored issues + PRs, and
review-requested PRs all flow through the morning brief, weekly
priorities, and the retro. v1 is **read-only** — arawn surfaces
what's happening; you act in GitHub.

## Setup

GitHub integration uses the **GitHub App** model (not a personal
access token, not an OAuth App). This gives finer scope control and
better refresh semantics. There's a one-time operator step to
register the app, then a per-user install step.

### Step 1 — Register the GitHub App (operator, once)

1. Open <https://github.com/settings/apps/new> (for a personal app)
   or your org's `Settings → Developer settings → GitHub Apps → New
   GitHub App` (for an org app).
2. Fill in:
   - **GitHub App name**: `arawn` (or whatever you like — the slug
     in the URL is what arawn needs).
   - **Homepage URL**: anything (use your own homepage or the
     arawn repo).
   - **Setup URL**: `http://localhost/oauth/callback` — arawn's
     callback path. The port is dynamic, so configure GitHub to
     accept any 127.0.0.1 redirect (GitHub allows wildcard ports
     on localhost).
   - **Webhook**: deactivate (arawn polls, no webhook ingestion in
     v1).
   - **Permissions** (read-only):
     - Repository → Contents: Read-only
     - Repository → Issues: Read-only
     - Repository → Pull requests: Read-only
     - Repository → Metadata: Read-only (mandatory)
     - Account → Email addresses: Read-only (optional, for
       attribution)
   - **Subscribe to events**: leave empty (no webhooks).
   - **Where can this app be installed?**: "Any account" if you
     want to share the app across orgs, "Only on this account"
     otherwise.
3. After clicking *Create GitHub App*, on the resulting page:
   - Note the **App ID** (numeric, near the top).
   - Note the **slug** in the URL (`github.com/apps/<slug>` —
     visible on the public app page).
   - Click *Generate a private key* at the bottom — downloads a
     `.pem` file. Store it somewhere safe; arawn will read it at
     startup.

### Step 2 — Tell arawn about the App

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
# or, if you'd rather inline the PEM body (less common):
# export ARAWN_GITHUB_PRIVATE_KEY_PEM="$(cat /path/to/key.pem)"
```

Restart arawn. You'll see `GitHub integration registered (read-only…)`
in the startup logs.

### Step 3 — Install the App for your account (per user)

Inside arawn:

```
/integrations connect github
```

Arawn publishes the public install URL
(`https://github.com/apps/<slug>/installations/new`); your browser
opens, you pick which org/repos the app can see, and GitHub
redirects to arawn's local callback. The integration captures the
`installation_id`, encrypts it on disk under
`<data_dir>/integrations/github/`, and reports success.

## What's stored

| Location | Encrypted? | Lifetime |
|---|---|---|
| `<data_dir>/integrations/github/github.bin` | Yes (ChaCha20Poly1305) | Until `/integrations disconnect github`. |
| In-memory installation access token | N/A (memory only) | 1 hour; auto-refreshed when within 5 min of expiry. |

The App private key never moves — arawn reads it from disk at
startup, signs JWTs in memory, and forwards those to GitHub to
mint short-lived installation-access-tokens.

## Disconnect

```
/integrations disconnect github
```

Removes the installation_id from disk. The app still appears in
your GitHub installations list — uninstall it there to revoke
arawn's access on GitHub's side.

## What you get

Feed templates land in follow-up tasks (T-0319 / T-0320 / T-0321):

- `github/notifications` — your `/notifications` inbox.
- `github/issues-and-prs` — open + recently-closed issues and PRs
  you authored or are assigned to.
- `github/review-queue` — PRs where you're a requested reviewer.

Bind them to workstreams via:

```
/watch github/notifications
/workstream bind <ws> github:repo:owner/name
/workstream bind <ws> github:org:owner
```

…with details in the workstream + feed docs once those tasks ship.
