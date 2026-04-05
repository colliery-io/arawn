---
id: arawn-technical-design
level: specification
title: "Arawn Technical Design"
short_code: "ARAWN-S-0002"
created_at: 2026-03-28T21:58:36.555992+00:00
updated_at: 2026-03-28T21:58:36.555992+00:00
parent: ARAWN-V-0001 
archived: false

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: falseno in
initiative_id: NULL
---

# Arawn Technical Design

## Overview

Technical architecture for Arawn — a single Rust binary personal assistant with TUI, hot-loadable tool plugins, scheduled execution, and workstream-based data isolation. This document covers crate structure, tool plugin system, data model, filesystem layout, key subsystem designs, and integration patterns.

## Crate Structure

```
arawn/
├── Cargo.toml              (workspace)
├── crates/
│   ├── arawn-core/         Domain types, traits, tool ABI, error types
│   ├── arawn-store/        SQLite persistence, graphqlite integration, migrations
│   ├── arawn-sandbox/      Sandboxed execution (bubblewrap/landlock)
│   ├── arawn-tools/        Tool plugin loader, compiler, registry, scheduler
│   ├── arawn-tui/          Ratatui-based terminal interface
│   └── arawn/              Binary crate — wires everything together
├── tools/                  Built-in tool source packages
│   ├── arawn-tool-email/   Email (IMAP/JMAP) tool plugin
│   └── arawn-tool-github/  GitHub API tool plugin
```

### Crate Responsibilities

**arawn-core**: Zero external dependencies beyond serde/chrono/uuid. Defines:
- `Workstream`, `Session`, `Message`, `ActionItem`, `Workflow`, `Schedule` domain types
- `Tool` trait — the stable ABI contract that all tool plugins implement
- `ToolManifest` — declares name, version, permissions, input/output schemas
- Error types, IDs, enums (ActionItemStatus, ToolState, etc.)

**arawn-store**: Owns all SQLite access. Depends on arawn-core.
- rusqlite for entity tables (workstreams, sessions, messages, action_items, tools, workflows, schedules)
- graphqlite for relationship graph (action item → tool, session → workstream, etc.)
- refinery for schema migrations
- Exposes a `Store` struct that encapsulates both databases

**arawn-sandbox**: Sandboxed command/process execution.
- Linux: bubblewrap (bwrap) or landlock for filesystem isolation
- macOS: sandbox-exec (dev) / process resource limits
- Accepts a workstream path as the allowed filesystem root
- Enforces CPU, memory, and time limits
- Tools execute within sandbox; permissions checked against `ToolManifest` declarations

**arawn-tools**: Tool plugin lifecycle management.
- **Compiler**: Takes tool source (Rust crate), compiles to `.dylib`/`.so` via `cargo build --release`
- **Loader**: `dlopen`/`libloading` to load compiled tool libraries at runtime
- **Registry**: Tracks installed tools, their manifests, compiled library paths, and versions
- **Orchestrator**: Runs workflows (tool chains) on configured cadences via cloacina + tokio timers
- **Hot-reload**: Watches for new/updated tools, recompiles and swaps without restart

**arawn-tui**: Terminal interface. Depends on arawn-core, arawn-store.
- ratatui + crossterm
- Panels: workstream sidebar, chat/session view, action item list
- Keyboard-driven navigation
- Async event loop for background tool/schedule notifications

**arawn** (binary): Composition root.
- Clap CLI for startup config
- Initializes store, loads tool registry, starts scheduler, launches TUI
- Wires tokio runtime for concurrent TUI + background tool execution

