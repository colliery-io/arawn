---
id: uat-scenario-for-tag-promoter
level: task
title: "UAT scenario for tag-promoter Extract→Suggest→Add cycle"
short_code: "ARAWN-T-0268"
created_at: 2026-05-14T20:48:26.287341+00:00
updated_at: 2026-05-14T20:59:59.163505+00:00
parent: ARAWN-I-0040
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: ARAWN-I-0040
---

# UAT scenario for tag-promoter Extract→Suggest→Add cycle

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

### 2026-05-14 — Scaffolding complete; awaiting UAT run

**Seed-side tag-promoter driver (`uat_fixture::drive_tag_promoter`).**
After `drive_extraction` populates entities (and therefore `tags_discovered`),
this helper instantiates `TagPromoterSubroutine::default()` and runs it
against each seeded workstream via a normal `SubroutineCtx`. Pure-stats —
no LLM cost. Proposals land in the workstream journal as `applied=false`
rows ready for the agent to discover via `workstream_refine`.

The harness in `uat.rs` now calls `drive_tag_promoter` immediately after
`drive_extraction` on every seeded scenario; it's a no-op when no
recurring discovered tags exist.

**`arawn-tests` Cargo.toml** gained an `arawn-steward` path dep so the
seed helper can reach `TagPromoterSubroutine` + `Journal` + `JournalGate`
+ `SubroutineCtx`.

**New scenario `tag-promoter-cycle`** (`tests/uat.rs`):
- Reuses the existing `signal-extraction-e2e.json` fixture (no new
  fixture file needed — the same 26-row seed produces enough recurring
  discovered tags for tag-promoter to find candidates).
- 4 turns:
  1. `workstream_switch work` + `workstream_show` — agent lists the seeded ontology.
  2. `workstream_refine` — agent surfaces pending tag-promoter proposals.
  3. `workstream_apply <id>` — agent commits one promotion; verifies via show/list.
  4. `workstream_rollback <id>` — agent undoes; verifies the tag is gone again.
- Mechanical: `min_memory_entities=4`, `max_tool_errors=2`.
- Excluded from default UAT runs unless `UAT_SCENARIO=tag-promoter-cycle` is set.

**Verification:**
- `cargo build -p arawn-tests --tests` clean.
- `cargo test -p arawn-tests --test uat_fixture --test uat_fixture_smoke` — 7/7.
- `angreal check clippy` exit 0.

**Next step (user-side):**
```
UAT_SCENARIO=tag-promoter-cycle angreal test uat
angreal test uat-judge --results /tmp/arawn-uat-<ts>
```

Findings from that run will land here. Until then this task stays
"active" so it shows up on actionable-work lists.