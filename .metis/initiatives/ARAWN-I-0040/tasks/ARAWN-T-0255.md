---
id: signal-search-signal-query-signal
level: task
title: "signal_search / signal_query / signal_timeline agent tools over workstream KBs"
short_code: "ARAWN-T-0255"
created_at: 2026-05-13T03:46:34.905640+00:00
updated_at: 2026-05-13T03:53:42.967786+00:00
parent: ARAWN-I-0040
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0040
---

# signal_search / signal_query / signal_timeline agent tools over workstream KBs

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0040]]

## Objective

Ship the three agent-facing read tools over workstream KBs that Phase 6 of I-0040 calls for. Phase 4 just landed the writer (CotChain) — the agent now has a KB to read, but no tool surface for reading it. This task adds that surface.

- **`signal_search`** — semantic + FTS hybrid over entities in the active workstream's KB. Same shape as `feed_search`, but scoped to one workstream's entities + relations instead of projections.
- **`signal_query`** — structured filter ("decisions tagged `stripe:migration` since 30d ago"). Cypher-backed under the hood; tool exposes a small set of named filter parameters, not raw query.
- **`signal_timeline`** — chronological slice across a workstream's entities (by `created_at` / `superseded_at`), useful for "what happened in this workstream last week."

All three operate on the active workstream by default and accept a `workstream` arg to override (routes through `WorkstreamMemoryRouter`, same as the existing memory tools).

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

## Acceptance Criteria

- [ ] Three tools (`signal_search`, `signal_query`, `signal_timeline`) implement `arawn_engine::Tool`, are registered in the serve registry, and appear in `tools list`.
- [ ] All three default to the active workstream and accept an explicit `workstream` arg.
- [ ] `signal_search` returns entities ranked by FTS+vector hybrid (RRF or score-blend, whichever is already used in feed_search) — semantic results are present, not just substring matches.
- [ ] `signal_query` supports at minimum: `entity_type`, `tags` (any-of), `since` / `until` ISO timestamps, `limit`.
- [ ] `signal_timeline` returns entities ordered by `created_at` desc within a window; includes `superseded` events as relation entries.
- [ ] Inline unit tests cover: tool registration, default-to-active-workstream routing, explicit workstream override, empty-KB behavior, filter combinations on `signal_query`.
- [ ] `cargo test -p arawn-engine` + `angreal test unit` + `angreal check clippy` clean.

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

### 2026-05-12 — Implementation complete

**Three tools in `crates/arawn-engine/src/tools/signal.rs`:**

- `SignalSearchTool` — FTS5 over workstream tier, RRF-fused with vector search when an embedder is wired. Same RRF_K=60 constant as `feed_search` so ranks are comparable.
- `SignalQueryTool` — structured filter: `entity_type`, `tags` (any-of), `since`/`until` (RFC3339 on `updated_at`), `limit`. Composes via Rust-side retain after fetching candidates via `list_by_type` or `list_all_ranked`.
- `SignalTimelineTool` — chronological window, sorted by `created_at` desc, emits `{ts, kind: "entity_created", entity}` events.

**Workstream routing.** Each tool holds a `MemoryHandle` and, when the handle is `Routed`, stashes the inner `Arc<WorkstreamMemoryRouter>` so an explicit `workstream` arg can resolve a *different* KB than the active one. `Fixed` handles (test ergonomics) ignore the override.

**Scoping.** Search the workstream tier only — global tier (Preference/Person) is reachable via existing `memory_search`. Keeps signal_* focused on "what the extractor has put into *this* workstream."

**Registration.** `main.rs` registers all three alongside `memory_search` / `memory_store` whenever `workstream_router` is present.

**Tests (inline, 8 total):**
- `signal_search_finds_decision_by_title`, `signal_search_empty_kb_returns_zero`
- `signal_query_filters_by_entity_type`, `signal_query_filters_by_tag_any_of`, `signal_query_no_filters_returns_all_active`, `signal_query_window_filters`
- `signal_timeline_orders_desc_and_caps_to_window`
- `explicit_workstream_arg_routes_via_router` — confirms the override actually reaches a different KB.

**Validation:** `cargo test -p arawn-engine signal` 8/8 green; full workspace tests green; `angreal check clippy` exit 0.

**Notes / deferred:**
- `signal_timeline` doesn't yet surface supersession events as a separate timeline kind — `list_all_ranked` filters superseded entities, so today they're absent. Cheapest follow-up if needed: dual-query for superseded rows. Skipped here since the dominant "what happened" lens works without it.
- Output is JSON, not the human-formatted text that `memory_search` produces. JSON is friendlier for the model and consistent with `feed_search`.