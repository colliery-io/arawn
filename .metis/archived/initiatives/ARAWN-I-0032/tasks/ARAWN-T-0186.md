---
id: drop-llmresolver-trait-inline
level: task
title: "Drop LlmResolver trait — inline resolution into EngineToolContext"
short_code: "ARAWN-T-0186"
created_at: 2026-04-18T14:13:30.292244+00:00
updated_at: 2026-04-18T14:36:18.120775+00:00
parent: ARAWN-I-0032
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0032
---

# Drop LlmResolver trait — inline resolution into EngineToolContext

## Parent Initiative

[[ARAWN-I-0032]]

## Objective

`LlmResolver` (trait in `arawn-tool/src/llm_preference.rs`) has exactly one real impl (`LlmClientPool` in arawn-bin) plus one test mock (`TestResolver` in `arawn-engine/src/tools/agent.rs`). The abstraction doesn't earn its keep.

Drop the trait. `EngineToolContext` stops holding `Option<Arc<dyn LlmResolver>>` and instead stores a concrete resolver closure (or a direct `Arc<LlmClientPool>` if the circular-dep story stays clean). The test side converts `TestResolver` to an inline closure wrapper.

Estimated size: **S** (~1 day).

### Priority
- [x] P2 - Medium (cleanup)

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

- [ ] `pub trait LlmResolver` removed from `arawn-tool/src/llm_preference.rs`
- [ ] `EngineToolContext.llm_resolver` field and `with_llm_resolver()` builder either replaced with a concrete closure-based alternative or retargeted to a specific type (no `dyn LlmResolver`)
- [ ] `ToolContext::resolve_llm` default impl in arawn-tool either kept (returning `None`) or removed; if kept, its signature no longer references `LlmResolver`
- [ ] `LocalService` no longer coerces `Arc<LlmClientPool>` through `Arc<dyn LlmResolver>`
- [ ] `TestResolver` in agent.rs replaced with an inline closure or equivalent
- [ ] Agent tool's `sub_agent_uses_resolved_llm_preference` test still passes
- [ ] `cargo check --workspace` clean; `angreal test unit` green
- [ ] Single focused commit

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

- `LlmResolver` trait removed from `arawn-tool/src/llm_preference.rs`. Replaced with `pub type LlmResolverFn = dyn Fn(&LlmPreference) -> LlmResolution + Send + Sync;`.
- `EngineToolContext.llm_resolver: Option<Arc<dyn LlmResolver>>` → `Option<Arc<LlmResolverFn>>`. `with_llm_resolver` and `resolve_llm` updated (closure call instead of trait method).
- `LocalService::prepare_session_context` wraps the pool in a closure: `Arc::new(move |pref| pool.resolve(pref))`.
- `LlmClientPool`'s `impl LlmResolver` deleted; inherent `resolve` method unchanged.
- `TestResolver` struct + trait impl in agent.rs tests replaced with a `test_resolver()` helper returning `Arc<LlmResolverFn>`.
- Landed as single commit: 7 files changed, 40 insertions, 60 deletions.
- `cargo build --workspace` clean. All 11 agent tests pass. Full workspace test suite green.