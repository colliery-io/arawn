# OAuth primer

*Explanation. Plain-English model of what OAuth is, what the four moving parts mean, and what arawn actually does with the tokens.*

If you've never set up an OAuth integration before, the provider docs assume more vocabulary than you have. The walkthroughs in this section ([Google](../how-to/connect-google.md), [Slack](../how-to/connect-slack.md), [Atlassian](../how-to/connect-atlassian.md), [GitHub](../how-to/connect-github.md)) tell you which buttons to click. This page tells you what those buttons mean.

## The one-sentence pitch

OAuth lets a third-party app (arawn) act on your behalf at a provider (Gmail, Slack, …) without you handing it your password. You authorize a specific scope of access; the provider issues a token; arawn presents that token instead of credentials.

## The four moving parts

Every OAuth provider asks you to register an "app" with four pieces. The names vary a little; the shapes don't.

| Piece | What it is | Where it lives |
|---|---|---|
| **Client ID** | A public identifier for the registered app. Not a secret — it's safe to commit. | Provider's developer console, copied into arawn config. |
| **Client secret** | A password the app uses to prove it's the registered one. Treat like an API key. | Provider's developer console, copied into arawn config. |
| **Scope(s)** | What permissions the app is asking for (`gmail.readonly`, `calendar.events`, `chat:write`). | Provider's developer console, fixed at app-registration time for some providers, requested per-flow for others. |
| **Redirect URI** | Where the provider sends the user back to after they click Allow. arawn runs a local callback server, so this is a localhost URL. | Provider's developer console, must exactly match what arawn presents at runtime. |

The flow at `/connect`-time, end to end:

1. arawn starts a local web server on a free port (e.g. `http://localhost:54321/callback`).
2. arawn opens your browser to the provider's consent page. The URL embeds the client_id, scopes, and the redirect URI it just bound.
3. You sign in at the provider, see "arawn wants access to X", click Allow.
4. The provider redirects your browser to the localhost URL, attaching an authorization code.
5. arawn's callback server catches the code, exchanges it (server-to-server) for an access token + refresh token using the client_secret.
6. arawn stores both tokens encrypted on disk (see below).
7. Next time the agent calls a Gmail/Slack/etc. tool, arawn uses the stored token — refreshing it silently when it expires.

## Why every provider feels different

There's a single OAuth 2.0 spec but each provider built their own developer console, their own scope vocabulary, and their own review process. The substance is the same; the chrome isn't.

- **Google** asks you to enable APIs *before* the matching scopes show up in the picker. Adding Drive scopes won't work until you enable the Drive API on the project.
- **Slack** distinguishes "bot tokens" (the app posts as itself) from "user tokens" (the app posts as you). arawn requests both.
- **Atlassian** wraps Jira and Confluence under one OAuth app and adds an extra step after consent — calling `accessible-resources` to discover the user's `cloud_id`, which is then bundled with the token.
- **GitHub** uses a different model entirely: a GitHub *App* (numeric `app_id` + RSA private key + URL slug) rather than client_id/client_secret pairs. arawn treats it as a feed-only read source.

The per-provider how-tos cover those specifics. This page is for understanding what they all share.

## The "this app is unverified" warning

Providers want apps to go through a review process before they can request sensitive data (Gmail content, Drive files, etc.) from arbitrary users. Until your app is verified, the consent screen warns the user that the app is unverified and limits how many "test users" the app can have (Google caps at 100).

For personal use, this is fine — you add yourself as a test user and click through the warning. The wall exists to slow down phishing apps, not solo developers using their own credentials.

A future arawn release may ship a verified shared OAuth app so users don't have to register their own; that's tracked by initiative I-0037 and is gated on the project's distribution model decision.

## What arawn does with the tokens

Once arawn has tokens, it stores them under `<data_dir>/tokens/` (default `~/.arawn/tokens/`). Per provider, one file:

```
<data_dir>/tokens/
├── key.bin                # 256-bit ChaCha20-Poly1305 key, generated once
├── gmail.json.enc         # encrypted refresh+access tokens
├── slack.json.enc
├── atlassian.json.enc
└── ...
```

Each `<provider>.json.enc` is a JSON blob encrypted with ChaCha20-Poly1305 using the key in `key.bin`. The directory and key file are created with restricted permissions (owner-only read/write).

`/disconnect <svc>` deletes the corresponding `.json.enc` file. The provider-side authorization isn't revoked — to fully cut the cord, also visit the provider's "connected apps" page (e.g. `myaccount.google.com/permissions` for Google) and revoke from there.

## What arawn doesn't do

- **No outbound token sharing.** Tokens never leave the machine arawn runs on. They are not uploaded to an arawn-operated server (there isn't one).
- **No multi-account per provider.** Today each provider stores one token under a fixed key (e.g. `gmail.json.enc`). Multi-account / multi-workspace support is per-provider future work (I-0034 covers Slack specifically).
- **No identity sharing across providers.** Each integration is its own OAuth grant. Disconnecting one doesn't touch the others.

## See also

- [Integrations overview](./integrations-overview.md) — what integrations arawn has and the decision tree for setting them up.
- [Integrations config reference](../reference/integrations-config.md) — the precise resolution rules and env-var names.
- [Permission model](./permission-model.md) — how the agent's tool access (separate from OAuth) is gated.
- [Debug OAuth failures](../how-to/debug-oauth-failures.md) — symptom-keyed troubleshooting.
