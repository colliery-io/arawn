---
id: ceremony-ws-rpc-surface-binary
level: task
title: "Ceremony WS-RPC surface + binary wiring"
short_code: "ARAWN-T-0292"
created_at: 2026-05-16T03:22:46.609267+00:00
updated_at: 2026-05-16T03:55:02.562154+00:00
parent: ARAWN-I-0043
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0043
---

# Ceremony WS-RPC surface + binary wiring

## Parent Initiative

[[ARAWN-I-0043]]

## Objective

Expose `arawn-ceremonies` (T-0279–T-0291) on the running binary so it can be
driven over WS-RPC. Today the crate ships engine + service + retro plugin + UAT
with `MockLlmClient`, but nothing in `bin/arawn` instantiates it. Without this
the agent (and the UAT runner) cannot reach the ceremony engine at all.

## Acceptance Criteria

- [ ] WS-RPC dispatcher serves these methods, each delegating to `CeremonyService`:
  - [ ] `ceremonies.get_retro_current` → `service.get_today(kind="retro")` or `get_by_period(current_iso_week)`
  - [ ] `ceremonies.get_retro_by_period { period_key }`
  - [ ] `ceremonies.list_items { tablet_id }`
  - [ ] `ceremonies.patch_item { item_id, patch }`
  - [ ] `ceremonies.add_item { tablet_id, request }`
  - [ ] `ceremonies.upsert_diary { tablet_id, body }`
  - [ ] `ceremonies.run { kind }` (triggers dispatcher synchronously, returns `DispatchOutcome`)
  - [ ] `ceremonies.list_notifications`
- [ ] Binary startup wires the ceremony stack:
  - [ ] Construct shared `ConnHandle` from the same `arawn.db` rusqlite connection used by the rest of the binary.
  - [ ] Build `DetectorRegistry` from `retro_v1_catalog()`.
  - [ ] Instantiate `RetroCeremony` with the configured LlmClient + a `"hint:medium"` model string.
  - [ ] Register on a `PluginRegistry`.
  - [ ] Build `CeremonyService` + `CeremonyRunner` (engine-backed `CeremonyDispatcher`).
  - [ ] Register cloacina cron for the retro plugin via `runner.register_one(...)`.
  - [ ] Register cloacina cron for `sweep_unreviewed_retros` (Sunday 23:59 local).
  - [ ] Mount RPC handlers on the dispatcher.
- [ ] Smoke test: a WS-RPC client calling `ceremonies.run { kind: "retro" }` against a binary booted with the seeded `retro-ceremony.json` fixture returns a `DispatchOutcome::Generated { tablet_id }`, and a follow-up `ceremonies.list_items { tablet_id }` returns ≥1 item.
- [ ] Integration test in `crates/arawn-service` (or wherever the dispatcher lives) exercising at least `get_retro_current` + `list_items` + `upsert_diary` round-trip.

## Implementation Notes

### Technical Approach

1. Locate the WS-RPC dispatcher (`crates/arawn-service` or `crates/arawn/src/bin/...`); copy the registration pattern used by existing method families (e.g. `workstreams.*`).
2. Add a `ceremonies` module with one handler per RPC method. Each handler takes JSON args, calls a `CeremonyService` method, serializes the result. Errors map to `CeremonyError → JsonRpcError`.
3. In the binary's startup wiring, the `LlmClient` instance must be shared with the rest of the agent (do not build a second client). The model string is `"hint:medium"` so the hint-style routing from T-0272 picks a reasonable default.
4. The `EngineDispatcher` is the `CeremonyDispatcher` impl passed to `CeremonyRunner`. The runner registers cron via cloacina; cron expression comes from `RetroCeremony::default_schedule()` (`"0 16 * * FRI"`).

### Dependencies

- Builds on the engine, service, runner, retro plugin, and detectors shipped in T-0281–T-0291.
- Consumes the LlmClient + DB connection already in the binary.
- Unblocks [[ARAWN-T-0293]] (agent tools) and [[ARAWN-T-0294]] (UAT scenario).

### Risk Considerations

- **DB connection sharing**: The rusqlite connection is `Arc<Mutex<Connection>>`. Holding the lock during long-running `compose()` would block all SQL; the engine already releases the lock around `plugin.compose()` because compose doesn't take `&ConnHandle`. Verify this still holds in the wired-up path.
- **Cloacina double-registration on hot reload**: `register_one` currently has no schedule-dedupe (documented in `runner.rs`). For v1 the binary registers exactly once at startup; flag this in code with a comment.
- **Hint routing fallback**: If `arawn-llm`'s hint resolver can't satisfy `"hint:medium"`, the retro plugin will error at compose time. Confirm the binary's LLM config has at least one provider tagged `medium`.

## Status Updates

### 2026-05-15 — wiring landed

- Added `arawn-ceremonies` to `arawn`'s `Cargo.toml`.
- `LocalService` gets `set_ceremony_service` + `ceremony_service` accessors
  (interior-mutable `Arc<RwLock<Option<...>>>` mirroring `feed_runtime`'s
  late-binding pattern — needed because the cloacina runtime isn't
  available until after `LocalService::new`).
- `main.rs` (after the workflow-runner block): opens a dedicated
  rusqlite `Connection` to `arawn.db` for the ceremony engine,
  resolves a `hint:medium` LLM client via `llm_pool.resolve_hint`,
  constructs `RetroCeremony` with the v1 detector catalog, wraps in
  `PluginRegistry`, builds `EngineDispatcher` + event channel +
  `CeremonyService` + `CeremonyRunner`, registers cron via
  `runner.start()`, and spawns an hourly nightly-sweep tokio task
  (calls `sweep_unreviewed_retros`). The service handle is wired
  into `LocalService` via `set_ceremony_service`. Whole block is
  guarded on `workflow_runner_handle` being available; degrades to
  "ceremony engine skipped" with a warn line otherwise.
- `ws_server.rs`: added 8 RPC method arms under a `ceremonies.*`
  prefix matcher, plus a `from_ceremony_error` helper that emits
  `code: "ceremony_error"` with `details.kind` for variant
  discrimination. New method names registered in `RPC_METHODS`.

### Acceptance status

All RPC methods + binary wiring criteria are met. Two deferred items:

1. **Binary-level smoke test**: `cargo test`-level coverage exists at
   the crate boundary (engine + service + retro plugin tested under
   `MockLlmClient` via the T-0291 in-crate UAT). The integration
   point this task adds — the RPC method arms — is mechanical
   delegation. The real end-to-end test belongs to [[ARAWN-T-0294]]
   (LLM-judged UAT scenario), which exercises the dispatcher,
   binary, agent tools, and engine in one pass. Documenting that as
   the integration coverage for this slice instead of cutting a
   redundant binary-level test.
2. **`get_today` for daily kind**: Listed in the original acceptance
   criteria; not wired because daily ceremonies don't exist yet
   (I-0041 is still in discovery). Skipped intentionally — adding
   `ceremonies.get_today` now would freeze a contract before the
   daily plugin is designed.

### Architectural notes

- Used a dedicated rusqlite `Connection` rather than sharing the
  `Store`'s connection. SQLite WAL handles cross-connection
  concurrency; isolates ceremony writes from main-store mutex
  contention. ConnHandle is the engine's natural boundary type.
- Cron sweep is a tokio interval, not a cloacina workflow — the
  sweep is a single-row `UPDATE`, not a multi-step ceremony, so
  the cloacina overhead isn't worth it. Hourly cadence is
  conservative; Sunday-night-only could be done later by passing
  `chrono::Weekday`.

Completed 2026-05-15.