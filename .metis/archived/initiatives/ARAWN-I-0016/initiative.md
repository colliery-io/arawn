---
id: embedded-workflow-engine-cloacina
level: initiative
title: "Embedded workflow engine — cloacina integration for scheduled agent workflows"
short_code: "ARAWN-I-0016"
created_at: 2026-04-05T18:30:28.722553+00:00
updated_at: 2026-04-09T13:11:33.962940+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: L
initiative_id: embedded-workflow-engine-cloacina
---

# Embedded workflow engine — cloacina integration for scheduled agent workflows Initiative

## Context

Arawn needs autonomous scheduled workflows — the agent authors multi-step task pipelines during a conversation, compiles them as `.cloacina` packages, and they run unattended on a cron cadence or trigger. This enables use cases like daily GitHub triage, email summarization, deployment monitoring, and any repeating multi-step automation.

Cloacina (colliery-io, v0.4.0) is an embedded workflow engine for Rust that handles DAG execution, cron scheduling, trigger-based firing, retry policies, and a pluggable workflow package system (`.cloacina` files). Both projects share the `fidius` FFI framework and tokio async runtime. Arawn already has a server mode — cloacina's `DefaultRunner` slots in as another background service alongside the engine, websocket server, and config watcher.

### Key insight: the agent is the developer

The agent authors real Rust workflow code during a conversation — using `#[workflow]` and `#[task]` macros, declaring dependencies, setting up cron triggers in `package.toml`. It compiles the crate, packages it, and drops it in the watched directory. The reconciler handles the rest.

### Three task flavors

Workflows are heterogeneous DAGs mixing:

- **Data tasks** — pure code. Hit APIs, query databases, scrape feeds. Fast, deterministic, retryable. No LLM.
- **Decision tasks** — multi-turn agent sessions. Receive upstream data + workstream context. The full QueryEngine loop with tools — the agent can take multiple turns to reason, read files, and produce structured decisions.
- **Action tasks** — execute decisions. Write files, send notifications, schedule follow-ups, spawn sub-workflows.

### Example: morning GitHub triage

```
morning-github-triage (cron: 0 8 * * 1-5)

[data]     fetch_github  →  GitHub API → raw PRs, notifications, issues
                |
[decision] triage         →  QueryEngine + workstream context ("oss-maintenance")
                |              multi-turn: reads PR diffs, checks priorities,
                |              decides what needs attention
                |
[action]   schedule_work  →  write daily todo, auto-merge dependabot,
                              flag critical PRs
```

Decision tasks run within the workstream's existing context — not blank-slate sessions. The exact session model (append to existing, inherit context into new, dedicated workflow session type) is a design question for the design phase.

## Goals & Non-Goals

**Goals:**
- Embed cloacina `DefaultRunner` in arawn's server startup pipeline (separate `workflows.db`)
- Agent can author, compile, and install `.cloacina` workflow packages during a conversation
- Cron-scheduled and trigger-based workflow execution runs unattended in the server background
- Workflow tasks can invoke the full QueryEngine agentic loop (multi-turn, with tools) for decision steps
- Decision tasks inherit workstream context for informed reasoning
- `FilesystemWorkflowRegistry` watches `~/.arawn/workflows/` for hot-loading packages
- Agent-facing tools for workflow management: create, schedule, list, delete

**Non-Goals:**
- Standalone daemon process (cloacina runs inside the existing server, not separately)
- Remote/cloud workflow execution (local only for now)
- Visual workflow editor / TUI workflow builder
- PostgreSQL backend (SQLite only for now)
- Multi-tenant isolation (single user)

## Use Cases

### Use Case 1: Agent authors a scheduled workflow
- **Actor**: User via agent conversation
- **Scenario**: User describes a recurring automation need. Agent writes a Rust workflow crate with tasks, dependencies, and a cron trigger. Agent compiles it, packages as `.cloacina`, installs to `~/.arawn/workflows/`. Reconciler picks it up and registers the cron schedule.
- **Expected Outcome**: Workflow runs autonomously on schedule from then on.

### Use Case 2: Data → Decision → Action pipeline
- **Actor**: Scheduled workflow (unattended)
- **Scenario**: Cron fires. Data task fetches external data (API calls). Decision task runs a multi-turn agent session with workstream context to triage/prioritize. Action task executes the decisions (writes files, sends notifications).
- **Expected Outcome**: Structured output produced and persisted without human intervention.

### Use Case 3: Workflow marketplace
- **Actor**: User
- **Scenario**: User installs a pre-packaged workflow from the marketplace. Package drops into the watched directory. Reconciler compiles and loads it. User configures cron schedule.
- **Expected Outcome**: Curated workflow templates installable like plugins.

