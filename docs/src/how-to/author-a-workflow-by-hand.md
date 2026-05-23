# Author a workflow by hand

*How-to. Write a workflow JSON spec yourself instead of dictating it to the agent.*

Normally you create workflows by asking the agent: *"build me a daily PR briefing that runs at 8 AM weekdays."* The agent composes a JSON spec, calls `workflow_create`, and the workflow runtime takes it from there. But sometimes you want full control — a complex task body, an unusual cron, fine-grained dependencies.

## Prerequisites

- arawn server running.
- Some Rust comfort. Task bodies are Rust async functions; you'll be writing real code.

## The JSON spec

A workflow is described by a JSON document:

```json
{
  "name": "daily-pr-briefing",
  "description": "Every weekday at 8 AM, summarize yesterday's PRs.",
  "cron": "0 8 * * 1-5",
  "cron_timezone": "America/New_York",
  "tasks": [
    {
      "id": "fetch_prs",
      "dependencies": [],
      "body": "// Rust async function body — gh CLI shell call, parses JSON"
    },
    {
      "id": "summarize",
      "dependencies": ["fetch_prs"],
      "body": "// Decision task — calls DecisionService"
    },
    {
      "id": "write_briefing",
      "dependencies": ["summarize"],
      "body": "// Action task — writes markdown file"
    }
  ]
}
```

Fields:

- **`name`** — workflow id. Must be unique in `<data_dir>/workflows/`.
- **`description`** — human-readable purpose.
- **`cron`** — standard 5-field cron, no extensions. See [workflow tools reference](../reference/workflow-tools.md) for syntax + common forms.
- **`cron_timezone`** — IANA timezone (`America/New_York`, `UTC`, etc.). Defaults to UTC.
- **`tasks[]`** — DAG nodes. Each:
  - **`id`** — unique within this workflow.
  - **`dependencies`** — list of upstream task ids that must complete first.
  - **`body`** — Rust async function body, as a string. Has access to the workflow context and the `DecisionService` for LLM calls.

## Task flavours

- **Data tasks** — fetch / transform. No side effects. Examples: pull GitHub issues, parse a transcript.
- **Decision tasks** — call the agent for judgement via `DecisionService`. Example: classify a bug's severity.
- **Action tasks** — produce side effects. Examples: send a Slack message, write a file, open an issue.

Conventional shape: `data → decision → action`, but any DAG works.

## Pass the spec to `workflow_create`

Save the JSON to a file (or pass inline if you're scripting). Then in the TUI or via RPC:

```
workflow_create <paste the JSON spec>
```

Behind the scenes:

1. `arawn-workflow::scaffold::generate` produces a compilable Cargo crate (`Cargo.toml`, `build.rs`, `package.toml`, `src/lib.rs`) with cloacina macros wiring the DAG.
2. It compiles into a `.cloacina` archive.
3. The archive installs to `<data_dir>/workflows/<name>/` — compiled `.dylib`/`.so` plus `package.toml`.
4. The reconciler picks it up and begins honoring the cron schedule immediately.

> **Note:** first creation in a project warms the compiler cache (~30 s); subsequent ones are faster.

## Inspecting and removing

```
workflow_list                 # all installed workflows
workflow_status <name>        # recent runs, success/failure counts, last error
workflow_delete <name>        # uninstall
```

## Examples worth reading

Three worked examples live in `examples/workflows/`:

- **`daily-pr-summary/`** — full buildable crate; linear fetch → process → save.
- **`work-signal-pipeline/`** — DAG with parallel ingestion (three data tasks fanning into one aggregator).
- **`issue-triage/`** — decision-task pattern (agent classifies, conditionally fires an action).

The UAT `work-signal-pipeline` scenario in `crates/arawn-tests/tests/uat.rs` exercises the full agent-authored flow against a real LLM — useful as a reference for what the agent's output looks like.

## Caveats

- Task bodies run **in the parent arawn process**, not the shell sandbox. They're real Rust with full host access. Treat `workflow_create` with the same trust level as `shell`.
- The cloacina runtime is single-host. No distributed scheduling.

## What's next

- Full workflow tool reference: [workflow tools](../reference/workflow-tools.md).
- When workflows shine vs. just asking the agent: [explanation: workflows](../explanation/workflows.md).
