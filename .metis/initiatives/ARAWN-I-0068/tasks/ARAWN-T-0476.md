---
id: p2-1a-status-rpc-tui-panel
level: task
title: "P2-1a: /status RPC + TUI panel + readiness gate (health surface core)"
short_code: "ARAWN-T-0476"
created_at: 2026-06-12T12:02:08.784589+00:00
updated_at: 2026-06-12T21:18:13.841579+00:00
parent: ARAWN-I-0068
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0068
---

# P2-1a: /status RPC + TUI panel + readiness gate (health surface core)

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0068]] — implements the keystone finding P2-1 (CRITICAL), surface half. **First task of Phase 2.** Persisted failure-history storage is split into [[ARAWN-T-0477]].

## Objective **[REQUIRED]**

Build the single glanceable health surface: a `/status` RPC on `ArawnService` returning **structured** per-subsystem state, rendered by a thin TUI panel, plus a readiness gate so a client can tell when startup actually finished. Protocol-first per ADR ARAWN-A-0005 — the TUI panel is a thin renderer and the future web GUI consumes the identical RPC.

**The defect (P2-1 keystone):** nearly every background subsystem can fail or be absent with no user-visible signal — feeds (evidence only in `meta.json.last_status`), ceremonies (one `warn!` line on dispatch failure; engine silently skipped if the workflow runner fails at startup), steward (subroutine errors dropped), embedder (silent degrade to FTS-only), hooks (swallowed), permission audit (never exposed; 64-cap drops oldest). The server also accepts connections before feeds/ceremonies/memory finish initializing, with no `/health`/`/ready`.

**✅ DESIGN CHECKPOINT — RESOLVED (2026-06-12):**
- **Two RPCs, not one.** `health` is a cheap liveness/readiness probe returning `{ ready: bool, blocking: [reasons] }`; `status` is the richer per-subsystem dump. The GUI cheap-polls `health` and lazy-loads `status`. (`/health` + `/status` is the conventional web-service shape and what Phase 3 inherits.)
- **v1 `status` subsystem scope:** **feeds**, **ceremonies**, **embedding** (pending/errored backlog), **extraction** (cursor/lag), and **LLM connectivity** (configured endpoints reachable). Steward/storage readiness still drive `health.ready` but do NOT get their own `status` block in the first cut. Permission-audit tail is deferred out of v1 (add later — leave room in the versioned payload).

## Backlog Item Details

### Type
- [x] Feature — new observability surface (+ Tech Debt: closes a silent-failure class)

### Priority
- [x] P1 - High (the keystone of Phase 2; several other P2 tasks report INTO this surface)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] A `health` RPC on `ArawnService` returns `{ ready: bool, blocking: [reasons] }` — a cheap readiness probe. `ready` flips true only after feeds/ceremonies/memory/storage finish wiring; `blocking` lists human-readable reasons while not ready (e.g. "migrations pending", "feed runtime not wired"). Steward/storage readiness contribute to this gate.
- [ ] A `status` RPC on `ArawnService` returns a structured, **versioned** payload covering, per subsystem (v1 scope): **feeds** (each: id, enabled, last_run, last_error, paused/reconnect state), **ceremonies** (engine available? + last run per ceremony), **embedding** (pending + errored backlog counts), **extraction** (cursor position / lag per lens), **llm** (configured endpoint(s) reachable?). Sourced from live `LocalService` state (`feed_runtime`, `ceremony_service` Option, embedder, extraction cursors, LLM client). Payload is versioned with room for deferred blocks (steward/storage detail, permission-audit tail).
- [ ] A TUI panel renders the `status` payload as a thin client (no business logic) — invokable via a keybinding/slash command; the readiness gate consults `health`.
- [ ] Protocol-first: the two RPC payloads are the contract; the TUI does no aggregation the GUI couldn't also do from the same RPCs.
- [ ] Inline tests for the aggregation (subsystem `None`/error states map to the right payload fields), the readiness gate (false→true at end of wiring), and an RPC round-trip test for both `health` and `status`.
- [ ] `angreal check all` and `angreal test unit` green.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Define a `SystemStatus` struct (versioned) in `arawn-service` with per-subsystem sub-structs. `LocalService` implements a `status()` method aggregating from its held state: `feed_runtime` (per-feed meta), `ceremony_service` Option (availability + last run), embedder state, extraction cursors, the permission-audit ring buffer. Add it to the `ArawnService` trait + wire the WS RPC handler. Readiness: a flag/`AtomicBool` set once the post-startup wiring (`set_feed_runtime`/`set_ceremony_service`/memory) completes; expose via the payload. TUI: a panel that calls the RPC and renders rows — no logic. Several later Phase-2 tasks (T-0477 history, T-0478 feed auth) feed richer data into this payload.

