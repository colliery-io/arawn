---
id: session-workstream-binding-memory
level: task
title: "Session-workstream binding + memory routing"
short_code: "ARAWN-T-0250"
created_at: 2026-05-12T23:25:51.157839+00:00
updated_at: 2026-05-12T23:56:53.699858+00:00
parent: ARAWN-I-0040
blocked_by: [ARAWN-T-0249]
archived: false

tags:
  - "#task"
  - "#phase/completed"


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

## Acceptance Criteria

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

### 2026-05-12 — Session + memory routing wired; switch lands in the new KB

**Files.**
- `crates/arawn-core/src/session.rs` — `Session` gained `workstream_name: String` (defaults to `scratch`). New `new_with_workstream(id, name)` factory and `set_workstream(name, id)` setter. `workstream_name()` accessor. Legacy `from_parts*` constructors default the field for back-compat with persisted sessions.
- `crates/arawn-engine/src/workstream_router.rs` — new. `WorkstreamMemoryRouter` opens (and caches) a `MemoryManager` per workstream name; `MemoryHandle::{Fixed, Routed}` enum lets tools accept either a fixed manager (tests) or a routed one (prod). `Into<MemoryHandle>` impls for `Arc<MemoryManager>` and `Arc<WorkstreamMemoryRouter>` keep test code unchanged.
- `crates/arawn-engine/src/tools/memory_store.rs` + `memory_search.rs` — both tools' constructors now take `impl Into<MemoryHandle>`. Internal `self.memory` access becomes `self.memory.manager()?` which returns the active `Arc<MemoryManager>`. Test code stayed unchanged thanks to the blanket conversion.
- `crates/arawn/src/main.rs` — at boot, builds an `Arc<WorkstreamMemoryRouter>` from a shared `SessionWorkstream`, threads it through both memory tools and (cloned) into the workstream slash commands.
- `crates/arawn-storage/src/store.rs` — `create_workstream` routes `scratch` to `ensure_scratch_workstream` for back-compat; the engine-side `workstream_new` tool refuses scratch explicitly so the user-facing error is still surfaced.

**Routing flow.** A user types `/workstream switch pat`:
1. `WorkstreamSwitchTool` calls `SessionWorkstream::set("pat")`.
2. Next time `memory_store` or `memory_search` runs, it calls `self.memory.manager()` which routes via `WorkstreamMemoryRouter::current()`.
3. `current()` reads `SessionWorkstream::current()` → `"pat"`, materializes (or pulls from cache) `MemoryManager::for_workstream("pat")`, returns it.
4. The tool's existing two-tier logic (`manager.global`, `manager.workstream`) picks the right stores. Auto-memory at session start uses the same routed handle.

**Tests.**
- `arawn-core` lib: **33 passed** (new fields don't break existing Session tests).
- `arawn-engine` lib: **543 passed** (memory tools unchanged at the test surface; 2 new `workstream_router` tests verify caching + fixed-handle dispatch).
- `arawn-storage` lib: **22 passed** (registry tests).
- `arawn-tests` integration: all green — session/workstream tests now use `ensure_scratch_workstream` indirectly via the routed `create_workstream(scratch)`.
- Full workspace test sweep: 0 failures.
- `angreal check workspace` + `angreal check clippy` clean (clippy auto-fixed two unused imports in `arawn/main.rs`).

**Decisions worth keeping.**
- **SessionWorkstream is the runtime carrier, not Session.** The Session struct's `workstream_name` is the persisted-state copy; the runtime active-workstream lives in the `Arc<Mutex<String>>` shim shared between the router and the slash commands. On session save/load, Session.workstream_name follows SessionWorkstream's current value — but at the routing decision point, we read the shim because Tool execute() doesn't have a `&mut Session` to read from.
- **`MemoryHandle` enum, not trait object.** `impl Into<MemoryHandle>` is enough to let `Arc<MemoryManager>` and `Arc<WorkstreamMemoryRouter>` both flow into the tools' constructors. Avoids a `dyn Trait` allocation and keeps the call site terse.
- **Lazy + cached.** First touch of a workstream pays the cost of opening sqlite + sqlite-vec + FTS5 indexes; subsequent touches are Arc-clone speed. At 15-50 workstreams the cache will never need eviction.
- **`scratch` reservation lives in the tool, not the Store.** `Store::create_workstream(scratch)` is a backward-compat alias for `ensure_scratch_workstream`. The user-facing slash command refuses `scratch` up front with a clear message.

**Acceptance criteria.**
- [x] `arawn-core::Session` has `workstream_name`; constructor defaults to `scratch`.
- [x] `/workstream switch` updates the shared `SessionWorkstream` which the router reads on every memory tool call.
- [x] `memory_search` queries the active workstream's KB + global tier; switching changes which workstream is queried.
- [x] Auto-memory injection pulls from `(global, active_workstream)` via the same router.
- [x] Engine-level memory tool tests pass without source changes (the `Into<MemoryHandle>` blanket let `Arc<MemoryManager>` flow into the new constructors).
- [x] `angreal check workspace` + `angreal check clippy` clean.

Phase 3 lands as a unit. Three things still to do as follow-ups (logged in T-0249/T-0250 task bodies, not blocking):

1. Persist `Session.workstream_name` to the sessions table (currently in-memory only). Touches a migration + SessionStore round-trip.
2. `/workstream promote` — move entities from scratch into a named workstream.
3. TUI banner surfacing on `/workstream switch` — the tool returns the banner text in its JSON; rendering it is a TUI concern.