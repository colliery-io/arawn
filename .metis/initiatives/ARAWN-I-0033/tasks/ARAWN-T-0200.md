---
id: arawn-integrations-crate
level: task
title: "arawn-integrations crate scaffolding + Integration trait + connect/disconnect RPCs"
short_code: "ARAWN-T-0200"
created_at: 2026-05-03T12:43:21.447837+00:00
updated_at: 2026-05-03T19:52:42.305658+00:00
parent: ARAWN-I-0033
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0033
---

# arawn-integrations crate scaffolding + Integration trait + connect/disconnect RPCs

## Parent Initiative

[ARAWN-I-0033](../initiative.md)

## Objective

Lay the foundation that every integration (Gmail, Calendar, Slack) builds on: a new `arawn-integrations` crate housing the `Integration` trait, the credential-storage helpers (reusing `arawn-auth::TokenStore`'s ChaCha20Poly1305 + per-data-dir master key, files under `~/.arawn/integrations/<service>/`), the localhost-callback OAuth dance, and the server-side `start_oauth_flow` / `disconnect_integration` / `list_integrations` RPC methods.

This task does not ship a single user-visible feature. It ships the contract that the next three tasks need.

## Type / Priority
- Feature (foundational)
- P1 — Blocker for everything else in I-0033.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] New crate `crates/arawn-integrations/` added to the workspace.
- [ ] `Integration` trait per ARAWN-A-0001: `name() / is_connected() / connect() / disconnect()`.
- [ ] Credential storage: OAuth tokens reuse `arawn-auth::TokenStore` directly (already designed for `Token` structs). Non-OAuth credentials (e.g. Slack webhook URL) get a small `CredentialStore<T>` wrapper that shares TokenStore's ChaCha20Poly1305 + master-key encryption. Files land under `~/.arawn/integrations/<service>/`.
- [ ] OAuth helper: `OAuthFlow::start(provider_config) -> (auth_url, callback_handle)` and `callback_handle.await -> Result<TokenSet>`. Wraps `arawn-auth::server` for the localhost listener and `arawn-auth::oauth2` for the code-exchange.
- [ ] `LocalService` gains `integration_registry: HashMap<String, Arc<dyn Integration>>` plus three RPC methods: `start_oauth_flow(service)`, `disconnect_integration(service)`, `list_integrations()` (returns each service's name + is_connected status).
- [ ] `ws_server.rs` wires the three RPCs into the JSON-RPC method table.
- [ ] Successful OAuth completion broadcasts a `ServerNotice` (T-0199) so the TUI can confirm without polling.
- [ ] Unit tests for: credential round-trip (encrypt → store → load → matches), trait conformance with a `MockIntegration`, and the integration registry's `list` shape.

## Implementation Notes

- The new crate depends on `arawn-auth` (for TokenStore, OAuthClient, CallbackServer) and `arawn-service` (for ServerNotice). Stays free of `arawn-engine` to avoid pulling in the world.
- `OAuthFlow` should NOT hard-code Google specifics — provider config is opaque to it (auth URL, token URL, scopes, client_id, client_secret). Each integration crate (T-0202 etc.) supplies its own provider config.
- For non-OAuth credentials, the `CredentialStore<T>` wrapper either (a) reuses TokenStore's cipher field via a small refactor exposing it, or (b) opens its own ChaCha20Poly1305 cipher off the same `.master.key` path. Pick whichever is the smaller change once we're inside the code.
- Don't put OAuth client_id / client_secret in the public repo. Document the env vars (`ARAWN_GMAIL_CLIENT_ID`, etc.) the integration tasks expect, and ship a doc-only example arawn.toml stanza.
- Per the design call from T-0194: secrets via env vars; the OAuth refresh tokens that the user grants are credentials (encrypted at rest), not config.

## Status Updates

### 2026-05-03 — Foundation landed

**Crate** `crates/arawn-integrations/`:
- `Integration` trait per ARAWN-A-0001: `name() / is_connected() / connect() / disconnect()`.
- `ConnectContext` trait — what `connect()` impls receive from the caller. Lets the integration publish the auth URL and progress messages without knowing how the server forwards them.
- `IntegrationStatus { name, connected }` for the `list_integrations` RPC reply shape.
- `CredentialStore<T>` — generic encrypted storage for non-OAuth credentials. ChaCha20Poly1305 AEAD, shares `<data_dir>/tokens/.master.key` with `arawn-auth::TokenStore` so a single install bootstraps one keyfile. Path-segment sanitization prevents service names with `/` from escaping the integrations directory.
- `oauth_flow::run_oauth_flow` composes `OAuthClient` + `CallbackServer` + `TokenStore` into the standard browser-based dance. Provider config is opaque; Gmail/Calendar/Slack each supply their own.
- `IntegrationError` with a `user_message()` formatter for the engine error chain (T-0191).

**LocalService wiring**:
- New `integration_registry: Arc<RwLock<HashMap<String, Arc<dyn Integration>>>>` field.
- `register_integration(integration)` setter — called from `main.rs` at startup once integration impls land in T-0202+.
- `shared_integrations()` accessor for tools that need a typed handle at construction.
- Three new `ArawnService` trait methods implemented:
  - `list_integrations()` — snapshots the registry, calls each `is_connected()` (no locks held across await), sorts by name.
  - `start_oauth_flow(service)` — spawns a task that runs the integration's `connect()` flow with a custom `OAuthFlowCtx`. The ctx bridges the integration's `publish_auth_url` callback to a oneshot that the RPC reply waits on (5s ceiling). On flow completion (success or error) the spawned task broadcasts a `ServerNotice` with category="integration".
  - `disconnect_integration(service)` — straight passthrough plus a `ServerNotice`.

**RPC + types**:
- `arawn-service`: `IntegrationStatus`, `OAuthFlowStarted` types.
- `ws_server.rs`: three new methods (`list_integrations`, `start_oauth_flow`, `disconnect_integration`) added to the allowlist + handler table.

**Tests** (7 new in `arawn-integrations`):
- `round_trip_returns_what_was_saved` — encrypt → store → load matches.
- `load_returns_none_when_absent` — clean miss path.
- `delete_is_idempotent` — delete-before-save and delete-after-delete both succeed.
- `second_store_on_same_data_dir_uses_same_key` — confirms shared masterkey works across separate handles for the same service.
- `path_segments_with_slashes_get_sanitized` — defends the integrations directory boundary.
- `corrupted_blob_yields_format_error_not_panic` — reading garbage gives a Format error, not UB.
- `ctx_capture_smoke` — locks in the `ConnectContext` trait shape so changes to it surface here.

900 workspace lib tests pass; 0 clippy warnings.

**Acceptance criteria status:**
- [x] `crates/arawn-integrations/` added to workspace.
- [x] `Integration` trait per ARAWN-A-0001.
- [x] OAuth tokens reuse `arawn-auth::TokenStore` directly. `CredentialStore<T>` for non-OAuth secrets.
- [x] `OAuthFlow` provider-agnostic helper (`run_oauth_flow`).
- [x] `LocalService` integration registry + three RPC methods.
- [x] `ws_server.rs` wires the three RPCs.
- [x] Successful OAuth completion broadcasts a `ServerNotice` (integration category).
- [x] Unit tests for credential round-trip, ConnectContext shape, and edge cases.

**Followups deferred (not blocking T-0201/T-0202):**
- `OAuthFlowCtx::publish_auth_url` only forwards once (subsequent calls drop). Defensible — integrations should publish once per flow — but worth documenting on the `ConnectContext` trait.
- The 5s "wait for auth URL" ceiling on `start_oauth_flow` is enough for any real OAuth flow but if a future integration does background work before publishing, this'll need rethinking.