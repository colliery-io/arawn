# Workflow Examples

Worked examples demonstrating arawn workflow patterns. Read [`docs/src/workflows.md`](../../docs/src/workflows.md) first for the conceptual overview.

## Layout

Each example is a standalone Rust crate (not part of the arawn workspace), matching what the agent's `workflow_create` tool generates.

| Example | Pattern demonstrated | Buildable? |
|---|---|---|
| [`daily-pr-summary/`](./daily-pr-summary/) | Linear pipeline (fetch → process → save) | **Yes** |
| [`work-signal-pipeline/`](./work-signal-pipeline/) | DAG with parallel ingestion + fan-in | Source listing only |
| [`issue-triage/`](./issue-triage/) | Decision task using the agent for LLM-based classification | Source listing only |

Only `daily-pr-summary/` ships as a complete buildable crate. The other two are
annotated `lib.rs` + `README.md` showing the variant patterns. To build either:
copy the `Cargo.toml`, `build.rs`, and `package.toml` from `daily-pr-summary/`,
swap in the example's `lib.rs`, and adjust the workflow `name` references.

## Building the linear example

```sh
cd daily-pr-summary
cargo build --release
```

The build emits a `.cdylib` under `target/release/`. To install into a running
arawn instance, point arawn at the output directory or have the agent install
it via `workflow_create`.

## How to read these

Each `lib.rs` shows three things:

1. The `#[workflow(...)]` module wraps everything.
2. Each `#[task(...)]` async function declares its `id` + `dependencies`. The DAG is the dependency graph.
3. An optional `#[trigger]` function with a cron expression turns the workflow into a scheduled job. Omit it for on-demand workflows.

Task bodies in these examples are illustrative — they show the shape, not the data integration. Where they say `// TODO: real fetch`, swap in your own shell call, HTTP request, or service client.

## Adding a fourth example

The cleanest path: ask the agent.

```
Use workflow_create to build a workflow that <does X>. Look at
examples/workflows/ for the patterns.
```

The agent will discover the patterns from these examples, scaffold a fresh
crate via the existing tool, and install it.
