---
id: t-f-truth-up-tool-descriptions-and
level: task
title: "T-F: Truth-up tool descriptions and docs to the memory/signal/lens model"
short_code: "ARAWN-T-0441"
created_at: 2026-05-27T20:18:49.413618+00:00
updated_at: 2026-05-28T14:48:58.420475+00:00
parent: ARAWN-I-0061
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0061
---

# T-F: Truth-up tool descriptions and docs to the memory/signal/lens model

## Parent Initiative
[[ARAWN-I-0061]]

## Objective
Make every user/LLM-facing surface describe the corrected model — **memory =
global facts; signals = cross-lens extracted stream; lens = memory-aware standing
extractor, not a place you switch into.** Remove all "active lens scopes your
view" / write-target / "reads-all-writes-one" framing.

## Scope
- **Tool descriptions** (`crates/arawn-engine/src/tools/`):
  - `signal_query` (`signal.rs:256`), `signal_timeline` (`signal.rs:424`), module
    doc (`signal.rs:1-9`) — say signals are cross-lens, labeled by source lens
    (not "active lens").
  - `memory_search` / `memory_store` — global facts/tunings (aligns with T-A/T-C).
  - Remove `lens_switch` references; `lens_promote` reframed or removed if it no
    longer fits (it filed scratch into a lens — revisit under global memory).
  - steward / `lens_tag` / `lens_show` — "write-target lens" wording dropped.
- **Docs** (`docs/src`): rewrite `explanation/lenses.md` to the memory/signal/lens
  model (drop "reads-all/writes-one"); fix `explanation/memory-design.md:22`,
  `explanation/the-agent-loop.md:14-15` (persona/lens metadata),
  `reference/lens-tools.md:5`, `reference/agent-tools.md:75,99`,
  `explanation/identity-by-lens.md` body, `tutorials/first-lens.md:36`.
- Add/refresh a short "memory vs signals vs lenses" explainer so the distinction
  is documented once, canonically.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] `rg -i "active (lens|workstream)|scoped to|write-target|reads-all" docs/src crates`
      returns only correct/intentional usages.
- [ ] `signal_*` descriptions say cross-lens + source-labeled; no "active lens."
- [ ] `lenses.md` leads with the memory/signal/lens model; no switching/write-target.
- [ ] `angreal docs build` + `angreal check workspace` clean.

## Dependencies
Follows the behavior tasks ([[ARAWN-T-0436]], [[ARAWN-T-0437]], [[ARAWN-T-0438]])
so docs describe shipped behavior.

## Status Updates

**2026-05-28 — Done.** Truth-up across user/LLM-facing surfaces.

LLM-facing descriptions: `signal.rs` module doc + `signal_query` +
`signal_timeline` rewritten (cross-lens, source-labeled, `lens=` narrows);
`steward.rs` module doc and `lens_journal` description reframed away from "the
active lens" to "the session's current lens (defaults to `scratch`)".

Canonical docs:
- `explanation/lenses.md` rewritten around memory/signal/lens — a lens is a
  standing, memory-aware extractor; reads roam every lens's signal stream;
  lenses aren't switched into.
- `explanation/memory-design.md` rewritten — memory is one global store, signals
  are extracted into per-lens palaces, the two are different concerns. Replaced
  the old two-tier scope-locking discussion with a comparison table.
- `explanation/the-agent-loop.md` system-prompt section reframed — persona
  always `assistant`, only global memory injected, no write-target language.
- `explanation/what-is-arawn.md` agent-loop diagram updated.
- `explanation/identity-by-lens.md` banner sharpened — fully historical.
- `explanation/ceremonies.md` "scopes to the active lens" → "runs per-lens".

Reference docs: `lens-tools.md` opener + `lens_show` defaults + lifecycle
table; `lens-cli.md` (drop `/lens switch` and `/promote` sections, reword
scratch); `slash-commands.md` (drop `/promote` from index + body, drop
`/lens switch`, reword `/memory`); `agent-tools.md` (Signal table reframed
cross-lens, lifecycle drops switch/promote); `todos-tools.md`,
`shell-sandbox.md` "active lens" → "session's current lens (defaults to
scratch)"; `tutorials/first-lens.md` stale status-bar line replaced.

UAT scenarios (`tests/uat.rs`): every "Switch to/back to" / "First call
lens_switch" instruction rewritten to either pass `lens=<name>` on the read
tools or phrase as "working in the `<name>` lens"; expectations updated.

Residual `rg` matches: plural list-context "active lenses" (= non-archived)
and internal-infrastructure comments about `SessionLens` as a session-default
shim. Both correct/intentional.

Verification: `angreal check workspace` exit 0; `angreal docs build` clean.