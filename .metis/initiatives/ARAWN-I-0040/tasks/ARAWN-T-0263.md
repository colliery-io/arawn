---
id: tag-ontology-schema-extract-split
level: task
title: "Tag ontology schema + Extract — split Entity.tags, workstream ontology storage, prompt rewrite"
short_code: "ARAWN-T-0263"
created_at: 2026-05-14T13:43:07.562907+00:00
updated_at: 2026-05-14T16:41:24.697624+00:00
parent: ARAWN-I-0040
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0040
---

# Tag ontology schema + Extract — split Entity.tags, workstream ontology storage, prompt rewrite

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0040]]

## Objective **[REQUIRED]**

{Clear statement of what this task accomplishes}

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [ ] Bug - Production issue that needs fixing
- [ ] Feature - New functionality or enhancement  
- [ ] Tech Debt - Code improvement or refactoring
- [ ] Chore - Maintenance or setup work

### Priority
- [ ] P0 - Critical (blocks users/revenue)
- [ ] P1 - High (important for user experience)
- [ ] P2 - Medium (nice to have)
- [ ] P3 - Low (when time permits)

### Impact Assessment **[CONDITIONAL: Bug]**
- **Affected Users**: {Number/percentage of users affected}
- **Reproduction Steps**: 
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected vs Actual**: {What should happen vs what happens}

### Business Justification **[CONDITIONAL: Feature]**
- **User Value**: {Why users need this}
- **Business Value**: {Impact on metrics/revenue}
- **Effort Estimate**: {Rough size - S/M/L/XL}

### Technical Debt Impact **[CONDITIONAL: Tech Debt]**
- **Current Problems**: {What's difficult/slow/buggy now}
- **Benefits of Fixing**: {What improves after refactoring}
- **Risk Assessment**: {Risks of not addressing this}

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] {Specific, testable requirement 1}
- [ ] {Specific, testable requirement 2}
- [ ] {Specific, testable requirement 3}

## Test Cases **[CONDITIONAL: Testing Task]**

{Delete unless this is a testing task}

### Test Case 1: {Test Case Name}
- **Test ID**: TC-001
- **Preconditions**: {What must be true before testing}
- **Steps**: 
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

### Test Case 2: {Test Case Name}
- **Test ID**: TC-002
- **Preconditions**: {What must be true before testing}
- **Steps**: 
  1. {Step 1}
  2. {Step 2}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

## Documentation Sections **[CONDITIONAL: Documentation Task]**

{Delete unless this is a documentation task}

