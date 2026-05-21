---
id: post-iteration-cruft-removal-dead
level: initiative
title: "Post-iteration cruft removal — dead code, backward-compat aliases, stale concepts"
short_code: "ARAWN-I-0053"
created_at: 2026-05-21T13:08:02.500626+00:00
updated_at: 2026-05-21T15:07:16.194161+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/active"


exit_criteria_met: false
estimated_complexity: M
initiative_id: post-iteration-cruft-removal-dead
---

# Post-iteration cruft removal — dead code, backward-compat aliases, stale concepts

## Context **[REQUIRED]**

Four-plus weeks of rapid iteration (215 commits in the last 14 days alone) have
shipped real capability — but they've also left the tree carrying visible
accumulated cruft. The codebase already self-documents some of it: explicit
"Backward-compatible alias", "Kept for backwards-compat", "Retained for backward
compatibility", "legacy", and `#[allow(dead_code)]` markers sit in the source
today. None of this is failing tests, but the surface area drags on
readability, increases the maintenance load, and slows future refactors.

Concrete sightings from initial survey (full inventory is the discovery
deliverable):

**Backward-compat aliases / shims:**
- `arawn_engine::ToolContext = EngineToolContext`
  (`crates/arawn-engine/src/lib.rs:32`) — explicit "backward-compatible alias"
  comment.
- `arawn_engine::tool` is now a six-line re-export shim over `arawn-tool`
  (`crates/arawn-engine/src/tool.rs:1-6`).
- `PermissionChecker` has three methods labeled "Kept for backwards-compat with
  the pre-T-0276 API" (`crates/arawn-engine/src/permissions/checker.rs:133/145/152`).
- `WorkstreamStore::delete` — "Retained for backward compatibility"
  (`crates/arawn-storage/src/workstream_store.rs:222`).
- `Entity::tags` field "kept for backwards compatibility"
  (`crates/arawn-memory/src/types.rs:186`).
- `monday_sunday_for_iso_week_public as monday_sunday_for_iso_week` re-alias
  (`crates/arawn-ceremonies/src/plugins/retro_detectors.rs:26`).

**Dead-code islands** (12 confirmed `#[allow(dead_code)]` / `#[allow(unused)]`
sites across 11 files in arawn-extractor, arawn-ceremonies, arawn-engine,
arawn-feeds, arawn-steward). 

**note from user** "We need to be very careful about the dead code flags, we should drop and immediately try to build a release and test version of the 
binaries as a whole, some of these are marked because of compilation issues not because they're actually dead code" Be aggressive here to either remove or re architect
to ensure that the dead code flag is not a red herring"

**Stale "kept for X" markers:**
- ceremonies `since=` cursor — comment says callers should use `between`
  (`crates/arawn-ceremonies/src/plugins/gather_sources.rs:51-54`).
- ceremonies `tablet_id_prefix` — "unused but kept for naming intent"
  (`crates/arawn-ceremonies/src/service.rs:1005`).
- `Archived/legacy` ceremony item kind — "kept for historical query"
  (`crates/arawn-ceremonies/src/types.rs:18`).
- feeds atlassian client uses deprecated `get_all_projects` with
  `#[allow(deprecated)]` (`crates/arawn-feeds/src/clients/atlassian.rs:521-525`).

**Stale concept candidates** (need go/no-go before any removal):
- `/accept` and `/plan` slash commands were dropped
  (`crates/arawn-tui/src/command.rs:84`) — audit for any residue elsewhere.
- The entire `arawn_engine::tool` module — if every consumer can move to
  `arawn_tool::*`, the shim deletes outright.
- Pre-T-0276 permission API surface — what calls it now?

The full picture is wider than just code. After alignment with the operator,
this initiative also covers:

- **Test cruft** — dead/redundant tests, stale UAT fixtures, unused test
  helpers (notably the 1,904-line `crates/arawn-engine/src/testing.rs`
  TestHarness). Aligns with operator memory `feedback_inline_tests` preferring
  inline tests over separate harness files.
- **Docs cruft** — outdated `.md` files referencing removed concepts, dead
  references in mdbook `SUMMARY.md`, stale how-to pages. Diataxis docs from
  I-0051 just landed, so the bulk is current, but older one-offs may remain.
- **Config / schema cruft** — unused TOML keys (cf. T-0348 which already
  dropped `[server].host`), stale migrations, dead env vars, deprecated config
  fields without callers.

