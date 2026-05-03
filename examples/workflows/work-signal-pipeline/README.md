# work-signal-pipeline

A DAG with **parallel ingestion** that fans into a single aggregation step,
then prioritizes and writes a daily briefing.

```
fetch_meeting_notes ─┐
fetch_slack_digest  ─┼─▶ aggregate_signals ─▶ prioritize_signals ─▶ write_briefing
fetch_jira_updates  ─┘
```

The three `fetch_*` tasks run **in parallel** because none of them depends on
the others. `aggregate_signals` waits for all three.

## Pattern

Multi-source ingestion + fan-in. Use this shape whenever the answer to
"what should the user do today?" needs data from several independent places
that don't talk to each other.

## How to build

This example ships as **source listing only** — no `Cargo.toml`, no
`build.rs`, no `package.toml`. To turn it into a runnable workflow:

1. Copy the four boilerplate files from `../daily-pr-summary/`:
   - `Cargo.toml` (rename `name = "daily-pr-summary"` → `name = "work-signal-pipeline"`)
   - `build.rs` (no changes)
   - `package.toml` (rename `daily-pr-summary` → `work-signal-pipeline`,
     update `workflow_name = "work_signal_pipeline"`, edit description)
   - `src/lib.rs` — replace with the contents of [`lib.rs`](./lib.rs) here
2. Build per the [linear example's README](../daily-pr-summary/README.md).

Or just ask the agent: "use workflow_create to build a parallel-fan-in
workflow following examples/workflows/work-signal-pipeline/lib.rs".

## What's different from the linear example

- Three `dependencies = []` data tasks at the top — the runner schedules them
  in parallel automatically.
- One task with `dependencies = ["fetch_meeting_notes", "fetch_slack_digest",
  "fetch_jira_updates"]` — the fan-in. Cloacina waits for all upstream tasks
  before invoking it.
- Same `#[trigger]` cron pattern as the linear example.
