# Arawn Docs — Diataxis Structural Fit Audit

*Phase A audit deliverable for ARAWN-I-0051. Returned inline by the Diataxis-fit agent on 2026-05-18; `/tmp` write was denied so this is the canonical copy.*

## Verdict

Arawn's user docs are best characterized as a **reference-and-explanation manual with a 627-line tutorial bolted on the front**. The current top-level structure (Getting Started → Reference → Integrations) is not a Diataxis structure: it conflates how-to (Integrations is mostly OAuth recipes), buries the largest block of how-to content (provider setup) inside a single tutorial page, and has zero pages whose primary purpose is *explanation* even though half the project's value lives in concepts (workstreams, palaces, feeds-vs-projections-vs-palaces, steward subroutines, the agent loop). The reference pages themselves are the strongest material — `feeds/template-catalog.md`, `feeds/feed-search.md`, and `palaces/agent-read-patterns.md` are nearly pure reference and only need light reshaping. Five of the seven "Reference" pages are actually hybrids: reference + significant explanation prose, often with embedded how-to. The integration pages are almost empty — they pretend to be a section but most pages just point readers back into the tutorial's middle.

---

## 1. Per-file classification

| Path | Claimed quadrant (SUMMARY) | Actual content breakdown (% est) | Top violations |
|---|---|---|---|
| `docs/src/intro.md` | Front matter | 100% landing-page prose | Not in any quadrant — fine as an `index`. No violation. |
| `docs/src/getting-started.md` | Tutorial | 25% tutorial (build + first message, §1-5), 55% how-to (§6 OAuth recipes for 3 providers, §7 watch feeds, §8 workstream quickstart), 15% reference (TOML examples, error tables), 5% troubleshooting | Massive scope creep. Tutorial proper ends at ~line 160. Lines 162-565 are stacked how-to recipes that should be standalone pages. §6 alone (162-490) is 4 distinct OAuth how-tos masquerading as one tutorial step. §7/§8 are tutorial-shaped wrappers around separate flows. Troubleshooting (577-619) is reference. |
| `docs/src/security.md` | Reference | 35% reference (sandbox table, rule format, mode table), 40% explanation (why a sandbox, why deny>allow>ask, rationale prose), 25% how-to ("Limiting blast radius" — three TOML recipes) | Hybrid. The three named setups (104-154) are how-to recipes ("how do I run arawn for CI?"). Rationale prose about deny>allow>ask belongs in explanation. |
| `docs/src/memory.md` | Reference | 30% reference (entity-type table, relation list, confidence table, storage layout), 50% explanation (two stores, scope-locking rationale, retrieval mechanics, FTS-vs-vector trade-off, embedding-model story), 20% other (WIP notes, worked agent example) | Hybrid. Reads like explanation of the memory model with reference tables embedded. Either split or accept it as explanation with tables in a sibling reference. |
| `docs/src/workflows.md` | Reference | 30% reference (cron syntax, task types, tool names, storage layout), 35% explanation (when-to-use, design rationale, "judgement-in-one-step"), 20% how-to (JSON spec walkthrough), 15% example pointers | Hybrid. Reads like explanation with a reference appendix. The 4-tool catalog + cron syntax + storage layout is pure reference; the "when to workflow vs conversation" framing and cloacina rationale are explanation. |
| `docs/src/feeds/index.md` | Reference | 25% reference (data dir layout, cadence table, status table), 60% explanation (what feeds are, when-to-feed, "agent uses Read/Grep/Glob" philosophy, backfill rationale), 15% how-to (two ways to create a feed) | Hybrid leaning explanation. Doing concept-introduction more than reference lookup. Tables move to reference; rest is explanation. |
| `docs/src/feeds/template-catalog.md` | Reference | 95% reference (12 templates: param table + cadence + on-disk shape each), 5% pattern framing | Almost pure reference. Strong fit. Keep as-is in the reference quadrant. |
| `docs/src/feeds/agent-read-patterns.md` | Reference | 70% how-to (10 prompt → tool-call recipes), 20% reference ("patterns that work well/avoid"), 10% explanation | Misclassified as reference. Each section is a recipe ("did anyone @ me about the launch?") — goal-oriented, not lookup. Should be how-to. |
| `docs/src/feeds/feed-search.md` | Reference | 65% reference (tool signature, feed-type table, ranking model, operational notes), 25% how-to (6 worked prompts with full JSON), 10% explanation (RRF rationale, "not for" framing) | Strong reference page with embedded how-to. Worked prompts could move to a how-to page or stay as examples appendix. |
| `docs/src/palaces/index.md` | Reference | 75% explanation (three-layer model, palace metaphor, lifecycle diagram, ADR anchors, when-to-use), 20% reference (entity/relation types), 5% navigation | Severely misclassified. This is *the* explanation page of the project. Move; entity-type catalog extracts to reference. |
| `docs/src/palaces/projections.md` | Reference | 55% reference (per-type metadata, schema, ProjectionRow struct), 35% explanation (when-to-read-X table, embedding-pass rationale, why projections are flat), 10% mechanics | Hybrid. Split like feeds/index.md: reference for schema/tables; explanation for rationale. |
| `docs/src/palaces/extraction.md` | Reference | 30% reference (return-shape JSON, config knob), 60% explanation (4-stage diagram + commentary, "why two tag fields" war story, ADR-0004 anchor, cursor semantics rationale), 10% mechanics | Misclassified as reference. Pipeline rationale + UAT war story = explanation. Only the JSON return shape and `[extraction] llm = …` are reference. |
| `docs/src/palaces/steward.md` | Reference | 30% reference (subroutine table, journal schema, apply/rollback dispatch table), 60% explanation (re-shelve/dust specifics, journal philosophy, blast-radius design), 10% mechanics | Misclassified as reference. Subsystem-design page. Tables extract; rest is explanation. |
| `docs/src/palaces/agent-read-patterns.md` | Reference | 55% reference (10 tool signatures), 30% how-to ("typical session" dialogue at 217-242), 15% mental-model framing | Hybrid leaning reference. Tool-by-tool surface is reference-quality. Session walkthrough at the end is how-to. |
| `docs/src/integrations/gmail.md` | Integrations | 50% reference (1-line tool list), 50% pointer-to-tutorial | Stub. Not really in any quadrant. |
| `docs/src/integrations/calendar.md` | Integrations | Same as gmail — 50% tool list, 50% pointer | Stub. |
| `docs/src/integrations/drive.md` | Integrations | 40% reference (tool list, scope rationale, read-content-dispatch table, permission notes), 30% pointer, 30% explanation ("why full read+write scope") | Notes are real reference; rest is pointer. |
| `docs/src/integrations/slack.md` | Integrations | 30% reference (tools, multi-workspace caveat), 70% pointer | Stub. |
| `docs/src/integrations/atlassian.md` | Integrations | 40% reference (tools, cloud-id auto-discovery, scope notes), 60% pointer + explanation | Notes are real reference; rest is pointer. |
| `docs/src/integrations/github.md` | (not in SUMMARY) | 70% how-to (Step 1-2-3 install recipe), 20% reference ("What's stored" table, what-you-get list), 10% explanation (App-vs-OAuth rationale) | Misclassified by being in `integrations/` while content is how-to. Also: missing from `SUMMARY.md`. |
| `docs/src/SUMMARY.md` | TOC | 1 tutorial, 7 hybrid "reference" entries, 5 integration stubs | "Integrations" is not a Diataxis quadrant (cross-cutting topic). "Reference" is currently a grab bag of explanation + reference + how-to. |

