---
id: llmpreference-llmcapabilities
level: task
title: "LlmPreference, LlmCapabilities, LlmResolution — types and pool resolution logic"
short_code: "ARAWN-T-0176"
created_at: 2026-04-16T20:21:43.822229+00:00
updated_at: 2026-04-17T02:32:02.664767+00:00
parent: ARAWN-I-0027
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0027
---

# LlmPreference, LlmCapabilities, LlmResolution — types and pool resolution logic

## Parent Initiative

[[ARAWN-I-0027]]

## Objective

Define the preference/resolution vocabulary that tools and agents use to request specific LLMs, and implement the resolution algorithm on `LlmClientPool`.

Types live in the `arawn-tool` crate so tools can depend on them without pulling in the binary crate:
- `LlmPreference { named: Option<String>, provider: Option<String>, model: Option<String>, capabilities: LlmCapabilities }`
- `LlmCapabilities { min_context_window: Option<u32>, tool_use: bool, vision: bool }`
- `LlmResolution { client: Arc<dyn LlmClient>, config: LlmConfig, match_quality: MatchQuality }`
- `MatchQuality { Exact, Capability, Fallback }`

`LlmClientPool::resolve(&self, &LlmPreference) -> LlmResolution` implements the resolution order:
1. **Named match** — `preference.named` present and in pool → `Exact`
2. **Provider+model match** — scan for exact provider+model → `Exact`
3. **Capability match** — first pool entry satisfying all capability bounds → `Capability`
4. **Fallback** — engine default LLM → `Fallback`

Capabilities are read from `LlmConfig` (not added to the `LlmClient` trait) — this task does not modify the `LlmClient` trait surface.

Estimated size: **M** (2 days).

### Priority
- [x] P2 - Medium (enables per-tool model selection)

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

- [ ] `LlmPreference`, `LlmCapabilities`, `LlmResolution`, `MatchQuality` defined in `crates/arawn-tool/src/llm_preference.rs` (or similar) and re-exported from the crate root
- [ ] `LlmClientPool::resolve(&LlmPreference) -> LlmResolution` implemented in `crates/arawn/src/llm_pool.rs`
- [ ] Resolution order matches spec: named → provider+model → capability → fallback
- [ ] `MatchQuality` is set correctly for each path (`Exact` for named/provider+model, `Capability` for capability-only match, `Fallback` for default)
- [ ] Unit tests cover every resolution path, including: empty pool → fallback, named mismatch → fallback, capability too strict → fallback, provider-only preference → capability match
- [ ] No changes to the `LlmClient` trait surface — capabilities are read from `LlmConfig`
- [ ] Depends on ARAWN-T-0175 (pool must exist)

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

- Created `crates/arawn-tool/src/llm_preference.rs` with `LlmPreference`, `LlmCapabilities`, `LlmResolution`, `MatchQuality`, `ResolvedLlmInfo`. Re-exported from crate root. Includes constructors `LlmPreference::any()`, `::named(...)`, `::provider_model(...)` and `LlmCapabilities::satisfied_by(&info)`.
- Extended `LlmConfig` with `tool_use: bool` (default true) and `vision: bool` (default false). Added `LlmConfig::to_resolved_info()` mapper to project into the capability-bearing tool-side type. The `LlmClient` trait surface is unchanged.
- Added `arawn-tool` to `arawn-bin`'s Cargo dependencies.
- Implemented `LlmClientPool::resolve(&LlmPreference) -> LlmResolution` following spec order: named exact → provider+model exact → capability (also handles provider-only / model-only / capabilities-only) → engine fallback.
- 7 new unit tests in llm_pool cover every resolution path; 5 unit tests in arawn-tool cover capability satisfaction logic + preference constructors.
- Decision: `LlmResolution.config: LlmConfig` from the initiative spec became `LlmResolution.info: ResolvedLlmInfo` so the type can live in `arawn-tool` without depending on `arawn-bin`'s `LlmConfig`. Conversion via `LlmConfig::to_resolved_info()`.
- All 28 arawn lib tests pass.