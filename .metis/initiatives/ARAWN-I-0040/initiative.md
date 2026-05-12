---
id: signal-extraction-graphqlite
level: initiative
title: "Signal extraction — graphqlite-backed memory + per-feed projections + workstream palaces"
short_code: "ARAWN-I-0040"
created_at: 2026-05-12T01:09:56.555061+00:00
updated_at: 2026-05-12T01:09:56.555061+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: L
initiative_id: signal-extraction-graphqlite
---

# Signal extraction — graphqlite-backed memory + per-feed projections + workstream palaces

## Parent Vision

[[ARAWN-V-0001]]

## Context

I-0039 (continual data feeds) gave us a local mirror of upstream
state — Slack, Gmail, Drive, Calendar, Jira, Confluence — in
`~/.arawn/data/`. The agent can already answer many questions by
grepping and globbing that tree. T-0218's read-pattern doc shows
that's enough for ~80% of questions cleanly.

The remaining 20% are the load-bearing ones:

- **Cross-source** — "what's the deploy status across slack + jira +
  email" — wants a unified topical layer.
- **Semantic** — "anything about the Stripe migration" — wants
  embeddings, not just substring matching.
- **Temporal-aggregate** — "what did Alice decide last quarter" —
  wants entity resolution + a timeline.

This initiative builds the substrate that answers those questions
without prescribing a global ontology.

The principle is **opinionated about method, not content**:

- Every workstream has the same lifecycle (declare → bind sources →
  extract → curate → query).
- Every workstream's KB has the same meta-shape (typed entities +
  typed relations + provenance, on graphqlite).
- Every extraction pass uses the same prompt-chain skeleton.
- The steward runs continuously over every KB with the same
  re-shelve / dust / map / door-watch subroutines.

The *content* of each workstream — what tags, what conventions, what
"decision" or "encounter" mean in that domain — is free.

The memory-palace metaphor is the design lens (and is already
partly realized in arawn-memory today): locality over global graphs;
relationships before attributes; vivid extraction over flattened
records; continuous gentle curation; provenance walks.

We own graphqlite; it becomes the storage layer for typed graph data
across arawn. arawn-memory migrates onto it as part of this
initiative.

There is no userbase yet, so we are free to break on-disk schemas,
drop migration tooling, and rebuild storage from scratch when it
sharpens the design.

## Goals & Non-Goals

**Goals:**

- Ship a three-layer architecture: **feeds** (raw, today) → **projections** (per-feed-type normalized + embedded views) → **workstream palaces** (typed entity graphs in graphqlite, per user-declared domain).
- Migrate `arawn-memory` to graphqlite as the storage layer. Memory + palaces share the same graph engine.
- Explicit, user-managed workstreams: `/workstream new dnd "track my Tuesday-night campaign"`, list, switch, bind to feed sources.
- A steward subsystem that continuously curates each workstream KB — dedupe, supersession, relation inference, stale summarization — with bounded blast radius and full journaling.
- `feed_search` agent tool over projections (works without any workstream); `signal_search` / `signal_query` / `signal_timeline` over workstream KBs.
- All of it wires into actual server runtime — no plumbing without integration. End-to-end smoke tests against real feeds per phase.

**Non-Goals:**

- Per-workstream ontologies as a separate type system. The closed `EntityType` / `RelationType` enums in arawn-memory stay closed; per-workstream variation lives in tag vocabularies, not in new entity types.
- Cross-workstream entity identity beyond what the Global tier (`Person`, `Preference`) already provides. Deferred unless a real use case appears.
- Migration tooling for existing memory data. No userbase → just rebuild storage.
- A general-purpose query language. Read patterns + Rust APIs + a small set of agent tools are the surface.
- Replacing feeds as the source of truth. Projections + KBs are derivable views; feeds remain the canonical local mirror.

## Architecture

### Three layers

| Layer | Storage | Owns |
|---|---|---|
| **Feeds** *(I-0039, exists)* | Files on disk under `~/.arawn/data/<provider>/<template>/<feed_id>/` | Fidelity: the bytes look like what the provider returned. |
| **Projections** *(new, this initiative)* | sqlite tables, per feed type (`gmail_messages`, `slack_messages`, `drive_files`, `jira_issues`, `confluence_pages`, `calendar_events`, ...) with normalized fields, raw text body, and an embedding column. FTS5 alongside. | Findability: cross-feed semantic + structured search without LLM extraction. |
| **Workstream KBs** *(new, this initiative)* | graphqlite, one DB per workstream at `~/.arawn/workstreams/<name>/kb.db`. Graph-shaped: typed nodes (entities), typed edges (relations), attributes, provenance. | Curated meaning: typed entities + relations populated by LLM extraction from projections, maintained by the steward. |

