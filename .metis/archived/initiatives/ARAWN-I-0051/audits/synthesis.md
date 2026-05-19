# Phase A Synthesis — ARAWN-I-0051

*Synthesizes the Diataxis-fit audit (`diataxis.md`) and Completeness audit (`completeness.md`). The Accuracy audit stalled (watchdog after 600s with no progress) — most accuracy findings are subsumed by Completeness (tool-name drift, stale counts, off-by-X feature claims); remaining claim-level drift will be caught inline during Phase C rewrites and re-checked in Phase D verification.*

## Headline numbers

- **292 enumerated items** across 12 categories (slash commands, CLI flags, agent tools, config keys, env vars, integrations, feed templates, workstream features, ceremonies/todos, plugins/MCP/skills/sub-agents, permission model, data-dir layout).
- **66 covered / 41 partial / 185 MISSING** (63 % of feature surface unsourced).
- **20 / 21 existing doc pages** are misclassified or structurally mixed (only `feeds/template-catalog.md` is a clean Diataxis fit out of the box).
- **Zero pages in the Explanation quadrant.**

## The 8 worst problems

1. **Entire ceremonies + todos subsystem is unsourced.** 28 agent tools, 4 slash commands (`/today`, `/week`, `/retro`, `/todo`), 3 cron schedules, retro detectors framework, multi-table persistence — zero coverage. Code in `crates/arawn-ceremonies/` and `crates/arawn-engine/src/tools/{ceremony,daily,weekly,todo}.rs`.

2. **Plugin / MCP / skill / sub-agent subsystems condensed to one bullet.** `intro.md` mentions "plugin system with hot reload" once. Code-side mature: manifest format, marketplaces (3 source types), hot reload, `arawn plugin` CLI subtree (8 subcommands), 2 built-in skills, 3 built-in sub-agent types, MCP server registry.

3. **38 of 74 agent tools undocumented.** Includes the file/shell/grep/glob/web tools the agent uses constantly, plus the `agent` / `task_*` sub-agent + background-task family.

4. **`getting-started.md` is 75 % how-to disguised as tutorial.** 627 lines, of which ~470 are stacked OAuth recipes for Google / Slack / Atlassian, a `/watch` recipe, a workstream quickstart, and a troubleshooting reference table.

5. **5 of 7 "Reference" pages are predominantly explanation.** `palaces/index.md`, `palaces/extraction.md`, `palaces/steward.md`, `feeds/index.md`, `memory.md` — concept exposition mislabeled as lookup material.

6. **The Explanation quadrant is empty.** Every "why does arawn work this way?" answer is either inferable from prose hidden inside reference pages or absent altogether. Missing concepts include: agent loop, three-layer data model, workstreams, permission philosophy, identity-by-workstream (ARAWN-I-0035, active), ceremonies, the local-first thesis.

7. **Name & count drift in existing pages.**
   - `slack_list_channels` / `slack_history` / `slack_post` / `jira_search` (code) ↔ `slack_channels_list` / `slack_channel_history` / `slack_post_message` / `jira_search_issues` (docs).
   - `feeds/template-catalog.md` opens with *"Twelve templates ship today"* — registry has 17.
   - `feeds/index.md` uses `/unwatch <feed>` — actual command is `/feeds rm <feed>`.
   - `memory.md` says `/remember` / `/memory` / `/forget` are "work-in-progress" — they're wired (command.rs:178, 183, 188).
   - `security.md` says `/permissions` is "on backlog" — registered at command.rs:144.
   - `integrations/github.md` says feed templates are "follow-up tasks (T-0319/T-0320/T-0321)" — those landed; T-0327 (repo-mirror) also shipped.

8. **Catalog gaps.** No single page lists slash commands, CLI flags, env vars, agent tools, config schema, or data-directory layout. All scattered across 6+ pages.

## What about the CLI metadata + system_prompt rewrite (today's T-0330/T-0331)?

Confirmed in-tree:
- `crates/arawn/src/main.rs:46` — `about = "Personal agentic assistant — watch, check, summarize, and nudge across your tools."`
- `crates/arawn-engine/src/system_prompt.rs:20-22` — `ASSISTANT_IDENTITY` / `CODING_IDENTITY` const pair, persona switched per workstream's `identity_profile`.
- `crates/arawn-storage/migrations/V10__workstream_identity_profile.sql` — new column.