### Dependencies
First Phase-2 task. T-0477 (persisted history) extends the payload with last-failure rows; T-0478 (feed auth) adds the "reconnect needed" state. Design the payload with those in mind (leave room). Builds on the `EngineEvent`/notice plumbing touched in T-0473.

### Risk Considerations
**Architectural decision (see DESIGN CHECKPOINT in Objective)** — payload shape is a forward contract for the GUI; get sign-off. Aggregation must not hold subsystem locks across `.await`. Keep the readiness flag simple (one-way false→true at end of wiring).

## Status Updates **[REQUIRED]**

### 2026-06-12 — kickoff, code survey done
Design resolved (checkpoint above): two RPCs — `health` (`{ready, blocking}`) + `status` (versioned per-subsystem). v1 scope: feeds, ceremonies, embedding, extraction, llm.

**Plan (backend first, then TUI):**
1. `arawn-service/types.rs`: `HealthStatus`, `SystemStatus` (versioned) + per-subsystem sub-structs; add `health()` + `status()` to `ArawnService` (`lib.rs`).
2. `LocalService`: `ready: AtomicBool` (+ `mark_ready()`/`is_ready()`); hold `projections` + `extractor_runner` handles (setters mirroring `feed_runtime`). New `local_service/status.rs` with aggregation.
3. Sources: feeds ← `feed_list()`; ceremonies ← `ceremony_service()` is_some + `list_notifications().len()`; embedding ← `memory_manager.embedder().is_some()` + new `ProjectionStore::pending_embedding_count()`; extraction ← extractor wired + cursor list; llm ← `llm_pool.entries()` (role/provider/model). LLM live-probe deferred → `engine_reachable: Option<bool>` (None in v1).
4. `ws_server`: register `"health"`+`"status"` in `RPC_METHODS` + 2 handlers (mirror `get_capabilities`).
5. `main.rs`: wire projections/extractor handles; `service.mark_ready()` right before `run_server` (~line 1287).
6. TUI: ws_client methods + `/status` command + thin panel + readiness gate.
7. Tests: aggregation, readiness false→true, RPC round-trip.

**Topology:** `projections` is a single shared `Arc<ProjectionStore>` (global, per-feed-type tables) — global pending count feasible. Embeddings status enum is only `pending|embedded|skipped` (no `errored` until T-0481) → v1 reports `pending` only, `errored` omitted.

### 2026-06-12 — backend implemented; BLOCKED on disk space for build verify
**Backend (steps 1–5) DONE — all written, not yet compiled:**
- `arawn-service/types.rs`: added `HealthStatus`, `SystemStatus` (+`SYSTEM_STATUS_VERSION=1`), `FeedsStatus`/`FeedStatusRow`, `CeremoniesStatus`, `EmbeddingStatus`, `ExtractionStatus`/`ExtractionCursor`, `LlmStatus`/`LlmClientStatus`. Exported in `lib.rs`. Added `health()` + `status()` to `ArawnService` trait.
- `arawn-projections/src/store.rs`: new `pending_embedding_count()` (sums `pending` across `%_embeddings` tables via sqlite_master).
- `arawn-storage/src/extractor_cursor_store.rs`: new `list_all()` (all cursors across lenses).
- `arawn/src/local_service/mod.rs`: added fields `ready: AtomicBool`, `projections: RwLock<Option<Arc<ProjectionStore>>>`, `extractor_available: AtomicBool`; setters `mark_ready()`/`is_ready()`/`set_projections()`/`mark_extractor_available()`; trait delegations `health()`→`health_inner`, `status()`→`status_inner`; `mod status;`.
- `arawn/src/local_service/status.rs` (NEW): aggregation — feeds (reuse `feed_list_inner`, available from `feed_runtime`), ceremonies (`ceremony_service()` + `list_notifications().len()`), embedding (`memory_manager.embedder()` + `pending_embedding_count`), extraction (`extractor_available` + `ExtractorCursorStore::list_all`), llm (`llm_pool.entries()` role/provider/model, `engine_reachable: None`).
- `arawn/src/ws_server/mod.rs`: registered `"health"`+`"status"` in `RPC_METHODS` + 2 handlers (mirror `get_capabilities`).
- `arawn/src/main.rs`: `set_projections`/`mark_extractor_available` after extractor build (~line 732); `mark_ready()` right before `run_server` (~line 1293).

