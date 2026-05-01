---
id: uat-mechanical-min-files-created
level: task
title: "UAT mechanical min_files_created false-negatives for non-workspace tools"
short_code: "ARAWN-T-0192"
created_at: 2026-04-30T16:13:17.126530+00:00
updated_at: 2026-05-01T13:29:46.801456+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# UAT mechanical min_files_created false-negatives for non-workspace tools

## Objective

The UAT mechanical pass criterion `min_files_created` only counts files in the workspace directory. Tools that succeed by writing into a different sink (e.g. `workflow_create` installs into the workflows DB) score 0 on this check even when the agent fully completed the objective.

## Type / Priority
- Bug
- P2 — Medium (judge still catches it, but mechanical FAIL muddies CI signals)

## Reproduction

From the 2026-04-30 gemma4:31b-cloud run on `work-signal-pipeline`:

```
Turns: 5 | Files: 1 | Tool errors: 0 | Mechanical: FAIL
  Turn 1: 1 tool(s) [skill] — 35s OK
  Turn 2: 2 tool(s) [think, workflow_create] — 663s OK
  Turn 3: 2 tool(s) [think, workflow_create] — 74s OK
  Turn 4: 2 tool(s) [think, workflow_create] — 200s OK
  Turn 5: 1 tool(s) [workflow_create] — 98s OK
```

Mechanical FAIL but the LLM-as-judge scored completion 5/5, quality 4/5, PASS — because all four `workflow_create` calls succeeded; the workflow was correctly installed; the agent achieved the objective. The harness just looked in the wrong place.

## Expected vs Actual

- **Expected**: mechanical pass criteria reflect the actual artifact sink for each scenario. `work-signal-pipeline`'s deliverable is an installed workflow, not files in `workspace/`.
- **Actual**: `min_files_created` is hardcoded to count workspace files; succeeded `workflow_create` tool calls are invisible to the check.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Each scenario can declare the *kinds* of artifacts it expects: workspace files, installed workflows, registered skills, memory entries, etc.
- [ ] Mechanical check evaluates the right sink per scenario; `work-signal-pipeline` mechanical PASS when the workflow is installed even with 0 workspace files
- [ ] Existing scenarios (`github-monitor`) keep working with no change in mechanical outcome
- [ ] The mechanical block emitted to the judge is consistent with the per-scenario rubric — no more "PASS by judge / FAIL by mechanical" mismatch on success cases

## Implementation Notes

- Logic lives in `crates/arawn-tests/tests/uat.rs` around `MechanicalChecks` (~line 270 in the transcript-writing path).
- Cleanest approach: replace `min_files_created: usize` with a per-scenario `expected_artifacts` enum/list — `Files(n)`, `Workflows(n)`, `Skills(n)`, etc. — and check the appropriate sink.
- Workflows sink: query `workflows.db` in the per-scenario data dir, count rows.
- Don't try to retrofit a generic "sink registry" — start with the two artifact kinds we have today (files, workflows) and grow it as scenarios grow.

## Status Updates

### 2026-05-01 — Implemented and verified

**Changes** (`crates/arawn-tests/tests/uat.rs`):
- `MechanicalThresholds` gained `min_workflows_created: usize` (`#[serde(default)]`).
- `MechanicalCheckResult` gained `workflows_created: usize`.
- New `count_installed_workflows()` counts subdirectories under `<data_dir>/workflows/` — each `workflow_create` install lands as `workflows/<name>/{lib.dylib, package.toml}`. (No SQLite coupling needed; the `workflow_packages` / `workflow_registry` tables are empty post-install — packages live on disk.)
- `mech_pass` now requires both file and workflow thresholds.
- Per-scenario and final summary print the new `Workflows` column.
- `github-monitor`: `min_files_created: 2, min_workflows_created: 0` (unchanged outcome).
- `work-signal-pipeline`: `min_files_created: 0, min_workflows_created: 1` (deliverable is the installed workflow).

**Verification** (gemma4:31b-cloud, 2026-05-01):
```
Turns: 5 | Files: 1 | Workflows: 1 | Tool errors: 0 | Mechanical: PASS
```
Same scenario that previously mech-FAILed now correctly PASSes via the workflow sink.

**Decision deferred:** The `expected_artifacts` enum-list approach was not adopted — two parallel thresholds keep the diff small. Revisit when a third sink (memory entries, registered skills, etc.) becomes scenario-relevant.