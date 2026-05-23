---
id: integration-docs-phase-1-hub-oauth
level: task
title: "Integration docs Phase 1 — hub, oauth-primer, integrations-config"
short_code: "ARAWN-T-0372"
created_at: 2026-05-20T16:00:00+00:00
updated_at: 2026-05-20T21:02:31.935914+00:00
parent: ARAWN-I-0038
blocked_by: []
archived: false

tags:
  - "#task"
  - "#docs"
  - "#integrations"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0038
---

# Phase 1 — Hub + concept docs

## Objective

Land the three foundation docs the per-provider how-tos
link out from. Unblocks Phases 2–4 (they reference the
config-resolution and OAuth concepts established here).

## Scope

Three new files under the existing Diátaxis layout:

- `docs/src/explanation/integrations-overview.md` — the hub.
  30-second framing, provider matrix, decision tree
  (`[integrations.<svc>]` per service vs `[integrations.google]`
  shared), linkouts to the four how-tos.
- `docs/src/explanation/oauth-primer.md` — OAuth in plain
  English: client_id / client_secret / scope / redirect URI;
  why every provider's flow differs; what the unverified-app
  warning means; what arawn does with tokens
  (`~/.arawn/tokens/` encrypted).
- `docs/src/reference/integrations-config.md` — the exact
  config-resolution order: `ARAWN_<SVC>_*` env vars →
  `[integrations.<svc>]` → `[integrations.google]` shared.
  TOML snippets, env-var names, what wins when both are set.

Plus `docs/src/SUMMARY.md` updates surfacing the new entries
under their respective Diátaxis sections.

## Correctness mandate

- Anchor every env-var name + TOML key against the actual
  config parser. Grep `crates/arawn/src/config.rs` and the
  integration crates first.
- The resolution order ("env → service-specific → shared
  google") must match what the loader actually does — verify,
  don't paraphrase.
- Token storage path: confirm against `crates/arawn-auth`
  before stating `~/.arawn/tokens/`.
- No provider-UI navigation in this task — Phase 2-4 handle
  those.

## Acceptance criteria

- [x] Three new files exist at the Diátaxis-aligned paths.
- [x] `docs/src/SUMMARY.md` surfaces all three
  (`integrations-config` under Reference;
  `integrations-overview` and `oauth-primer` under
  Explanation).
- [x] Resolution order in `integrations-config.md` matches
  the actual `resolve` closure at
  `crates/arawn/src/main.rs:1043-1078` — including per-field
  independence and the Google-only shared fallback. Worked
  examples cover every combination.
- [x] OAuth primer covers the four pieces, the localhost
  callback flow, the unverified-app warning + 100-test-user
  cap, and the ChaCha20-Poly1305 encrypted token store
  (`arawn-auth/src/token_store.rs`).
- [x] `angreal docs build` clean. Cross-links verified
  (every linked file exists).
- [ ] User docs-UAT pending (planned tonight).

## Status Updates — 2026-05-20

Landed.

**Fact anchoring done before drafting prose:**

- Resolution algorithm: `crates/arawn/src/main.rs:1043-1078`
  (`resolve` closure + `or_else` fallback to `ARAWN_GOOGLE_*`
  / `[integrations.google]` per Google service).
- Env var catalog: cross-referenced against main.rs greps —
  Gmail `_GMAIL_`, Calendar `_GCAL_`, Drive `_GDRIVE_`,
  shared `_GOOGLE_`, Slack `_SLACK_`, Atlassian
  `_ATLASSIAN_`, GitHub App fields.
- Config struct: `crates/arawn/src/config.rs::IntegrationsConfig`
  with sub-blocks slack / google / gmail / calendar / drive /
  atlassian / github.
- Token store: `crates/arawn-auth/src/token_store.rs` —
  `{data_dir}/tokens/<provider>.json.enc` + `key.bin`,
  ChaCha20-Poly1305, restricted dir perms.
- Redirect URI: `crates/arawn-auth/src/server.rs:74` —
  `http://localhost:{bound_port}/{path}` uniformly (not
  `127.0.0.1`).

**No invented claims.** Tool counts, provider list, and
reference cross-links match the existing
`docs/src/reference/integrations.md` content.

**Cross-links verified.** Every `(../<path>.md)` reference
resolves to an existing file.

Three files shipped (~270 lines total):
- `docs/src/reference/integrations-config.md`
- `docs/src/explanation/oauth-primer.md`
- `docs/src/explanation/integrations-overview.md`

SUMMARY.md updated with both Reference and Explanation entries.

Ready for user docs-UAT.

Parent: [[ARAWN-I-0038]].