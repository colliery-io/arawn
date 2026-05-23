# daily-pr-summary

A linear three-task workflow that runs every weekday morning and writes a
markdown briefing of open pull requests.

```
fetch_prs ─▶ summarize_prs ─▶ save_briefing
```

## Pattern

Strict sequential DAG. Each task adds something to the shared `Context<Value>`
that the next task reads.

## Build

```sh
cd examples/workflows/daily-pr-summary
cargo build --release
```

Output: `target/release/libdaily_pr_summary.dylib` (or `.so` on Linux).

## Install into a running arawn

```sh
mkdir -p ~/.arawn/workflows/daily-pr-summary
cp target/release/libdaily_pr_summary.dylib \
   ~/.arawn/workflows/daily-pr-summary/
cp package.toml \
   ~/.arawn/workflows/daily-pr-summary/
```

(Or ask the agent to do it: "use workflow_create to install
daily-pr-summary from the example crate at examples/workflows/daily-pr-summary".)

## Reading the source

`src/lib.rs` shows three things:

- The `#[workflow(...)]` macro wraps a module that holds all tasks for one DAG.
- Each `#[task(id = "...", dependencies = [...])]` declares a node. The
  dependency list IS the DAG edge.
- The `#[trigger(on = "...", cron = "...")]` function turns the workflow into
  a scheduled job. Drop it for an on-demand workflow that the agent or RPC
  invokes manually.

## Customizing

Both `fetch_prs` and `save_briefing` have stubbed bodies (search for `TODO:
real fetch` and `TODO: real write`) that show the real-life shape commented
out. Uncomment, swap in your repo / output path, rebuild.
