---
id: llmclientpool-eager-pool-of-named
level: task
title: "LlmClientPool — eager pool of named LLM clients with fail-fast startup"
short_code: "ARAWN-T-0175"
created_at: 2026-04-16T20:21:42.953533+00:00
updated_at: 2026-04-17T02:17:15.425669+00:00
parent: ARAWN-I-0027
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0027
---

# LlmClientPool — eager pool of named LLM clients with fail-fast startup

## Parent Initiative

[[ARAWN-I-0027]]

## Objective

Introduce `LlmClientPool` as the single authority for resolving named LLM clients at runtime. Replace the per-component `build_llm_client()` calls established in ARAWN-T-0174 with pool access.

The pool is constructed once in `main.rs` from `&ArawnConfig`, eagerly builds an `Arc<dyn LlmClient>` for every `[llm.*]` entry (wrapped in `RetryClient`), and stores them in a `HashMap<String, Arc<dyn LlmClient>>`. Startup is fail-fast: a missing API key or an unreachable provider surfaces at launch, not mid-session.

Lives in `crates/arawn/src/llm_pool.rs` (binary crate) because it depends on `build_llm_client` and config types.

Estimated size: **M** (2–3 days).

### Priority
- [x] P2 - Medium (foundation for per-component routing)

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

- [ ] New file `crates/arawn/src/llm_pool.rs` with `LlmClientPool` struct
- [ ] `LlmClientPool::from_config(&ArawnConfig)` eagerly constructs an `Arc<dyn LlmClient>` for every `[llm.*]` entry, each wrapped in `RetryClient`
- [ ] Accessors: `get(name: &str) -> Option<Arc<dyn LlmClient>>`, `engine() -> Arc<dyn LlmClient>`, `compactor() -> Arc<dyn LlmClient>` (last two follow config-driven fallback)
- [ ] `main.rs` constructs the pool once; `LocalService::new` accepts `&LlmClientPool` (or an `Arc<LlmClientPool>`) instead of individual clients
- [ ] Startup is fail-fast: a misconfigured `[llm.*]` entry (bad API key format, missing env var) surfaces a clear error at launch
- [ ] Integration test: a config with two named `[llm.*]` entries produces a pool with two distinct clients addressable by name
- [ ] Integration test: a config with one bad entry fails pool construction
- [ ] Depends on ARAWN-T-0174 (must have compactor-client plumbing in place first)

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

- Created `crates/arawn/src/llm_pool.rs` with `LlmClientPool`. `from_config(&ArawnConfig, F: Fn(&LlmConfig) -> Result<Arc<dyn LlmClient>>)` eagerly builds every `[llm.*]` entry, wraps each in `RetryClient`, and resolves engine/compactor names with the same fallback semantics as `ArawnConfig::engine_llm` / `compactor_llm`.
- Accessors: `get(name)`, `engine()`, `compactor()`, `engine_config()`, `compactor_config()`, `engine_name()`, `compactor_name()`, `entries()`, `len()`, plus convenience constructors `from_clients(...)` and `single(...)` for tests/one-off use.
- Errors are anyhow-wrapped with the offending entry name in the message — fail-fast at startup.
- `LocalService` refactored: stores `Arc<LlmClientPool>` instead of separate engine/compactor Arcs. `LocalService::new` signature simplified to `(store, data_dir, llm_pool, registry, config)`. `shared_llm`, `shared_compactor_llm`, `compactor_model` accessors now delegate to the pool. New `shared_llm_pool()` accessor exposes the pool.
- `main.rs` builds the pool once with `LlmClientPool::from_config(&config, |cfg| build_llm_client(cfg))?` and passes it into `LocalService::new`. Logs entry count + resolved engine/compactor names at startup.
- Test fixtures in `arawn-tests` (local_service.rs, websocket.rs) updated to use `LlmClientPool::single(...)` and `from_clients(...)`.
- 5 unit tests in llm_pool cover: build every entry; distinct clients when configured; fallback when compactor unconfigured; fallback when pointing at missing entry; fail-fast when any builder errors. All workspace tests still pass.