## Goals & Non-Goals **[REQUIRED]**

**Goals:**

1. Every backward-compat alias / shim in arawn crates either removed, or
   documented with a concrete future-need rationale plus a tracking task.
2. Every `#[allow(dead_code)]`, `#[allow(unused_*)]`, and `#[allow(deprecated)]`
   site resolved through the validation methodology below — proven dead and
   deleted, proven live and re-architected so the allow is no longer needed,
   or documented with a precise inline comment naming the cfg/test path that
   keeps it visible. No dangling unjustified allows.
3. Every "kept for X" / "legacy" / "retained for" comment in arawn-owned code
   has a one-line judgment recorded in the inventory: *delete*, *keep with
   rationale*, or *defer with reason*.
4. Stale-concept candidates (Tier 3) surfaced to the operator for go/no-go
   *before* any removal task is created. Each approved removal ships as its own
   task with a rationale.
5. Test cruft swept: dead test fixtures removed, redundant test harness code
   trimmed or inlined per operator preference, unreachable tests deleted.
6. Docs cruft swept: every `.md` page referencing a removed concept either
   updated or deleted. `SUMMARY.md` references resolve.
7. Config / schema cruft swept: unused TOML keys, unused env vars, deprecated
   config fields without callers all removed or justified.
8. `cargo check --workspace` clean and workspace unit + integration tests
   pass after every removal task. Full UAT green at each tier boundary and at
   initiative completion.

**Non-Goals:**

- File splitting, module restructuring, or moving code between crates. (That
  is a separate follow-on initiative once we see what's left after this clean.)
- I-0048 §A (typed per-domain config refactor) / §B (typed cross-module event
  bus) — those are architectural decisions, intentionally out of scope here.
- Adding new abstractions, helper layers, or APIs.
- Crate-graph dependency reorganization.
- Behavior or feature redesign — removals must preserve all current
  user-visible behavior that we choose to keep. Anything we *choose* to remove
  goes through Tier 3 go/no-go first.

## Detailed Design **[REQUIRED]**

### Working tiers

The work fans out into three tiers, ordered by blast radius:

**Tier 1 — Mechanical removals.** Things whose removal is obvious and local:
- `#[allow(dead_code)]` / `#[allow(unused_*)]` sites where the suppressed
  symbol is genuinely never called (verified by grep).
- `pub use Foo as Bar` backward-compat aliases.
- Pure re-export shim modules (e.g. `arawn_engine::tool`).
- Commented-out blocks of code with no `TODO:` linking them to future work.
- Unused imports the compiler hasn't yet complained about (rare but possible
  behind cfg-gates).

Tier 1 is a couple of focused tasks. Each task batches a cohesive cluster
(e.g. "delete pre-T-0276 permission API surface", "drop the
`arawn_engine::tool` shim and migrate all callers to `arawn_tool::*`",
"resolve all engine `#[allow(dead_code)]` sites").

**Tier 2 — Documented "kept for X" cases.** Each gets a one-line judgment:
- *Delete:* no real future need; remove the comment, remove the code.
- *Keep:* concrete future-need rationale recorded in the inventory; the
  comment in code stays but is updated to link to that rationale.
- *Defer:* not safe to delete this pass; logged with a reason and a follow-up
  task short-code.

Tier 2 mostly produces individual deletion tasks per cluster.

**Tier 3 — Stale concept candidates.** Concepts/features that look unused or
dead-ended but where removal is a directional call:
- Surfaced to the operator as a single review document with one paragraph
  per candidate (what it is, where it lives, evidence of staleness, blast
  radius of removal).
- Operator gives go/no-go per candidate.
- *Go* candidates become their own removal tasks.
- *No-go* candidates get a one-line rationale logged in the inventory so we
  don't re-litigate next quarter.

Tier 3 is the only place behavior surface changes; everything else is purely
internal.

### Dead-code validation methodology

`#[allow(dead_code)]` is a red herring class. Per operator note, some sites
are flagged not because the symbol is truly unused, but because it is only
reachable from test builds, behind cfg-gates, or through trait-object
dispatch the compiler can't statically prove. Removing the symbol would break
those callsites; removing only the `allow` would surface noise without
finding real dead code.

The discovery sweep must therefore *validate* each `allow(dead_code)` /
`allow(unused_*)` site, not just enumerate it. The validation protocol per
site:

1. **Hypothesis check.** Grep for the symbol name across the entire workspace
   (`*.rs` + `tests/` + `examples/` + `benches/`). Note every callsite found,
   including cfg-gated ones.

2. **Strip the allow and rebuild on three profiles:**
   - `angreal build workspace` (debug build of the workspace)
   - `cargo build --workspace --release` (release profile — flags cfg-gated
     usage that disappears under `--release`)
   - `cargo test --workspace --no-run` (compiles test binaries without
     running them — flags symbols only used by `#[cfg(test)]` code)

3. **Classify outcome:**
   - **Proven dead** — all three builds compile clean and grep found no
     callsites. → *Delete the symbol*, drop the allow.
   - **Proven live but compiler-blind** — at least one build fails because
     the symbol is needed. → *Re-architect so the compiler can see the
     usage* (move into a `#[cfg(test)]` module sibling, use proper feature
     gating, expose via a trait method the compiler can resolve). The allow
     becomes unnecessary and is dropped.
   - **Genuinely conditional and unavoidable** — there is a real reason the
     symbol must exist for a specific build configuration. → *Replace the
     bare allow with a precise inline comment* naming the configuration that
     needs it (e.g., `#[cfg_attr(not(test), allow(dead_code))] // used only
     by integration tests in arawn-tests/tests/foo.rs`). The annotation
     becomes self-documenting.

The bias is aggressive: prefer re-architecture (option 2) over a
configuration-gated retention (option 3). Option 3 is the fallback for
cases where re-architecture would force a worse design.

Validation logs (build outcomes per site) live in the inventory document
alongside each row so the decision is auditable.

### Sweep surfaces

Per operator alignment, the inventory covers four surfaces:

1. **Code surface** — all 20 crates, every `*.rs` file. Looking for the cruft
   classes above.
2. **Test surface** — `tests/` directories, `mod tests` blocks, UAT fixtures,
   and the `arawn-engine/src/testing.rs` harness. Operator preference
   (`feedback_inline_tests`) is toward inline tests; harness code that exists
   only to support a since-deleted test gets removed.
3. **Docs surface** — `docs/src/**/*.md` plus `docs/src/SUMMARY.md`. Find
   pages that reference removed concepts, broken cross-references, dead
   how-to pages.
4. **Config / schema surface** — `arawn.toml` keys, env vars consumed in
   `config.rs`, migration files under `crates/arawn-storage/migrations/`.

### Discovery deliverable

A single markdown inventory document filed inside this initiative directory at
`.metis/initiatives/ARAWN-I-0053/inventory.md`, with three sections matching
the tiers. Each row in each section names:
- File path + line range.
- One-line description of what it is.
- Proposed outcome (delete / keep / defer + reason).
- For Tier 3 only: blast-radius note and operator-decision field (filled in
  during go/no-go review).

The inventory is the input to the decompose phase. No removal tasks are filed
until the inventory exists and (for Tier 3) the operator has decided.

### Decomposition shape

Roughly 6–10 tasks expected, organized by cohesive cluster rather than by
tier. Examples (final list comes out of decompose phase):
- "Drop `arawn_engine::tool` re-export shim; migrate callers to `arawn_tool`."
- "Remove pre-T-0276 permission API surface."
- "Resolve all `#[allow(dead_code)]` in arawn-engine."
- "Resolve all `#[allow(dead_code)]` in arawn-ceremonies / arawn-feeds /
  arawn-steward / arawn-extractor."
- "Trim `arawn-engine/src/testing.rs` — inline what's still used, delete
  what's not."
- "Sweep docs for references to removed concepts."
- "Sweep TOML config keys and migration files for dead surface."
- One task per Tier 3 *go* removal.

Each task has acceptance criteria, runs workspace unit + integration tests,
and ends with `cargo check --workspace` clean.

### Verification strategy

Per operator alignment, the safety floor for in-flight work is workspace
unit + integration tests after every removal task (UAT requires a live LLM
and is therefore slow and costly to run every iteration). UAT runs at:

1. End of each tier (Tier 1 done, Tier 2 done, Tier 3 done).
2. Initiative completion.

This balances `feedback_uat_must_pass` (no shipping while UAT is red) against
the practical cost of running UAT after every single mechanical removal.

Concretely, per-task verification is:
- `angreal check workspace` (cargo check)
- `cargo build --workspace --release` (release profile — catches symbols that
  vanish under `--release` cfg-gating)
