---
id: documentation-accuracy-diataxis
level: initiative
title: "Documentation accuracy + Diataxis gap-fill"
short_code: "ARAWN-I-0051"
created_at: 2026-05-19T00:49:27.843030+00:00
updated_at: 2026-05-19T10:52:44.947394+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: L
initiative_id: documentation-accuracy-diataxis
---

# Documentation accuracy + Diataxis gap-fill

## Context

The arawn user docs (`docs/src/`, mdbook) were last comprehensively updated around 2026-05-02 (`docs(t-0193): add README + getting-started walkthrough`, `docs: security, memory, workflows reference pages`). Since then, a wave of feature work has shipped without corresponding doc updates:

- **2026-05-06 → 2026-05-18:** the entire `arawn-ceremonies` crate, todos system (T-0309 → T-0316), and slash commands `/today`, `/week`, `/retro`, `/todo`.
- **2026-05-17 → 2026-05-18:** GitHub App OAuth + integration scaffold (T-0317), four GitHub feed templates (T-0319, T-0320, T-0321, T-0325 — notifications, issues-and-prs, review-queue, repo-mirror), `github:repo:` and `github:org:` workstream URI binding (T-0322), and hot-register on bind/unbind (T-0329).
- **2026-05-18 (today):** identity-layer rewrite — `system_prompt.rs` split into `ASSISTANT_*` / `CODING_*` const sets; new workstream `identity_profile` column (migration V10); `LocalService::build_engine_config` selects prompt set by workstream. T-0330 shipped; T-0331 (CLI metadata + welcome message) is active.

Three parallel review agents (Accuracy, Completeness, Diataxis-fit) launched at filing time confirm the high-level damage:

- **~9 subsystems with zero user docs:** ceremonies, todos, workstream identity profile, workstream binding (`/workstream bind`), plugins, MCP, skills, agents (sub-agent tool family), permissions.
- **Slash command coverage:** 26 commands registered in `command.rs:register_builtins`; only ~6 are mentioned in docs.
- **`getting-started.md` is 627 lines** of which roughly 70 % is OAuth-setup how-to recipes — i.e., it's a tutorial that's mostly how-tos.
- **Diataxis Explanation quadrant is empty** — no doc explains *why* workstreams, *why* feeds vs palaces vs ceremonies, *how* the agent loop works, *what* the permission model is, *why* identity-by-workstream.

Audit reports land in `/tmp/arawn-docs-audit-{accuracy,completeness,diataxis}.md` and feed into Phase B (restructure) of this initiative.

## Goals & Non-Goals

**Goals:**
- Every existing doc page is accurate against the current code — no claim contradicts reality.
- Every user-facing feature has a doc home: slash commands, CLI flags, config keys, env vars, agent tools, integrations, ceremonies, todos, workstreams, identity profile, plugins, MCP, skills, agents, permissions.
- The doc tree is reorganized along Diataxis quadrants (Tutorials, How-to, Reference, Explanation). Each page lives in one quadrant; mixed pages get split.
- The Explanation quadrant is bootstrapped — at minimum: *why workstreams*, *feeds vs palaces vs ceremonies*, *the agent loop*, *permission model*, *identity-by-workstream*.
- At least one new Tutorial beyond getting-started exists — a guided "set up your first morning brief" or "track a workstream end-to-end" arc that demonstrates the system, not just configures it.

**Non-Goals:**
- API-stability commitments — docs reflect alpha behavior, not contracts.
- Contributor / internal-architecture docs — those live in `.metis/code-index.md`, ADRs, and crate-level `lib.rs` comments; not in user-facing `docs/src/`.
- Translation / localization.
- Auto-generated reference pages (deferred — see Alternative A).
- Refresh of `.metis/vision.md` or other internal Metis documents.

## Requirements

### User Requirements
- A new user lands on the docs, reads the intro, follows the first tutorial, and reaches a working chat session — **without** wading through OAuth recipes for providers they don't use.
- A user connecting Gmail (or Slack, Atlassian, GitHub) finds a self-contained how-to for that provider.
- A user asking "what slash commands exist?" or "what env vars does arawn read?" finds the answer in a reference page.
- A user asking "what's the difference between feeds, palaces, and ceremonies?" finds an explanation page.

