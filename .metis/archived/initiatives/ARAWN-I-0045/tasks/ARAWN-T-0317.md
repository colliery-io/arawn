---
id: github-app-oauth-arawn
level: task
title: "GitHub App OAuth + arawn-integrations scaffold"
short_code: "ARAWN-T-0317"
created_at: 2026-05-18T12:15:07.992922+00:00
updated_at: 2026-05-18T12:36:19.636847+00:00
parent: ARAWN-I-0045
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0045
---

# GitHub App OAuth + arawn-integrations scaffold

## Parent Initiative

[[ARAWN-I-0045]]

## Objective

Stand up `crates/arawn-integrations/src/github/` with the GitHub
App installation flow, encrypted token storage, and an
`Integration` trait impl that's ready for feed templates to call.
No data pull yet — substrate only.

## Acceptance Criteria

## Acceptance Criteria

- [x] New module `arawn-integrations::github` with `mod.rs`,
      `integration.rs`, `install_flow.rs`, `client.rs`. (App
      model is different enough from Gmail's OAuth-app flow that
      `install_flow` warranted its own file.)
- [x] `connect()` opens the public install URL
      `https://github.com/apps/<slug>/installations/new` with a
      CSRF `state` parameter; the user picks org/repos and is
      redirected to arawn's local callback.
- [x] Callback handler captures `installation_id` (+ `setup_action`)
      via the new `CallbackServer::listen_raw` helper added on
      `arawn-auth`. CSRF state verified before persisting.
- [x] Per-call installation-access-token minted from RS256 JWT
      via `POST /app/installations/{id}/access_tokens`. Cached
      in-memory with `expires_at`; auto-refreshed when within 5
      min of expiry.
- [x] `GithubCredentials` (installation_id + setup_action) stored
      encrypted in `CredentialStore<GithubCredentials>` —
      ChaCha20Poly1305 + per-data-dir master key (ARAWN-A-0001).
- [x] `Integration::is_connected` is a disk-only check; tools/feeds
      will call `client.health_check()` (GET
      `/installation/repositories?per_page=1`) on demand.
- [x] Config `[integrations.github]` accepts `app_id`,
      `app_slug`, `private_key_path`. Env overrides:
      `ARAWN_GITHUB_APP_ID`, `ARAWN_GITHUB_APP_SLUG`,
      `ARAWN_GITHUB_PRIVATE_KEY_PATH` /
      `ARAWN_GITHUB_PRIVATE_KEY_PEM`.
- [x] 9 unit tests covering JWT signing, token freshness window,
      install-URL shape, credential round-trip, disconnect, and
      capabilities-summary gating.
- [x] Operator setup docs at `docs/src/integrations/github.md`.

## Status Updates

### 2026-05-18 — substrate landed

- `arawn-auth::CallbackServer::listen_raw` added to support
  non-OAuth-shaped callbacks (GitHub's install callback uses
  `installation_id` + `setup_action` + `state`, no `code`).
- `arawn-integrations::github::install_flow` builds the install
  URL, runs the callback, CSRF-verifies, and returns a typed
  outcome.
- `arawn-integrations::github::client` signs RS256 JWTs with
  `jsonwebtoken` and trades them for 1-hour
  installation-access-tokens. In-memory cache with a 5-min
  refresh-lead window.
- `arawn-integrations::github::integration` implements the
  `Integration` trait: install URL builder, install flow driver,
  credential persistence (`CredentialStore<GithubCredentials>`),
  and a `client()` accessor for downstream tools/feed templates.
- main.rs wires the integration when `ARAWN_GITHUB_APP_ID +
  APP_SLUG + private key` are resolvable. Skipped silently
  otherwise.
- Workspace 1823/0 (1814 → 1823, 9 new GitHub tests).

## Implementation Notes

### Technical Approach

- GitHub App auth is two-step: (1) sign a JWT with the App's
  private key using `RS256`, (2) trade the JWT for an
  installation-access-token. Use the `jsonwebtoken` crate.
- Token lifetime is 1 hour — store the expiry alongside the
  token; refresh on the next call when within 5 minutes of
  expiry.
- The installation flow surfaces in `/integrations connect github`
  the same way Gmail does. The redirect URI receives
  `installation_id` + `setup_action`; we record the
  installation_id in the credential row.
- Rate-limit handling lives in `client.rs` as a generic wrapper
  (used by all 3 feed templates in T-0319 / T-0320 / T-0321).

### Dependencies

- Foundation for [[ARAWN-T-0318]] (projection schemas) and the
  3 feed templates ([[ARAWN-T-0319]], [[ARAWN-T-0320]],
  [[ARAWN-T-0321]]).

### Risk Considerations

- GitHub App registration is a one-time setup step; the user
  has to create an app in their GitHub UI and supply the
  private key. Document this clearly in the integration's
  README and in the `/integrations connect github` flow's
  prompts.