**Doc implications:**
- README.md still describes arawn as "self-hosted personal agentic assistant" (good) but the CLI block doesn't mention the persona switch.
- No doc explains *what* `identity_profile` does or *how* to set it (`workstream_describe`-and-update is the path, but undocumented).
- The intro.md / vision framing is still coding-flavored ("draft a commit message") in places.

These get addressed inside Phase C-6 (workstream-cli + identity-profile reference) and Phase C-10 (`explanation/identity-by-workstream.md`).

## What Phase A did NOT find

- **No semantic-level accuracy issues are surfaced here** — the Accuracy agent stalled. The drift items above all come from Completeness (which enumerates) and Diataxis-fit (which structurally classifies). Subtler claim-level issues (e.g., "this OAuth scope no longer exists at provider X", "this TOML example no longer parses") will be caught inline during Phase C rewrites and verified in Phase D.
- **No prose-quality assessment** — Diataxis-fit was explicitly told to focus on structure, not clarity. Quality pass happens implicitly during Phase C rewrites.

## Recommended new tree (Phase B proposal)

Reconciled from both audits + my initial draft. Flatter than the current `feeds/` and `palaces/` subdir clusters — every reference page becomes a single-purpose file findable by name. Diataxis quadrant is encoded by top-level directory.

```
docs/src/
├── intro.md                              (landing page — orient users to four quadrants)
├── SUMMARY.md
│
├── tutorials/                            (learning-oriented; ~3 pages)
│   ├── index.md
│   ├── first-chat.md                     (~200 lines, trimmed from current getting-started §1-5)
│   └── first-workstream.md               (end-to-end: create → bind → see palace populate → signal_search)
│
├── how-to/                               (task-oriented; ~12 pages)
│   ├── index.md
│   ├── connect-google.md                 (extracted from getting-started §6)
│   ├── connect-slack.md                  (extracted)
│   ├── connect-atlassian.md              (extracted)
│   ├── connect-github.md                 (extracted from current integrations/github.md setup)
│   ├── create-a-feed.md                  (/watch recipe)
│   ├── bind-a-workstream-to-a-feed.md
│   ├── curate-a-workstream.md            (refine / apply / rollback)
│   ├── read-feeds-with-the-agent.md      (current feeds/agent-read-patterns.md)
│   ├── lock-down-permissions.md          (security.md "Limiting blast radius")
│   ├── author-a-workflow-by-hand.md
│   ├── debug-oauth-failures.md
│   └── recover-from-llm-warmup-failure.md
│
├── reference/                            (info-oriented; ~25 pages, flat)
│   ├── index.md
│   ├── cli.md                            (every flag + subcommand)
│   ├── slash-commands.md                 (all 25 commands, alphabetized)
│   ├── config-schema.md                  (every arawn.toml section + key)
│   ├── env-vars.md                       (18 env vars)
│   ├── data-directory.md                 (full ~/.arawn map)
│   ├── agent-tools.md                    (74 tools, signatures + permission behavior)
│   ├── permissions.md                    (rule format + modes + audit)
│   ├── shell-sandbox.md                  (sandbox + safe-env allowlist)
│   ├── memory-model.md                   (tables + storage; from current memory.md)
│   ├── feeds-overview.md                 (on-disk layout, cadence, status from feeds/index.md)
│   ├── feed-templates.md                 (current feeds/template-catalog.md, refreshed to 17)
│   ├── feed-search-tool.md               (current feeds/feed-search.md)
│   ├── projection-tables.md              (schema + per-type from palaces/projections.md)
│   ├── palace-types.md                   (entity/relation catalog from palaces/index.md)
│   ├── workstream-tools.md               (10+3 tools from palaces/agent-read-patterns.md)
│   ├── workstream-cli.md                 (slug rules, lifecycle, identity_profile column)
│   ├── ceremonies-tools.md               (5+7+5 daily/weekly/retro tool refs)
│   ├── todos-tools.md                    (8 todo_* tools)
│   ├── steward-subroutines.md            (from palaces/steward.md)
│   ├── workflow-tools.md                 (4 workflow_* tools + cron + storage from workflows.md)
│   ├── plugins.md                        (manifest, marketplaces, lifecycle, scopes)
│   ├── mcp.md                            (server config + tool exposure)
│   ├── skills.md                         (skill format + invocation)
│   ├── sub-agents.md                     (agent tool + task_* family + agent_defs)
│   ├── integrations.md                   (per-provider scopes/tools/permission-prompts consolidated)
│   └── troubleshooting.md                (all "error → cause → fix" tables consolidated)
│
└── explanation/                          (understanding-oriented; ~14 pages, all NEW)
    ├── index.md
    ├── what-is-arawn.md                  (vision + agent loop + self-hosted thesis)
    ├── the-agent-loop.md                 (turn flow: prompt → LLM → tool → permission → exec → response)
    ├── three-layer-data-model.md         (feeds → projections → palaces)
    ├── feeds.md                          (rationale + when-to-use)
    ├── projections.md                    (why flat, when-to-read-which-type)
    ├── palaces.md                        (palace metaphor + lifecycle)
    ├── extraction.md                     (4-stage CoT + two-tag rationale)
    ├── steward.md                        (bounded blast radius + journal)
    ├── workstreams.md                    (what + when + vs scratch)
    ├── memory-design.md                  (global vs workstream, FTS-vs-vector, scope-lock)
    ├── workflows.md                      (when-to-workflow + cloacina rationale)
    ├── permission-model.md               (deny > allow > ask + plan-mode philosophy)
    ├── identity-by-workstream.md         (ARAWN-I-0035: why persona is a workstream attribute)
    └── ceremonies.md                     (morning brief / weekly / retro philosophy + nightly recovery)
```