Layer boundaries are walked, not bridged: feeds populate projections; projections feed extractors that write into workstream KBs. Each layer can be queried independently.

### Why graphqlite

We own it; making it the home for typed graph storage across arawn (memory + palaces) keeps us from running two property-graph implementations. Schema management lives in graphqlite, not in palace code, so any future consumer benefits.

Memory's current sqlite schema (entities, relations, fts, vectors) becomes the first graphqlite schema. Palace KBs share the same skeleton with per-workstream tag vocabularies.

### Workstream lifecycle

1. `/workstream new <name> "<description>"` — registers the workstream, creates `~/.arawn/workstreams/<name>/` with `workstream.toml`, an empty `kb.db`, and a `journal.jsonl`. The description shapes the LLM's tag vocabulary (no explicit ontology).
2. `/workstream bind <name> <feed_id>` — binds a feed as a source. Schedules an extraction workflow for that workstream over that source's projection rows.
3. **Extractor** runs on the workstream's cadence. Reads new projection rows since the workstream's cursor, runs the extraction chain (LLM emits entities + relations + tags scoped to this workstream's vocabulary), writes via `store_fact` semantics (search-before-create dedup, reinforcement, supersession).
4. **Steward** runs on a slower cadence. Re-reads the KB, runs the four maintenance subroutines (re-shelve, dust, map, door-watch). Writes diffs to the journal.
5. `/workstream query <name> "<question>"` — agent answers using `signal_search` + `signal_query` over the workstream KB.

### The steward, concretely

Four bounded subroutines, each a separate prompt-chain:

| Subroutine | Job | Blast radius cap |
|---|---|---|
| **Re-shelve** | Entity dedupe; merge near-duplicates. | N merges per pass; never delete provenance. |
| **Dust** | Summarize stale clusters (e.g. an issue idle for months gets a steward-written summary node). | M summaries per pass; original nodes preserved. |
| **Map** | Notice co-occurrence patterns; propose new relations or candidate new entity-tags. | Proposals only; user accepts via `/workstream refine`. |
| **Door-watch** | Look for cross-workstream identity candidates. | Proposals only. |

Every steward action writes a journal entry with: subroutine, inputs (entity ids), outputs (diff), prompt-chain hash, model used. The user can `/workstream journal <name>` to inspect or `/workstream rollback <action_id>` to undo.

## Detailed Design

### Phase 0 — graphqlite inventory spike (DONE 2026-05-11)

**Findings:**

GraphQLite (v0.4.4) is a SQLite extension with full Cypher query support, EAV property-graph storage, typed labels + relationships, parameterized queries (`cypher_builder`), bulk insert, and built-in graph algorithms. Rust binding exists and is sufficient.

**What's there:**

- Multi-label nodes (maps to `EntityType` — Fact, Decision, Convention, Preference, Person, Note).
- Typed relationships (maps to `RelationType` — RelatesTo, Contradicts, Supports, Supersedes, ExtractedFrom, Mentions, BelongsTo).
- Properties typed by inference (int / text / real / bool / json) with covering indexes — fits `confidence_source`, `reinforcement_count`, `superseded`, etc. cleanly.
- Full Cypher (MATCH, CREATE, MERGE, SET, DELETE, WITH, UNWIND, RETURN).
- `Graph` high-level API for upsert / get / delete / traversal; `Connection.cypher_builder` for parameterized queries.

**What's intentionally not there:**

- **No declarative schema.** EAV is schemaless by design; any property can be any type on any node.
- **No FTS.** No full-text search built in.
- **No vector search.** No embedding similarity built in.
- **No migration tooling** — not needed because schemaless.

**Decisions taken from the spike:**

1. **Do not add schema-management to graphqlite as part of this initiative.** Schema enforcement for arawn-memory's small, domain-coupled type system lives in Rust at the public API. This is `ARAWN-A-0002`.
2. **FTS5 + vector indexes stay in arawn-memory's existing tables, colocated in the same sqlite DB as the graphqlite tables.** All-in-one-DB makes cross-index joins free (raw SQL escape hatch where Cypher can't see those tables).
3. **Default to Cypher for memory's read/write paths.** Raw SQL only for the FTS / vector / cross-index paths Cypher can't reach. The `cypher_builder` parameterized API is the canonical surface.
4. **Phase 1 is smaller than originally sketched** — no graphqlite schema-mgmt to build means Phase 1 is just "rewrite MemoryStore against graphqlite + keep FTS/vectors alongside."

### Phase 1 — arawn-memory rebuilt on graphqlite