## Architecture

### Server integration

```
arawn start →
  ├── Store (SQLite — arawn.db)
  ├── QueryEngine (LLM + tools)
  ├── WebSocket server
  ├── ConfigWatcher
  ├── PluginRuntime
  └── DefaultRunner (cloacina — workflows.db)  ← NEW
       ├── TaskScheduler (dependency resolution)
       ├── Unified Scheduler (cron + triggers)
       └── RegistryReconciler (watches ~/.arawn/workflows/)
```

Single process, single tokio runtime. Separate SQLite databases for isolation.

### Crate structure

```
crates/
  arawn-workflow/     # NEW — embeds cloacina, provides:
                      #   - AgentTaskExecutor (wraps QueryEngine for decision tasks)
                      #   - Workflow scaffold/template for agent authoring
                      #   - Package build utilities
                      #   - Runner configuration
  arawn-engine/       # EXISTING — gains workflow trigger integration
  arawn/              # EXISTING — wires DefaultRunner into server startup
```

### Package authoring flow

```
Agent conversation:
  1. Agent generates workflow crate (Cargo.toml, package.toml, src/lib.rs)
  2. Agent runs `cargo build --release` to compile cdylib
  3. Agent packages as .cloacina (fidius pack)
  4. Agent copies to ~/.arawn/workflows/

Background:
  5. RegistryReconciler detects new package
  6. Loads cdylib, calls get_task_metadata()
  7. Registers tasks, workflows, triggers
  8. Cron scheduler picks up schedule
  9. Runs on cadence from then on
```

## Detailed Design

### Embedding cloacina's DefaultRunner

Cloacina supports three deployment modes: embedded library, daemon, and API server. We use **embedded library mode** — `DefaultRunner` lives inside the arawn server process, shares the tokio runtime, and uses SQLite for persistence.

**Initialization** (in `crates/arawn/src/main.rs`, serve mode block):
```rust
use cloacina::prelude::*;

// Separate database for workflow state (not arawn.db)
let workflows_db = format!("sqlite://{}/workflows.db", data_dir);

let mut runner_config = DefaultRunnerConfig::default();
runner_config.enable_registry_reconciler = true;
runner_config.enable_cron_scheduling = true;
runner_config.registry_storage_path = Some(PathBuf::from(&data_dir).join("workflows"));

let runner = DefaultRunner::with_config(&workflows_db, runner_config).await?;

// Start background services (scheduler, reconciler, cron)
let runner_handle = runner.start().await?;
```

The runner runs alongside the existing server components. On shutdown, `runner.shutdown()` drains in-flight pipelines.

**Key `DefaultRunner` capabilities used:**
- `runner.execute(workflow_name, context)` — programmatic execution
- `RegistryReconciler` — watches `~/.arawn/workflows/` for `.cloacina` packages
- `CronScheduler` — time-based execution with recovery for missed runs
- `TaskScheduler` — DAG dependency resolution
- `ThreadTaskExecutor` — concurrent task execution with semaphore

### Workflow package authoring (what the agent writes)

Workflow packages use `cloacina-workflow` (not the full `cloacina` crate) — minimal types + macros, fast compilation, no database drivers.

**Cargo.toml** (generated by agent):
```toml
[package]
name = "github-triage"
version = "1.0.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
cloacina-workflow = { version = "0.4", features = ["packaged"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
```

**src/lib.rs** (generated by agent):
```rust
use cloacina_workflow::{workflow, task, trigger, Context, TaskError};
use serde_json::Value;

#[workflow(name = "github_triage", package = "github_triage")]
pub mod github_triage {
    use super::*;

    #[task(id = "fetch_prs", dependencies = [], retry_attempts = 3)]
    pub async fn fetch_prs(context: &mut Context<Value>) -> Result<(), TaskError> {
        // Data task: hit GitHub API
        let prs = reqwest::get("https://api.github.com/repos/org/repo/pulls")
            .await.map_err(|e| TaskError::ExecutionFailed {
                message: e.to_string(), task_id: "fetch_prs".into(), timestamp: chrono::Utc::now(),
            })?
            .json::<Value>().await.map_err(|e| TaskError::ExecutionFailed {
                message: e.to_string(), task_id: "fetch_prs".into(), timestamp: chrono::Utc::now(),
            })?;
        context.insert("prs", prs)?;
        Ok(())
    }

    #[task(id = "triage", dependencies = ["fetch_prs"])]
    pub async fn triage(context: &mut Context<Value>) -> Result<(), TaskError> {
        // Decision task: will be intercepted by AgentTaskExecutor
        // The context key "arawn_decision" signals this is an agent task
        let prs = context.get("prs").unwrap_or(&Value::Null).clone();
        context.insert("arawn_decision", serde_json::json!({
            "prompt": format!("Triage these PRs: {}", prs),
            "workstream": "oss-maintenance",
        }))?;
        Ok(())
    }
}

#[trigger(on = "github_triage", cron = "0 8 * * 1-5")]
pub async fn weekday_morning() {}
```

