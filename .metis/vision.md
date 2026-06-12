---
id: arawn
level: vision
title: "arawn"
short_code: "ARAWN-V-0001"
created_at: 2026-03-28T14:20:34.170334+00:00
updated_at: 2026-05-28T00:00:00+00:00
archived: false

tags:
  - "#vision"
  - "#phase/published"


exit_criteria_met: false
initiative_id: NULL
---

# Arawn Vision

A lightweight, self-hosted personal agentic assistant that ingests your tools
(email, calendar, GitHub, Slack, filesystem, …), extracts what matters into
topic-specific knowledge palaces, and surfaces an interactive chat that knows
who you are and what's been happening — all from a single Rust binary with a
minimal resource footprint.

## Purpose

Help you stay organized and on top of life — work and non-work alike. Arawn
watches, checks, summarizes, and nudges so you don't have to keep everything
in your head. The same memory · signals · lenses substrate serves the
single user whether they're running an engineering org or planning a vacation.

## Current State

- Previous attempt grew too large without testing; core functionality never
  stabilized.
- colliery-io ecosystem (graphqlite, cloacina) under active refinement and
  available as foundations.
- Clotho demonstrates the viability of: Rust TUI (ratatui), graph-backed data
  (graphqlite), multi-crate workspace architecture.

## Future State

A single Rust binary that:

- Provides an interactive TUI for chat, and a web GUI (served from the same
  binary) for reviewing action items, briefs, signals, and system health
  (ADR ARAWN-A-0005).
- Runs **feeds** that ingest external sources on configurable cadences (email,
  calendar, GitHub, Slack, filesystem) into a queryable corpus.
- Runs **lenses** — standing, memory-aware extractors that continuously pull
  typed signals (decisions, mentions, events) out of that corpus into per-topic
  palaces.
- Keeps a global **memory** of facts and preferences the agent treats as
  always-known.
- Surfaces findings as actionable items you can review, snooze, or dismiss.
- Persists state in SQLite (graphqlite for relationships, raw tables for
  entities).
- Orchestrates multi-step agent workflows via cloacina.
- Integrates with an LLM for natural conversation, intelligent summarization,
  and the extraction pipeline.

## Major Features

### Memory · Signals · Lenses

The core data model has three distinct layers (kept distinct on purpose):

- **Memory** — global statements of fact, structured entities, and behavioral
  tuning. Facts can be opaque ("prefer terse responses", "calendar is in PT")
  or structured: a `Person` entity for "Pat Collins" (role, reports_to, team,
  growth themes, open commitments); a `Project` entity for "Project Atlas"
  (decisions log, risks, bindings); a `Team` entity for per-team roll-ups.
  Typed relations between entities — `manages`, `reports_to`, `on_team`,
  `peer_of`, `belongs_to_project` — turn the memory into a navigable model of
  who and what surrounds the user. Stored once, globally. Read by the chat
  and by every lens's extractor when deciding scope.
- **Signals** — extracted activity/events. Each lens's standing extractor
  pulls typed entities out of incoming feed material; the chat reads them
  across every lens with the agent's `signal_*` tools (each hit labeled by
  source lens).
- **Lenses** — independently-authored, memory-aware standing extractors over
  the corpus. Each lens carries its own prompt + tag ontology; you don't
  switch into a lens, you define one and it runs.

### Feeds

External-source ingesters that mirror upstream content (Gmail, Calendar, Drive,
Slack, GitHub, filesystem) into a queryable on-disk corpus on a cadence. Feeds
are the entry point — feeds in → projections + lens extraction → signals →
chat.

### Interactive Chat

Conversational interface for asking questions and giving instructions —
available in the TUI (the fast terminal path) and the web GUI; structured
*review* of what Arawn has found lives primarily in the GUI (ADR
ARAWN-A-0005). The chat reads signals across every lens
and the global memory, and labels its sources so the user sees which lenses an
answer drew from.

A **scratch** session context exists for one-off / ad-hoc chats that aren't
anchored to a topic — useful for tinkering. Scratch runs no extractor of its
own; named lenses do.

### Action Item Surface

A unified view of things that need your attention, sourced from extracted
signals (via ceremonies and steward subroutines) or manual capture. Action
items can be reviewed, snoozed, or dismissed — primarily from the web GUI
(ADR ARAWN-A-0005), exposed protocol-first so any client can render them.

### Knowledge Persistence

