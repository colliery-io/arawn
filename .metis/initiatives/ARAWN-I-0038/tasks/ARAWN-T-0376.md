---
id: integration-docs-phase-5-troubleshooting
level: task
title: "Integration docs Phase 5 — troubleshooting matrix + maintainer guide"
short_code: "ARAWN-T-0376"
parent: ARAWN-I-0038
blocked_by: ["ARAWN-T-0373", "ARAWN-T-0374", "ARAWN-T-0375"]
archived: false

tags:
  - "#task"
  - "#docs"
  - "#integrations"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0038
---

# Phase 5 — Troubleshooting + maintainer guide

## Objective

Extend the existing `debug-oauth-failures.md` with a complete
symptom-keyed troubleshooting matrix, and file a maintainer-
facing guide that captures how to keep these docs accurate over
time.

Blocks on Phases 2-4 so the matrix can absorb troubleshooting
items those tasks surface during writing.

## Scope

Two files:

- EXTEND `docs/src/how-to/debug-oauth-failures.md` — append (or
  promote to a top-level section) a symptom → cause → fix
  table covering every `tracing::warn!` / `tracing::error!`
  string in `crates/arawn-integrations` + `crates/arawn-auth`.
- NEW `docs/src/contributing/integration-docs.md` — maintainer
  guide:
  - When to update each how-to (provider UI change, new scope,
    new tool).
  - What to grep (scope URIs, tool names, error strings).
  - How to date `<!-- VERIFY: <date> -->` markers and when to
    audit them.
  - Re-screenshot policy (deferred to a future initiative; note
    the placeholder).

## Correctness mandate

- Every matrix row must trace to either:
  - A `tracing::{warn,error}!` string in arawn-integrations
    or arawn-auth (cite the file:line in a comment), OR
  - A captured incident from prior Metis tasks
    (e.g. T-0235's "Atlassian token refresh drops sites
    extras" — surface as a documented edge case).
- No invented error messages.
- Provider-side errors (Google's HTTP 400 redirect_uri_mismatch,
  Slack's invalid_redirect, etc.) come from the providers'
  public OAuth error vocabulary — those are stable, but
  mark `VERIFY` if I'm unsure whether arawn passes them
  through verbatim.

## Acceptance criteria

- [ ] Every grep-able `tracing::warn!`/`error!` in
  `arawn-integrations` + `arawn-auth` has a matrix row.
- [ ] Each row's "fix" is actionable — references a specific
  `/disconnect` / `/connect` / config edit / browser action.
- [ ] Maintainer guide at
  `docs/src/contributing/integration-docs.md` exists with
  grep targets + audit cadence.
- [ ] `angreal docs build` clean.

Parent: [[ARAWN-I-0038]].
