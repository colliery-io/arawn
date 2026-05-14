---
id: tag-promotion-accept-path
level: task
title: "Tag promotion accept path + workstream_tag manual management tool (Add)"
short_code: "ARAWN-T-0266"
created_at: 2026-05-14T13:43:31.717462+00:00
updated_at: 2026-05-14T20:42:43.808730+00:00
parent: ARAWN-I-0040
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0040
---

# Tag promotion accept path + workstream_tag manual management tool (Add)

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

Closes the Add half of the Extract→Suggest→Add cycle from ADR-0004.

**Accept-path dispatch (`arawn-steward::accept`).**
- Introduced `AcceptCtx { kb, workstream_root }` — the previous `(row, kb)` signature didn't carry the workstream root, but tag promotion writes to `workstream_tag_ontology` which lives at the workstream's KB dir.
- New arm `(tag-promoter, promote_tag)` — opens `TagOntologyStore::open_at(workstream_root)` and inserts the tag with `added_via = AddedVia::Promotion`. Idempotent at the storage layer (re-applying preserves first-`via`).
- All existing accept-path arms (`dust/summarize`, `map/propose_relation`, `doorwatch/propose_identity`, reshelve/identity no-ops) ported to the new ctx.
- Test `tag_promoter_apply_adds_to_ontology` pins the round-trip.

**Rollback-path dispatch (`arawn-steward::rollback`).**
- Mirrored `AcceptCtx` → `RollbackCtx { kb, workstream_root }`.
- New arm `(tag-promoter, promote_tag)` — opens the ontology store and removes the tag. Returns `Ok` whether the row existed or not (idempotent revert).
- Test `tag_promoter_inverse_removes_from_ontology` pins it.

**`workstream_tag` agent tool (`arawn-engine`).** Manual CRUD outside the propose/accept cycle:
- `op: "list"` → returns every tag with `added_via` provenance + count.
- `op: "add"` → inserts a tag (normalized + idempotent; `added_via = Manual`).
- `op: "remove"` → deletes a tag; reports `removed` vs `not_found`.
- Optional `workstream` override; defaults to active.
- Registered alongside the rest of the workstream tools in `main.rs`.

**Engine-side tool wiring updated.** `WorkstreamApplyTool` and `WorkstreamRollbackTool` both construct the new `AcceptCtx` / `RollbackCtx` from `(self.data_dir, workstream)` so per-subroutine arms that need the ontology table see it. The router still resolves the KB; the ws_root path is computed from `data_dir + workstreams + name`.

**Tests added:**
- `accept::tag_promoter_apply_adds_to_ontology`
- `rollback::tag_promoter_inverse_removes_from_ontology`
- `tools::steward::tests::workstream_tag_list_add_remove_round_trip`
- `tools::steward::tests::workstream_apply_promotes_tag_into_ontology` — end-to-end via WorkstreamApplyTool + WorkstreamRollbackTool through the engine tool surface.

**Validation:** workspace builds clean. `cargo test -p arawn-steward` 45/45. `cargo test -p arawn-engine steward` 9/9 (2 new). clippy exit 0.

**Cycle status:** End-to-end Extract → (auto) Suggest → (human) Add → Rollback path now works for `tag-promoter` proposals exactly like it does for `dust` proposals. The agent can `workstream_refine` to see pending promotions, `workstream_apply <id>` to commit, `workstream_rollback <id>` to undo. Manual escape hatch via `workstream_tag list|add|remove`.