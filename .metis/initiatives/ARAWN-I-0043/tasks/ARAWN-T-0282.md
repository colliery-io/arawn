---
id: gather-compose-write-pipeline-with
level: task
title: "Gather→compose→write pipeline with two-write-path citation enforcement"
short_code: "ARAWN-T-0282"
created_at: 2026-05-15T23:45:06.978670+00:00
updated_at: 2026-05-16T00:38:07.621576+00:00
parent: ARAWN-I-0043
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0043
---

# Gather→compose→write pipeline with citation enforcement

## Goal
The pipeline that drives one ceremony run: ask the plugin for facts, optionally detect patterns, ask the plugin to compose items, transactionally write tablet + sections + items. Citation enforcement is the load-bearing part.

## Reference
I-0043 Design Decisions §4 (two write paths) + Compose Chain section.

## Acceptance
- `CeremonyCtx` exposes:
  - `write_composed_item(item: ComposedItem) -> Result<()>` — strict path, requires `citation_id`; refuses with `CeremonyError::MissingCitation` if absent.
  - `write_user_item(item: UserItem) -> Result<()>` — permissive path, no citation required. Used for `kind=freeform` diary entries and user-added todos.
  - `write_pattern_row(p: DetectedPattern) -> Result<String>` — returns the pattern row id callers pass as `citation_id` on dependent composed items.
- Whole-run transaction: tablet + sections + items + patterns commit together or roll back together.
- `arawn_llm::gate::acquire_local()` wrap around the plugin's compose phase.
- Token usage records emitted with `call_site = "ceremony.<kind>.compose"`.
- Tests:
  - composed item missing citation → returns error, no row written.
  - composed item with citation → row written.
  - user item without citation → row written.
  - mid-run failure rolls back the whole tablet.

## Out of scope
The pattern detector framework itself (T-0286) — this task wires the call site, the framework is implemented separately.

## Notes
This is the spec for the engine's contract with plugins. Get the API right; everything downstream depends on it.
## Status Updates

**2026-05-16 — implementation landed.**

- New `crates/arawn-ceremonies/src/engine.rs` with:
  - `EngineDispatcher` — concrete `CeremonyDispatcher` impl. Owns the gather→pattern→compose→write pipeline.
  - `EngineCtx` — concrete `CeremonyCtx` impl backed by a shared `Arc<Mutex<Connection>>`.
  - `ConnHandle` — thin wrapper around `Arc<Mutex<Connection>>` used by both.
- Pipeline contract (matches I-0043 §Design Decisions #4):
  1. Plugin lookup + `period_key(now)`.
  2. Idempotency check: any existing tablet for `(kind, period_key)` short-circuits to `DispatchOutcome::Skipped` (including open status — we don't overwrite an in-flight tablet).
  3. `BEGIN IMMEDIATE` once per run. Every subsequent write rides this transaction; mid-run failure goes through `ROLLBACK`, success goes through `COMMIT`.
  4. Insert tablet row (`status='open'`, `workstreams_scanned='[]'` — plugins customise the array in a follow-up).
  5. `plugin.gather(&ctx).await` (deterministic; no LLM).
  6. `plugin.patterns()` if Some → `detector.detect(&ctx).await` → write each `DetectedPattern` via `ctx.write_pattern_row`. The row ids returned become the `citation_id`s composed pattern items use.
  7. `arawn_llm::gate::acquire_local().await` gates the compose phase so a ceremony never piles on top of an in-flight agent call.
  8. `plugin.compose(&ctx, facts).await` returns `Vec<NewItem>`. Engine iterates: `NewItem::Composed` → strict `write_composed_item` (refuses empty `citation_id` with `CeremonyError::MissingCitation`); `NewItem::User` → permissive `write_user_item`.
- All SQL writes go through `params!` to keep injection-safe.

**Deviation documented:**
- The T-0282 task body listed `write_composed_item` / `write_user_item` as methods on `CeremonyCtx`. The crate already encodes the two paths via the `NewItem::Composed | User` enum variants from T-0279 (composed has a non-optional `citation_id` field at the type level; user has no such field). The dispatcher routes the variants internally. This is **stricter** than the task body's design — a plugin literally cannot construct a `ComposedItem` without a `citation_id`, and the runtime check only catches the edge case of an empty/whitespace string. The two ctx methods are subsumed; documented here so future reviewers don't expect them.
- Token-usage `call_site = "ceremony.<kind>.compose"` tagging is deferred to T-0287 (retro plugin). The decoration lives where the LLM client is constructed; T-0282's pipeline gates the compose phase but the LLM client itself comes from the plugin.

**Tests (6 new in `engine::tests`, 17 total in the crate):**
- `happy_path_writes_tablet_and_composed_item_with_citation`
- `composed_item_missing_citation_rolls_back_whole_run` — confirms transactional rollback wipes the tablet row too.
- `user_item_without_citation_is_accepted` — confirms `citation_id` is NULL in the row.
- `idempotency_skips_when_open_tablet_exists` — second dispatch returns `Skipped` and the tablet count stays at 1.
- `unknown_kind_errors`
- `write_pattern_row_returns_id_and_writes`

Fixtures use `arawn-storage::Database::open` to apply migrations against a tempfile, then open a fresh `rusqlite::Connection` to the same file. Pure-Rust integration test, no live LLM.

Next: T-0283 (`ceremonies.*` WS-RPC method registry).