---

## 2. Section-level violations

### `getting-started.md` § "6. Connect integrations" (lines 162-491)
Content type: How-to (3 distinct OAuth recipes for Google, Slack, Atlassian, plus verification + 7-row common-errors table).
Quadrant fit: Inside a tutorial but stops being one. Each provider section is a standalone, task-shaped recipe; the goal is provider-specific setup, not learning arawn. Move to `how-to/connect-google.md` (Gmail/Calendar/Drive share the app, so they share the page), `how-to/connect-slack.md`, `how-to/connect-atlassian.md`. "Verifying" + "Common integration errors" become a shared `how-to/verify-and-troubleshoot-integrations.md` or fold into each provider page.

### `getting-started.md` § "7. Continual data feeds (optional)" (lines 492-520)
Content type: How-to (creates a feed via `/watch`).
Quadrant fit: Tutorial-shaped wrapper around a how-to. Move to `how-to/create-a-feed.md`. The conceptual framing already exists in `feeds/index.md`.

### `getting-started.md` § "8. Workstream palaces (optional)" (lines 522-565)
Content type: How-to + tutorial framing.
Quadrant fit: Closest thing to a *second* tutorial in the docs. End-to-end `/workstream create`, `/workstream bind`, `signal_search`. Recommended split: a proper `tutorials/first-workstream.md` ("track your first workstream") taking the reader through a concrete outcome — separate from the build/configure tutorial.

