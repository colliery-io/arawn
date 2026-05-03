---
id: arawn-integrations-crate
level: task
title: "arawn-integrations crate scaffolding + Integration trait + connect/disconnect RPCs"
short_code: "ARAWN-T-0200"
created_at: 2026-05-03T12:43:21.447837+00:00
updated_at: 2026-05-03T12:43:21.447837+00:00
parent: ARAWN-I-0033
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0033
---

# arawn-integrations crate scaffolding + Integration trait + connect/disconnect RPCs

## Parent Initiative

[ARAWN-I-0033](../initiative.md)

## Objective

Lay the foundation that every integration (Gmail, Calendar, Slack) builds on: a new `arawn-integrations` crate housing the `Integration` trait, the credential-storage helpers (age-encrypted under `~/.arawn/integrations/<service>/`), the localhost-callback OAuth dance, and the server-side `start_oauth_flow` / `disconnect_integration` / `list_integrations` RPC methods.

This task does not ship a single user-visible feature. It ships the contract that the next three tasks need.

## Type / Priority
- Feature (foundational)
- P1 — Blocker for everything else in I-0033.

## Acceptance Criteria

- [ ] New crate `crates/arawn-integrations/` added to the workspace.
- [ ] `Integration` trait per ARAWN-A-0001: `name() / is_connected() / connect() / disconnect()`.
- [ ] Credential helpers: `store_credentials(service, &Credentials)` / `load_credentials(service) -> Option<Credentials>` / `remove_credentials(service)`. All operations age-encrypt against `~/.arawn/identity.age`. Per-service path under `~/.arawn/integrations/<service>/credentials.age`.
- [ ] OAuth helper: `OAuthFlow::start(provider_config) -> (auth_url, callback_handle)` and `callback_handle.await -> Result<TokenSet>`. Wraps `arawn-auth::server` for the localhost listener and `arawn-auth::oauth2` for the code-exchange.
- [ ] `LocalService` gains `integration_registry: HashMap<String, Arc<dyn Integration>>` plus three RPC methods: `start_oauth_flow(service)`, `disconnect_integration(service)`, `list_integrations()` (returns each service's name + is_connected status).
- [ ] `ws_server.rs` wires the three RPCs into the JSON-RPC method table.
- [ ] Successful OAuth completion broadcasts a `ServerNotice` (T-0199) so the TUI can confirm without polling.
- [ ] Unit tests for: credential round-trip (encrypt → store → load → matches), trait conformance with a `MockIntegration`, and the integration registry's `list` shape.

## Implementation Notes

- The new crate has dependencies on `arawn-auth`, `arawn-service` (for ServerNotice), and probably `age`. Keep it free of `arawn-engine` to avoid pulling in the world.
- `OAuthFlow` should NOT hard-code Google specifics — provider config is opaque to it (auth URL, token URL, scopes, client_id, client_secret). Each integration crate (T-0202 etc.) supplies its own provider config.
- For credential storage, mirror the existing `secrets.age` pattern: read identity.age once, derive a single age recipient/identity, use it for every encrypt/decrypt. Don't introduce a new key system.
- Don't put OAuth client_id / client_secret in the public repo. Document the env vars (`ARAWN_GMAIL_CLIENT_ID`, etc.) the integration tasks expect, and ship a doc-only example arawn.toml stanza.
- Per the design call from T-0194: secrets via env vars; the OAuth refresh tokens that the user grants are credentials (encrypted at rest), not config.

## Status Updates

*To be added during implementation*
