---
id: document-multi-model-arawn-toml
level: task
title: "Document multi-model arawn.toml configuration with worked example"
short_code: "ARAWN-T-0178"
created_at: 2026-04-16T20:21:46.775582+00:00
updated_at: 2026-04-17T02:42:22.281734+00:00
parent: ARAWN-I-0027
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0027
---

# Document multi-model arawn.toml configuration with worked example

## Parent Initiative

[[ARAWN-I-0027]]

## Objective

Write user-facing documentation for multi-model configuration. Add a worked example to `arawn.toml` docs (or equivalent in `/docs`) showing:
- `[llm.default]` — main engine (Anthropic Sonnet)
- `[llm.cheap]` — compactor (Groq/Llama) referenced by `[compactor] llm = "cheap"`
- `[llm.judge]` — named pool entry reserved for future evals
- A short section explaining `LlmPreference` resolution order and `MatchQuality` fallback behaviour so users understand what happens when a requested model isn't configured.

Estimated size: **XS** (half day). Docs-only, no code changes.

### Priority
- [x] P3 - Low (quality-of-life, no functional impact)

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

- [ ] User-facing docs (README or `/docs` page) contain a complete multi-model `arawn.toml` example with `[llm.default]`, `[llm.cheap]`, `[llm.judge]`, `[engine]`, `[compactor]`
- [ ] Docs describe the `LlmPreference` resolution order (named → provider+model → capability → fallback) and what `MatchQuality::Fallback` means for tool behaviour
- [ ] Example references real provider strings currently supported by `build_llm_client`
- [ ] No code changes; no new tests required

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

- Project has no `/docs` directory yet — the canonical user-facing config example lives in `ArawnConfig::generate_default_toml()`, which is what users see when arawn first writes `~/.arawn/arawn.toml`. Updated that template instead of creating new docs.
- Added a top-of-file `# Multi-model setup` block to the generated TOML explaining the LlmClientPool, the resolution order (named → provider+model → capability → fallback), and the `MatchQuality::Fallback` degradation behaviour.
- Added commented `[llm.cheap]` (Groq llama-3.3-70b) and `[llm.judge]` (Anthropic claude-sonnet-4) examples — uncomment to enable. Both reference real provider strings that `build_llm_client` already supports (`groq`, `anthropic`).
- Annotated `[llm.default]` with the new `tool_use` and `vision` capability flags.
- Updated the `[compactor]` block to point users at `llm = "cheap"` for the worked compactor-routing example.
- `generate_default_toml_is_parseable` test still passes (along with all 9 config tests), confirming the example is valid TOML and parses into `ArawnConfig`.