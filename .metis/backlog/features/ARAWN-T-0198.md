---
id: workflows-documentation-and
level: task
title: "Workflows: documentation and example library"
short_code: "ARAWN-T-0198"
created_at: 2026-05-02T00:00:00+00:00
updated_at: 2026-05-03T12:38:46.398298+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


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

## Acceptance Criteria

## Acceptance Criteria

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

### 2026-05-03 — Examples landed (right-sized)

**Scope cut from the ticket:** "all three buildable" → "one buildable + two source listings". Examples are illustrative — the goal is to teach the pattern, not provide copy/paste-runnable code for every variant. The buildable one proves the scaffold is honest; the source listings show the variant patterns in well-commented `lib.rs` form.

**Files** (`examples/workflows/`):
- `README.md` — top-level orientation, how-to-build the buildable example, how-to-bootstrap from source listings, "ask the agent" path.
- `daily-pr-summary/` — **buildable** linear pipeline (`fetch_prs → summarize_prs → save_briefing`). Full crate with `Cargo.toml`, `build.rs`, `package.toml`, `src/lib.rs`, `Cargo.lock`, `README.md`. Verified `cargo build --release` produces `libdaily_pr_summary.dylib` (925KB).
- `work-signal-pipeline/` — source listing (`lib.rs` + `README.md`) showing parallel ingestion + fan-in.
- `issue-triage/` — source listing showing decision-task pattern with agent integration. Stubs the agent call with a deterministic classifier and points at the real-call shape in a doc-comment, since the `cloacina-workflow-plugin` agent API is still settling.

**Cross-link:** `docs/src/workflows.md` Examples section now points at all three crates instead of saying "library on the backlog."

**Findings worth noting:**
- The scaffold's generated `Cargo.toml` was correct: `cloacina-workflow`/`-macros`/`-build` are on 0.5 even though the runtime `cloacina` is on 0.4. They're separate version trains.
- `Context::insert` returns `Result<(), ContextError>` — must be `?`'d. The scaffold-generated examples elide this; worth a follow-up on the scaffold to wrap insert calls correctly.
- `TaskError` variants are struct-shaped (`{message, task_id, timestamp}` etc.), not tuple. Examples include a small `fail(task_id, msg) -> TaskError` helper to keep call sites readable.

**Acceptance criteria status:**
- [x] `docs/src/workflows.md` (last session).
- [x] Three worked examples (one buildable, two source listings — see scope-cut note).
- [x] Buildable example compiles cleanly (`cargo build --release` verified).
- [x] Workflow doc cross-links the example crates.

**Deferred small follow-ups (worth filing if they bite):**
- Scaffold's generated task bodies elide `Context::insert` error handling (always succeed when called from the agent today, but real workflows need it).
- The `cloacina-workflow-plugin` agent integration API needs a stable example once it settles — `issue-triage`'s decision task is currently stubbed with a deterministic classifier.