### `getting-started.md` § "Troubleshooting" (lines 577-619)
Content type: Reference (error → cause → fix tables).
Quadrant fit: Appended to a tutorial; pure lookup material. Move to `reference/troubleshooting.md`, cross-referenced from how-tos.

### `security.md` § "Limiting blast radius" (lines 104-154)
Content type: How-to (three named setups: "paranoid", "hands-off CI", "strict review").
Quadrant fit: Reference page with how-to cookbook embedded. Move to `how-to/lock-down-permissions.md` (or split into three). Rule format + mode table stay as reference.

### `memory.md` (whole page)
Content type: Mostly explanation with reference tables embedded.
Quadrant fit: Split. `reference/memory-model.md` (entity-type table, relation list, confidence table, storage layout); `explanation/memory-design.md` (two-stores rationale, scope-locking, FTS-vs-vector, retrieval flow).

### `workflows.md` (whole page)
Content type: Explanation + reference hybrid.
Quadrant fit: Same split as memory. `explanation/workflows.md` (when-to-workflow, task-type framing, cloacina rationale); `reference/workflow-tools.md` (cron syntax, 4-tool catalog, storage layout). Authoring-by-hand becomes `how-to/author-a-workflow-by-hand.md`.

### `feeds/index.md` (whole page)
Content type: Explanation with reference tables.
Quadrant fit: Doing concept work — "what is a feed, when does it pay off, how does the data layout fit the agent's mental model." Split: `explanation/feeds.md` (substrate framing, when-to-feed, backfill mechanics, disk-usage rationale); `reference/feeds-overview.md` (on-disk layout, cadence table, status table).

### `feeds/agent-read-patterns.md` (whole page)
Content type: How-to (10 prompt → tool-call recipes).
Quadrant fit: Misclassified as reference. Each section is goal-oriented. Move whole file to `how-to/read-feeds-with-the-agent.md`. Bottom "patterns" sections stay as small reference appendix.

### `palaces/index.md` (whole page)
Content type: Almost entirely explanation (three-layer model, palace metaphor, lifecycle ASCII, ADR pointers, when-to-use).
Quadrant fit: Severely misclassified. Move to `explanation/palaces.md`. Entity/relation types extract to `reference/palace-types.md`.

### `palaces/extraction.md` (whole page)
Content type: Explanation (stage diagram + commentary, "why two tag fields" war story, ADR-0004 anchor).
Quadrant fit: Misclassified as reference. Move to `explanation/extraction.md`. Config knob is a one-line reference entry that moves to a config reference page.

### `palaces/steward.md` (whole page)
Content type: Explanation with reference inserts.
Quadrant fit: Misclassified as reference. Move to `explanation/steward.md`. Subroutine table + journal schema + apply/rollback dispatch table extract to `reference/steward-subroutines.md`.

### `palaces/projections.md` (whole page)
Content type: Hybrid (schema + when-to-use rationale).
Quadrant fit: Same split shape as feeds/index.md. `explanation/projections.md` (rationale + when-to-use); `reference/projection-tables.md` (schema + per-type metadata + ProjectionRow struct + embedding mechanics).

### `palaces/agent-read-patterns.md` § "A typical session" (lines 217-242)
Content type: How-to dialogue.
Quadrant fit: Reference page with one how-to subsection. Keep bulk as `reference/workstream-tools.md`; move typical-session walkthrough to `tutorials/first-workstream.md` or `how-to/curate-a-workstream.md`.

### `integrations/github.md` § "Setup" (lines 9-91)
Content type: How-to (Step 1-2-3 register/install recipe).
Quadrant fit: Buried under `integrations/`. Move setup to `how-to/connect-github.md`. "What's stored" and "Disconnect" stay as reference.

### `integrations/{gmail,calendar,drive,slack,atlassian}.md` (whole pages)
Content type: Stubs pointing back to the tutorial.
Quadrant fit: No clear quadrant. Once each provider's how-to lives at `how-to/connect-<provider>.md`, the per-provider tool list should consolidate into a single `reference/integrations.md` with one row per provider (tools + scopes + permission-prompt rules). Five small reference pages would also work, but consolidation matches the scale.

