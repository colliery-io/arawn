---
id: arawn-product-requirements-document
level: specification
title: "Arawn Product Requirements Document"
short_code: "ARAWN-S-0001"
created_at: 2026-03-28T21:58:36.050290+00:00
updated_at: 2026-03-28T21:58:36.050290+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: false
initiative_id: NULL
---

# Arawn Product Requirements Document

## Overview

Arawn is a personal agentic assistant — a single Rust binary with a TUI that helps a single user stay organized by running tools on schedules, surfacing action items, and providing an interactive chat interface backed by an LLM. Tools are hot-loadable dynamic library plugins — distributed as source, compiled on import. All data is organized into workstreams with filesystem-level isolation.

## System Context

### Actors
- **User**: Interacts via TUI — chats, reviews action items, configures tools and schedules, manages workstreams
- **Tools**: Hot-loadable dynamic library plugins that provide capabilities (email, GitHub, file ops, etc.). Invocable by the agent, the user, or on a schedule.
- **LLM (Claude)**: Provides conversational intelligence, summarization, and intelligent triage of tool output

### External Systems
- **Email (IMAP/JMAP)**: Read-only polling for new messages, flagged items, threads requiring response
- **GitHub API**: Poll repositories for issues, PRs, review requests, notifications
- **Claude API (via muninn/OAuth)**: LLM for chat, summarization, and intelligent triage
- **Future integrations**: Calendar (CalDAV), RSS feeds, other APIs — added as tool plugins without recompiling

### Boundaries
- **In scope**: Chat, workstreams, sessions, hot-loadable tools, scheduled execution, action items, sandboxed tool execution, knowledge persistence
- **Out of scope**: Multi-user, web UI, voice/audio, messaging platforms (WhatsApp/Discord/etc.)

## Requirements

### Functional Requirements

**Workstreams**

| ID | Requirement | Rationale |
|----|-------------|-----------|
| REQ-1.1.1 | User can create, list, rename, and archive workstreams | Primary organizational unit |
| REQ-1.1.2 | Each workstream has its own filesystem directory for data isolation | Enables FS-gating for sandbox and agent scoping |
| REQ-1.1.3 | A default "scratch" space exists for ad-hoc sessions | Not everything belongs to a workstream immediately |
| REQ-1.1.4 | Scratch sessions can be promoted into a named workstream | One-off work sometimes becomes ongoing |

**Sessions (Chat)**

| ID | Requirement | Rationale |
|----|-------------|-----------|
| REQ-1.2.1 | User can start a new chat session within a workstream or scratch | Core interaction model |
| REQ-1.2.2 | Sessions persist messages across restarts | Continuity across TUI sessions |
| REQ-1.2.3 | User can list and resume prior sessions within a workstream | Context switching between topics |
| REQ-1.2.4 | Sessions have access to conversation history for LLM context | LLM needs prior context to be useful |

**Tools (Plugin System)**

| ID | Requirement | Rationale |
|----|-------------|-----------|
| REQ-1.3.1 | Tools are distributed as source code and compiled to dynamic libraries on import | No recompile of the main binary to add capabilities; compile-once-on-import model |
| REQ-1.3.2 | Tools conform to a defined trait interface (inputs, outputs, permissions declaration) | Uniform invocation by agent, user, or scheduler; enables sandbox scoping |
| REQ-1.3.3 | Tools are hot-loadable — new tools can be added and existing tools updated without restarting | Continuous operation; don't interrupt the TUI or running schedules |
| REQ-1.3.4 | User can list installed tools, view their declared capabilities, and remove them | Visibility and control over what the agent can do |
| REQ-1.3.5 | Tools declare their permission requirements (network, filesystem scope, etc.) | Sandbox needs to know what to allow; user needs to approve |
| REQ-1.3.6 | At least email (IMAP) and GitHub tool implementations for v1 | Concrete utility from day one |

**Workflows & Scheduling (Orchestration)**

| ID | Requirement | Rationale |
|----|-------------|-----------|
| REQ-1.9.1 | User can define workflows that chain multiple tools together with data passing between steps | Real tasks require multiple tools — "check email, summarize with LLM, create action items" is three steps |
| REQ-1.9.2 | Workflows support conditional branching based on tool output | Not every email needs triage; not every PR needs attention — the workflow should decide |
| REQ-1.9.3 | Workflows can be triggered on a cadence, scoped to a workstream | Autonomous operation without manual invocation |
| REQ-1.9.4 | Workflows can also be triggered manually or by the agent during chat | Same workflow definition usable in all execution modes |
| REQ-1.9.5 | Scheduled workflows execute in the background without user interaction | Must work unattended |
| REQ-1.9.6 | Workflow output produces action items attributed to the target workstream | Findings must be surfaced, not just logged |
| REQ-1.9.7 | Workflows are defined declaratively (TOML/YAML), not in code | Non-developers should be able to modify workflows; agent can generate them |
| REQ-1.9.8 | Schedules can be paused, resumed, and deleted | User control over autonomous behavior |
| REQ-1.9.9 | TUI displays workflow/schedule status, last-run time, step progress, and errors | Visibility into autonomous operations |
| REQ-1.9.10 | Workflow orchestration is powered by cloacina | Leverage existing orchestration engine rather than reinventing |

