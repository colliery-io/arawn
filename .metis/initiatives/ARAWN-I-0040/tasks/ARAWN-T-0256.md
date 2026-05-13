---
id: steward-scaffolding-cloacina
level: task
title: "Steward scaffolding — cloacina workflow, journal table, rollback infra"
short_code: "ARAWN-T-0256"
created_at: 2026-05-13T03:47:04.271108+00:00
updated_at: 2026-05-13T04:08:52.015086+00:00
parent: ARAWN-I-0040
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0040
---

# Steward scaffolding — cloacina workflow, journal table, rollback infra

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

### 2026-05-13 — Scaffolding complete

**New crate `arawn-steward` (5 modules, ~700 lines).**

- `error::StewardError` — local error type with `From` impls for `rusqlite`, `serde_json`, and `arawn_memory::MemoryError`.
- `journal::Journal` — opens (or creates) `steward_journal` colocated with each workstream's `memory.db` via a separate rusqlite connection. CRUD: `write_ahead(record) -> id` (returns sqlite rowid), `get(id)`, `recent(limit)`, `pending_proposals(limit)`, `revert(id) -> RevertResult { newly_reverted }` (idempotent). Schema matches ADR-0003 (ts/subroutine/action/inputs_json/outputs_json/model/prompt_hash/applied/reverted_at) plus two indexes (ts; applied+reverted_at for pending-proposal queries). `Journal::prompt_hash(input)` is a deterministic uuid-v5 string used by subroutines.
- `subroutine::StewardSubroutine` trait + `IdentitySubroutine` no-op so the scaffolding is exercisable end-to-end before T-0257 lands re-shelve/dust.
- `runner::StewardRunner` — walks active workstreams via `Store::list_workstreams`, runs each configured subroutine sequentially per workstream, caches `Journal` instances per workstream to keep sqlite handles warm. `SubroutineCaps` is per-subroutine with a default fallback — defaults are placeholders per ADR-0003 (real values come from the Phase-5 harness later).

**Wiring (arawn binary).**
- Added `arawn-steward` to workspace + `crates/arawn/Cargo.toml` deps.
- `main.rs` spawns a tokio interval task (every 1h for dev) that calls `runner.run_pass_for_all()`. Gated on the workstream router being present so steward + memory routing stay in lockstep.

**Tests (15 total across the crate, 9 inline + 6 in submodules):**
- `journal::tests` — write/read round-trip; revert idempotency; recent ordering; pending_proposals filter on applied+reverted; deterministic prompt_hash; schema idempotent on reopen.
- `runner::tests` — pass visits every active workstream and skips archived; caps overrides take precedence; journal persists across passes (two passes → two rows).

`cargo test -p arawn-steward` → 9/9. Full workspace tests + clippy green.

**Design notes / deferred:**
- Cloacina workflow per workstream was the original sketch in I-0040; I chose a tokio interval task (mirrors the embed pass) for v1. Cleaner, fewer moving parts, and the per-workstream-cron flavor wasn't pulling its weight at this scale. Easy to swap to cloacina later if cadence-per-workstream config becomes a real need.
- Subroutine error during a pass is logged + counted but does not abort remaining subroutines on the same workstream. Matches the extractor's "soft-fail per workstream" pattern.
- The journal lives in the *same sqlite file* as the workstream's graphqlite KB (per ADR-A-0002 colocation pattern). Multiple rusqlite connections to the file are fine; journal table is disjoint from graphqlite tables.
- `IdentitySubroutine` deliberately reports `is_mutating() = false` so the proposal-shaped journal path gets exercised. The trait's `is_mutating()` flag isn't enforced yet — T-0257 wires the runner to refuse mutating writes from a non-mutating subroutine.