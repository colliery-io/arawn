# Integrations overview

*Explanation. What integrations arawn ships, why setup is what it is today, and how to decide which config path fits you.*

## What an integration is, in 30 seconds

An **integration** wires arawn to one external account — your Gmail, your Slack workspace, your Atlassian site. Two things happen when an integration is connected:

1. A handful of agent **tools** register (`gmail_search`, `slack_post`, `jira_get_issue`, …) — the agent can read and act on that provider during a turn.
2. **Feed templates** for that provider become usable — recurring ingest jobs that mirror data locally for grep-friendly access. See the [feeds explanation](./feeds.md).

Today arawn ships integrations for Google (Gmail + Calendar + Drive), Slack, Atlassian (Jira + Confluence), and GitHub. The exact tools-per-provider and feed templates are in the [integrations reference](../reference/integrations.md).

## The BYO reality

Connecting an integration today is a **bring-your-own-OAuth-app** affair. You register an OAuth app at the provider (Google Cloud Console, api.slack.com, developer.atlassian.com), copy the credentials into arawn, then `/connect <svc>`. The fact that you have to register an app at all is a documented v1 trade-off — initiative I-0037 covers the longer-term plan to ship a vendor-owned shared OAuth client so users can `/connect` without any pre-setup. Until that lands, BYO is the path.

If you've never set up an OAuth integration before, read the [OAuth primer](./oauth-primer.md) first. It explains what a client_id is, what scopes mean, and what the "this app is unverified" warning is about. The per-provider how-tos assume that vocabulary.

## Choosing a config path

Each integration needs two things: a `client_id` and a `client_secret`. You can source them from either:

- **An environment variable** — `ARAWN_GMAIL_CLIENT_ID`, `ARAWN_SLACK_CLIENT_SECRET`, etc.
- **An `arawn.toml` block** — `[integrations.gmail]`, `[integrations.slack]`, etc.

For Google services there's a third option: a **shared `[integrations.google]` block** that covers Gmail, Calendar, and Drive at once. Most Google Cloud projects host one OAuth client used by all three services, so the shared block is the recommended default.

The full resolution order (env → service-specific TOML → shared Google fallback, per field) is documented in [the integrations config reference](../reference/integrations-config.md).

### Pick the path that fits how you'll edit credentials

| If you... | Pick |
|---|---|
| Manage one Google Cloud project, no secret-management tooling | `[integrations.google]` in `arawn.toml`, plaintext |
| Want secrets out of `arawn.toml` (it's `git` adjacent, etc.) | env vars (`ARAWN_*_CLIENT_SECRET=…` in your shell) |
| Want isolated OAuth clients per Google service | `[integrations.gmail]` / `[integrations.calendar]` / `[integrations.drive]` |
| Rotate credentials per shell session for testing | env vars override TOML — set them ad-hoc |

You can mix freely — the resolver walks each field through its chain independently, so a TOML `client_id` paired with an env `client_secret` works fine.

## Why does setup take so long?

Each provider's developer console wants 8–12 clicks before they hand you a client_id. The walkthroughs in this section budget about 10 minutes per provider; the bottleneck isn't arawn, it's the consent-screen forms.

Two things that account for most of the time:

- **Enabling the right APIs.** Especially on Google, scopes you'll want don't show up in the scope picker until the underlying API is enabled in the project.
- **Configuring the OAuth consent screen.** Most providers want a name, support email, scope justifications, and a list of test users before the app can be used. None of it is hard; there's just a lot of it.

The [debug OAuth failures how-to](../how-to/debug-oauth-failures.md) covers the common ways setup goes sideways.

## Where the tokens live

After `/connect` completes, the access + refresh tokens get stored encrypted under `<data_dir>/tokens/<provider>.json.enc` using ChaCha20-Poly1305 with a key generated once at `<data_dir>/tokens/key.bin`. Tokens never leave the machine. `/disconnect <svc>` deletes the file; revoking the provider-side authorization is a separate step (see the [OAuth primer](./oauth-primer.md#what-arawn-does-with-the-tokens)).

## Start here

The fastest path is `arawn setup`. It shows each provider's console steps and scopes, asks for the credentials, and writes `arawn.toml` for you. See [`arawn setup`](../reference/cli.md#arawn-setup). The pages below give the full detail.

1. **Concept:** [OAuth primer](./oauth-primer.md) — read first if you're new to OAuth.
2. **Reference:** [Integrations config](../reference/integrations-config.md) — exact env var names and TOML keys; [Integrations reference](../reference/integrations.md) — per-provider tools and scopes.
3. **Walkthroughs:** [Google](../how-to/connect-google.md), [Slack](../how-to/connect-slack.md), [Atlassian](../how-to/connect-atlassian.md), [GitHub](../how-to/connect-github.md).
4. **When something breaks:** [Debug OAuth failures](../how-to/debug-oauth-failures.md).