### User Guide Content
- **Feature Description**: {What this feature does and why it's useful}
- **Prerequisites**: {What users need before using this feature}
- **Step-by-Step Instructions**:
  1. {Step 1 with screenshots/examples}
  2. {Step 2 with screenshots/examples}
  3. {Step 3 with screenshots/examples}

### Troubleshooting Guide
- **Common Issue 1**: {Problem description and solution}
- **Common Issue 2**: {Problem description and solution}
- **Error Messages**: {List of error messages and what they mean}

### API Documentation **[CONDITIONAL: API Documentation]**
- **Endpoint**: {API endpoint description}
- **Parameters**: {Required and optional parameters}
- **Example Request**: {Code example}
- **Example Response**: {Expected response format}

## Implementation Notes **[CONDITIONAL: Technical Task]**

{Keep for technical tasks, delete for non-technical. Technical details, approach, or important considerations}

### Technical Approach
{How this will be implemented}

### Dependencies
{Other tasks or systems this depends on}

### Risk Considerations
{Technical risks and mitigation strategies}

## Status Updates

### 2026-05-14 — Complete (with two UAT-surfaced bug fixes folded in)

**Schema (`arawn-memory`):**
- `Entity` gained `tags_ontology: Vec<String>` with `#[serde(default)]` for backwards-compatible deserialization. The existing `tags` field is the discovered (free-form) half; ADR-0004 calls out the rename-vs-keep tradeoff and we kept `tags` to avoid touching 80+ caller sites for what's purely a name.
- Builders: `with_tags`, `with_tags_discovered` (alias), `with_tags_ontology`.
- Cypher serialization (`entity_to_props`, `node_to_entity`) carries both arrays. `cypher_upsert_entity`'s SET / CREATE clauses include the new `tags_ontology` property.
- FTS5 indexing folds both tag arrays into the indexed `content` blob for recall via `signal_search`.

**Per-workstream ontology storage (`arawn-memory::ontology`):**
- `TagOntologyStore` sits next to the journal in `<data_dir>/workstreams/<name>/memory.db`. `open(data_dir, name)` and `open_at(ws_dir)` constructors.
- CRUD: `add(tag, via)` (idempotent, preserves first `added_via`), `add_many`, `remove`, `contains`, `list`, `tags`, `count`, `get`, `filter` (extractor-side validator).
- `normalize_tag` (lowercase + trim) at both write and read sites so `Falcon`, `falcon `, `FALCON` resolve to one tag.
- 8 inline tests; schema is idempotent on reopen.

**Workstream creation requires ontology (`arawn-engine::tools::workstream`):**
- `WorkstreamCreateTool` parameters: `name`, `description`, `tags_ontology` are all required. `display_name` optional. Non-empty ontology enforced; dedup + normalize before insert.
- Tool description references ADR-0004 + names `workstream_propose_ontology` as the agent-side helper (T-0264).
- 4 new tests covering happy path, missing description, empty ontology, dedup/normalize.

**Extract → filter against ontology (`arawn-extractor::cot`):**
- `ExtractedCandidate` now has `tags_ontology` + `tags_discovered`.
- Chain reads workstream ontology via `TagOntologyStore::open_at(&workstream.root_dir)` (soft-fails to empty), passes the ontology list in the user prompt, and prompts the LLM to emit two separate arrays.
- Write step filters LLM-emitted ontology tags against the declared list — anything not in the list is dropped silently. Discovered tags pass through normalized.

**Tool updates:**
- `signal_query`: filters on `tags_ontology` by default. New `include_discovered: bool` widens to the free-form recall set.
- `entity_summary` (signal output): returns `tags_ontology` and `tags_discovered` as separate fields.
- `workstream_dust` clusters on `tags_ontology` only (deterministic substrate). Summary entities now carry the cluster key in `tags_ontology` + the `steward:dust` marker on the discovered set.

**UAT fixture format:**
- `WorkstreamFixture` gained `tags_ontology: Vec<String>` (serde default for legacy fixtures). `uat_fixture::apply` opens `TagOntologyStore` and seeds the declared tags.
- `signal-extraction-e2e.json` carries 10-tag ontologies for `work` and `dnd`.

**Two UAT-surfaced pre-existing bug fixes (folded into this commit):**

1. **FTS migrate destructive drop (`arawn-memory::store::migrate`).** The migration code was unconditionally `DROP TABLE entities_fts` on every `MemoryStore::open`. Seed-side extraction populated FTS; server reopen wiped it; `signal_search` returned empty on every run. Removed the destructive drop — `CREATE TABLE IF NOT EXISTS` already handles re-open. Pre-existing bug from the T-0239 migration.

2. **Session workstream persistence (`arawn-engine::tools::workstream::WorkstreamSwitchTool` + `arawn-storage::Store::update_session_workstream_name`).** `LocalService` auto-restores `SessionWorkstream` from `meta.workstream_name` on every session-load (turn boundary on WS). `WorkstreamSwitchTool` only updated the in-memory shim, so every switch reverted to scratch on the next turn. Added `Store::update_session_workstream_name` (delegates to `SessionStore::update_workstream_name` which already existed) and the switch tool now writes to it after updating the shim. Verified by UAT 2026-05-14 14:42: turn 1's switch was the only one that took effect; all subsequent turn-2-onward queries routed to scratch and returned empty.

**Validation:**
- `cargo test --workspace` serial: all crates green (gemma4:31b's graphqlite-using crates flake under parallel test execution; serial passes consistently).
- `angreal check clippy`: exit 0.
- UAT run 2026-05-14 13:43 (post-T-0263 only): tag emission validated — all 3 falcon entities tagged `falcon`, all postgres entities tagged `postgres`+`ledger`, dnd produced 5 entities tagged `calidor` (was 0 in pre-T-0263 runs).
- UAT run 2026-05-14 14:42 (with both bug fixes): FTS now persists across reopen (17 indexed rows, was 0). Turn-1 signal_search returns 6 hits and the agent correctly quotes RFC-0042. The session-persistence fix is staged but not yet UAT-validated — next run will exercise it.

**Out of scope for this task (T-0264 territory):**
- Agent doesn't yet know the workstream's ontology, so it guesses tags (e.g. `falcon-project` vs `falcon`). Surfacing the ontology via `workstream_show` and/or in `workstream_dust`'s empty-cluster response is T-0264's UX layer.
- `workstream_propose_ontology` tool and the `/workstream-create` agent skill are T-0264.

**Documents created alongside this task:**
- ADR-0004 (decided): codifies the ontology-required bet + the Extract→Suggest→Add cycle as the replacement for I-0040's failed "drift toward stability" mechanism.
- T-0264, T-0265, T-0266: stubs for the rest of the cycle (workstream create UX, tag-promoter Suggest, accept-path Add).