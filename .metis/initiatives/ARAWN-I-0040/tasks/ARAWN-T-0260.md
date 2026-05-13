---
id: dust-subroutine-manual-trigger
level: task
title: "Dust subroutine (manual trigger) + generic workstream_apply for proposals"
short_code: "ARAWN-T-0260"
created_at: 2026-05-13T12:35:21.383898+00:00
updated_at: 2026-05-13T12:45:05.297680+00:00
parent: ARAWN-I-0040
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0040
---

# Dust subroutine (manual trigger) + generic workstream_apply for proposals

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

User direction: dust is manual-only (no auto cadence). Dry-run is the default — the tool always writes proposals (no separate `dry_run` flag) and a follow-up `workstream_apply <id>` commits whichever the user picks. This unifies acceptance across dust + map + door-watch.

**`arawn-memory` change:**
- Added `RelationType::Summarizes` (closed enum) with cypher schema entries. Allowed verb per ARAWN-A-0003 for the dust subroutine; not proposable by `map` (the map subroutine's allowlist remains unchanged).

**`arawn-steward` additions:**

- `dust::DustEngine` — runs a single pass. Two cluster modes:
  - `ClusterMode::Tag` (default): group by shared tag, skipping prior dust outputs (`steward:dust` tag). `tag_filter` restricts to specific tag keys.
  - `ClusterMode::Provenance`: group by shared `EXTRACTED_FROM` target so "all entities from this one Slack thread / Gmail message" form a cluster.
  - Trigger: every member of a cluster must have `updated_at < now - idle_days` (default 30). `min_cluster_size` (default 3) and `limit` (default 5 proposals per run) bound size.
  - Writes proposals as `applied = false` journal rows; `outputs_json` carries the proposed summary entity (Note + `steward:dust` tag + cluster_key tag) plus the full source id list and member counts.
  - LLM is called once per cluster; the agent sees real summary text in the proposal payload.
- `accept::apply_forward(journal_row, &MemoryManager)` — symmetric to `rollback::apply_inverse`:
  - `dust/summarize` → insert the proposed summary entity + add SUMMARIZES edges to every source.
  - `map/propose_relation` → add the relation.
  - `doorwatch/propose_identity` → no graph change; the `applied = true` flip is the acceptance record. We don't have cross-workstream merge primitives yet — designing those is outside this task's scope.
  - reshelve / identity actions → no-op (they were already applied at journal-write time).
- `Journal::mark_applied(id) -> AppliedResult { newly_applied }` — symmetric to `Journal::revert`. Refuses to flip a reverted row. Idempotent.

**`arawn-engine` tools (added to `tools/steward.rs`):**

- `WorkstreamDustTool` (`workstream_dust`) — args: `workstream?`, `cluster_by? ("tag" | "provenance")`, `min_cluster_size?`, `idle_days?`, `limit?`, `tags?` (restrict). Returns `{clusters_found, proposals_written, limit_hit, proposals: [...]}` with each proposal's full payload so the agent can show the user the proposed summary text before they commit.
- `WorkstreamApplyTool` (`workstream_apply`) — args: `id`, `workstream?`. Looks up the row, dispatches `accept::apply_forward`, flips `applied = true`. Refuses to apply a reverted row. Returns `{ "id": N, "status": "applied" | "already_applied" }`.

**Wiring (`main.rs`):**
- `WorkstreamDustTool` + `WorkstreamApplyTool` register alongside the existing journal / refine / rollback tools whenever `workstream_router` is present.

**Tests (12 new across the changes):**

- `arawn-steward`:
  - `dust::tag_cluster_writes_proposal_when_all_idle`
  - `dust::cluster_with_one_fresh_member_is_skipped` — every member must be idle
  - `dust::min_cluster_size_filters_out_small_clusters`
  - `dust::limit_caps_proposals_per_run`
  - `dust::prior_dust_summaries_are_excluded_from_new_clusters`
  - `accept::map_apply_adds_relation`
  - `accept::dust_apply_inserts_summary_and_edges`
  - `accept::doorwatch_apply_is_noop`
  - `accept::unknown_action_errors`
- `arawn-engine`:
  - `apply_then_rollback_round_trip_for_map_proposal` — apply twice → second call returns `already_applied`.
  - `apply_refuses_reverted_row` — apply on a row that's been rolled back returns an error.

`cargo test -p arawn-steward`: **33/33** (12 new). `cargo test -p arawn-engine steward`: **7/7** (2 new). Workspace tests + clippy exit 0.

**Notes:**
- Dust does *not* hook into the auto-pass `StewardRunner` subroutine list — by design, per user. The engine + runner know nothing about dust; the tool layer is where it lives.
- Future cross-workstream merge primitives would let `accept::doorwatch_apply` actually do something useful (create a shared Person entity, link both workstreams, etc.). For now the journal row is the canonical "user accepted these are the same identity" record.
- `steward:dust` tag on summary entities acts as a soft marker so future re-shelve hardening can skip merging dust outputs with each other.