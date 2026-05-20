---
id: integration-docs-phase-1-hub
level: task
title: "Integration docs Phase 1 — hub, oauth-primer, integrations-config"
short_code: "ARAWN-T-0372"
parent: ARAWN-I-0038
blocked_by: []
archived: false

tags:
  - "#task"
  - "#docs"
  - "#integrations"
  - "#phase/todo"


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

- [ ] Three new files exist at the paths above.
- [ ] `docs/src/SUMMARY.md` includes entries for all three.
- [ ] Config-resolution order in `integrations-config.md`
  matches the code (cite the function name + file:line in a
  hidden comment if helpful for future maintainers).
- [ ] OAuth primer covers the four pieces (client_id,
  client_secret, scope, redirect URI) and the
  unverified-app warning.
- [ ] `angreal docs build` clean, no broken links.
- [ ] User-runnable UAT: open the rendered overview page,
  follow links to oauth-primer and integrations-config —
  reader understands what they need before opening any
  per-provider how-to.

Parent: [[ARAWN-I-0038]].
