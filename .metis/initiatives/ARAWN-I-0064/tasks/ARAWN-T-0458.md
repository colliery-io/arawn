---
id: t-c-memory-store-inference-upgrade
level: task
title: "T-C: memory_store inference upgrade — phrasing-aware Person + relation capture"
short_code: "ARAWN-T-0458"
created_at: 2026-05-30T20:36:36.154236+00:00
updated_at: 2026-05-30T21:19:06.119432+00:00
parent: ARAWN-I-0064
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0064
---

# T-C: memory_store inference upgrade — phrasing-aware Person + relation capture

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0064]]

## Objective

LLM classifier turns natural-language person memories into structured `PersonProfile` rows + typed graph edges. *"Sarah is someone I manage"* → `Person(Sarah)` + `PersonProfile{relation_to_user=Manages}`. *"Marcus reports to Sarah"* → two `Person` entities + `Manages` + `ReportsTo` graph edges (and the misleading single entity with title="Marcus reports to Sarah" is NOT created).

## Scope decision

Limited to the agent-facing `memory_store` tool path. `infer_entity_type` in `local_service/memory.rs` (`/remember` slash-command path) is NOT touched in this task — that's a follow-up for I-0065 alongside any service-layer LLM plumbing. Filed as known follow-up.

## Status Updates

**2026-05-30 — shipped.**

New code:
- `crates/arawn-engine/src/person_intent.rs` (~190 LOC, new): `PersonIntent` + `BetweenPeople` structs (serde-derive), `classify_person_intent` async fn, lenient JSON parser that strips ```json fences, fallback-on-error so a classifier blip never blocks memory writes. 6 unit tests for parse behavior.
- `crates/arawn-engine/src/lib.rs`: module + re-export.
- `crates/arawn-engine/src/tools/memory_store.rs`:
  - `MemoryStoreTool` gains optional `classifier_llm` + `classifier_model` fields and a `with_classifier()` builder
  - `maybe_classify` helper: short-circuits unless `entity_type=Person` AND classifier configured AND text yields a non-empty intent
  - `write_between_people_relation` helper: ensures both Person entities exist (search-before-create via `store_fact`), writes `Manages` and `ReportsTo` graph edges, returns a clear "Sarah manages Marcus" success message
  - `upsert_relation_to_user` helper: merges into existing `PersonProfile` so an earlier role/concerns row isn't clobbered
  - `ensure_person` helper: search-before-create stub for a named Person
  - 5 new integration tests using inline `CannedIntentLlm` mock (mirrors `query_engine::tests::MockLlm` pattern)
- `crates/arawn/src/main.rs`: `MemoryStoreTool` construction now chains `.with_classifier(llm_pool.engine(), llm_pool.engine_config().model.clone())`

Verification:
- `angreal check workspace` — clean
- `cargo test -p arawn-engine --lib tools::memory_store` — 10/10 pass (5 new T-C)
- `cargo test -p arawn-engine --lib person_intent::tests` — 6/6 pass
- Pre-existing test parallelism flake (`harness_shell_tool_receives_arguments`) noted; passes in isolation, not affected by T-C changes.

Behavior in production:
- Every `memory_store(entity_type=person)` call adds ~200-500ms latency for the classifier round-trip. The user chose this tradeoff over rule-based when given the choice.
- Classifier failures are non-blocking — memory write completes with the Person entity stored as-is.
- Tool output now appends `[relation_to_user=manages]` (or similar) to make the structural write visible to the agent + user.

## Known follow-up (deferred)

- `infer_entity_type` (regex-based) still drives `/remember`. Upgrading it to use the classifier requires threading an `LlmClient` through `LocalService::remember_fact_inner` — better landed alongside other service-layer changes in I-0065.

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

## Status Updates **[REQUIRED]**

*To be added during implementation*