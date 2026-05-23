---
id: wire-llmpreference-into-tool-trait
level: task
title: "Wire LlmPreference into Tool trait, ToolContext, and agent spawn"
short_code: "ARAWN-T-0177"
created_at: 2026-04-16T20:21:45.397123+00:00
updated_at: 2026-04-17T02:40:54.321026+00:00
parent: ARAWN-I-0027
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0027
---

# Wire LlmPreference into Tool trait, ToolContext, and agent spawn

## Parent Initiative

[[ARAWN-I-0027]]

## Objective

Plumb `LlmPreference` through the execution path so tools and agents can request specific models at runtime with graceful fallback.

- Add a default method `fn llm_preference(&self) -> Option<LlmPreference> { None }` to the `Tool` trait.
- Add `fn resolve_llm(&self, pref: &LlmPreference) -> LlmResolution` to the `ToolContext` trait. `EngineToolContext` delegates to the pool from ARAWN-T-0175.
- The engine calls `tool.llm_preference()` before invoking `execute`; if `Some`, it resolves and makes the selected `Arc<dyn LlmClient>` available to the tool via context helpers (e.g., a new `preferred_llm()` accessor).
- Agent spawn (sub-conversations from the Agent tool) accepts an optional `LlmPreference`. The spawned agent runs on the resolved client — which may differ from the parent agent's.
- Tools can inspect `MatchQuality` to degrade behaviour (e.g., skip summarization if only `Fallback`).

Estimated size: **M** (3–4 days). Touches `arawn-tool` trait, `arawn-engine` context + engine loop + agent tool.

### Priority
- [x] P2 - Medium (completes the per-component routing story)

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

- [ ] `Tool` trait gains `fn llm_preference(&self) -> Option<LlmPreference> { None }` (default)
- [ ] `ToolContext` trait gains `fn resolve_llm(&self, pref: &LlmPreference) -> LlmResolution`
- [ ] `EngineToolContext::resolve_llm` delegates to the `LlmClientPool` captured at construction time
- [ ] The engine resolves a tool's preference before calling `execute`; the resolved client is accessible through the context (e.g., `ctx.preferred_llm() -> Option<&Arc<dyn LlmClient>>`)
- [ ] The Agent tool's spawn path accepts an optional `LlmPreference` and runs the sub-conversation on the resolved client
- [ ] Integration test: tool declaring a satisfiable preference receives `MatchQuality::Exact` and the requested client
- [ ] Integration test: tool declaring an unsatisfiable preference receives `MatchQuality::Fallback` and the engine client
- [ ] Integration test: agent spawned with a preference runs on the resolved client (observable via the resolved `LlmConfig.model`)
- [ ] All existing tool/agent tests still pass
- [ ] Depends on ARAWN-T-0176

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

- Added `Tool::llm_preference(&self) -> Option<LlmPreference>` (default `None`) to the `Tool` trait.
- Added `LlmResolver` trait in `arawn-tool` so engine code can hold a resolver without depending on `arawn-bin`. `LlmClientPool` implements it (`impl LlmResolver for LlmClientPool`).
- Added `ToolContext::resolve_llm(&LlmPreference) -> Option<LlmResolution>` (default `None`) to the trait.
- `EngineToolContext` gained an `llm_resolver: Option<Arc<dyn LlmResolver>>` field, a `with_llm_resolver(...)` builder, and an override for `resolve_llm` that delegates to the attached resolver.
- `LocalService::prepare_session_context` now attaches the pool as resolver: `let resolver: Arc<dyn LlmResolver> = pool; ctx.with_llm_resolver(resolver)`.
- **Design deviation from initiative spec**: dropped engine-side pre-resolution and the `preferred_llm()` accessor. The original "engine resolves before execute" required either per-call context cloning or interior mutability, both awkward given the existing `Tool::execute(&dyn ToolContext)` signature and concurrent read-only tool execution. Cleaner pattern: tools call `ctx.resolve_llm(&self.llm_preference().unwrap_or_default())` themselves inside `execute()` when they care. The `Tool::llm_preference()` declaration remains for discoverability/future telemetry.
- Wired `LlmPreference` into the Agent tool's spawn path: new optional `llm` JSON parameter (named pool entry). When supplied and resolved with non-`Fallback` quality, the sub-agent runs on the resolved client + model. Existing definition.model + parent_model fallback chain still works.
- Two new integration tests in `tools::agent::tests`:
  - `sub_agent_uses_resolved_llm_preference` — wires a `TestResolver` mapping `"cheap"` to a separate mock; spawns sub-agent with `llm: "cheap"`; asserts the parent mock is never called and the cheap mock returns the response.
  - `sub_agent_falls_back_to_parent_llm_when_resolution_unavailable` — no resolver attached; agent tool falls back to ctx.llm() (parent).
- All 11 agent tests pass; full workspace `angreal test unit` is green.