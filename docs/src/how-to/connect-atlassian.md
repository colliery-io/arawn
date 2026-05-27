# Connect Atlassian (Jira + Confluence)

*How-to. ~10 minutes. One OAuth 2.0 (3LO) app covers both Jira and Confluence on every site you authorize.*

By the end of this guide arawn will have an OAuth token for one or more Atlassian cloud sites (your Jira + Confluence instances), and the agent will be able to search issues, transition them, read Confluence pages, and write content. 6 Jira tools + 5 Confluence tools register together.

If you've never set up an OAuth integration, skim the [OAuth primer](../explanation/oauth-primer.md) first.

## Prerequisites

- arawn server running ([first chat session](../tutorials/first-chat.md)).
- An Atlassian cloud account with Jira and/or Confluence enabled. (One Atlassian login can have multiple sites — work, personal, etc. — and arawn supports them all from a single OAuth grant; see step 7.)
- Port 8080 free on your machine. Atlassian's redirect-URI allowlist is exact-match (same shape as Slack), so arawn pins to 8080.
- About 10 minutes in front of <https://developer.atlassian.com/console/myapps/>.

## 1. Create the OAuth 2.0 (3LO) integration

<!-- VERIFY: 2026-05-20 — developer.atlassian.com console layout. -->

Go to <https://developer.atlassian.com/console/myapps/> → **Create → OAuth 2.0 integration**.

- **Name:** anything (`arawn-personal` works).
- Accept the developer terms; click **Create**.

## 2. Add APIs

<!-- VERIFY: 2026-05-20 — Permissions tab path. -->

In the app's settings, open the **Permissions** tab. You need to add three APIs:

- **Jira API** — click **Add**, then **Configure**.
- **Confluence API** — click **Add**, then **Configure**.
- **User identity API** — click **Add**.

The Confluence API page surfaces both **classic** and **granular** scopes. arawn uses both — the classic scopes authorize v1 endpoints (CQL search, space metadata), the granular scopes authorize v2 endpoints (the page surface). You need to enable both to get the full Confluence tool set working.

## 3. Add the scopes

In each API's **Configure** view, add the scope strings below. These are pulled verbatim from `crates/arawn-integrations/src/atlassian/integration.rs` and the per-tool `*_SCOPES` constants in `jira.rs` / `confluence.rs`. Atlassian's picker matches the exact strings.

**Jira API (3 scopes):**

```
read:jira-work
write:jira-work
read:jira-user
```

**Confluence API — classic (5 scopes):**

```
read:confluence-content.all
read:confluence-content.summary
read:confluence-space.summary
write:confluence-content
search:confluence
```

**Confluence API — granular (4 scopes — required by v2 endpoints):**

```
read:space:confluence
read:page:confluence
write:page:confluence
read:content-details:confluence
```

**Refresh-token scope (1 scope, surfaced under the OAuth 2.0 (3LO) section, not under an API):**

```
offline_access
```

> **Why so many scopes?** Atlassian's v1 and v2 API generations use different scope vocabularies. Classic scopes (v1) authorize CQL search and space-metadata reads; granular scopes (v2) authorize the page surface that the Confluence get/create/update page tools use. `offline_access` is what makes the issued token come with a refresh token — without it the access token expires in ~1 hour and you'd have to re-`/connect` constantly.

## 4. Set the callback URL

<!-- VERIFY: 2026-05-20 — Authorization tab → Callback URL section path. -->

Open the **Authorization** tab. Under **Callback URL**, enter exactly:

```
http://localhost:8080/oauth/callback
```

> **`localhost`, not `127.0.0.1`.** Same string-match quirk as Slack — Atlassian compares the redirect URI string-for-string and rejects `127.0.0.1` even though it resolves to the same address. arawn's callback server emits the `localhost` form (`arawn-auth/src/server.rs:69-73`).

Save.

## 5. Get the client_id and client_secret

<!-- VERIFY: 2026-05-20 — Settings tab → Authentication details section path. -->

Open the **Settings** tab → **Authentication details**. Copy:

- **Client ID**
- **Secret**

## 6. Paste into arawn.toml (or set env vars)

```toml
# ~/.arawn/arawn.toml
[integrations.atlassian]
client_id     = "your-client-id"
client_secret = "your-client-secret"
```

Or via env:

```sh
export ARAWN_ATLASSIAN_CLIENT_ID="your-client-id"
export ARAWN_ATLASSIAN_CLIENT_SECRET="your-client-secret"
```

See the [integrations config reference](../reference/integrations-config.md) for the full resolution chain.

## 7. Restart and connect

```sh
arawn serve   # restart
```

In the TUI:

```
/connect atlassian
```

Atlassian's consent screen asks which **site** to grant access to. If your account has multiple cloud sites (e.g. a work and a personal Jira), you'll see a picker — select **every site** you want arawn to be able to read/write.

After consent, arawn:

1. Exchanges the authorization code for an access + refresh token (refresh because `offline_access` is in the scope set).
2. Calls `https://api.atlassian.com/oauth/token/accessible-resources` with that access token to discover which `cloud_id`s the consent covers.
3. Persists the access token, refresh token, and `cloud_id` list in the encrypted token store. Tools later substitute the right `cloud_id` into per-site API URLs (`https://api.atlassian.com/ex/jira/<cloud_id>/...`).

On success the TUI shows `ℹ [integration] connected: atlassian` along with the discovered sites.

## 8. Verify

```
/integrations
```

Atlassian should appear as `connected`. Then try a Jira prompt:

```
show me my open Jira issues
```

The agent calls `jira_search` (filtering on `assignee = currentUser()` when no other filter is given) and returns your open issues. For Confluence:

```
list the spaces in my Confluence
```

calls `confluence_list_spaces`.

## Troubleshooting

See [debug OAuth failures](./debug-oauth-failures.md) for the symptom → cause → fix table. Atlassian-specific notes:

- **`redirect_uri_mismatch`.** Step 4's URL must be exactly `http://localhost:8080/oauth/callback` — `localhost` not `127.0.0.1`, port 8080, no trailing slash, path `/oauth/callback`.
- **`Confluence v2 endpoints return 401 / insufficient scope`.** You added the classic Confluence scopes but missed the granular v2 ones (step 3 third block). `/disconnect atlassian`, add the missing scopes, `/connect atlassian` again.
- **`Token expired` after an hour, can't refresh.** You didn't add `offline_access` in step 3. Add it and reconnect — without it, no refresh token is issued at all.
- **Picked the wrong site in the consent flow.** `/disconnect atlassian` then `/connect atlassian` to redo the picker. Or pick all your sites the second time; arawn handles multi-site from a single grant.
- **Token refresh drops site list.** Recovered by [T-0235](https://github.com/) and the token-refresh path retains the persisted site list. If `/integrations` shows Atlassian connected but tools error with "no accessible resources", `/disconnect` + `/connect` once to re-run `accessible-resources` discovery.

## What's next

- [Integrations reference](../reference/integrations.md) — every Jira and Confluence tool the agent has.
- [Bind a lens to a feed](./bind-a-lens-to-a-feed.md) — turn a Jira project into a local archive.
- [Create a feed](./create-a-feed.md) — `/watch jira/project-tracker` for ongoing mirroring.
