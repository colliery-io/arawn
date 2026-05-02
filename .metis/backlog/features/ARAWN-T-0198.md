---
id: workflows-documentation-and
level: task
title: "Workflows: documentation and example library"
short_code: "ARAWN-T-0198"
created_at: 2026-05-02T00:00:00+00:00
updated_at: 2026-05-02T00:00:00+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


exit_criteria_met: false
initiative_id: NULL
---

# Workflows: documentation and example library

## Objective

The workflows runtime is built (`crates/arawn-workflow`), the scaffold generates compilable crates, and the agent has a `workflow_create` tool with a skill guide. But there are no worked examples a user (or the agent) can copy from to understand what's possible. Workflows are arawn's killer feature for "set this up to run on a schedule" use cases — and right now they're invisible.

## Type / Priority
- Feature (documentation + example artifacts)
- P2 — Important. Not a blocker for "user can chat" but a blocker for "user discovers what arawn is uniquely good at."

## Acceptance Criteria

- [ ] `docs/workflows.md` covering: what a workflow is, when to use one (vs an ad-hoc task), the DAG model (data / decision / action task types), cron scheduling, where workflows install, how to inspect runs.
- [ ] At least 3 worked-example workflows in `examples/workflows/` (or equivalent location), each with a one-paragraph README explaining the use case:
  - One simple linear pipeline (e.g. "fetch → summarize → save")
  - One DAG with parallel branches (e.g. "ingest 3 sources → aggregate → prioritize → write briefing" — basically what the UAT scenario builds)
  - One that uses the LLM agent task type for a decision step (e.g. "fetch issues → agent classifies severity → notify if any P0")
- [ ] Each example compiles and installs cleanly via `arawn` (verified manually; document the steps).
- [ ] Workflow authoring skill guide (existing) is cross-linked from the new doc.

## Implementation Notes

- The UAT `work-signal-pipeline` scenario already produces a working DAG — that artifact is a starting point for one of the examples (clean it up, not copy-paste verbatim).
- Avoid bloating the repo with a dozen toy examples — three solid ones beat ten thin ones.
- Don't touch the workflow scaffold or runtime in this ticket. If something is awkward to document, file a follow-up.

## Status Updates

### 2026-05-02 — Docs portion done

`docs/src/workflows.md` written. Covers what a workflow is, when to reach for one vs an ad-hoc conversation (decision matrix), the three task flavours (data/decision/action), how `workflow_create` authors them step-by-step (scaffold → compile → install → reconciler), cron syntax with common forms, the four workflow management tools (`workflow_list/status/delete/create`), storage layout, and caveats (compile time, host trust, single-host scheduling). Linked from SUMMARY.md.

Code/example work still open under this ticket:
- [ ] At least 3 worked-example workflow crates in `examples/workflows/`:
  - linear pipeline (fetch → summarize → save)
  - DAG with parallel branches (basically work-signal-pipeline cleaned up)
  - decision task using the agent (fetch issues → agent classifies severity → notify if P0)
- [ ] Each compiles and installs cleanly; document the steps in `examples/workflows/README.md`
- [ ] Workflow skill guide cross-linked from `docs/src/workflows.md` (currently points only at the UAT scenario)

The example library is the larger half of this ticket and was deliberately deferred — workflow-crate authoring is its own focused session.
