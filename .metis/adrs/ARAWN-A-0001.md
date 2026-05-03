---
id: 001-credential-storage-and-oauth-ux
level: adr
title: "Credential storage and OAuth UX for external integrations"
number: 1
short_code: "ARAWN-A-0001"
created_at: 2026-05-03T12:41:26.990405+00:00
updated_at: 2026-05-03T12:41:26.990405+00:00
decision_date: 2026-05-03
decision_maker: dstorey

tags:
  - "#adr"
  - "#phase/decided"

initiative_id: ARAWN-I-0033
---

# ARAWN-A-0001: Credential storage and OAuth UX for external integrations

## Status

Decided 2026-05-03.

## Context

ARAWN-I-0033 introduces external integrations (Gmail, Calendar, Slack/notifications). Each integration needs:

1. A way to store and refresh OAuth credentials safely.
2. A user flow for granting access in the first place — the TUI runs in a terminal, the OAuth provider expects a browser callback.

The `arawn-auth` crate already exists with token storage, an OAuth2 helper, and a localhost callback server (used by an earlier provider). The decision below extends that pattern to multiple integrations rather than re-litigating it per service.

## Decision

### 1. Credential storage: filesystem, encrypted at rest

OAuth tokens for external integrations are stored on the filesystem under `~/.arawn/integrations/<service>/credentials.age`, encrypted with the same identity used for the existing `~/.arawn/secrets.age`.

Rationale:
- Matches the existing `secrets.age` pattern. Users already have one identity; adding integrations doesn't introduce a new ceremony.
- Filesystem is portable across the platforms arawn targets. OS keychain (`keyring` crate) would be more secure but: (a) varies per OS, (b) breaks the "your data lives in `~/.arawn`" mental model, (c) age-encrypted-at-rest is good enough for v1 given the threat model (single-user, local-only server).
- Refresh tokens are durable; access tokens are derived/cached in-memory only.

Revisit when: integrations carry tokens with broader scopes (e.g. write-everywhere admin tokens), or arawn ships a multi-user mode.

### 2. OAuth flow: in-TUI, browser callback to localhost

The TUI gets a `/connect <service>` slash-command. On invocation:

1. The agent or TUI calls `start_oauth_flow(service)` RPC on the server.
2. Server starts a localhost callback listener on a high port and returns the authorization URL.
3. TUI prints the URL and attempts `open` (macOS) / `xdg-open` (Linux) / equivalent. Falls back to "open this URL in your browser" copy-pasteable text if no opener is available.
4. User completes the flow in the browser. Provider redirects to the localhost callback with the auth code.
5. Server exchanges the code for tokens, encrypts and stores them, shuts down the callback listener, sends a `ServerNotice` to the TUI confirming success.
6. TUI shows the confirmation as a system message and the integration becomes usable.

Rationale:
- Keeps the TUI as the single user surface. No separate `arawn auth gmail` CLI subcommand to discover.
- The TUI-as-broker model means an SSH user (no local browser) can still complete the flow by copying the URL to a browser elsewhere — the localhost callback works as long as port forwarding or tunneling is set up. Document this caveat.
- Reuses the existing `arawn-auth::server` callback handling.
- A `ServerNotice` post-flow piggybacks on the broadcast plumbing landed in T-0199.

Revisit when: a use case for non-interactive credential setup emerges (e.g. headless automation) — at which point a `arawn auth import-tokens` CLI becomes the right addition, *complementing* the TUI flow rather than replacing it.

### 3. Integration trait shape (for consumers)

Each integration crate exposes:

```rust
#[async_trait]
pub trait Integration: Send + Sync {
    /// Stable service name — "gmail", "google_calendar", "slack".
    fn name(&self) -> &str;

    /// True if credentials are present and (probably) valid.
    /// Cheap check — does NOT round-trip to the provider.
    async fn is_connected(&self) -> bool;

    /// Drive the OAuth flow end-to-end. Used by `/connect <service>`.
    /// Returns when tokens have been stored or the flow was abandoned.
    async fn connect(&self, ctx: &dyn ConnectContext) -> Result<(), IntegrationError>;

    /// Drop stored credentials. Used by `/disconnect <service>`.
    async fn disconnect(&self) -> Result<(), IntegrationError>;
}
```

Tools that wrap an integration are normal `arawn-engine::Tool` impls — they receive the engine context, look up the integration by name, and use whatever provider-specific client it exposes. The `Integration` trait is for *connection lifecycle*, not for the tool surface.

Rationale: keeps the agent-facing tool API identical to today. Integrations are a behind-the-scenes concept that tools depend on; the agent never sees them directly.

## Consequences

- New crate `arawn-integrations` (or similar) hosts the trait + the OAuth UX plumbing. Each service is a sub-module or sibling crate.
- `LocalService` gains an `integration_registry: HashMap<String, Arc<dyn Integration>>` field plus `start_oauth_flow(service)` and `disconnect_integration(service)` RPCs.
- Tools that need an integration (e.g. `gmail_inbox_read`) take `Arc<GmailIntegration>` at construction time, just like memory tools take `Arc<MemoryManager>`.
- Per-service credential paths under `~/.arawn/integrations/<service>/`. The single age identity from `~/.arawn/identity.age` decrypts all of them.
- Tokens are NOT in `arawn.toml` — keeps the design call from T-0194 ("TOML for config, env vars for secrets, credentials encrypted at rest in `~/.arawn/`") consistent.

## Alternatives considered

- **OS keychain via `keyring` crate.** Better security; per-platform variability and "where did my data go?" UX cost outweigh it for v1.
- **Out-of-band CLI for OAuth (`arawn auth gmail`).** Simpler to implement (no TUI ↔ server dance) but discoverable only by reading docs. Rejected for v1; can be added later as a complement.
- **Per-integration trait that subsumes the tool layer.** Considered exposing tools via the integration trait directly. Rejected — integrations and tools have different lifetimes (integrations connect once, tools fire many times) and different audiences (integrations are operator concerns, tools are agent concerns).
