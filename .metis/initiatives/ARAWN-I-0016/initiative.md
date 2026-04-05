---
id: embedded-workflow-engine-cloacina
level: initiative
title: "Embedded workflow engine — cloacina integration for scheduled agent workflows"
short_code: "ARAWN-I-0016"
created_at: 2026-04-05T18:30:28.722553+00:00
updated_at: 2026-04-05T18:30:28.722553+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


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

*To be developed during design phase. Key areas to resolve:*

1. **Session model for decision tasks** — how do workflow-initiated agent sessions relate to workstream sessions? Options: inherit context into fresh session, append to a "workflow" session per workstream, or a new session type.
2. **Agent authoring UX** — scaffold template, what the agent needs to write vs what's generated, compile/package/install tooling.
3. **Context passing** — how cloacina's `Context<Value>` maps to/from arawn's session/workstream data model.
4. **Credential management** — workflow tasks need API keys, tokens. How do they access arawn's secret store?
5. **Error reporting** — when a scheduled workflow fails at 3am, how does the user find out?
6. **Resource limits** — max concurrent workflows, per-task timeouts, LLM token budgets for decision tasks.

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