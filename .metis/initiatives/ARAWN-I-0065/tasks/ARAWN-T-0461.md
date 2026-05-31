---
id: t-a-person-brief-tool-what-s-been
level: task
title: "T-A: person_brief tool — what's been going on with X (orchestrator)"
short_code: "ARAWN-T-0461"
created_at: 2026-05-31T15:44:53.262672+00:00
updated_at: 2026-05-31T16:04:56.574714+00:00
parent: ARAWN-I-0065
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0065
---

# T-A: person_brief tool — "what's been going on with X" orchestrator

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0065]]

## Objective

Build `person_brief` — the agent-facing tool that answers "what's been going on with X" by reading the I-0064 substrate: Person entity + PersonProfile sidecar + memory entries mentioning the person + graph relations. This is the highest-frequency exec ask and the most visible win from I-0064's org model.

## Scope decision (first cut)

Memory + sidecar + graph only. Calendar/Gmail/Slack fanout is **deliberately deferred** to a follow-up task — those need integration-typed `Arc<XIntegration>` references at construction time and inflate this task from M to L. The memory-only version is the single highest-leverage delta on day one.

## Status Updates

**2026-05-31 — shipped.**

- `crates/arawn-engine/src/tools/person_brief.rs` (new): `PersonBriefTool` with name + include_related params; resolves Person via exact-title (case-insensitive) then substring fallback; reads PersonProfile sidecar; FTS-searches global memory for entries mentioning the person; pulls graph-related entities. Markdown output with sections for identity line, cadence, growth themes, current concerns, notes, recent memory hits, connected entities. Helpful no-match message hints at the `memory_store(entity_type=person, ...)` capture flow.
- `crates/arawn-engine/src/tools/mod.rs` + `lib.rs`: module + re-export
- `crates/arawn/src/main.rs:484-491`: registered in the lens-routed memory-tools block
- 10 new tests covering: unknown person message, exact-match with profile, substring fallback, exact-match-wins-over-partial, growth/concerns rendering, no-sidecar hint, related memory hits, graph-relations on/off, missing-name error

Verification: `cargo test -p arawn-engine --lib tools::person_brief` — 10/10 pass; `angreal check workspace` — clean.

## Known follow-up (deferred)

- Calendar fanout — read recent events involving the person
- Gmail fanout — recent threads with the person
- Slack fanout — recent threads / DMs
- Each requires integration-typed Arc reference at tool construction; tracked as I-0065 follow-up

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