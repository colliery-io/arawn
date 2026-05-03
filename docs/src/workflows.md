# Workflows

A **workflow** in arawn is a scheduled DAG pipeline — a set of Rust
tasks with dependencies, optionally on a cron schedule. Workflows are
how you turn one-off agent conversations into recurring jobs: "every
weekday morning, fetch yesterday's PRs, summarize them, write a
briefing."

The runtime embeds [cloacina](https://github.com/colliery-io/cloacina)
for DAG execution.

## When to use a workflow vs a regular conversation

| Use a workflow when... | Use a conversation when... |
|---|---|
| It runs on a schedule | It's a one-off |
| It runs while you're not at the keyboard | It's interactive |
| It needs to retry failed steps with backoff | A single failure is fine to ignore |
| Multiple steps depend on each other in a fixed order | The flow is dynamic / decided turn-by-turn |
| You want to wire LLM judgement into one step of a larger pipeline | The whole thing is judgement |

Workflows are not the right tool for "do this thing once right now" —
just ask the agent. They shine when the same shape repeats over time.

## Task types

A workflow's DAG is built from three task flavours:

- **Data tasks** — fetch / transform / produce structured output. No
  side effects beyond the workflow context. Examples: pull GitHub
  issues, parse a transcript, query a database.
- **Decision tasks** — call the agent (LLM) for judgement on the
  upstream data. Examples: classify a bug's severity, pick the
  highest-impact item from a list. The `DecisionService` plumbs the
  agent into these.
- **Action tasks** — produce the side effects you actually wanted.
  Examples: send a Slack message, write a markdown briefing, open a
  GitHub issue.

The conventional shape is `data → decision → action`, but any DAG
works.

## How they're authored

The agent authors workflows during conversation via the **`workflow_create`**
tool. Tell it what you want, and it composes a JSON spec like:

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

Behind the scenes, `workflow_create`:

1. Calls `arawn-workflow::scaffold::generate` to produce a complete
   compilable Cargo crate (`Cargo.toml`, `build.rs`, `package.toml`,
   `src/lib.rs`) with cloacina macros wiring the DAG.
2. Compiles it into a `.cloacina` archive.
3. Installs it to `<data_dir>/workflows/<name>/` as a directory
   containing the compiled `.dylib`/`.so` plus `package.toml`.
4. Hands it to the running reconciler, which begins honoring the cron
   schedule immediately.

You can also author workflows by hand if you want full control —
generate the scaffold, then edit the produced files. Useful for complex
task bodies you'd rather write than describe.

## Cron syntax

Standard 5-field cron, no extensions:

```
* * * * *
│ │ │ │ └── day of week (0-6, Sunday = 0)
│ │ │ └──── month (1-12)
│ │ └────── day of month (1-31)
│ └──────── hour (0-23)
└────────── minute (0-59)
```

Common forms:

| Expression | Means |
|---|---|
| `0 8 * * 1-5` | 8:00 AM every weekday |
| `0 9 * * *` | 9:00 AM every day |
| `*/15 * * * *` | Every 15 minutes |
| `0 0 1 * *` | Midnight on the 1st of every month |

Use [crontab.guru](https://crontab.guru) when in doubt. Timezone defaults
to UTC; set `cron_timezone` to override.

## Inspecting workflows

Three more tools complement `workflow_create`:

- **`workflow_list`** — all installed workflows, with cron schedules and
  enabled state.
- **`workflow_status <name>`** — recent runs, success/failure counts,
  last error.
- **`workflow_delete <name>`** — uninstall.

You can ask the agent to use these conversationally ("how's my
daily-pr-briefing workflow doing?"), or call them via direct RPC if
you're scripting.

## Storage layout

```
<data_dir>/
├── workflows.db                              # cloacina state (runs, schedules)
└── workflows/
    └── <workflow-name>/
        ├── package.toml                      # workflow metadata
        └── lib<workflow_name>.{dylib,so}     # compiled task code
```

`workflows.db` carries cloacina's bookkeeping (pipeline executions, task
attempts, schedule state). Don't delete it while arawn is running.

## Examples

Three worked examples live in [`examples/workflows/`](https://github.com/dstorey/arawn/tree/main/examples/workflows):

- **`daily-pr-summary/`** — full buildable crate showing the linear
  fetch → process → save pattern. `cargo build --release` produces a
  `.cdylib` you can install.
- **`work-signal-pipeline/`** — source listing for a DAG with parallel
  ingestion (three data tasks fanning into one aggregator).
- **`issue-triage/`** — source listing for a decision-task pattern that
  uses the agent for LLM-backed classification and conditionally fires
  an action.

Read [`examples/workflows/README.md`](https://github.com/dstorey/arawn/blob/main/examples/workflows/README.md) first.

The UAT `work-signal-pipeline` scenario in
`crates/arawn-tests/tests/uat.rs` exercises the full agent-authored flow
end-to-end against a real LLM — useful as a reference for "what does the
agent's output look like."

## Caveats

- Workflows compile Rust code on creation. First creation in a project
  warms the compiler cache (~30s); subsequent ones are faster.
- Task bodies run in the parent arawn process, not the shell sandbox.
  They're real Rust code with full host access — treat the
  `workflow_create` tool with the same trust level as `shell`.
- The cloacina runtime is single-host. There's no distributed scheduling.
