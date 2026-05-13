---
id: workstream-refine-journal-rollback
level: task
title: "/workstream refine + journal + rollback commands + steward e2e tests"
short_code: "ARAWN-T-0259"
created_at: 2026-05-13T03:47:14.503763+00:00
updated_at: 2026-05-13T11:25:13.926654+00:00
parent: ARAWN-I-0040
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0040
---

# /workstream refine + journal + rollback commands + steward e2e tests

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

### 2026-05-13 — Complete

Three new agent tools + per-subroutine rollback logic. UX is "confirm by id" per user direction; accept/apply of proposals is deferred (v2).

**Rollback infrastructure (`crates/arawn-steward/src/rollback.rs`):**
- `apply_inverse(journal_row, &MemoryManager)` dispatches by `(subroutine, action)`:
  - `reshelve/merge` → restore both pre-state entities via `update_entity` (clears `superseded` on the deprecated) and delete the SUPERSEDES edge we added.
  - `reshelve/delete` → re-insert the full entity snapshot stored in `outputs_json`.
  - `map/propose_relation` and `doorwatch/propose_identity` → no-op (proposals never mutated; the metadata flip via `Journal::revert` is the whole rollback).
  - identity / unknown → no-op or error.
- 3 inline tests: proposal no-op; reshelve delete reinsert; unknown action errors.

**Agent tools (`crates/arawn-engine/src/tools/steward.rs`):**
- `WorkstreamJournalTool` (`workstream_journal`) — recent rows (limit default 20, max 200). Defaults to active workstream; accepts explicit `workstream` arg.
- `WorkstreamRefineTool` (`workstream_refine`) — pending proposals only (`applied = false AND reverted_at IS NULL`). Same args.
- `WorkstreamRollbackTool` (`workstream_rollback`) — `id` (required) + optional `workstream`. Looks up the row, applies the per-subroutine inverse, flips `reverted_at`. Output is the minimal `{ "id": N, "status": "reverted" | "already_reverted" }` confirmation the user asked for.

**Engine ↔ steward dep edge:** added `arawn-steward` to `arawn-engine` Cargo.toml so engine tools can reach the journal + rollback module. `WorkstreamMemoryRouter` got a small `current_name()` accessor so tools can resolve the active workstream by name without re-deriving it.

**Wiring (`main.rs`):** all three tools register alongside the rest of the workstream tool family when the workstream router is present.

**Tests (inline, 5 new in `tools::steward::tests`):**
- `journal_lists_recent_rows`
- `refine_returns_pending_proposals_only` — applied rows hidden.
- `rollback_reverts_delete_action_end_to_end` — write reshelve/delete journal row → call rollback → entity reappears in the KB.
- `rollback_is_idempotent` — second call returns `already_reverted`.
- `rollback_unknown_id_errors`.

Workspace: `cargo test -p arawn-steward` 24/24, `cargo test -p arawn-engine steward` 5/5, full workspace tests + clippy exit 0.

**Notes / deferred:**
- Accept/apply for proposals is v2 — `/workstream refine` currently lists pending proposals read-only; rejecting them is `workstream_rollback <id>` (metadata flip).
- Hardening: the runner still trusts subroutines' `is_mutating()` flag rather than enforcing it; punt to a follow-up hardening pass.
- The unused `resolve_workstream` helper is `#[allow(dead_code)]` against future callers; remove once a second consumer comes along.