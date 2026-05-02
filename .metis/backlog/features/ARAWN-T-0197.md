---
id: memory-discoverability-wire
level: task
title: "Memory discoverability: wire /remember, /memory inspection, fallback warning"
short_code: "ARAWN-T-0197"
created_at: 2026-05-02T00:00:00+00:00
updated_at: 2026-05-02T00:00:00+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


exit_criteria_met: false
initiative_id: NULL
---

# Memory discoverability: wire /remember, /memory inspection, fallback warning

## Objective

The memory system works under the hood — `MemoryStoreTool` and `MemorySearchTool` are registered, embeddings load if the model is present, FTS fallback exists. But a user can't tell what's been remembered, can't deliberately store a fact themselves, and won't notice if the embedder fell back to FTS-only (semantic search silently degrades). Make memory legible and intentionally usable.

## Type / Priority
- Feature
- P1 — Blocker. A memory system the user can't see or control isn't a memory system; it's a black box.

## Acceptance Criteria

- [ ] `/remember <text>` (TUI command, paired with T-0195) writes a fact to the global KB via the memory store. Confirmation line shown in the TUI.
- [ ] `/memory` opens an inspection view listing recent entries (id, type, title or summary, confidence, last reinforced). Scope: global + active workstream. Pagination acceptable; full search not required.
- [ ] If the embedder fails to load at startup, the TUI shows a one-time WARNING banner: "memory falling back to keyword search — install the embedding model for semantic recall (see docs/memory.md)."
- [ ] `docs/memory.md` (short) covers: what the memory system does, global vs workstream KB, how the agent reads/writes it autonomously vs how the user does so explicitly, what to do when the embedder isn't available.
- [ ] At least one test verifying `/remember` round-trips through the store — value can be retrieved by `/memory` listing immediately after.

## Implementation Notes

- Memory manager init lives in `crates/arawn/src/main.rs` (~line 256). Embedding load failure currently logs `warn!` server-side only; need a TUI-visible signal (RPC notification or banner state).
- For `/memory` listing, prefer a new RPC method (`memory_list { scope, limit }`) over forcing it through tool calls — this is operator UI, not agentic behavior.
- Confidence score is already on entities (`ConfidenceSource` enum). Surface it as a column.
- Coordinate with T-0195 for the command-routing wiring.

## Status Updates

### 2026-05-02 — Docs portion done

`docs/src/memory.md` written. Covers two-store layout (global vs workstream with scope-locking per entity type), the six entity types and seven relation types with concrete examples, the three confidence sources and base scores, the FTS-vs-vector retrieval paths, the embedding model location and silent-FTS-fallback caveat, how the agent uses `memory_store` / `memory_search` autonomously, and a worked flow for "user states preference → agent stores → next session retrieves." Calls out `/remember`/`/memory`/`/forget` as TUI work-in-progress, points at direct `sqlite3` access as the current escape hatch. Linked from SUMMARY.md.

Code work still open under this ticket:
- [ ] `/remember <text>` wired (paired with T-0195)
- [ ] `/memory` inspection view (modal or pane listing recent entries)
- [ ] `/forget <id-or-title>` — verify whether MemoryForgetTool exists; either wire or remove from /help
- [ ] Embedder-fallback warning surfaced in TUI banner (currently server-log only)
- [ ] Round-trip test: `/remember` → store → retrievable via `/memory`