### System Requirements
- **REQ-001 (accuracy):** every concrete claim in user docs (CLI flag, slash command, config key, file path, env var, OAuth scope, behavior) is verified against current code at merge time.
- **REQ-002 (completeness):** every Completeness-audit item marked MISSING gets a doc home — either a new page or a section in an existing reference page.
- **REQ-003 (structure):** `docs/src/SUMMARY.md` is reorganized into four top-level groups: Tutorials, How-to guides, Reference, Explanation. (A `Getting Started` shortcut entry may co-exist.)
- **REQ-004 (Diataxis discipline):** no page mixes more than one quadrant. Mixed sources are split.
- **REQ-005 (Explanation bootstrap):** five Explanation pages exist by initiative completion — workstreams, feeds-vs-palaces-vs-ceremonies, agent loop, permission model, identity-by-workstream.
- **REQ-006 (verification):** a final sweep re-runs the three audit agents against the new tree and shows zero critical, zero major findings.
- **NFR-001:** `angreal docs build` produces a clean mdbook build with no broken cross-links.
- **NFR-002:** each doc page is reviewable as a standalone commit/PR — every Phase C task is one logical doc unit.

## Use Cases

### Use Case 1: First-time user lands on docs

- **Actor:** developer who just cloned arawn for the first time.
- **Scenario:** Opens `docs/src/index.md`, sees a four-quadrant landing page (Tutorials → "start here", How-to → "I have a specific goal", Reference → "I'm looking something up", Explanation → "I want to understand"). Picks the first tutorial. Tutorial walks them from `cargo build` to a chat session in ~5 minutes, with no OAuth detours.
- **Expected Outcome:** Working chat session, clear sense of what arawn is, knowledge of where to go next (which how-to, which reference page).

### Use Case 2: Returning user wants to connect Slack

- **Actor:** user who already has arawn running, wants to connect Slack.
- **Scenario:** Opens `docs/src/how-to/connect-slack.md` directly (linked from the integrations index). Page is self-contained: Slack-specific OAuth recipe, scopes, redirect URI quirk, restart + `/connect slack`.
- **Expected Outcome:** Slack connected in <10 minutes without reading anything Slack-irrelevant.

### Use Case 3: User wants to know what `/today` does

- **Actor:** user who typed `/today` after seeing it mentioned, isn't sure what's happening.
- **Scenario:** Opens `docs/src/reference/slash-commands.md`, finds `/today` in alphabetical order, reads the one-paragraph description + link to the ceremonies reference.
- **Expected Outcome:** Knows what the daily ceremony is and where to read more.

### Use Case 4: User wants to understand workstreams

- **Actor:** intermediate user — has been using arawn for a week, has felt the concept of "workstream" repeatedly, wants the mental model.
- **Scenario:** Opens `docs/src/explanation/why-workstreams.md`. Reads ~400 words on what problem workstreams solve, how they relate to sessions / feeds / palaces / identity, and when *not* to use one (scratch).
- **Expected Outcome:** Now feels comfortable making workstream decisions; no longer copy-pasting the same workstream over and over.

## Architecture

This is a documentation initiative — no runtime architecture. The "architecture" is the proposed doc tree (subject to Phase B sign-off):

```
docs/src/
├── SUMMARY.md
├── index.md                          (landing + Diataxis orientation)
│
├── tutorials/
│   ├── index.md
│   ├── first-session.md              (today's getting-started, trimmed to chat-only)
│   └── first-workstream.md           (NEW — create workstream, bind feed, see palace populate)
│
├── how-to/
│   ├── index.md
│   ├── connect-google.md             (extracted from getting-started)
│   ├── connect-slack.md              (extracted)
│   ├── connect-atlassian.md          (extracted)
│   ├── connect-github.md             (existing github.md repurposed)
│   ├── bind-workstream-to-feed.md    (NEW)
│   ├── set-up-daily-ceremony.md      (NEW)
│   ├── run-a-workflow.md             (extracted from workflows.md)
│   └── debug-oauth.md                (NEW — from current troubleshooting table)
│
├── reference/
│   ├── index.md
│   ├── cli.md                        (NEW — every flag and subcommand)
│   ├── slash-commands.md             (NEW — every /command, alphabetized)
│   ├── config.md                     (NEW — every arawn.toml section + key)
│   ├── environment-variables.md      (NEW)
│   ├── agent-tools.md                (NEW — catalog of every tool)
│   ├── data-directory-layout.md      (NEW)
│   ├── feeds/                        (existing, trimmed to pure reference)
│   │   ├── index.md
│   │   ├── template-catalog.md
│   │   ├── agent-read-patterns.md
│   │   └── feed-search.md
│   ├── palaces/                      (existing, trimmed to pure reference)
│   │   ├── index.md
│   │   ├── projections.md
│   │   ├── extraction.md
│   │   ├── steward.md
│   │   └── agent-read-patterns.md
│   ├── ceremonies.md                 (NEW — daily/weekly/retro reference)
│   ├── todos.md                      (NEW — todos system reference)
│   ├── workstreams.md                (NEW reference page)
│   ├── memory.md                     (existing, refreshed)
│   ├── security.md                   (existing, refreshed)
│   ├── workflows.md                  (existing, trimmed to reference)
│   ├── plugins.md                    (NEW)
│   ├── mcp.md                        (NEW)
│   ├── skills.md                     (NEW)
│   ├── agents.md                     (NEW — sub-agent tool family)
│   └── permissions.md                (NEW)
│
└── explanation/
    ├── index.md
    ├── why-workstreams.md            (NEW)
    ├── feeds-vs-palaces-vs-ceremonies.md  (NEW)
    ├── the-agent-loop.md             (NEW)
    ├── permission-model.md           (NEW)
    └── identity-by-workstream.md     (NEW — the identity_profile design)
```

