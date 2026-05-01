---
id: wire-compactor-to-compactor-llm
level: task
title: "Wire compactor to compactor_llm() — build separate client, remove dead-code gap"
short_code: "ARAWN-T-0174"
created_at: 2026-04-16T20:21:40.778765+00:00
updated_at: 2026-04-17T02:12:51.788660+00:00
parent: ARAWN-I-0027
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0027
---

# Wire compactor to compactor_llm() — build separate client, remove dead-code gap

## Parent Initiative

[[ARAWN-I-0027]]

## Objective

Today `main.rs` calls `build_llm_client()` once against the engine config and passes that single `Arc<dyn LlmClient>` to both the engine and the compactor (`local_service.rs:249`). The `compactor_llm()` resolution method in `config.rs` is dead code — changing `[compactor]` in `arawn.toml` has no effect.

Build a second `Arc<dyn LlmClient>` from `config.compactor_llm()` and thread it separately through `LocalService::new` → `build_engine` → `Compactor::new`. When `[compactor]` is absent the resolution already falls back to the engine config, so behaviour is unchanged for existing single-model configs.

Estimated size: **S** (1–2 days). Targeted fix, mostly plumbing.

### Priority
- [x] P2 - Medium (unblocks cheaper-compactor configs)

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

## Acceptance Criteria

- [ ] `main.rs` builds two distinct `Arc<dyn LlmClient>` values — one from `config.engine_llm()`, one from `config.compactor_llm()`
- [ ] `LocalService::new` accepts both clients and threads the compactor client into `build_engine` → `Compactor::new`
- [ ] `LocalService::shared_llm()` continues to return the engine client
- [ ] When `[compactor]` is absent from config, compactor client equals engine client (existing behaviour preserved)
- [ ] Unit/integration test: two different `[llm.*]` entries referenced by `[engine]` and `[compactor]` produce clients whose configs differ (by model name)
- [ ] All existing tests still pass

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

- `LocalService` gained `compactor_llm: Arc<dyn LlmClient>` and `compactor_model: String` fields. `LocalService::new` now requires both; `build_engine` constructs `Compactor::new(self.compactor_llm.clone(), self.compactor_model.clone())`.
- `main.rs` builds a separate `RetryClient`-wrapped `Arc<dyn LlmClient>` from `config.compactor_llm()` and passes it (plus the model name) into `LocalService::new`. Logs separately when the compactor model differs from engine.
- New accessors `shared_compactor_llm()` and `compactor_model()` exposed on `LocalService`.
- Updated test fixtures in `arawn-tests` (local_service.rs, websocket.rs) to pass both clients.
- Added `separate_engine_and_compactor_llms_are_stored_distinctly` test verifying distinct Arcs and distinct model strings.
- Existing `compactor_uses_own_llm_when_specified` config test continues to pass — the resolution layer + LocalService wiring now both honour `[compactor].llm`.
- Workspace builds clean; arawn-tests local_service (16 tests) and arawn config tests (9 tests) all green.