**Action Items**

| ID | Requirement | Rationale |
|----|-------------|-----------|
| REQ-1.4.1 | Action items are surfaced in the TUI grouped by workstream | Unified view of what needs attention |
| REQ-1.4.2 | User can review, snooze, or dismiss action items | Not everything needs immediate action |
| REQ-1.4.3 | Action items can be created manually (from chat or TUI) | Not all action items come from scheduled tools |
| REQ-1.4.4 | Action items track source (which tool/schedule or manual) and creation time | Traceability |

**Sandbox**

| ID | Requirement | Rationale |
|----|-------------|-----------|
| REQ-1.5.1 | Tool execution (shell commands, file ops) runs in a sandbox by default | Agent safety — autonomous actions must be constrained |
| REQ-1.5.2 | Sandbox restricts filesystem access to the active workstream's directory | Workstream isolation enforcement |
| REQ-1.5.3 | Sandbox enforces resource limits (CPU, memory, time) | Prevent runaway processes |

**TUI**

| ID | Requirement | Rationale |
|----|-------------|-----------|
| REQ-1.6.1 | TUI provides chat input/output, workstream navigation, and action item review | Single interface for all interaction |
| REQ-1.6.2 | User can switch between workstreams and sessions without restarting | Fluid navigation |
| REQ-1.6.3 | TUI displays installed tools and their status | Visibility into available capabilities |

**Adaptive Prompts**

| ID | Requirement | Rationale |
|----|-------------|-----------|
| REQ-1.8.1 | Agent prompts are stored as configurable templates, not hardcoded strings | Prompts are a tuning surface — must be editable without recompiling |
| REQ-1.8.2 | Agent can append learned context (preferences, patterns, corrections) to its prompt over time | Self-tuning from experience is core to a useful personal assistant |
| REQ-1.8.3 | Prompt history is versioned — prior versions are retained, not overwritten | Rollback if self-tuning degrades quality; diffable for eval comparison |
| REQ-1.8.4 | User can review and edit the agent's accumulated learned context | Human oversight over what the agent "remembers" about how to behave |
| REQ-1.8.5 | Learned context is scoped per-workstream where appropriate | "How to triage GitHub PRs" differs from "how to handle finance emails" |

**Agent Evals**

| ID | Requirement | Rationale |
|----|-------------|-----------|
| REQ-1.7.1 | Eval harness can run scripted scenarios against the agent (chat + tool use) without the TUI | Automated testing of agent behavior during development |
| REQ-1.7.2 | Evals define expected outcomes (action items created, tools called, responses matching criteria) | Measurable quality bar for agent behavior |
| REQ-1.7.3 | Eval results are scored and diffable across runs | Detect regressions when prompts, tools, or workflows change |
| REQ-1.7.4 | Evals cover tool output triage quality (does the LLM correctly prioritize/summarize tool output into action items?) | Tool → action item pipeline is LLM-dependent and needs tuning |
| REQ-1.7.5 | Eval scenarios are versioned alongside the code | Evals drift if not maintained with the codebase |

### Non-Functional Requirements

| ID | Requirement | Rationale |
|----|-------------|-----------|
| NFR-1.1.1 | Memory usage under 500MB under normal load | Must run on resource-constrained hardware |
| NFR-1.1.2 | Single static binary, no runtime dependencies beyond libc | Deployment simplicity |
| NFR-1.1.3 | Cross-compiles for ARM64 (Raspberry Pi 4/5) | Edge deployment target |
| NFR-1.2.1 | Comprehensive test coverage — unit, integration, and end-to-end | Test-first principle; previous attempt failed here |
| NFR-1.2.2 | Cold start under 3 seconds | TUI should feel responsive |
| NFR-1.3.1 | All persistent data in SQLite (no external database) | Single-file portability and backup |
| NFR-1.3.2 | Workstream data physically partitioned on filesystem | FS-gating and sandbox isolation depend on this |

## Constraints

### Technical Constraints
- Rust stable toolchain
- SQLite as sole database (graphqlite for graph layer)
- cloacina for workflow orchestration
- No heavy runtimes (Node.js, JVM, Python)

### Scope Constraints
- Single-user only
- TUI only (no web UI in v1)
- Read-only external integrations in v1 (no sending email, no creating GitHub issues)
- No voice/audio processing