**Build + Package flow** (agent runs these via shell tool):
```bash
cd /tmp/github-triage && cargo build --release
cloacina-ctl package . -o ~/.arawn/workflows/github-triage.cloacina
```

The `#[workflow]` macro with `features = ["packaged"]` generates FFI exports. `cloacina-ctl package` bundles the cdylib into a `.cloacina` archive. The reconciler picks it up automatically.

### Decision task executor (Phase 3)

Decision tasks need the full arawn QueryEngine. The approach:

1. Register a custom `TaskExecutor` with the `DefaultRunner` that intercepts tasks marked as decision tasks (via `arawn_decision` in context)
2. The executor creates an arawn session in the specified workstream, injects upstream context from the cloacina pipeline, runs the engine loop, and writes the result back to the cloacina context

```rust
// crates/arawn-workflow/src/agent_executor.rs
pub struct AgentTaskExecutor {
    store: Arc<Mutex<Store>>,
    llm: Arc<dyn LlmClient>,
    registry: Arc<ToolRegistry>,
    // ... same deps as LocalService
}

impl TaskExecutorTrait for AgentTaskExecutor {
    async fn execute(&self, event: TaskReadyEvent) -> Result<ExecutionResult, ExecutorError> {
        // Check if this is a decision task
        let context = load_context(event.pipeline_execution_id);
        if let Some(decision) = context.get("arawn_decision") {
            // Run full QueryEngine loop
            let prompt = decision["prompt"].as_str().unwrap();
            let workstream = decision["workstream"].as_str().unwrap();
            // ... create session, run engine, collect result
        } else {
            // Regular data/action task — run directly
            // ... standard execution
        }
    }
}
```

The `DefaultDispatcher` routes tasks to executors based on glob patterns — we register the `AgentTaskExecutor` for decision task patterns.

### Context bridge: cloacina ↔ arawn

Cloacina uses `Context<Value>` (serde_json). Arawn uses session messages + ToolContext. The bridge:

- **Into arawn**: Decision task's `arawn_decision.prompt` becomes the user message. Upstream pipeline data is injected as a system prompt preamble.
- **Out of arawn**: The engine's final_text response is written back to the cloacina context as the task result.

### Design decisions

1. **Session model**: Default is fresh session per execution (avoid context bloat — prior context comes from workstream KB). But the workflow itself can override this via config (e.g., append to a named session for workflows that need conversational continuity).
2. **Credentials**: Inherit server process environment. No secrets backend yet — workflows get the same env vars as arawn (GROQ_API_KEY, etc.).
3. **Error reporting**: Failures logged to cloacina's execution DB. `/workflows` TUI command shows recent execution status (pass/fail/running). Notifications are a future concern.
4. **Resource limits**: Use cloacina's `DefaultRunnerConfig` defaults (`max_concurrent_tasks`, `task_timeout`, retry policies). Decision task LLM budget set via `QueryEngineConfig::max_tokens` on the `AgentTaskExecutor`.

## Alternatives Considered

- **Custom scheduler (no cloacina)**: Reimplements DAG execution, cron, retries, persistence. Significant effort for solved problems.
- **Prompt-chain DSL (no compiled code)**: Agent describes tasks declaratively, interpreter runs them. More flexible but less powerful — can't do arbitrary API integrations, harder to debug, no type safety.
- **Separate daemon process**: Adds process management complexity. The server already runs continuously — another background service is simpler.
- **Claude Code's cron model (single-prompt-per-fire)**: Too limited. No multi-step DAGs, no inter-task data flow, no retries, no execution history.

## Implementation Plan

*To be refined during design phase. Rough phasing:*

**Phase 1**: `arawn-workflow` crate — embed cloacina, wire `DefaultRunner` into server startup, separate workflows.db, reconciler watching `~/.arawn/workflows/`

**Phase 2**: Agent authoring — scaffold template, compile/package/install tools, agent can create and install a basic workflow

**Phase 3**: Decision task executor — `AgentTaskExecutor` that runs QueryEngine with workstream context for multi-turn decision steps

**Phase 4**: Agent-facing tools — `WorkflowCreate`, `WorkflowSchedule`, `WorkflowList`, `WorkflowDelete` for managing workflows from conversation

**Phase 5**: Workflow marketplace integration — `.cloacina` packages installable via existing plugin marketplace system