- Add `graphqlite` as a workspace dependency.
- Define the memory schema as Rust types — entity/relation type enums validated at the public API boundary. No graphqlite-side schema features.
- Rewrite `MemoryStore` so entity / relation CRUD goes through `Graph` (or `Connection.cypher_builder` for parameterized queries). One node per `Entity` (labelled with its `EntityType`), one edge per `Relation` (typed by `RelationType`). Confidence fields, reinforcement, supersession all become node properties.
- Keep `MemoryManager`'s scope routing (Global vs. Workstream) intact — each workstream and the global tier each get their own sqlite DB with graphqlite extension loaded.
- Keep arawn-memory's FTS5 virtual tables and vector tables in the same DB. Implement search-before-create dedup as a SQL/Cypher hybrid: FTS+vector filter via SQL, fetch + write via Cypher.
- Public Rust API (`MemoryStore::insert_entity`, `MemoryStore::search`, etc.) stays stable so engine tools and auto-memory keep working unchanged.
- LongMemEval bench passes; all existing memory tests pass.
- No migration tooling. Existing memory DBs get blown away on next boot — there is no userbase.

Sizing: ~1 week of focused work.

### Phase 2 — per-feed-type projections + `feed_search`

For each feed type (`gmail_messages`, `slack_messages`, `slack_thread_messages`, `drive_files`, `jira_issues`, `jira_comments`, `jira_history`, `confluence_pages`, `calendar_events`):

- sqlite table with: stable id, feed_id, source_ts, normalized fields (sender/channel/path/etc.), `body_text`, `embedding`, FTS row.
- Per-type ingestion hook in `dispatch::run_feed` — after a successful feed run, write projection rows for new items.
- Embedding pipeline: reuse arawn-memory's embedder; batch on the way in; throttle config.
- Backfill: re-project existing feed data on first run.
- `feed_search` agent tool: filter by feed type, time range, structured fields; rank by semantic similarity.

Ships independent value: cross-feed semantic search without any workstream declared.

### Phase 3 — explicit workstream management

- Workstream registry (probably sqlite, flat table). Persisted across restarts.
- Slash commands: `/workstream new`, `/workstream list`, `/workstream switch`, `/workstream show`, `/workstream describe`, `/workstream bind`, `/workstream unbind`, `/workstream delete`.
- `MemoryManager` takes a workstream name, loads/creates the KB lazily.
- Boot path: load registered workstreams' KBs, register extraction + steward workflows for each.

### Phase 4 — projection → workstream extractor

Per workstream, per bound source:

- Cloacina workflow that reads new projection rows since the workstream's cursor (per-source cursor in workstream meta).
- Extraction chain (multi-stage prompt): classify → extract → link → write.
- Output: typed entities + relations + tags, written via `store_fact` semantics.
- Vocabulary: the workstream description gets fed into the chain prompt; the chain emits tags consistent with that vocabulary. The vocabulary refines over time as the LLM observes recurring tag patterns.
- Backfill: when a workstream binds a source that already has projection rows, walk all of them in a backfill loop (same spawn-loop convergence pattern as I-0039 / T-0227).

### Phase 5 — the steward

- Periodic cloacina workflow per workstream (probably daily by default).
- Four subroutines as described above. Each is its own task in the workflow.
- Journal written per action; bounded blast radius.
- `/workstream refine <name>` exposes accumulated `map` / `door-watch` proposals for user accept/reject.

### Phase 6 — agent tools

- `signal_search` — semantic over a workstream KB (entities + relations).
- `signal_query` — structured filter ("decisions tagged stripe:migration in last 30 days").
- `signal_timeline` — chronological slice across a workstream.
- `feed_search` from Phase 2 stays as the no-workstream fallback.

### Phase 7 — UAT + docs

- Run all six provider feeds against two real workstreams end-to-end (e.g., a `work` workstream tracking ENG decisions and a `dnd` workstream tracking a campaign).
- Drive 10+ representative prompts; capture how the agent uses the new tools.
- Documentation deliverables under `docs/src/palaces/`:
  - `index.md` — what palaces are, three-layer model, lifecycle.
  - `projections.md` — what's in each projection table; sample queries.
  - `extraction.md` — how the extractor works; the tag-vocabulary philosophy.
  - `steward.md` — what the steward does + doesn't; journal inspection; rollback.
  - `agent-read-patterns.md` — recipes for the new tools, analogous to feeds.
- Update `getting-started.md` with a "Palaces" section.

## Alternatives Considered

### Pure organic / memory dump

Dump every feed item directly into memory as embedded chunks. Zero modeling effort. Rejected: doesn't scale to multi-hop questions; doesn't separate fidelity from curation; loses per-feed-type structure.

### Clotho-style fixed ontology

Define one global ontology (Person, Project, Decision, ...) with prompt chains mapping all text into it. Rejected: ontology is the bet, and for a personal assistant where user domains are unpredictable, the wrong ontology is worse than no ontology. Also maximum LLM cost upfront with no incremental shipping.

