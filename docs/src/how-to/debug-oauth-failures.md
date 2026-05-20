# Debug OAuth failures

*How-to. Diagnose common OAuth errors when connecting an integration.*

This page expands the error table from the connect-* how-tos. Find your symptom in the left column, follow the diagnosis steps. The bottom section is an exhaustive matrix of every error variant the integration crates raise, in case the narrative sections don't cover your case.

## Prerequisites

- Provider configured in `arawn.toml` per its [connect how-to](./connect-google.md).
- `arawn serve` restarted after any config change.

## Common errors

### `Error 400: redirect_uri_mismatch`

**Cause.** The redirect URI arawn requested doesn't match what the provider has on its allowlist.

**Diagnose.** Read the full error body in the server log. It includes the requested redirect URI. Compare to what's in the provider console.

**Fix.**

- **Slack / Atlassian:** redirect URI must be **exactly** `http://localhost:8080/oauth/callback`. Use `localhost` (not `127.0.0.1` — they string-compare). Port is fixed at 8080.
- **Google (Desktop app):** no redirect URI configuration needed; Google accepts any localhost callback. If you see this error with Google, you probably configured a Web app instead of Desktop — recreate the OAuth client as Desktop.
- **GitHub (App):** the Setup URL should be `http://localhost/oauth/callback` (no port — arawn binds an ephemeral port and GitHub accepts any localhost).

### `Error 403: access_denied`

**Cause.** Either you clicked Deny on the consent screen, or your Google/Atlassian account isn't on the test-users list.

**Fix.**

- Restart `/connect <svc>` and approve. If you mis-clicked Deny, that's it.
- Otherwise add yourself as a test user:
  - Google: **Google Auth Platform → Audience → Test users → + Add Users**.
  - Atlassian: the integration must be in **Production** status (developer console → app → distribution), or your account must be the developer account.

### `[integration] error: insufficient_scope`

**Cause.** You connected with one scope set; the agent is now trying a tool that needs another scope.

**Diagnose.** The error includes the missing scope name. Cross-reference against the scope list in your provider's connect how-to.

**Fix.** Add the missing scope in the provider console, then re-auth:

```
/disconnect <svc>
/connect <svc>
```

> **Slack:** changing scopes requires re-installing the Slack app to your workspace (in addition to `/disconnect` + `/connect`). Slack's tokens are scope-locked at issue time.

### Atlassian: Confluence v2 endpoints fail with 401 / insufficient_scope

**Cause.** Your Atlassian OAuth app has the **classic** Confluence scopes (`read:confluence-content.all` etc.) but is missing the **granular** v2 scopes. The Confluence v2 endpoints (`/wiki/api/v2/...`) require the granular set; classic scopes don't authorize them.

**Fix.** In the developer.atlassian.com console, add the four granular v2 scopes to the Confluence API permissions:

```
read:space:confluence
read:page:confluence
write:page:confluence
read:content-details:confluence
```

