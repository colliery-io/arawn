# issue-triage

A workflow that uses the **arawn agent** as a judgement step in the middle
of the pipeline. The agent reads each issue and labels it with a severity;
the action task only fires when there's at least one P0.

```
fetch_open_issues ─▶ classify_severity ─▶ notify_if_p0
                       (decision task —
                        calls the agent)
```

## Pattern

Three task flavours in a row:

- **Data task** (`fetch_open_issues`) — pull structured input.
- **Decision task** (`classify_severity`) — feed input to the LLM via the
  `cloacina-workflow-plugin` agent integration; get back classifications.
- **Action task** (`notify_if_p0`) — produce a side effect, *but only if the
  decision warrants it*. Conditional action.

The decision step is what makes this expensive (LLM calls). Keep the data
task wide enough to be worth one classification round, but not so wide that
the agent runs out of context.

## How to build

Source listing only — see [`lib.rs`](./lib.rs). Same boilerplate-copy
procedure as [`../work-signal-pipeline/`](../work-signal-pipeline/README.md):
copy `Cargo.toml`, `build.rs`, `package.toml` from
[`../daily-pr-summary/`](../daily-pr-summary/), rename references, drop in
this `lib.rs`.

## Decision-task wiring (read this carefully)

The `lib.rs` shows the **shape** of an agent-backed decision task but stubs
the actual agent invocation with a placeholder return. The real call goes
through `cloacina-workflow-plugin`'s agent service, which arawn provides
when it loads the workflow. The exact API is still settling — when in doubt,
ask the agent to scaffold a fresh decision-task workflow via
`workflow_create` and read what it generates.

The TODO comment in `classify_severity` marks the swap point.

## Conditional action

`notify_if_p0` early-returns `Ok(())` when no P0 is present. That's the
"only fire when the decision warrants it" pattern — the task still runs
(cloacina has no notion of skipping based on data), but does no work.
For genuine skipping, look at cloacina's trigger rules
(`#[task(trigger_rules = ...)]`) — out of scope for this example.