### Two parallel graph systems (memory stays, palaces on graphqlite)

Considered. Rejected: running two property-graph implementations in the same binary is a code-smell, and memory's storage shape is already what graphqlite wants to be. Migrate once.

### Sibling initiative for projections (I-0041)

Considered when shipability under pressure was a factor. Rejected after the "no userbase" constraint surfaced — coherence beats independent shippability when there's no one to ship to.

### Per-workstream ontology types (open `EntityType`)

Considered. Rejected for now in favour of tag vocabularies on a closed type system. The six existing types have done real work; opening them is a one-way door we shouldn't take without strong evidence.

## Acceptance Criteria

- [x] Phase 0 spike done; findings captured in this initiative under "Phase 0 — graphqlite inventory spike".
- [ ] `arawn-memory` runs on graphqlite. Entity/Relation CRUD goes through Cypher; FTS5 + vector tables colocated in the same DB. Schema enforcement lives in Rust at the public API. LongMemEval bench passes. All existing memory tests green. Engine tools (`memory_store`, `memory_search`, auto-memory) unchanged from a caller's perspective.
- [ ] Projections written automatically by the feed runtime for every feed type. Backfill on existing feed data. `feed_search` registered as an agent tool and exercised by an end-to-end test.
- [ ] Workstream lifecycle commands (`new`/`list`/`switch`/`bind`/`describe`/`unbind`/`delete`) wired into the TUI and routed through the engine. Workstreams persist across restarts; KBs auto-loaded on boot.
- [ ] Per-workstream extractor runs as a cloacina workflow against bound sources. Backfill mode covers pre-existing projection rows. Idempotent re-runs.
- [ ] Steward runs on a cadence; journal records every action with diffs; blast-radius caps enforced; `/workstream rollback` works for any journaled action.
- [ ] Agent tools (`signal_search`, `signal_query`, `signal_timeline`) registered and exercised by an end-to-end test against at least one real workstream.
- [ ] Documentation deliverables under `docs/src/palaces/` shipped; `getting-started.md` updated; SUMMARY.md links them.
- [ ] `angreal check workspace` + `angreal check clippy` clean throughout.
- [ ] End-to-end smoke test per phase: each phase wires into `arawn serve` and exercises a real feed; no plumbing-without-integration.

## ADRs (Phase 1 deliverables)

- **ARAWN-A-0002** — Memory storage on graphqlite, schema enforcement in Rust. Pin: graphqlite stays schemaless (EAV); arawn-memory's entity/relation type system is enforced at the Rust public-API boundary; we don't add schema-management to graphqlite as part of this initiative; FTS5 + vector indexes colocate in the same sqlite DB as graphqlite tables; default to Cypher for query paths with raw SQL only as an escape hatch where Cypher can't see the table.
- **ARAWN-A-0003** — Workstream tag vocabularies on closed entity types. The "no per-workstream ontology" decision in writing.
- **ARAWN-A-0004** — Steward bounded blast radius. What it can change, what it can't, journaling and rollback contract.

## Risks

- **graphqlite scope creep.** Tempting to upstream "useful" features into graphqlite as we build palaces. Mitigation: graphqlite is treated as a stable dependency for this initiative; if we hit a missing feature, file it as a graphqlite issue and work around it in arawn. Don't fork-and-extend.
- **Steward going off the rails.** A continuously-running LLM rewriting its own KB is powerful and scary. Mitigation: bounded blast radius per pass; journal everything with diffs; explicit rollback; never delete provenance.
- **Vocabulary drift.** Tag conventions per workstream may degrade over time (LLM uses inconsistent tags). Mitigation: the `map` subroutine flags inconsistencies; refine flow gives the user a knob.
- **Embedding cost at scale.** Projecting every feed item produces a lot of embeddings. Mitigation: cheap local embedder (already in arawn-memory), batched on ingest, throttle config.
- **Coupling between layers.** Three layers means three places to keep in sync; bugs may surface across boundaries. Mitigation: explicit per-layer cursors; idempotent re-runs at every layer; each layer can be rebuilt from the one below.

## Implementation Plan

Phase tasks will be decomposed in the `decompose` phase after the Phase 0 spike. Sketch:

| Phase | Description | Sizing |
|---|---|---|
| 0 | graphqlite inventory spike | ✓ done |
| 1 | arawn-memory rebuilt on graphqlite | ~1 week |
| 2 | projections + `feed_search` | 1–2 weeks |
| 3 | workstream management | 3–5 days |
| 4 | projection → workstream extractor | 1–2 weeks |
| 5 | steward | 1–2 weeks |
| 6 | agent tools | 3–5 days |
| 7 | UAT + docs | 3–5 days |

Total: ~5–7 weeks of focused work. Each phase exits with a runtime-integrated, smoke-tested deliverable.
