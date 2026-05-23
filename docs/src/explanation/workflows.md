# Workflows

*Explanation. When to workflow vs. converse, why a DAG, why cloacina.*

A workflow is a scheduled DAG pipeline — a set of Rust tasks with dependencies, optionally on a cron schedule. For the JSON spec and tool reference, see [workflow tools reference](../reference/workflow-tools.md). For hand-authoring, see [author a workflow by hand](../how-to/author-a-workflow-by-hand.md). This page is about when to reach for one.

## When to use a workflow vs. a regular conversation

| Use a workflow when... | Use a conversation when... |
|---|---|
| It runs on a schedule | It's a one-off |
| It runs while you're not at the keyboard | It's interactive |
| It needs to retry failed steps with backoff | A single failure is fine to ignore |
| Multiple steps depend on each other in a fixed order | The flow is dynamic / decided turn-by-turn |
| You want to wire LLM judgment into one step of a larger pipeline | The whole thing is judgment |

Workflows are not the right tool for "do this thing once right now" — just ask the agent. They shine when the same shape repeats over time.

## The three task flavours

A workflow's DAG is built from three task types:

- **Data tasks** — fetch / transform / produce structured output. No side effects beyond the workflow context. Examples: pull GitHub issues, parse a transcript, query a database.
- **Decision tasks** — call the agent (LLM) for judgment on the upstream data via `DecisionService`. Examples: classify a bug's severity, pick the highest-impact item from a list.
- **Action tasks** — produce the side effects you actually wanted. Examples: send a Slack message, write a markdown briefing, open a GitHub issue.

The conventional shape is `data → decision → action`. Why?

- Data tasks are deterministic and replayable — re-running them produces the same result.
- Decision tasks are the LLM-shaped piece — exactly *one* step needs judgment.
- Action tasks have side effects — failures matter; retries are dangerous (sent a message twice?). They go at the end so the LLM has already decided.

A DAG with all judgment ("LLM classifies, then LLM summarizes, then LLM picks") is technically possible but rarely useful. The trick of workflows is that they let you write code for the deterministic parts and only ask the LLM for the parts that need judgment.

## Why a DAG, not a linear pipeline

The DAG shape costs a little complexity vs. linear `step1 → step2 → step3`, but pays for itself when:

- **Multiple data sources fan in.** Pull from Slack AND Jira AND GitHub, then aggregate. Three data tasks, one aggregator task.
- **A decision has multiple consequences.** Classify a bug as "critical" → both notify the on-call AND open a P0 ticket. Two action tasks, one decision.
- **You want parallelism.** Independent branches can run concurrently. cloacina schedules them.

Linear is a degenerate DAG. If you don't need branching, you get linear for free.

## Why cloacina

[cloacina](https://github.com/colliery-io/cloacina) is the underlying scheduler and runtime — same author/team as graphqlite. It gives arawn three things that would otherwise be a custom build:

- **DAG execution.** Task dependencies, parallelism, partial-failure semantics.
- **Cron scheduling.** Native cron-driven re-runs without arawn having to host its own scheduler.
- **Persistence.** Run history, attempt counts, retry state — stored in `workflows.db` (cloacina's own SQLite schema).

We could have written a workflow runner from scratch. We didn't, because:

- The cloacina abstractions (typed tasks, derive macros, dynamic library packaging) match what arawn needs almost exactly.
- The colliery-io ecosystem (graphqlite, cloacina) is co-developed; building on it shrinks the maintenance surface.
- The single-host scheduling tradeoff (cloacina doesn't distribute) matches arawn's single-host thesis. If you want distributed scheduling, you've outgrown arawn anyway.

## Why workflows are compiled Rust, not interpreted

A workflow's tasks are async Rust functions. `workflow_create` calls `arawn-workflow::scaffold::generate` to produce a complete compilable Cargo crate, then compiles it into a `.cloacina` archive that installs as `<data_dir>/workflows/<name>/lib<name>.{dylib,so}`.

Why this much ceremony?

- **Type-safety.** Task inputs and outputs are typed; cloacina's macros enforce the DAG. Errors are compile errors, not runtime crashes.
- **Performance.** Compiled Rust is fast. A workflow that processes 1000 GitHub issues in a "data" task finishes in seconds, not minutes.
- **Sandboxing isn't free.** If workflow task bodies ran in a sandbox (Wasm, JS, scripted), we'd lose direct access to arawn's internal services (the `DecisionService`, the workstream router). Compiled Rust runs in-process.

The cost: first creation in a project warms the compiler cache (~30s). Subsequent ones are faster. The agent's `workflow_create` handles this transparently; you wait once.

## Why task bodies run unsandboxed

A workflow task body is real Rust code with full host access. It's not gated by the shell sandbox, the permission rules, or anything else. That sounds scary; it's deliberate.

- The trust path is *workflow creation*, not *task execution*. When you (or the agent on your behalf) call `workflow_create`, you're authorizing the code in the spec to run on your machine in perpetuity. That's the gate.
- Once authorized, the task body needs full access to do useful things — call internal arawn services, write files anywhere in `<data_dir>`, hit the network without re-asking permission every time.
- The `workflow_create` tool itself is gated. In `default` permission mode it asks. The recommendation in the docs is to treat `workflow_create` with the same trust level as `shell`.

If you want sandboxed automation, you want the agent + shell + permission rules, not workflows.

## What workflows aren't good at

- **Interactive flows.** No way to ask the user a question mid-workflow. If you need that, drop back into the agent.
- **Sub-minute latency.** Cron is the minimum cadence (`* * * * *`); on-demand runs work but have startup overhead.
- **Cross-host coordination.** Single-host scheduling. No distributed locks, no replicated state.
- **Things that change shape often.** Workflows are compiled; modifying a task body means re-running `workflow_create`. If the shape changes weekly, just have a conversation each week.

## The arawn flagship use case

The morning brief. The vision wants arawn to *watch, check, summarize, nudge* — and workflows are the "watch + summarize" part:

```
8:00 AM every weekday:
  1. fetch_overnight_signals    (data — pulls from Slack, Gmail, Jira)
  2. extract_action_items       (decision — LLM picks what matters)
  3. write_morning_brief        (action — writes a tablet for the daily ceremony)
```

This is currently three concurrent watchers; the next phase will collapse them into a single named workflow. The agent will author it via `workflow_create` from a one-line user prompt.

[Author a workflow by hand](../how-to/author-a-workflow-by-hand.md) covers the manual recipe; the auto path lands in a follow-up to I-0033.

## Related

- [Workflow tools reference](../reference/workflow-tools.md) — JSON spec, cron syntax, storage.
- [Author a workflow by hand how-to](../how-to/author-a-workflow-by-hand.md).
- [Agent loop](./the-agent-loop.md) — what's NOT in the workflow runtime.
- [Ceremonies explanation](./ceremonies.md) — the scheduled-reflection sibling.
