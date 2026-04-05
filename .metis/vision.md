---
id: arawn
level: vision
title: "arawn"
short_code: "ARAWN-V-0001"
created_at: 2026-03-28T14:20:34.170334+00:00
updated_at: 2026-03-28T21:58:32.054870+00:00
archived: false

tags:
  - "#vision"
  - "#phase/published"


exit_criteria_met: false
initiative_id: NULL
---

# Arawn Vision

A lightweight, self-hosted personal agentic assistant that runs scheduled tasks, monitors channels (email, GitHub, etc.), surfaces action items, and provides an interactive chat interface — all from a single Rust binary with a minimal resource footprint.

## Purpose

Help you stay organized and on top of non-work life. Arawn watches, checks, summarizes, and nudges — so you don't have to keep everything in your head.

## Current State

- Previous attempt grew too large without testing; core functionality never stabilized
- colliery-io ecosystem (graphqlite, cloacina) under active refinement and available as foundations
- Clotho demonstrates the viability of: Rust TUI (ratatui), graph-backed data (graphqlite), multi-crate workspace architecture

## Future State

A single Rust binary that:

- Provides an interactive TUI for chat and reviewing action items
- Runs autonomous tasks on configurable schedules (check email, scan GitHub repos for issues/PRs, etc.)
- Surfaces findings as actionable items you can review, snooze, or dismiss
- Organizes all work into workstreams with filesystem-level isolation
- Persists state in SQLite (graphqlite for relationships, raw tables for entities)
- Orchestrates multi-step agent workflows via cloacina
- Integrates with an LLM for natural conversation and intelligent summarization

## Major Features

### Workstreams
The primary organizational unit. A workstream is a logical grouping (e.g., "Home Maintenance", "Finances", "Health", "Side Projects") that partitions data at the filesystem level. Sessions, watchers, and action items all belong to a workstream. Filesystem isolation enables FS-gating so agents and tools can be scoped to only the files within a given workstream.

A **scratch space** exists for one-off tasks and ad-hoc sessions that don't belong to a workstream yet. Scratch sessions can be promoted into a workstream when they prove to have lasting value.

### Interactive Chat
TUI-based conversational interface for asking questions, giving instructions, and reviewing what Arawn has found. Sessions are persistent and organized within their parent workstream.

### Scheduled Watchers
Configurable tasks that run on cadences (check email, poll GitHub repos for issues/PRs, etc.) and produce action items. Each watcher belongs to a workstream and its output is scoped accordingly.

### Action Item Surface
A unified view of things that need your attention, sourced from watchers or manual capture. Action items belong to a workstream and can be reviewed, snoozed, or dismissed from the TUI.

### Knowledge Persistence
Conversations, findings, and action items persisted across sessions via SQLite + graphqlite. Workstream-level partitioning means data is physically organized on disk, not just tagged.

### Sandboxed Tool Execution
Agent-driven tool execution (shell commands, API calls, file operations) runs inside a sandbox by default. Workstream-level filesystem isolation pairs with the sandbox to ensure tools can only access data within their workstream's partition. This is a safety prerequisite — not a later enhancement — since watchers and chat-driven actions execute autonomously.

### LLM Integration
Claude (via muninn/OAuth) for chat, summarization, and intelligent triage of watcher output.

## Success Criteria

1. Can hold a useful chat conversation with context persistence
2. Workstreams organize sessions, watchers, and action items with filesystem isolation
3. At least 2 working watchers (email + GitHub) running on schedule
4. Action items surfaced in TUI from watcher output
5. Scratch space works for ad-hoc sessions with promotion path to workstreams
6. Stable on a low-resource system (<500MB memory)
7. Comprehensive test coverage from day one

## Principles

### Test-First
Nothing ships without tests. This is what went wrong last time. Every initiative delivers tested functionality.

### Incremental Delivery
Each initiative delivers working, tested functionality end-to-end. No building five subsystems in parallel and hoping they integrate.

### Small Footprint
Rust, SQLite, single binary. No heavy runtimes. Must run comfortably on resource-constrained hardware.

### Simple Before Smart
Get the plumbing working before adding intelligence. A working watcher that dumps raw output is better than a broken one with LLM summarization.

### Composable Foundations
Build on graphqlite and cloacina rather than reinventing. Use standard protocols. Keep module boundaries clean.

## Constraints

### Technical
- Rust (stable toolchain), must cross-compile for ARM64
- SQLite as sole database (graphqlite for graph, raw tables for entities)
- cloacina for workflow orchestration
- No heavy runtimes (no Node.js, no JVM)

### Scope
- TUI as primary interface (no web UI in v1)
- Single-user only
- No voice/audio processing
- No messaging platform integrations (WhatsApp, Discord, etc.)

## Future Directions

### Headless Server Mode
The engine is designed with a channel-based protocol (`EngineRequest`/`EngineEvent`) that decouples it from any specific UI. This enables a future headless server mode where the engine runs as a daemon, exposing the channel protocol over a socket (HTTP, gRPC, or WebSocket). The TUI becomes one client among many — other clients could include a web UI, mobile app, IDE extension, or remote terminal. The persistent storage layer (SQLite + JSONL) is already filesystem-based and accessible from any process. Sessions are resumable from any client. This is not v1 scope, but the architecture supports it by design.