Conversations, memory, signals, and action items persist across sessions via
SQLite + graphqlite. Memory is one global store; signals are partitioned per
lens on disk so a lens can be backed up, archived, or wiped with `rm -rf
<data_dir>/lenses/<slug>`.

### Sandboxed Tool Execution

Agent-driven tool execution (shell commands, API calls, file operations) runs
inside a sandbox by default. Per-lens filesystem boundaries (`workspace/`
directories) pair with the sandbox so autonomously-running tools can only
touch what they should. Safety prerequisite, not a later enhancement —
extraction and chat-driven actions run unsupervised.

### LLM Integration

A configurable LLM provider drives the chat, the extraction chain, and
ceremony summarization. Provider choice is per-deployment (`arawn.toml`); the
extractor uses the same client.

### Audiences & Voice

The chat speaks in the user's voice when drafting external communication.
The single user might be writing a board update, a 1:1 follow-up to a
direct, a peer-leader heads-up, an all-hands talking point, or a casual
reminder to themselves. These have different registers; the system models
them explicitly. Voice profiles:

- `personal` — the default register. Direct, warm, no corporate hedging.
- `exec-comms` — work-context drafting with audience sub-modes (`board`,
  `peer`, `direct-report`, `skip-level`, `all-hands`). Each sub-mode carries
  a register guide that shapes tone, hedging, and structure without
  re-prompting per draft.

Voice profile selection is part of how drafting tools (`gmail_send`, future
`slack_reply`, future `compose`) attach context. The user can override per
request and set defaults via `arawn.md`.

## Success Criteria

1. Can hold a useful chat conversation with context persistence — the chat
   sees global memory and signals from every lens.
2. Lenses extract signals from at least two feed families (Gmail + GitHub,
   say) into per-lens palaces.
3. Action items surfaced in a client (web GUI as primary review surface)
   from extracted signals.
4. Memory writes (`/remember`, `memory_store`) land globally and influence
   every lens's extraction.
5. Stable on a low-resource system (<500 MB memory).
6. Comprehensive test coverage from day one (unit + integration + UAT).
7. Can answer person-centric and project-centric questions ("what's been
   going on with Sarah this week?", "where's Atlas right now?") by joining
   structured entities to signals, at the same cadence as topic-centric
   questions.

## Principles

### Test-First

Nothing ships without tests. This is what went wrong last time. Every initiative
delivers tested functionality.

### Incremental Delivery

Each initiative delivers working, tested functionality end-to-end. No building
five subsystems in parallel and hoping they integrate.

### Small Footprint

Rust, SQLite, single binary. No heavy runtimes. Must run comfortably on
resource-constrained hardware.

### Simple Before Smart

Get the plumbing working before adding intelligence. A working feed that dumps
raw output is better than a broken one with LLM summarization.

### Composable Foundations

Build on graphqlite and cloacina rather than reinventing. Use standard
protocols. Keep module boundaries clean.

## Constraints

### Technical

- Rust (stable toolchain), must cross-compile for ARM64.
- SQLite as sole database (graphqlite for graph, raw tables for entities).
- cloacina for workflow orchestration.
- No heavy runtimes shipped or required at runtime (no Node.js, no JVM).
  Frontend build tooling for the web GUI runs at build time only
  (ADR ARAWN-A-0005).

### Scope

- TUI is the chat client; a web GUI served from the binary is the primary
  review/triage surface (amended 2026-06-11 per ADR ARAWN-A-0005; was "TUI as
  primary interface, no web UI in v1"). The GUI ships only after the
  daily-drivability hardening lands (ARAWN-I-0067 Phases 1–2).
- Single-user only — the multi-user reshape is the separate **AWEN** project.
- No voice/audio processing.
- No messaging platform integrations beyond what we already wire (WhatsApp,
  Discord, etc. are out).

## Future Directions

### Headless Server Mode

The engine is designed with a channel-based protocol
(`EngineRequest`/`EngineEvent`) that decouples it from any specific UI. This
enables a future headless server mode where the engine runs as a daemon,
exposing the channel protocol over a socket (HTTP, gRPC, or WebSocket). The
TUI becomes one client among many — other clients could include a web UI,
mobile app, IDE extension, or remote terminal. The persistent storage layer
(SQLite + JSONL) is already filesystem-based and accessible from any process.
Sessions are resumable from any client. This is not v1 scope, but the
architecture supports it by design.