---

## 3. Structural gaps

### Tutorials (currently 1 — and 90% of it isn't tutorial)
Missing:
- A real tutorial ending at line ~160 of getting-started.md. Current page bleeds into how-to after "First message" and never recovers. A clean tutorial is 200-250 lines max.
- A second tutorial: **"Set up your first workstream."** End-to-end create → bind → watch extraction → query with `signal_search`. Current §8 of getting-started is a 40-line stub.
- (Optional) **"Build your first morning brief."** workflows.md mentions `daily-pr-briefing` but there's no tutorial walking through writing it with the agent and seeing the first run land. Flagship feature without a flagship tutorial.

### How-to guides (currently 0 as standalone pages)
- `how-to/connect-google.md` (extract from getting-started §6 Google)
- `how-to/connect-slack.md` (extract from §6 Slack)
- `how-to/connect-atlassian.md` (extract from §6 Atlassian)
- `how-to/connect-github.md` (extract from integrations/github.md)
- `how-to/create-a-feed.md` (`/watch` recipe)
- `how-to/bind-a-workstream-to-a-feed.md`
- `how-to/curate-a-workstream.md` (refine/apply/rollback flow)
- `how-to/read-feeds-with-the-agent.md` (current feeds/agent-read-patterns.md)
- `how-to/lock-down-permissions.md` (security.md "Limiting blast radius")
- `how-to/debug-oauth-failures.md` (getting-started "Common integration errors")
- `how-to/recover-from-llm-warmup-failure.md` (getting-started Troubleshooting)
- `how-to/author-a-workflow-by-hand.md` (currently a single sentence in workflows.md)
- `how-to/use-a-cheaper-extractor-model.md` (the 5-line config at end of palaces/extraction.md)

### Reference (scattered + mixed)
Missing:
- **Tools catalog.** No single page lists every agent tool with signature, scope, and permission behavior. Tools are mentioned across getting-started.md (`shell`), security.md (`Read`/`file_*`/`web_fetch`), memory.md (`memory_store`/`memory_search`), workflows.md (`workflow_create`/`list`/`status`/`delete`), feeds/feed-search.md (`feed_search`), feeds/agent-read-patterns.md (`Read`/`Glob`/`Grep`), palaces/agent-read-patterns.md (10 workstream + 3 signal tools), and each integration stub (5-11 provider tools each).
- **Config-schema reference.** TOML examples scattered across getting-started.md (4 examples), security.md (3 setups), integrations/github.md, feeds/index.md, palaces/extraction.md. No single page documents every `[section]` and key.
- **Slash-command reference.** Across the docs: `/connect`, `/disconnect`, `/integrations`, `/watch`, `/unwatch`, `/feeds`, `/workstream create|switch|bind`, `/help`, `/remember` (stub), `/memory` (stub), `/forget` (stub), `/permissions` (stub). No reference page lists which are real, which are stubs, what each does.
- **CLI-flag reference.** `arawn serve`, `arawn tui`, `arawn tui --url`, one-shot mode, `arawn --list-sessions`, `arawn --session <uuid>` — scattered.
- **Env-var reference.** `GROQ_API_KEY`, `OLLAMA_API_KEY`, `ARAWN_GITHUB_APP_ID/SLUG/PRIVATE_KEY_PATH/PEM`, `RUST_LOG`, plus the safe-env allowlist referenced in security.md.
- **Data-directory reference.** `<data_dir>/memory.db`, `<data_dir>/workstreams/<name>/memory.db`, `<data_dir>/workflows/`, `<data_dir>/workflows.db`, `<data_dir>/data/<provider>/<template>/<feed_id>/`, `<data_dir>/projections.db`, `<data_dir>/tokens/`, `<data_dir>/integrations/github/`, `<data_dir>/models/all-MiniLM-L6-v2/model.onnx` — scattered across 6 pages. A "data directory map" would be high-value.
- **Troubleshooting reference.** Currently lives at the bottom of getting-started.md (LLM/TUI/key issues) and inside the "Common integration errors" table mid-tutorial. Should be one top-level reference page organized by symptom.