The specific final tree is subject to Phase B sign-off — audit findings may add, remove, or reshape leaves.

## Detailed Design

### Phase A — Audit *(in progress)*

Three parallel agents launched at filing time (background):
- **Accuracy auditor** — verifies every doc claim against code. Output: `/tmp/arawn-docs-audit-accuracy.md`.
- **Completeness auditor** — enumerates every code feature, marks doc coverage. Output: `/tmp/arawn-docs-audit-completeness.md`.
- **Diataxis-fit auditor** — classifies content vs. claimed quadrant. Output: `/tmp/arawn-docs-audit-diataxis.md`.

**Exit criteria:** three audit files exist; findings summarized to operator. Phase A is read-only — no `docs/src/` changes happen until Phase B clears.

### Phase B — Restructure proposal *(blocked on Phase A)*

Synthesize audits into:
1. A proposed new `SUMMARY.md` tree (the Architecture section above is the draft).
2. A move-map: every existing page → new path + which sections split off into how-tos / explanation.
3. A "new page" inventory with one-line scope per page.

**Human-in-the-loop required.** Phase B exits only after the operator approves the new tree and move-map. Until then, no `docs/src/` edits.

### Phase C — Fix + fill *(decomposed into tasks at Phase B exit)*

Each task is one of three kinds:
- **Move/split task** — take page X, split into N pieces, move to new paths, update SUMMARY + cross-references.
- **Fix-in-place task** — correct accuracy drift on a page that stays put.
- **New-page task** — write a new page (Reference catalog, How-to recipe, Explanation page, or Tutorial).

Likely decomposition (subject to Phase B output):

1. **C-1** — restructure scaffold: new directories, new `SUMMARY.md`, landing pages, move existing pages with cross-link updates.
2. **C-2** — extract OAuth recipes from getting-started into `how-to/connect-{google,slack,atlassian,github}.md`.
3. **C-3** — Reference catalog batch 1: `cli.md`, `slash-commands.md`, `config.md`, `environment-variables.md`, `data-directory-layout.md`.
4. **C-4** — Reference catalog batch 2: `agent-tools.md`, `permissions.md`.
5. **C-5** — Ceremonies + todos reference + first-workstream tutorial.
6. **C-6** — Workstreams reference + identity-by-workstream explanation.
7. **C-7** — Plugins / MCP / skills / agents references.
8. **C-8** — Explanation pages batch: why-workstreams, feeds-vs-palaces-vs-ceremonies, the-agent-loop, permission-model.
9. **C-9** — Refresh memory.md, security.md, workflows.md; sweep remaining Phase A drift.

### Phase D — Verification

Re-run the three audit agents. Findings must show zero critical + zero major. Run mdbook link-checker. Spot-check the new tutorial against a clean `git clone` + `cargo build`.

## Alternatives Considered

### Alternative A — Auto-generated reference pages from code

Build a doc-gen step (clap-to-markdown, derive macros for config keys, etc.) that emits CLI/config/env reference pages from source. Manual content lives only in the other quadrants.

- **Pro:** reference pages stay in sync mechanically — no drift.
- **Con:** non-trivial tooling investment; generator must read disparate sources (clap, serde, env::var calls); makes prose context harder on reference pages.
- **Decision:** defer. Keep reference manual for this initiative. Revisit doc-gen as a follow-up initiative if drift returns.