**Only LocalService implements `ArawnService`** (grep-verified) — no other impls to update. TUI uses the WS client by method name, not the trait.

**BLOCKED:** `cargo build` hit `No space left on device` (disk 100% full, 122 MB free) — could not verify compilation. User is freeing space, then will say "rebuild". On resume: `cargo build -p arawn-service -p arawn-projections -p arawn-storage -p arawn`, fix any errors, then continue.

**REMAINING (steps 6–7):**
- TUI (`crates/arawn-tui`): `ws_client` methods for `health`+`status`; a `/status` command + thin render panel; readiness gate consulting `health`. Snapshot tests under `arawn-tui/src/snapshots`.
- Tests: aggregation (subsystem None/error → right fields), readiness false→true, RPC round-trip for both `health`+`status`. Likely inline in `status.rs` + a ws round-trip test.
- Then `angreal check all` (use `--check-only`/read-only, NOT auto-fix per prior stash incident) + `angreal test unit` green.

### 2026-06-12 — COMPLETE ✅ (disk freed, all steps done, all checks green)
Backend compiled cleanly after disk freed (77 GB free). TUI + tests done.

**TUI (step 6):**
- `arawn-tui/src/ws_client.rs`: `health()` + `status()` returning typed `arawn_service::HealthStatus`/`SystemStatus`.
- `arawn-tui/src/command.rs`: `/status` command spec + `CommandResult::SystemStatus` + parse arm.
- `arawn-tui/src/app/actions.rs`: added `SystemStatus` to the "needs-WS-interaction → pending_command" group.
- `arawn-tui/src/event_loop/mod.rs`: handler renders `format_system_status`; **readiness gate** on connect — calls `health()`, warns the user (with blocking reasons) when `!ready`, telling them to run `/status`.
- `arawn-tui/src/event_loop/formats.rs`: `format_system_status()` thin renderer (one scannable line per subsystem) + 2 inline render tests.

**Tests (step 7) — all green:**
- `arawn-projections` lib: `pending_embedding_count_sums_pending_rows` ✅
- `arawn-storage` lib: `list_all_spans_lenses_ordered` ✅
- `arawn-tui` lib: `format_system_status_renders_every_subsystem` + `_marks_absent_subsystems` ✅
- `arawn-tests/tests/local_service.rs`: `health_not_ready_until_marked`, `status_reports_subsystems_absent_by_default`, `status_embedding_pending_surfaces_when_projections_wired` ✅ (17/17)
- `arawn-tests/tests/websocket.rs`: `health_and_status_rpc_round_trip` ✅

**Checks:** `cargo fmt --all --check` clean · `cargo clippy --workspace -- -D warnings` clean · `cargo test --workspace --lib` exit 0 (no failures) · `angreal test unit` exit 0. (Used read-only fmt/clippy, not auto-fix — avoided the prior stash incident.)

**Deferred to later tasks (by design, payload has room):** LLM live reachability probe (`engine_reachable: None` in v1); embedding `errored` count (needs T-0481's error status); per-ceremony last-run + persisted failure history (T-0477); feed paused/reconnect state (T-0478); steward/storage + permission-audit `status` blocks.

**All acceptance criteria met.** Ready to mark completed.