Total: ~55 doc files (vs 21 today). Net growth driven by splits (the existing 21 files contain content that belongs in ~35 pages) plus 14 NEW Explanation pages and ~10 catalog reference pages.

## Phase C task decomposition (proposed)

Each task is one coherent doc batch, one PR, one Metis task:

1. **C-1 — Scaffold + SUMMARY rewrite + landing pages.** Create directories, move existing pages (no content change yet), write new `SUMMARY.md` and the four index pages.
2. **C-2 — Tutorial extraction & rewrite.** Trim `getting-started.md` core into `tutorials/first-chat.md` (~200 lines). Promote workstream quickstart to `tutorials/first-workstream.md` and flesh it out.
3. **C-3 — How-tos: provider connections.** `connect-google.md`, `connect-slack.md`, `connect-atlassian.md`, `connect-github.md` (extract from getting-started §6 + current integrations/github.md).
4. **C-4 — How-tos: usage recipes.** `create-a-feed.md`, `bind-a-workstream-to-a-feed.md`, `curate-a-workstream.md`, `read-feeds-with-the-agent.md`, `lock-down-permissions.md`, `author-a-workflow-by-hand.md`, `debug-oauth-failures.md`, `recover-from-llm-warmup-failure.md`.
5. **C-5 — Reference catalogs (cross-cutting).** `cli.md`, `slash-commands.md`, `config-schema.md`, `env-vars.md`, `data-directory.md`, `troubleshooting.md`.
6. **C-6 — Reference catalogs (agent surface).** `agent-tools.md`, `permissions.md`, `shell-sandbox.md`, `integrations.md`.
7. **C-7 — Reference: workstream + ceremonies + todos + identity.** `workstream-tools.md`, `workstream-cli.md`, `ceremonies-tools.md`, `todos-tools.md`, `steward-subroutines.md`, `feed-search-tool.md`.
8. **C-8 — Reference: data-model.** `feeds-overview.md`, `feed-templates.md`, `projection-tables.md`, `palace-types.md`, `memory-model.md`, `workflow-tools.md`.
9. **C-9 — Reference: extensibility.** `plugins.md`, `mcp.md`, `skills.md`, `sub-agents.md`.
10. **C-10 — Explanation pages.** All 14 (split across two tasks if too big at hand-off time).

Each task carries its slice of the C-9-equivalent "fix Phase A drift" sweep — the page being rewritten gets all its accuracy issues corrected as part of that task's commit.