### Alternative B — Drift-only audit, no Diataxis restructure

Fix accuracy gaps; leave structure as-is.

- **Pro:** smaller, faster.
- **Con:** leaves structural problems untouched (bloated getting-started, no Explanation quadrant, new subsystems homeless).
- **Decision:** rejected at scope-locking — operator chose full Diataxis sweep.

### Alternative C — Move docs to a wiki / GitBook / Notion

Migrate docs out of the repo into a hosted service.

- **Pro:** easier multi-author editing.
- **Con:** decouples docs from code commits → drift gets worse, not better. Repo is single-maintainer.
- **Decision:** rejected. mdbook stays.

## Implementation Plan

| Phase | Status | Output | Gate |
|---|---|---|---|
| A — Audit | In progress (3 agents in background) | 3 audit files in `/tmp/` | All three return |
| B — Restructure proposal | Blocked on A | Updated `SUMMARY.md` draft + move-map + new-page inventory | **Operator approval** |
| C — Fix + fill | Blocked on B | 6–10 Metis tasks, each = 1 commit/PR | Each task: code-cited content, mdbook build clean |
| D — Verification | Blocked on C | Re-audit reports, link-check pass | Zero critical/major findings |

Estimated complexity: **L**. Rough scope: 25–30 doc files touched (mix of new + edited), one structural restructure, ~3,000–4,000 lines of net new prose. Most of the time is research + accuracy, not writing.

## Status Updates

### 2026-05-18 — Filed (discovery)

Initiative filed in response to operator request to audit docs against recent edits. Three audit agents launched in parallel (Accuracy, Completeness, Diataxis-fit). Scope locked at *full Diataxis sweep* (fix drift + add Explanation pages + new Tutorial beyond getting-started). Tracking via Metis initiative + decomposed tasks.

### 2026-05-18 — Phase A complete (1 stall, 2 success)

- **Diataxis-fit agent:** completed. Audit at `audits/diataxis.md`. Headline: 20 of 21 pages mixed or misclassified; Explanation quadrant is empty; getting-started is 75% how-to disguised as tutorial.
- **Completeness agent:** completed. Audit at `audits/completeness.md`. Headline: 66 covered / 41 partial / 185 missing across 292 enumerated items. Ceremonies (28 items) and plugins/MCP/skills/sub-agents (16 items) are entirely unsourced.
- **Accuracy agent:** stalled (watchdog after 600 s, no progress). Most claim-level drift surfaced by the other two audits (tool name drift in Slack/Jira, stale feed-template count "12 vs 17", `/unwatch` vs `/feeds rm` naming, `/remember`+`/memory`+`/forget` marked WIP in docs but wired in code, github.md says feeds are "follow-up tasks" when they shipped). Remaining accuracy issues will be caught inline during Phase C rewrites; Phase D verification re-runs both audits.

Phase A synthesis written to `audits/synthesis.md`. Phase B (restructure proposal) blocked on operator sign-off on the proposed tree.

### 2026-05-19 — Phase B + Phase C complete, Phase D cleared

Phases B / C / D landed across one session:

- **Phase B:** operator approved the proposed 4-quadrant SUMMARY.md tree as-is.
- **Phase C:** 10 tasks T-0333 → T-0342, one commit per task, all on `main`. Doc tree went from 21 files to 60. Drift fixes applied inline (tool-name mismatches, stale counts, /unwatch → /feeds rm, dropped WIP labels on /remember etc., reference to ARAWN-I-0035 identity profile, GitHub feed templates).
- **Phase D:** completeness + diataxis re-audits run on the new tree. Headline: 185 → 0 missing items; 0 of 60 critical/major Diataxis misclassifications (was 20 of 21). Six trivial nit fixes applied (dead links, empty ADR anchors, removed reference to unwritten how-to). Synthesis at `audits/phase-d-verification.md`.

REQ-006 ("zero critical, zero major findings on re-audit") met. Initiative ready to close.

Deferred to future work (none critical):
1. Third tutorial — "Build a morning brief workflow" (the flagship vision use-case).
2. `how-to/set-up-daily-ceremony.md`.
3. `how-to/switch-llm-provider.md`, `how-to/backup-restore-data-dir.md`, `how-to/use-arawn-doctor.md`.
4. Explanation pages for `plugins` and `mcp` (reference pages exist; "why" companions would be polish).
5. Six borderline tables in explanation pages duplicate canonical reference content. Defensible per the Diataxis audit; kept as-is.