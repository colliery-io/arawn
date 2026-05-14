---
id: workstream-create-ux-ontology
level: task
title: "Workstream create UX — ontology required, propose tool, agent skill playbook"
short_code: "ARAWN-T-0264"
created_at: 2026-05-14T13:43:14.201211+00:00
updated_at: 2026-05-14T18:02:48.650532+00:00
parent: ARAWN-I-0040
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0040
---

# Workstream create UX — ontology required, propose tool, agent skill playbook

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

### 2026-05-14 — Complete

Four deliverables shipped, all driven by the UAT 17:27 finding that the agent guessed `falcon-project` instead of the actual ontology tag `falcon` and didn't self-correct:

**1. `workstream_show` surfaces the ontology.**
- Added `tags_ontology: [...]` to the JSON payload. Description updated to call out the use case: "use this before `workstream_dust` or `signal_query` with a tag filter so you pick a tag that actually exists."
- Soft-fails to empty when `ctx.data_dir()` is missing — keeps tests that don't seed an ontology working.
- Inline test `show_includes_ontology`.

**2. `workstream_dust` gives self-recovery hints on zero clusters.**
- When `outcome.clusters_found == 0`, the response now also includes:
  - `available_tags` — the workstream's full declared ontology, so the agent can spot the right string.
  - `suggestions` — context-aware tips (drop the `tags` filter; lower `min_cluster_size`; lower `idle_days`).
  - `hint` — one-line summary telling the agent how to retry.
- No structural change when clusters were found.

**3. `workstream_propose_ontology` tool.**
- New LLM-backed tool: takes a description, returns `{ tags: [...], rationale: "..." }`.
- System prompt is explicit about slug format (`lowercase-with-dashes`), encourages 5–12 tags, and emphasizes specific identifiers over generic categories — the lesson from the failed "drift toward stability" bet.
- Tags returned by the LLM are normalized + deduped before being handed back to the agent.
- Embedded a tiny `propose_llm_call` helper + `extract_json_block` inline. There are now four consumers of this pattern (extractor / steward / engine / engine again); consolidating into `arawn-llm` is a separate cleanup. Comment marks the intent.

**4. `workstream-create` agent skill.**
- Markdown playbook at `crates/arawn-engine/src/skills/builtin/workstream-create.md`, registered in the builtin skill list.
- Walks the three phases: ask for description → propose+confirm → confirm name+create.
- Triggers on `/workstream create`, "make a workstream for...", etc.
- Documents related tools and the v2 growth path via tag-promoter.

**Wiring (`main.rs`):**
- `WorkstreamProposeOntologyTool` registered with `llm_pool.engine()` + the configured model alongside the rest of the workstream tools.

**Tests:**
- 1 new in workstream.rs (`show_includes_ontology`). Existing 23 workstream tests + 7 steward tool tests all pass; clippy exit 0.

**Out of scope (T-0265 / T-0266):**
- `tag-promoter` steward subroutine (Suggest stage of the cycle).
- `workstream_apply` dispatch for `tag-promoter` proposals + `workstream_tag list/add/remove` (Add stage).