- `cargo test --workspace --no-run` (compiles test binaries — catches symbols
  reachable only from `#[cfg(test)]` code)
- `angreal test unit` (workspace unit tests)
- `angreal test integration` (workspace integration tests)
- `angreal check clippy` for the touched crates

The three build profiles (debug check, release build, test no-run) are the
non-negotiable safety floor for any task that touches `allow(dead_code)` or
`allow(unused_*)` sites — they directly verify the dead-code validation
methodology above. Other tasks can skip the release/test-no-run build if the
change provably doesn't touch cfg-gated surface.

Per-tier and initiative-end verification adds:
- `angreal test uat`
- `angreal test uat-judge`

## Alternatives Considered **[REQUIRED]**

- **Fold this into I-0048.** Rejected: I-0048 is the architectural drive-out
  (typed config, event bus, summarizer, learning, triage). Mixing cruft
  removal into it would conflate "delete dead surface" with "decide which
  way to evolve" — two different decision postures, two different risk
  profiles. Tier-3 concept removals here also explicitly avoid the §A/§B
  questions that I-0048 owns.
- **Removal + restructuring in one initiative.** Rejected per operator
  alignment: removing first makes restructuring smaller and more obvious.
  Doing both in one swing inflates blast radius and lengthens the time the
  tree is in flux.
- **Internals-only, no Tier 3.** Considered. Rejected per operator
  alignment: leaving stale concepts in the tree means restructuring later
  has to carry them forward. Better to surface them now and make conscious
  go/no-go calls, even if many resolve to "keep".
- **Hotspot-only discovery (just the 8–10 biggest files).** Rejected per
  operator alignment: small files harbor cruft too, and the cost of a
  full sweep is bounded — we have the tree-sitter index already.

## Implementation Plan **[REQUIRED]**

### Phase 1 — Discovery (this phase)

1. Produce the comprehensive cruft inventory document at
   `.metis/initiatives/ARAWN-I-0053/inventory.md`, covering all 20 crates
   plus test, docs, and config surfaces. Three sections (Tier 1 / Tier 2 /
   Tier 3).
2. For Tier 3, draft the candidate list with operator-decision fields.
3. Walk Tier 3 candidates with the operator. Record decisions inline in the
   inventory.
4. Exit discovery once the inventory is complete and Tier 3 decisions are
   landed.

### Phase 2 — Decompose

1. Cluster the *delete* outcomes from Tiers 1, 2, and 3 into ~6–10 tasks.
2. File tasks under this initiative with acceptance criteria, file lists,
   and verification steps.
3. Order tasks by blast radius (Tier 1 mechanical first, Tier 3 concept
   removals last).

### Phase 3 — Active

1. Ship tasks in order. Each task ends with `angreal check workspace`,
   `angreal test unit`, and `angreal test integration` green.
2. Run full UAT (`angreal test uat` + `angreal test uat-judge`) at each tier
   boundary.
3. Update the inventory document inline as tasks land (mark rows resolved
   with the commit short-hash).

### Phase 4 — Completed

1. Final UAT pass green.
2. Inventory document marks every row resolved.
3. `cargo check --workspace` clean with zero `#[allow(dead_code)]`,
   `#[allow(unused_*)]`, or `#[allow(deprecated)]` in arawn-owned code
   that lacks a one-line justification.
4. Transition to completed. The follow-on restructuring initiative becomes
   the natural next step (separately scoped, separately filed).

## Exit Criteria

- Discovery inventory document is complete, with outcomes recorded for every
  row across all three tiers.
- All Tier 3 candidates have an operator decision recorded.
- All *delete* outcomes have been shipped as tasks and merged.
- No `#[allow(dead_code)]`, `#[allow(unused_*)]`, or `#[allow(deprecated)]` in
  arawn-owned code without a one-line justification or a linked follow-up
  task short-code.
- No `pub use Foo as Bar` backward-compat alias in any arawn crate.
- `cargo check --workspace` clean.
- Workspace unit + integration tests green.
- Full UAT green.

## Related

- ARAWN-V-0001 — vision.
- ARAWN-I-0048 — tier-3 architectural drive-out (deliberately *not* touched
  here; §A/§B questions remain open under that initiative).
- ARAWN-S-0004 — openhuman comparative spec (referenced by I-0048; not in
  scope for this initiative).
- Follow-on (not yet filed): module restructuring / file-split initiative,
  which will be informed by what this initiative leaves behind.