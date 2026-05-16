---
id: ceremony-agent-tools-retro
level: task
title: "Ceremony agent tools (retro_*)"
short_code: "ARAWN-T-0293"
created_at: 2026-05-16T03:22:46.609267+00:00
updated_at: 2026-05-16T12:30:21.244840+00:00
parent: ARAWN-I-0043
blocked_by: [ARAWN-T-0292]
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0043
---

# Ceremony agent tools (retro_*)

## Parent Initiative

[[ARAWN-I-0043]]

## Objective

Expose the retro ceremony surface as agent-callable tools in the arawn-tool
registry. The agent loop in arawn-engine calls tools via JSON-schema'd
definitions; today there is no `retro_*` family. Without these, the UAT runner
(which drives the agent by user prompt → tool calls) cannot exercise the retro
engine.

## Acceptance Criteria

## Acceptance Criteria

- [ ] Tools registered in `arawn-tool`'s catalog and visible to the agent's tool registry:
  - [ ] `retro_run { }` — fires the dispatcher for the current ISO week. Returns `{ tablet_id, status: "generated" | "skipped", reason? }`.
  - [ ] `retro_current { }` — returns the current week's `TabletDto`, or `null` if not yet generated.
  - [ ] `retro_list_items { tablet_id }` — returns `Vec<ItemDto>`.
  - [ ] `retro_save_diary { tablet_id, body }` — upserts the diary row; returns the resulting tablet status.
  - [ ] `retro_patch_item { item_id, patch }` — proxies `ItemPatch` (`done_at`, `body`, `kind`).
- [ ] Each tool's handler calls into `CeremonyService` (in-process), not back through WS-RPC.
- [ ] JSON schemas declared so the LLM gets useful prompts.
- [ ] Unit tests in `arawn-tool` per tool using `MockLlmClient` + a `CeremonyService` backed by tempdir SQLite.
- [ ] Tools opted-in by default for the standard agent (gated by config flag if other ceremony surfaces are added later).

## Implementation Notes

### Technical Approach

1. Add a `ceremonies` submodule under `crates/arawn-tool/src/tools/`. Each tool is a `BoxedTool` impl with input schema + handler.
2. Tools take an `Arc<CeremonyService>` injected from the agent's tool-init plumbing (same pattern as how `workstream_show` gets its store).
3. Output is `serde_json::Value` — reuse the DTO types from `arawn-ceremonies` and serialize directly.
4. For `retro_run`, error variant `Skipped { reason }` becomes `{ status: "skipped", reason }` instead of an error — the agent should see "already generated" as a normal outcome.

### Dependencies

- Blocked by [[ARAWN-T-0292]] — needs `CeremonyService` instantiated in the binary so it can be injected at tool-init.
- Unblocks [[ARAWN-T-0294]] (UAT scenario).

### Risk Considerations

- **Citation-grounding hallucination**: The judge will be checking that items carry citation_ids that match seeded source rows. The tool surface returns `ItemDto.citation_id` verbatim from the DB; nothing to do here except ensure the field reaches the LLM intact.
- **Tool count bloat**: Five new tools is non-trivial. If this causes the agent to thrash on tool selection, consider collapsing into a single `retro` tool with an `action` parameter — but ship the five-tool version first since it matches the existing `workstream_*` granularity.

## Status Updates

### 2026-05-16 — five tools shipped

- Added `arawn-ceremonies` to `arawn-engine`'s `Cargo.toml`; `rusqlite` + `uuid`
  added as dev-deps for the test module.
- Added `ToolCategory::Ceremony` to `arawn-tool`; `query_engine.rs`
  activates the category on `retro`/`ceremony`/`standup`/`diary`
  mentions in the last user message so the agent only sees these
  tools when relevant.
- New `crates/arawn-engine/src/tools/ceremony.rs` with:
  - `RetroRunTool` (`retro_run`) — fires the dispatcher, returns
    `{ status: "generated", tablet_id }` or
    `{ status: "skipped", reason }`.
  - `RetroCurrentTool` (`retro_current`, read-only) — returns the
    current ISO week's `TabletDto` or `null`.
  - `RetroListItemsTool` (`retro_list_items`, read-only) — returns
    items with `citation_id` preserved; description explicitly
    instructs the agent to quote citation ids verbatim (judge
    grounding criterion for T-0294).
  - `RetroSaveDiaryTool` (`retro_save_diary`) — proxies
    `upsert_diary`.
  - `RetroPatchItemTool` (`retro_patch_item`) — proxies
    `patch_item` with `ItemPatch` deserialisation.
- All five exported from `arawn-engine::lib` and registered in
  `main.rs` right after `set_ceremony_service` (registry's interior
  mutability means late registration is fine).
- Eight unit tests covering validation paths, error mapping, and
  schema shape (`tools::ceremony::tests::*` — all green).

### Acceptance status

All five tools registered, JSON schemas declared, citation
forwarding verified. End-to-end coverage rolls into [[ARAWN-T-0294]].

Completed 2026-05-16.