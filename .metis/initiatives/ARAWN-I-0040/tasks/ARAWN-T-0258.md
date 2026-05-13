---
id: steward-map-door-watch-subroutines
level: task
title: "Steward map + door-watch subroutines (proposal-only)"
short_code: "ARAWN-T-0258"
created_at: 2026-05-13T03:47:11.229919+00:00
updated_at: 2026-05-13T11:07:41.770556+00:00
parent: ARAWN-I-0040
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0040
---

# Steward map + door-watch subroutines (proposal-only)

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

Both proposal-only subroutines wired and journaled per ARAWN-A-0003.

**Map (`crates/arawn-steward/src/map.rs`):**
- Trigger: cursor-based on `updated_at` (mirrors reshelve).
- For each focus entity, the LLM is given the focus + a sample of neighbor entities (top-of-`list_all_ranked` minus self, capped by config) and asked for missing relations. Allowed `rel` set: `relates_to | supports | contradicts | mentions | belongs_to`. `supersedes` and `extracted_from` are explicitly excluded (their owners are reshelve and the extractor).
- Validation: rel must parse + be in the proposable set; self-loops are dropped; proposals that don't reference the focus on either side are dropped (LLM-hallucinated ids).
- Each surviving proposal is journaled as `applied = false` so it surfaces through `pending_proposals` and is rejectable via `Journal::revert`.

**Door-watch (`crates/arawn-steward/src/doorwatch.rs`):**
- Trigger: cursor-based on the source workstream's entities.
- Cross-workstream by design — it pulls the active workstream list from `Store::list_workstreams` and uses a `MemoryResolver` to open each *other* workstream's KB (cached). A sample of `neighbors_per_workstream` entities from each bucket is fed to the LLM alongside the focus.
- Scans *all* entity types per user direction (no filter).
- Validation: the LLM's `to_id` must actually appear in the cited `to_workstream` bucket — hallucinated ids are dropped.
- Proposals journal in the *source* workstream and never mutate either side.

**Wiring (`crates/arawn/src/main.rs`):**
- Subroutine vector now `[reshelve, map_sub, doorwatch]`. All three use the engine pool's client + configured model; default 1h cadence remains.
- Door-watch threads `service.shared_store()` and a dedicated memory resolver through the router so it can scan other workstreams without re-opening the master store per call.

**Tests (inline, 6 new):**
- `map::proposes_valid_edges_and_drops_invalid` — confirms one valid edge survives while a self-loop and a `supersedes` rel get rejected.
- `map::cap_stops_after_n_proposals` — blast cap honored mid-batch.
- `map::cursor_advances_and_skips_on_rerun` — second pass calls zero LLM.
- `doorwatch::proposes_identity_when_match_found` — happy-path cross-workstream proposal.
- `doorwatch::hallucinated_target_id_is_dropped` — guards against bogus uuids.
- `doorwatch::no_other_workstreams_means_zero_proposals` — single-workstream environment is a no-op (no LLM call, cursor still progresses).

`cargo test -p arawn-steward`: **21/21**. Workspace + clippy green.

**Notes / deferred:**
- Door-watch is O(W × focus_batch) LLM calls per pass where W = #other workstreams — fine at ~15 workstreams (i.e. ~5 focus × 15 → 75 LLM calls/hour cap is the bound, not workstream count). Revisit if scale changes.
- Proposals are stored in source workstream's journal; the target workstream's user only sees them via cross-workstream tooling that doesn't exist yet — T-0259 surfaces them in `/workstream refine`.
- `is_mutating()` flag exists on the trait but the runner still trusts subroutines; T-0259 / a future hardening pass can wire the check explicitly.