### Explanation (currently 0 pages explicitly in this quadrant — biggest gap)
- **Three-layer data model** (feeds → projections → palaces) — currently the opening of palaces/index.md.
- **Workstreams as a concept** — what is one, when do I create one, how is it different from a feed. Inferable from palaces/index.md + §8 stub, never directly explained.
- **Identity profile** — ARAWN-I-0035 is active (visible in git status as `crates/arawn-storage/migrations/V10__workstream_identity_profile.sql`) but the concept is absent from docs.
- **Agent loop and tool dispatch** — how a turn flows: prompt → LLM → tool call → permission check → execute → response. Nothing in docs explains this.
- **Permission-model rationale** — why deny>allow>ask, why plan mode. Currently mixed into security.md.
- **Why feeds are local-first** — the "you stay sovereign over your data" thesis. Touched in feeds/index.md but never gathered.
- **Memory model: global vs workstream** — why the two-tier split, what's scope-locked and why. Currently mixed into memory.md.
- **Extractor pipeline rationale** — why a 4-stage CoT, why two tag fields, UAT war story. Currently mixed into palaces/extraction.md.
- **Steward's blast-radius philosophy** — why bounded, journaled, propose-vs-apply. Mixed into palaces/steward.md + ADR-0003.
- **Workflows as a concept** — when workflows shine vs conversations, why a DAG, why cloacina. Mixed into workflows.md.
- **Todo system / agent self-coordination** — referenced indirectly; nothing explains it.
- **Ceremonies** (morning brief, weekly priorities, retro) — integrations/github.md mentions these as a real concept; never explained anywhere.

---

## Proposed new top-level structure (Diataxis agent's suggestion)

```
docs/src/
├── intro.md                              (keep — landing page)
│
├── tutorials/
│   ├── index.md
│   ├── first-chat.md                     (build/configure/first message; cleaned core of current getting-started §1-5)
│   ├── first-workstream.md               (create → bind → extract → query)
│   └── first-morning-brief.md            (NEW: agent-authored workflow end-to-end)
│
├── how-to/
│   ├── index.md
│   ├── connect-google.md
│   ├── connect-slack.md
│   ├── connect-atlassian.md
│   ├── connect-github.md
│   ├── create-a-feed.md
│   ├── bind-a-workstream-to-a-feed.md
│   ├── curate-a-workstream.md
│   ├── read-feeds-with-the-agent.md
│   ├── lock-down-permissions.md
│   ├── author-a-workflow-by-hand.md
│   ├── use-a-cheaper-extractor-model.md
│   ├── debug-oauth-failures.md
│   └── recover-from-llm-warmup-failure.md
│
├── reference/
│   ├── index.md
│   ├── config-schema.md
│   ├── cli-flags.md
│   ├── env-vars.md
│   ├── slash-commands.md
│   ├── data-directory.md
│   ├── tools.md
│   ├── permissions.md
│   ├── shell-sandbox.md
│   ├── memory-model.md
│   ├── feeds-overview.md
│   ├── feed-templates.md
│   ├── feed-search-tool.md
│   ├── projection-tables.md
│   ├── palace-types.md
│   ├── workstream-tools.md
│   ├── steward-subroutines.md
│   ├── workflow-tools.md
│   ├── integrations.md
│   └── troubleshooting.md
│
└── explanation/
    ├── index.md
    ├── what-is-arawn.md
    ├── three-layer-data-model.md
    ├── feeds.md
    ├── projections.md
    ├── palaces.md
    ├── extraction.md
    ├── steward.md
    ├── workstreams.md
    ├── memory-model.md
    ├── workflows.md
    ├── permission-model.md
    ├── identity-profile.md
    └── ceremonies.md
```

## Top 3 structural problems

1. **`getting-started.md` is 627 lines, ~75% of which is how-to content** — OAuth recipes for Google/Slack/Atlassian, `/watch` recipes, a workstream quickstart, plus a reference troubleshooting table. Should split into ~10-12 standalone how-to pages and leave a real ~200-line tutorial.

2. **5 of 7 "Reference" pages are predominantly explanation prose** with reference tables embedded (`palaces/index.md`, `palaces/extraction.md`, `palaces/steward.md`, `feeds/index.md`, `memory.md`) — concept exposition mislabeled as lookup material.

3. **The Explanation quadrant is empty as a top-level section.** Every "why does arawn work this way?" answer — the three-layer data model, the workstream concept, the permission philosophy, the identity profile (ARAWN-I-0035, active in the repo), and ceremonies (referenced but never defined) — is either inferable from prose hidden inside reference pages or completely absent.
