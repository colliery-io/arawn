---
id: session-workstream-binding-memory
level: task
title: "Session-workstream binding + memory routing"
short_code: "ARAWN-T-0250"
created_at: 2026-05-12T23:25:51.157839+00:00
updated_at: 2026-05-12T23:25:51.157839+00:00
parent: ARAWN-I-0040
blocked_by: [ARAWN-T-0249]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0040
---

# Session-workstream binding + memory routing

## Parent Initiative

[[ARAWN-I-0040]]

## Objective

Make the workstream concept load-bearing at runtime: sessions carry a workstream name, and every memory read/write routes to that workstream's KB + global. This is the change that turns T-0248's registry from a passive table into the active context that messages flow through.

## Scope

### `Session` gains a workstream

`arawn-core::Session` gets `workstream_name: String`. Defaults to `scratch`. Persisted with the rest of the session if the session is persisted; otherwise lives in-memory.

Constructor / factory: a new session is created with `workstream_name` set explicitly (the cli/TUI picks it from context) or defaults to `scratch`. New sessions for non-`scratch` workstreams are short-lived per the "tightly scoped" model from the design discussion; `scratch` sessions are continued.

### `/workstream switch` actually mutates session state

T-0249's `switch` command calls into this task's primitive. Implementation note: the active workstream lives on the `Session` struct, accessed via the existing service handle. Switching mid-session is allowed but the slash command surfaces a one-line banner so chat history doesn't silently re-target a different KB.

### Memory tool routing

`memory_store`, `memory_search`, and the auto-memory loop in `arawn-engine` currently see a `MemoryManager` with one workstream baked in. Replace with a per-call lookup: read the active workstream from the session, materialize / cache `MemoryManager::for_workstream(name)`, and use that for the call.

The two-tier model (global + workstream) stays. What changes is which workstream KB sits in the second tier.

Caching: a `WorkstreamMemoryCache` keyed by workstream name keeps each KB hot once first touched. LRU with no eviction in v1 — at 15-50 workstreams the working set fits.

### Memory injection (auto-memory at session start)

`load_memories_for_injection` currently pulls from one fixed workstream. Update to pull from `(global, active_workstream)`. If the active workstream is `scratch`, that's still the right second tier.

### What's deferred

- Session-resumption UX (perpetual scratch vs fresh-per-workstream) — CLI concern.
- `signal_search` with multi-workstream scope flag — Phase 6.
- Promotion (move scratch entities to a real workstream) — separate follow-up.
- Workstream-aware feed extraction — Phase 4.

## Acceptance Criteria

- [ ] `arawn-core::Session` has a `workstream_name` field; constructor defaults to `scratch`.
- [ ] `/workstream switch <name>` updates the session's workstream and the next `memory_store` lands in that workstream's KB.
- [ ] `memory_search` returns hits from both the global tier and the active workstream tier; switching changes which workstream is queried.
- [ ] Auto-memory injection at session start pulls from `(global, active_workstream)`.
- [ ] Engine-level memory tool tests pass: `memory_tools.rs` and `memory_stack.rs` updated as needed to construct a workstream-aware MemoryManager.
- [ ] `angreal check workspace` + `angreal check clippy` clean.

## Implementation Notes

### Where memory routing lives

The current memory tools take an `Arc<MemoryManager>` at registration time, fixed for the process. That has to flip: tools take an `Arc<dyn WorkstreamMemoryRouter>` (or some indirection) that resolves the active workstream per call. The router owns the cache.

A clean shape: `WorkstreamMemoryRouter::for_session(&Session) -> &MemoryManager`. Cheap clones; expensive opens.

### Service-level wiring

arawn's main constructs the global memory store once and the router on top. The session-active-workstream lookup happens inside the tool; tools don't need to know about the registry.

### Dependencies

- T-0248 (registry, `MemoryManager::for_workstream`).
- T-0249 (slash commands; in particular `/workstream switch` calls into this primitive).

### Risk considerations

- **Race between switch and in-flight tool call.** A `memory_store` queued before a `switch` lands in the old workstream. Probably acceptable; document that switch takes effect for the next message, not retroactively.
- **First-touch latency.** First memory_search after a switch pays the cost of opening the workstream KB + its FTS + vec0 indexes. For 384-d vectors on a small KB this is sub-second; surface in the slash banner if it matters.
- **Session persistence.** If sessions are serialized to disk, `workstream_name` is part of that. Make sure deserialization tolerates a missing field (default to `scratch`) for backward compat.

## Status Updates

*To be added during implementation*