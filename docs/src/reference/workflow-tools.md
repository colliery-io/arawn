# Workflow tools

*Reference. The four `workflow_*` agent tools + cron syntax + task types + storage layout.*

A workflow is a scheduled DAG pipeline — a set of Rust tasks with dependencies, optionally on a cron schedule. For the conceptual framing ("when to workflow vs. converse"), see [workflows explanation](../explanation/workflows.md). For the hand-authoring recipe, see [author a workflow by hand](../how-to/author-a-workflow-by-hand.md).

Source: `crates/arawn-workflow/`. Runtime: [cloacina](https://github.com/colliery-io/cloacina).

## Tools

| Tool | Description |
|---|---|
| `workflow_create` | Author + compile + install a workflow from a JSON spec. |
| `workflow_list` | List installed workflows with cron schedules and enabled state. |
| `workflow_status <name>` | Recent runs, success/failure counts, last error. |
| `workflow_delete <name>` | Uninstall. |

## Spec format

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
      "body": "// Rust async function body"
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

Field reference is at [author a workflow by hand](../how-to/author-a-workflow-by-hand.md).

## Task types

| Type | Side effects? | Examples |
|---|---|---|
| **Data tasks** | No | Fetch from upstream, parse a transcript, query a database. |
| **Decision tasks** | No (calls LLM) | Classify a bug's severity, pick the highest-impact item, summarize. |
| **Action tasks** | Yes | Send a Slack message, write a markdown briefing, open a GitHub issue. |

Conventional shape: `data → decision → action`. Any DAG works.

## Cron syntax

Standard 5-field, no extensions:

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

Use [crontab.guru](https://crontab.guru) when in doubt. Timezone defaults to UTC; set `cron_timezone` to override (IANA names).

## Storage layout

```
<data_dir>/
├── workflows.db                              # cloacina state — runs, schedules, attempts
└── workflows/
    └── <workflow-name>/
        ├── package.toml                      # workflow metadata
        └── lib<crate_name>.{dylib,so}        # compiled task code (crate_name = workflow-name with '-' → '_')
```

For a workflow named `daily-pr-briefing`, the dylib lands at `<data_dir>/workflows/daily-pr-briefing/libdaily_pr_briefing.{dylib,so}`. Hyphens become underscores in the library filename per Cargo conventions.

`workflows.db` carries cloacina's bookkeeping (pipeline executions, task attempts, schedule state). Don't delete it while arawn is running.

## What `workflow_create` does

1. Calls `arawn-workflow::scaffold::generate` to produce a complete compilable Cargo crate (`Cargo.toml`, `build.rs`, `package.toml`, `src/lib.rs`) with cloacina macros wiring the DAG.
2. Compiles it into a `.cloacina` archive.
3. Installs it to `<data_dir>/workflows/<name>/` as a directory containing the compiled `.dylib`/`.so` plus `package.toml`.
4. Hands it to the running reconciler, which begins honoring the cron schedule immediately.

First creation in a project warms the Rust compiler cache (~30 s). Subsequent ones are faster.

## Examples

Three worked examples live in `examples/workflows/`:

- **`daily-pr-summary/`** — full buildable crate showing the linear fetch → process → save pattern.
- **`work-signal-pipeline/`** — DAG with parallel ingestion (three data tasks fanning into one aggregator).
- **`issue-triage/`** — decision-task pattern (agent classifies, conditionally fires an action).

The UAT `work-signal-pipeline` scenario in `crates/arawn-tests/tests/uat.rs` exercises the full agent-authored flow against a real LLM.

## Caveats

- **Task bodies run in the parent arawn process**, not the shell sandbox. They're real Rust with full host access. Treat `workflow_create` with the same trust level as `shell`.
- **The cloacina runtime is single-host.** No distributed scheduling.

## Related

- [Workflows explanation](../explanation/workflows.md) — when to use, why a DAG, cloacina rationale.
- [Author a workflow by hand](../how-to/author-a-workflow-by-hand.md) — recipe for hand-editing.
- [Agent tools reference](./agent-tools.md) — `workflow_create` / `_list` / `_status` / `_delete`.
