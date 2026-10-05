# Integrations config

*Reference. How arawn resolves OAuth credentials per integration — env vars, `[integrations.<svc>]` blocks, and the shared `[integrations.google]` fallback.*

Source: `crates/arawn/src/oauth_clients.rs` (`OAuthClientResolver`). Server startup, `arawn doctor` and `arawn setup` all use this resolver.

## The lookup precedence

For each Google, Slack and Atlassian integration, arawn tries these tiers in sequence. The first tier that gives **both** a `client_id` and a `client_secret` supplies the client:

1. **Service tier.** The service env vars (for example `ARAWN_GMAIL_CLIENT_ID`), then the service TOML block (for example `[integrations.gmail]`).
2. **Shared Google tier** (Gmail, Calendar and Drive only). The `ARAWN_GOOGLE_*` env vars, then the `[integrations.google]` block.
3. **Bundled tier.** A shared client that is compiled into the arawn binary. This build has no bundled clients.
4. If no tier is complete, arawn skips the integration. No tools register, and the server starts without it.

In a tier, each field is found separately: the env var first, then the TOML value. Thus you can put `client_id` in an env var and `client_secret` in `arawn.toml`.

A tier does not borrow fields from a different tier. If `[integrations.gmail]` has a `client_id` but no `client_secret`, arawn does not use the shared Google secret with it. It uses the full shared Google client.

An **empty string counts as unset**. A blank TOML field does not hide the next source.

## Per-service catalog

### Google (Gmail, Calendar, Drive)

| Service | Specific env vars | Specific TOML block | Shared fallback |
|---|---|---|---|
| Gmail | `ARAWN_GMAIL_CLIENT_ID`, `ARAWN_GMAIL_CLIENT_SECRET` | `[integrations.gmail]` | `ARAWN_GOOGLE_*` → `[integrations.google]` |
| Calendar | `ARAWN_GCAL_CLIENT_ID`, `ARAWN_GCAL_CLIENT_SECRET` | `[integrations.calendar]` | `ARAWN_GOOGLE_*` → `[integrations.google]` |
| Drive | `ARAWN_GDRIVE_CLIENT_ID`, `ARAWN_GDRIVE_CLIENT_SECRET` | `[integrations.drive]` | `ARAWN_GOOGLE_*` → `[integrations.google]` |

Note the env-var naming: Calendar uses `GCAL`, Drive uses `GDRIVE`, Gmail uses `GMAIL`. They don't all share a "google" prefix.

### Slack and Atlassian

These don't have a shared fallback — only the service-specific path resolves them.

| Service | Env vars | TOML block |
|---|---|---|
| Slack | `ARAWN_SLACK_CLIENT_ID`, `ARAWN_SLACK_CLIENT_SECRET` | `[integrations.slack]` |
| Atlassian (Jira + Confluence) | `ARAWN_ATLASSIAN_CLIENT_ID`, `ARAWN_ATLASSIAN_CLIENT_SECRET` | `[integrations.atlassian]` |

One Atlassian client covers both Jira and Confluence — they're a single OAuth app under Atlassian's developer console.

### GitHub

GitHub uses the App model, not OAuth. Different shape:

| Env var | TOML field in `[integrations.github]` | Description |
|---|---|---|
| `ARAWN_GITHUB_APP_ID` | `app_id` | Numeric App ID. |
| `ARAWN_GITHUB_APP_SLUG` | `app_slug` | URL slug from the App's settings page. |
| `ARAWN_GITHUB_PRIVATE_KEY_PATH` | `private_key_path` | Filesystem path to the RSA private key PEM. Preferred. |
| `ARAWN_GITHUB_PRIVATE_KEY_PEM` | *(none)* | Inline PEM body. Env-only, alternative to `_PATH`. |

## Pick a path: per-service vs shared Google

You have one decision when configuring Gmail/Calendar/Drive: one OAuth client for all three, or one per service.

| Path | When to pick it |
|---|---|
| **Shared `[integrations.google]`** | You manage one Google Cloud project. One client_id, one consent screen, scopes from all three services land on it. **Recommended default** — matches how the Google Cloud Console wants you to work. |
| **Per-service `[integrations.gmail]` / `[integrations.calendar]` / `[integrations.drive]`** | You want isolated OAuth clients per service — different consent-screen branding per service, scope-rotation independence, etc. Two or three Google Cloud projects, or one project with multiple OAuth clients. |

You can mix: define `[integrations.google]` for the services you want to share, and `[integrations.gmail]` for the one you want isolated. The per-service block, when set, beats the shared fallback.

## Worked examples

### One Google client, plaintext in arawn.toml

Easiest case. One OAuth client registered in Google Cloud; both id and secret in TOML.

```toml
[integrations.google]
client_id     = "955517163683-xxxxxxxxxxxxxxxxxxxxxxxx.apps.googleusercontent.com"
client_secret = "GOCSPX-xxxxxxxxxxxxxxxxxxxxxxxx"
```

All three Google services pick this up. `/connect gmail`, `/connect google_calendar`, `/connect google_drive` all use it.

### Same Google client, secret in env (paranoid mode)

Keep the secret out of arawn.toml by sourcing it from the shell:

```toml
[integrations.google]
client_id = "955517163683-xxxxxxxxxxxxxxxxxxxxxxxx.apps.googleusercontent.com"
# client_secret intentionally omitted — comes from ARAWN_GOOGLE_CLIENT_SECRET
```

```sh
export ARAWN_GOOGLE_CLIENT_SECRET="GOCSPX-xxxxxxxxxxxxxxxxxxxxxxxx"
arawn serve
```

Resolution: `client_id` comes from the TOML block (env unset → empty → falls through to TOML). `client_secret` comes from the env var (env set → wins).

### Per-service Gmail with shared fallback for Calendar + Drive

```toml
[integrations.gmail]
client_id     = "<gmail-specific-id>.apps.googleusercontent.com"
client_secret = "GOCSPX-<gmail-specific>"

[integrations.google]
client_id     = "<shared-id>.apps.googleusercontent.com"
client_secret = "GOCSPX-<shared>"
```

Resolution: Gmail uses the gmail-specific client (chain step 2 wins). Calendar and Drive fall through to the shared `[integrations.google]` (chain step 4).

### Slack via env-only (no TOML edit needed)

```sh
export ARAWN_SLACK_CLIENT_ID="123456789.0123456789"
export ARAWN_SLACK_CLIENT_SECRET="0123456789abcdef..."
arawn serve
```

No `[integrations.slack]` block required. Slack registers if both env vars are non-empty; skipped otherwise.

## When an integration silently doesn't register

If both the specific and shared resolution chains end empty, arawn logs a `debug!` line at startup ("Gmail integration skipped — set ARAWN_GMAIL_CLIENT_ID + ARAWN_GMAIL_CLIENT_SECRET (env) or [integrations.gmail] (config) to enable.") and proceeds without that integration. The `/integrations` slash command will show only the services that registered.

If you expected an integration to register and it didn't, run with `RUST_LOG=arawn=debug arawn serve` to see the skip message.

## See also

- [Environment variables reference](./env-vars.md) — the complete env-var catalog including non-integration variables.
- [Integrations reference](./integrations.md) — per-provider matrix: scopes, tools, feed templates, permission behaviour.
- [Configuration schema reference](./config-schema.md) — the full `arawn.toml` shape.
- [Connect Google how-to](../how-to/connect-google.md), [Slack](../how-to/connect-slack.md), [Atlassian](../how-to/connect-atlassian.md), [GitHub](../how-to/connect-github.md) — provider-side setup walkthroughs.
