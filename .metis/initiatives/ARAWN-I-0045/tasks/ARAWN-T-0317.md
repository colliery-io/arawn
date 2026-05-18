---
id: github-app-oauth-arawn
level: task
title: "GitHub App OAuth + arawn-integrations scaffold"
short_code: "ARAWN-T-0317"
created_at: 2026-05-18T12:15:07.992922+00:00
updated_at: 2026-05-18T12:15:07.992922+00:00
parent: ARAWN-I-0045
blocked_by: []
effort: M
archived: false

tags:
  - "#task"
  - "#phase/todo"


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

- [ ] New module `arawn-integrations::github` with `mod.rs`,
      `integration.rs`, `client.rs` mirroring `gmail/` structure.
- [ ] `start_oauth_flow` returns the GitHub App install URL
      (`https://github.com/apps/<app-slug>/installations/new`).
- [ ] OAuth callback handler exchanges the user-to-server code
      for an installation-access-token via
      `POST /app/installations/{installation_id}/access_tokens`
      using a JWT signed with the App's private key.
- [ ] Installation-access-tokens are short-lived (1 hour);
      refresh loop re-mints them ahead of expiry.
- [ ] Token stored encrypted via the existing
      `credential_store` (ChaCha20Poly1305, per ARAWN-A-0001).
- [ ] `Integration` trait health check confirms the token via
      `GET /installation/repositories`.
- [ ] Config: `[integrations.github]` block accepts
      `app_id`, `private_key_path` (or env), `client_id`,
      `client_secret`, `installation_id`.
- [ ] Unit tests: mock HTTP for the token exchange + refresh
      paths; round-trip the encrypted token through the store.

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
