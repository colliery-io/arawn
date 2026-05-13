---
id: steward-re-shelve-dust-subroutines
level: task
title: "Steward re-shelve + dust subroutines (dedupe + stale summarization)"
short_code: "ARAWN-T-0257"
created_at: 2026-05-13T03:47:07.876525+00:00
updated_at: 2026-05-13T09:51:51.166753+00:00
parent: ARAWN-I-0040
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0040
---

# Steward re-shelve + dust subroutines (dedupe + stale summarization)

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

### 2026-05-13 — Rescoped to re-shelve only; complete

Dust deferred — design question on the stale-cluster trigger isn't settled yet (per user). Spinning out a follow-up task once we have a definition for "stale" worth keying off.

**Design choices applied (per user):**
- Re-shelve trigger: only entities `updated_at > cursor` (created or touched since last pass). A `steward_cursors(subroutine, last_updated_at)` table colocates with the journal in each workstream's KB sqlite, monotonic via `ON CONFLICT ... CASE WHEN excluded > existing`.
- Merge survivor: most-reinforced wins; tie-break on newer `created_at`. The LLM is *not* consulted for survivor identity — it only judges duplicate / erroneous, and proposes `combined_content` when duplicate.
- "Combine content fields" is allowed per amended ADR-0003: tag union, LLM-proposed content (falling back to survivor's existing or the deprecated's if survivor was empty), reinforcement_count summed + 1.
- "Delete entity" allowed when LLM judges the entity *erroneous* (verdict carries `delete_target: "focus" | "candidate"`).

**Implementation (`crates/arawn-steward`):**
- `cursor::CursorStore` — sqlite-backed cursor table colocated with the workstream's `memory.db`. Monotonic advance via SQL CASE.
- `llm_text` — local copy of the streaming-drain helper from arawn-extractor; promote to arawn-llm when a 3rd consumer appears.
- `reshelve::ReshelveSubroutine` — full pipeline: walk touched entities since cursor → FTS candidates per focus → LLM verdict per pair → apply merge / delete with write-ahead journal payload sufficient for revert. Per-iteration re-fetch of focus state so prior actions in the same pass don't operate on stale snapshots.
- `lib.rs` re-exports.
- `main.rs` swaps `IdentitySubroutine` out for `ReshelveSubroutine` wired to `llm_pool.engine()` + the engine's configured model.

**Tests (inline, 5 new):**
- `merge_picks_most_reinforced_survivor` — verifies survivor identity, content replacement, supersession, reinforcement merge.
- `erroneous_deletes_focus` — exercises `delete_target: "candidate"` path.
- `none_verdict_leaves_kb_untouched_but_advances_cursor` — proves the cursor moves even with zero mutations so we don't re-scan.
- `second_pass_skips_already_processed_entities` — cursor effective on second pass (would panic on empty ScriptedMock otherwise).
- `cap_stops_after_n_applied` — blast-radius cap is honored.

Total `cargo test -p arawn-steward`: **15/15** green. Workspace tests + clippy exit 0.

**Deferred / known limits:**
- Dust subroutine deferred (see context above).
- Per-pass transaction isolation (ADR-0003 §5) isn't wired yet — write-ahead row + mutation happen as separate sqlite ops on different connections (journal vs graphqlite). For revert purposes this is fine (journal contains pre-state); for crash atomicity it's not. Land that when journal & memory share a connection.
- Cap counts *applied actions*, not *entities considered*; this is the right shape but worth re-litigating once we have telemetry.