Then `/disconnect atlassian` and `/connect atlassian` to issue a fresh token with the new scope set. See [connect Atlassian step 3](./connect-atlassian.md#3-add-the-scopes) for the complete scope list.

### Atlassian: `accessible-resources discovery failed`

**Cause.** After OAuth consent, arawn calls `https://api.atlassian.com/oauth/token/accessible-resources` to discover which `cloud_id`(s) your grant covers. If that call fails (network blip, rate limit, transient API), arawn retries up to a few times with exponential backoff and then surfaces a `NotConnected` error. The warning lands in the server log as `accessible-resources discovery attempt failed`.

**Fix.**

- If transient: just `/disconnect atlassian` and `/connect atlassian` again — the retry on a fresh flow usually succeeds.
- If persistent: the consent didn't actually grant access to any site. In the Atlassian consent flow you must pick at least one site; a "consented but no sites selected" outcome leaves accessible-resources empty. Redo `/connect` and confirm you pick a site.

### `[integration] error: invalid_grant` or `AuthExpired`

**Cause.** The stored refresh token expired or was revoked.

**Fix.**

```
/disconnect <svc>
/connect <svc>
```

If this keeps happening, your provider may be invalidating tokens early — check the provider's app/integration status (especially for Atlassian, which can deactivate untested apps). For Atlassian specifically, make sure your scope set includes `offline_access` — without it no refresh token is issued at all and the access token dies in ~1 hour.

### `Connection error: failed to reach <host>` / `network error`

**Cause.** Either the API isn't enabled in the provider console, or you have a network issue.

**Fix (Google specifically).** Each Google API needs its own enable click in the API library. Confirm Gmail / Calendar / Drive are each individually enabled at:

```
https://console.cloud.google.com/apis/library?project=<PROJECT>
```

(See step 2 of [connect Google](./connect-google.md).)

### `[integration] connected` but tools error with permission issues

**Cause.** Token cache vs. scope mismatch — common after adding new scopes; the token in the cache still has the old scope set.

**Fix.**

1. Revoke the existing grant at the provider's permissions page:
   - Google: <https://myaccount.google.com/permissions>
   - Slack: workspace settings → manage apps → your app
   - Atlassian: <https://id.atlassian.com/manage-profile/apps>
2. `/connect <svc>` fresh — you'll re-grant with the new scope set.

### `Address already in use (port 8080)` during Slack/Atlassian connect

**Cause.** Slack and Atlassian require an exact-match callback at `localhost:8080`. arawn binds to 8080 for those flows; if something else is already on the port, the bind fails.

**Fix.**

- Find and stop the conflicting process: `lsof -iTCP:8080 -sTCP:LISTEN`.
- Or wait 30-60 seconds — a recently-closed socket may still be in TIME_WAIT.

### `failed to persist refreshed token`

**Cause.** Google refreshed the access token successfully but arawn couldn't write the new token to `<data_dir>/tokens/<provider>.json.enc`. Almost always a filesystem permission / disk-full issue. The agent still works for the duration of the refreshed token (cached in memory), but the next restart will need a `/connect` again.

**Fix.** Check `ls -la <data_dir>/tokens/` for ownership / permissions issues. Make sure the disk has free space.

### `credential format error: ... decrypt failed (tampered?)`

**Cause.** The encrypted token file (`<data_dir>/tokens/<provider>.json.enc`) couldn't be decrypted with the key in `<data_dir>/tokens/key.bin`. Either:

- The `key.bin` file was regenerated/replaced after the token was written (very common: you copied the tokens directory between machines but not the key).
- The `.json.enc` file was modified / truncated.

**Fix.** `/disconnect <svc>` deletes the corrupted file. Then `/connect <svc>` to issue a fresh token under the current key.

## Exhaustive error reference

Every error variant raised by `arawn-integrations` and `arawn-auth`, what triggers it, and the user-facing message you'll see. Use the section above first for narrative debugging; this table is the catalog.

| Source | Variant | When | User-facing message |
|---|---|---|---|
| `IntegrationError` | `UnknownService(name)` | Slash command names an integration not registered | `No integration named 'X' is registered. Run /integrations to see what's available.` |
| `IntegrationError` | `NotConnected(name)` | Tool fired before `/connect` (or token deleted under it) | `Integration 'X' is not connected. Run /connect X to set it up.` |
| `IntegrationError` | `Auth(AuthError)` | Wraps every `AuthError` from the OAuth layer | `Authentication error: <inner>` |
| `IntegrationError` | `Io(io::Error)` | Disk failure reading/writing the credential file | `Credential storage error: <inner>` |
| `IntegrationError` | `Format(msg)` | Stored token decrypted-but-malformed, or post-decrypt JSON parse failed | `Credential format error: <msg>` |
| `IntegrationError` | `Provider(msg)` | Catch-all for upstream API failures (HTTP errors, network), and for provider-specific errors that don't fit other variants | `Provider error: <msg>` |
| `IntegrationError` | `RateLimited { retry_after }` | Upstream returned 429 (or typed equivalent). `retry_after` from `Retry-After` header. The feeds layer translates this to back-off | `Rate limited. Retry after Xs.` |
| `IntegrationError` | `Cancelled` | Browser-tab closed mid-flow, or timeout on the callback server | `OAuth flow cancelled.` |
| `AuthError` | `AuthExpired` | Refresh failed (refresh token revoked, etc.) | `authentication expired or invalid (re-run \`arawn setup\`)` |
| `AuthError` | `ApiError { status, body }` | Provider returned non-2xx HTTP. `body` is the response payload | `provider API error (<status>): <body>` |
| `AuthError` | `Network(msg)` | DNS / TLS / connect failure during OAuth flow | `network error: <msg>` |
| `AuthError` | `InvalidConfig(msg)` | Malformed redirect URL, missing OAuth params, CSRF state mismatch, encryption-key length wrong, etc. | `invalid configuration: <msg>` |
| `AuthError` | `Decode(msg)` | Provider's response body didn't parse, or on-disk token decryption failed | `failed to decode: <msg>` |

### Server-log warnings worth knowing

These never reach the user as an error, but appear in `<data_dir>/logs/server.log.*` and explain unexpected behaviour:

| Log message | Source | Meaning |
|---|---|---|
| `failed to persist refreshed token` | `google_common.rs:138` | Refreshed token couldn't write to disk; in-memory cache still works until next restart. |
| `accessible-resources discovery attempt failed` | `atlassian/integration.rs:392` | One retry attempt failed; arawn will retry with backoff. If all retries fail, surfaces as `NotConnected`. |
| `atlassian accessible-resources discovery failed` | `atlassian/integration.rs:312` | All retries exhausted. See "Atlassian: `accessible-resources discovery failed`" above. |

## When none of the above

Run `arawn serve` with verbose logging:

```
RUST_LOG=arawn=debug,arawn_integrations=trace arawn serve
```

The OAuth flow's HTTP traffic logs to stderr — request URLs, response status, error bodies. Most failures become obvious when you see the actual provider response.

## What's next

- Back to the per-provider connect guides: [Google](./connect-google.md), [Slack](./connect-slack.md), [Atlassian](./connect-atlassian.md), [GitHub](./connect-github.md).
- Token storage layout and encryption: [OAuth primer](../explanation/oauth-primer.md#what-arawn-does-with-the-tokens).
- Configuration resolution rules: [integrations config reference](../reference/integrations-config.md).