**tools/** (out-of-tree): Built-in tool source packages, compiled on first run.
- Each tool is a standalone Rust crate that depends only on arawn-core (for the `Tool` trait)
- Ships with the repo but compiled separately — same pipeline as third-party tools
- `arawn-tool-email`: IMAP/JMAP polling, produces action items for new/flagged mail
- `arawn-tool-github`: GitHub API polling for issues, PRs, review requests

## Filesystem Layout

```
~/.arawn/                           # Application root
├── config.toml                     # Global config (LLM settings, defaults)
├── arawn.db                        # SQLite database (entities, FTS)
├── graph.db                        # graphqlite relation graph
├── tools/
│   ├── sources/                    # Tool source code (Rust crates)
│   │   ├── arawn-tool-email/
│   │   └── arawn-tool-github/
│   ├── compiled/                   # Compiled dylibs (.dylib on macOS, .so on Linux)
│   │   ├── arawn_tool_email.dylib
│   │   └── arawn_tool_github.dylib
│   └── registry.toml              # Installed tools, versions, manifest cache
├── workstreams/
│   ├── finances/
│   │   ├── sessions/
│   │   │   ├── 2026-03-28-budget-review.jsonl
│   │   │   └── 2026-03-25-tax-prep.jsonl
│   │   ├── workflows/
│   │   │   └── email-triage.toml  # "fetch email → filter → LLM triage → action items"
│   │   └── sandbox/               # Isolated area for sandboxed tool execution
│   ├── home-maintenance/
│   │   ├── sessions/
│   │   ├── workflows/
│   │   └── sandbox/
│   └── ...
├── scratch/
│   ├── sessions/
│   └── sandbox/
└── logs/
    └── tools.log
```

Key design decisions:
- **Physical partitioning**: Each workstream is a directory. Sandbox restricts to `workstreams/<name>/`.
- **Sessions as JSONL**: Append-only message logs. Cheap, grep-friendly, easy to replay.
- **Tool sources alongside compiled artifacts**: Source stays on disk for recompilation on updates or architecture changes.
- **Workflow definitions as TOML**: Per-workstream, declarative, human-readable. A workflow chains tools with data passing, conditions, and optional scheduling.
- **SQLite databases at root**: Shared across workstreams for cross-workstream queries (e.g., "all action items"). The DB references workstream paths but doesn't store message content — that lives in the JSONL files.

## Data Model

### SQLite Tables (arawn-store)

```sql
-- Workstreams
CREATE TABLE workstreams (
    id          TEXT PRIMARY KEY,    -- UUID
    name        TEXT NOT NULL UNIQUE,
    slug        TEXT NOT NULL UNIQUE, -- filesystem-safe name
    created_at  TEXT NOT NULL,
    archived    INTEGER DEFAULT 0
);

-- Sessions
CREATE TABLE sessions (
    id              TEXT PRIMARY KEY,
    workstream_id   TEXT,             -- NULL = scratch
    title           TEXT,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,
    message_count   INTEGER DEFAULT 0,
    FOREIGN KEY (workstream_id) REFERENCES workstreams(id)
);

-- Action Items
CREATE TABLE action_items (
    id              TEXT PRIMARY KEY,
    workstream_id   TEXT,
    source_type     TEXT NOT NULL,     -- 'workflow', 'manual', 'chat'
    source_id       TEXT,              -- workflow ID or session ID
    title           TEXT NOT NULL,
    body            TEXT,
    status          TEXT NOT NULL DEFAULT 'pending',  -- pending, snoozed, dismissed, done
    snoozed_until   TEXT,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,
    FOREIGN KEY (workstream_id) REFERENCES workstreams(id)
);

-- Installed Tools (registry mirror in SQLite for queries)
CREATE TABLE tools (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL UNIQUE,
    version         TEXT NOT NULL,
    source_path     TEXT NOT NULL,     -- path to source crate
    lib_path        TEXT NOT NULL,     -- path to compiled dylib
    manifest        TEXT NOT NULL,     -- JSON: permissions, input/output schemas
    state           TEXT NOT NULL DEFAULT 'active', -- active, disabled
    compiled_at     TEXT NOT NULL,
    created_at      TEXT NOT NULL
);

-- Workflows (declarative tool chains, definitions stored as TOML on disk)
CREATE TABLE workflows (
    id              TEXT PRIMARY KEY,
    workstream_id   TEXT NOT NULL,
    name            TEXT NOT NULL,
    description     TEXT,
    file_path       TEXT NOT NULL,     -- path to workflow TOML definition
    step_count      INTEGER NOT NULL,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,
    FOREIGN KEY (workstream_id) REFERENCES workstreams(id)
);

-- Schedules (binds a workflow to a cadence)
CREATE TABLE schedules (
    id              TEXT PRIMARY KEY,
    workflow_id     TEXT NOT NULL,
    name            TEXT NOT NULL,
    cadence_secs    INTEGER NOT NULL,
    state           TEXT NOT NULL DEFAULT 'active', -- active, paused
    last_run_at     TEXT,
    last_error      TEXT,
    created_at      TEXT NOT NULL,
    FOREIGN KEY (workflow_id) REFERENCES workflows(id)
);
```

### graphqlite Relations

```
(Session)-[:BELONGS_TO]->(Workstream)
(ActionItem)-[:BELONGS_TO]->(Workstream)
(ActionItem)-[:PRODUCED_BY]->(Workflow)
(ActionItem)-[:SPAWNED_FROM]->(Session)
(Workflow)-[:BELONGS_TO]->(Workstream)
(Workflow)-[:USES]->(Tool)          // one edge per tool referenced in workflow steps
(Schedule)-[:RUNS]->(Workflow)
```

The graph layer is intentionally thin in v1 — primarily for traversal queries ("show me everything related to this workstream") rather than complex graph analytics.

## Key Subsystem Designs

### Tool Plugin System

**Tool ABI contract** (defined in arawn-core):
```rust
/// Every tool plugin exports these symbols from its dylib
#[no_mangle] pub extern "C" fn tool_manifest() -> ToolManifest;
#[no_mangle] pub extern "C" fn tool_create(config: &[u8]) -> Box<dyn Tool>;

pub trait Tool: Send + Sync {
    /// Execute the tool with given input, return structured output
    fn execute(&self, input: ToolInput) -> Result<ToolOutput, ToolError>;
    /// Human-readable description for the LLM to understand capabilities
    fn description(&self) -> &str;
}
```

**Import flow:**
1. User provides tool source (git URL, local path, or archive)
2. Source is placed in `~/.arawn/tools/sources/<tool-name>/`
3. `arawn-tools` compiler runs `cargo build --release --target-dir ...` on the crate
4. Compiled `.dylib`/`.so` placed in `~/.arawn/tools/compiled/`
5. `libloading` loads the library, calls `tool_manifest()` to read permissions/schemas
6. Tool registered in SQLite + `registry.toml`
7. Tool available immediately for chat invocation, agent use, or scheduling

**Hot-reload:**
- Filesystem watcher on `tools/sources/` detects changes
- Recompile triggered, new dylib loaded, old one unloaded after in-flight calls drain
- Registry updated atomically

**ABI stability:**
- `Tool` trait and `ToolManifest` are the stable contract — changes require a major version bump
- Tools depend only on `arawn-core` — no access to store, sandbox internals, or TUI

### Workflow Orchestrator

Workflows chain tools together into multi-step pipelines. Powered by cloacina for DAG execution, with scheduling layered on top.

**Workflow definition** (declarative TOML):
```toml
# ~/.arawn/workstreams/finances/workflows/email-triage.toml
[workflow]
name = "email-triage"
description = "Check finance email, summarize new messages, create action items"

[[steps]]
id = "fetch"
tool = "arawn-tool-email"
config = { folder = "INBOX", since = "last_run", filter = "unread" }

[[steps]]
id = "filter"
tool = "arawn-tool-filter"
input_from = "fetch"
config = { rules = ["subject contains 'invoice'", "from in contacts"] }
condition = "fetch.output.count > 0"

[[steps]]
id = "triage"
tool = "builtin:llm-triage"
input_from = "filter"
config = { prompt = "triage", create_action_items = true }
condition = "filter.output.count > 0"
```

**Key concepts:**
- **Steps**: Each step invokes a tool with config. Steps declare `input_from` to receive prior step output.
- **Conditions**: Steps can be skipped based on prior step output (`condition` field). Supports simple expressions.
- **Built-in tools**: `builtin:llm-triage`, `builtin:llm-summarize` — thin wrappers that pipe tool output through the LLM with a specified prompt template.
- **DAG execution**: cloacina handles dependency resolution, parallel execution of independent steps, and error propagation.

**Execution flow:**
```
tokio::spawn → loop {
    for each active schedule:
        if now >= last_run_at + cadence:
            workflow = load_workflow(schedule.workflow_path)
            dag = cloacina::compile(workflow.steps)
            result = dag.execute(|step| {
                tool = registry.load(step.tool)
                sandbox.execute(tool, step.config, schedule.workstream)
            })
            match result:
                Ok(output) → store.insert_action_items(output.items)
                Err(e) → store.update_schedule_error(id, e)
            store.update_schedule_last_run(id, now)
    sleep(tick_interval)
}
```

- Runs in a background tokio task alongside the TUI
- cloacina compiles workflow steps into a DAG and manages execution order
- Each step's tool executes within the sandbox, scoped to the schedule's workstream
- Step outputs are passed as inputs to downstream steps via cloacina's data-passing mechanism
- Errors at any step halt the workflow (configurable: fail-fast vs continue-on-error)
- Same workflow definition reusable across workstreams with different configs
- Workflows triggerable three ways: on schedule, manually from TUI, or by the agent during chat

### Sandbox Architecture

Two-tier approach depending on platform:

**Linux (production target)**:
- bubblewrap (bwrap) for filesystem namespace isolation
- Bind-mount only the target workstream directory as read-write
- Mount /usr, /lib as read-only for tool binaries
- seccomp filter for syscall restriction (optional, v2)
- cgroups for resource limits

**macOS (development)**:
- Process-level resource limits (setrlimit)
- Filesystem access checking at the application level (validate paths before operations)
- Not a true sandbox — acceptable for dev, not for production

### LLM Integration

- muninn crate handles Claude OAuth token management
- Chat messages sent as structured API calls with conversation history
- Tool output can optionally be summarized by LLM before creating action items
- LLM calls are async and non-blocking to the TUI

### Session Promotion (Scratch → Workstream)

1. User selects a scratch session and chooses "promote"
2. User picks or creates a target workstream
3. Session JSONL file moves from `scratch/sessions/` to `workstreams/<target>/sessions/`
4. `sessions.workstream_id` updated in SQLite
5. Graph edges updated (BELONGS_TO → new workstream)

### Adaptive Prompt System

Prompts are not static — the agent starts with base prompts and refines them over time based on experience.

**Structure:**
```
~/.arawn/prompts/
├── base/
│   ├── chat.md              # Base system prompt for conversations
│   ├── triage.md            # Base prompt for tool output triage
│   └── tool-selection.md    # Base prompt for tool dispatch decisions
├── learned/
│   ├── global.md            # Cross-workstream learned context
│   └── workstreams/
│       ├── finances.md      # Learned context specific to finances workstream
│       └── github-oss.md    # Learned context specific to OSS workstream
└── history/
    ├── chat.v1.md           # Prior versions for rollback/diffing
    ├── chat.v2.md
    └── ...
```

**How it works:**
- **Base prompts** are shipped with the binary but copied to `~/.arawn/prompts/base/` on first run. User can edit them directly.
- **Learned context** is appended material the agent accumulates: user corrections ("don't flag PR drafts as action items"), observed patterns ("user always snoozes meeting reminders on weekends"), and workstream-specific knowledge.
- **Assembly**: At prompt time, the active prompt = base template + global learned context + workstream-specific learned context (if applicable).
- **Versioning**: When a base prompt or learned context file is modified, the prior version is copied to `history/` with a version suffix. Eval runs can pin a specific prompt version.

**Self-tuning loop:**
1. Agent takes an action (triage, tool call, response)
2. User provides implicit feedback (snooze, dismiss, correction in chat) or explicit feedback ("stop doing X")
3. Agent proposes an addition to learned context
4. Addition is written to the appropriate learned context file (global or workstream-scoped)
5. Next interaction assembles the updated prompt

**Guardrails:**
- Learned context is append-only text, not executable — no prompt injection risk from the learning loop
- User can view and edit learned context via TUI or directly on disk
- Eval harness can run with/without learned context to measure its impact

### Agent Eval Harness

Evals run the agent's decision-making logic (LLM calls + tool dispatch) against scripted scenarios without requiring the TUI or live external services.

**Components:**
- **Scenario files**: TOML/YAML definitions with setup state, user input sequence, and expected outcomes
- **Mock backends**: Pluggable mock implementations of the `Tool` trait and external APIs (email, GitHub) that return canned data
- **Eval runner**: Drives scenarios through the agent, captures tool calls made, action items produced, and response content
- **Scorer**: Compares actual outcomes against expected — exact matches for tool calls/action items, LLM-as-judge or regex for response quality
- **Report**: Per-scenario pass/fail with diffs, aggregate score, comparison against previous run

**What gets evaluated:**
- Chat quality: Does the agent respond appropriately given conversation context?
- Tool selection: Does the agent pick the right tools for a given instruction?
- Tool output triage: Given raw tool output, does the LLM produce correct action items with appropriate priority?
- Sandbox compliance: Does the agent respect workstream boundaries in its tool calls?
- Regression detection: Score deltas across prompt/model/workflow changes

**Integration:**
- Runs as `cargo test` integration tests (no special runner needed)
- Scenario files live in `evals/` at the workspace root, versioned with the code
- CI runs evals on prompt or workflow changes; scores tracked over time
- Mock tools implement the same `Tool` trait as production plugins (no separate mock framework)

## Architecture Framing

### Decision Area: Tool ABI Design
- **Context**: Tools are compiled dylibs loaded at runtime — the ABI between arawn and tool plugins must be stable
- **Constraints**: Rust has no stable ABI; `extern "C"` limits expressiveness; must support async tools
- **Required Capabilities**: Stable function signatures, structured input/output, error propagation, version negotiation
- **ADR**: TBD — research needed on `abi_stable` crate vs hand-rolled `extern "C"` FFI vs cbindgen approach

### Decision Area: Sandbox Technology
- **Context**: Agent tool execution must be isolated per-workstream
- **Constraints**: Must work on Linux ARM64 (Pi); macOS for dev is acceptable with weaker guarantees
- **Required Capabilities**: Filesystem isolation, resource limits, workstream-scoped access
- **ADR**: TBD — research needed on bubblewrap vs landlock vs both

### Decision Area: Message Storage Format
- **Context**: Chat messages need persistence and LLM replay
- **Constraints**: Must be efficient for append and sequential read; searchable
- **Required Capabilities**: Append-only writes, full conversation replay, optional FTS
- **ADR**: TBD — JSONL proposed, alternatives (SQLite messages table) worth evaluating

### Decision Area: LLM Context Management
- **Context**: Conversations grow; LLM context windows are finite
- **Constraints**: Must work within Claude's context limits; don't re-send entire history every turn
- **Required Capabilities**: Sliding window or summarization of older messages
- **ADR**: TBD

## Constraints

### Technical Constraints
- Rust stable toolchain, must cross-compile for ARM64
- SQLite as sole database engine (graphqlite wraps SQLite)
- cloacina for workflow orchestration of multi-step agent tasks
- No heavy runtimes
- bubblewrap available on Linux only — macOS sandbox is dev-quality

### Testing Constraints
- Every crate must have unit tests for core logic
- Integration tests for store operations against real SQLite (no mocks)
- End-to-end tests for tool → workflow → schedule → action item pipeline
- TUI tested via headless/event-driven test harness where feasible