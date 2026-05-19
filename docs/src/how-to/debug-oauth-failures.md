# Debug OAuth failures

*How-to. Diagnose common OAuth errors when connecting an integration.*

This page expands the error table from the connect-* how-tos. Find your symptom in the left column, follow the diagnosis steps.

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

### `[integration] error: invalid_grant`

**Cause.** The stored refresh token expired or was revoked.

**Fix.**

```
/disconnect <svc>
/connect <svc>
```

If this keeps happening, your provider may be invalidating tokens early — check the provider's app/integration status (especially for Atlassian, which can deactivate untested apps).

### `Connection error: failed to reach <host>`

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

## When none of the above

Run `arawn serve` with verbose logging:

```
RUST_LOG=arawn=debug,arawn_integrations=trace arawn serve
```

The OAuth flow's HTTP traffic logs to stderr — request URLs, response status, error bodies. Most failures become obvious when you see the actual provider response.

## What's next

- Back to the per-provider connect guides: [Google](./connect-google.md), [Slack](./connect-slack.md), [Atlassian](./connect-atlassian.md), [GitHub](./connect-github.md).
- Token storage layout: [data directory reference](../reference/data-directory.md).
