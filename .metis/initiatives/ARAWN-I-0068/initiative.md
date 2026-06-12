---
id: daily-drivable-phase-2-trustworthy
level: initiative
title: "Daily-drivable Phase 2: trustworthy background brain — status surface, failure history, data integrity"
short_code: "ARAWN-I-0068"
created_at: 2026-06-11T11:19:25.206917+00:00
updated_at: 2026-06-11T11:19:25.206917+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: L
initiative_id: daily-drivable-phase-2-trustworthy
---

# Daily-drivable Phase 2: trustworthy background brain — status surface, failure history, data integrity Initiative

## Context **[REQUIRED]**

Spun out of [[ARAWN-I-0067]] (full-codebase review, 2026-06-11) as the Phase 2 execution initiative. The review's dominant cross-cutting finding: **nearly every background subsystem — feeds, ceremonies, steward, embedder, extraction, hooks, permission audit — can fail or be silently absent with no user-visible signal**, plus a set of data-integrity defects (non-transactional ceremony writes, missing session promotion, stale embeddings, memory-store races) that corrupt quietly over time. This initiative makes the background brain trustworthy enough to run unattended for weeks — the half of the product the vision calls the point ("watches, checks, summarizes, and nudges").

**Gating:** starts when ARAWN-I-0067 (Phase 1, chat trustworthiness) lands; the status-surface work (P2-1) may begin as Phase 1 wraps. Completion of this initiative (then ARAWN-I-0069) gates scaffolding the web GUI initiative per ADR ARAWN-A-0005.

**Protocol-first constraint (ADR ARAWN-A-0005):** every surface built here — `/status`, ceremony history, action-item review — is a structured RPC on `ArawnService` with the TUI as a thin renderer, so the future web GUI consumes the identical contract. The status surface is itself a GUI prerequisite.

**Related work items:** ARAWN-T-0012 (Unified Store + session promotion, todo — absorbed by this initiative's P2-4 task at decomposition); ARAWN-T-0026 (RemoteService — out of scope here); findings carried verbatim from the I-0067 review catalogue; **(verified)** markers indicate claims re-confirmed by direct source reading during review synthesis. This document is canonical for Phase 2 execution; I-0067 retains the original catalogue as the review record.

## Goals & Non-Goals **[REQUIRED]**

**Goals:**
- One glanceable health/status surface (`/status` RPC + TUI panel + readiness gate) covering every background subsystem, backed by persisted failure history (ceremony run-history table, steward error log).
- Feeds survive auth failure gracefully: auto-pause with a "reconnect needed" reason instead of indefinite retry spam; token-refresh persistence hardened.
- Data integrity: transactional ceremony writes with force-regenerate, atomic `Store::promote_session` + lens creation (closes ARAWN-T-0012), embedding `body_hash` validation, memory-store race fixes.
- Extraction becomes debuggable and idempotent: confidence floor wired, per-run provenance IDs, explain/rerun/dismiss tooling.
- An integration test suite over the seams (session lifecycle, store atomicity under failure, startup with absent subsystems) pins all of the above.

**Non-Goals:**
- Building the web GUI (its own initiative, gated on this + ARAWN-I-0069 landing; ADR ARAWN-A-0005).
- RemoteService / multi-client daemon mode (ARAWN-T-0026).
- New feed integrations or completing the filesystem-watch stub.
- TUI experience features (review surfaces live in the GUI; TUI correctness residue is ARAWN-I-0069).

## Detailed Design **[REQUIRED]**

The findings below are the design input: each is a concrete defect or gap with location, mechanism, and fix sketch, carried from the I-0067 review catalogue (IDs preserved). Severity scale: critical / high / medium / low.

### P2-1 · CRITICAL · No unified health/status surface; failures are invisible (the keystone finding)
Every reviewer independently converged here. Current state of each silent-failure point:
- **Feeds**: failure evidence is only `meta.json.last_status`, visible only via manual `/feeds list` (`arawn-feeds/src/dispatch.rs:308`). No proactive notification when a feed enters a failed state.
- **Ceremonies**: dispatch failure = one `warn!` line; no run-history row, no failed status, nothing queryable (`arawn-ceremonies/src/runner.rs:256-262`). User sees "no tablet today" and can't distinguish error from not-scheduled.
- **Ceremony engine absent**: if the workflow runner fails at startup, the entire ceremony engine is silently skipped (`crates/arawn/src/startup/ceremonies.rs:40-41`); `LocalService` holds `Option<CeremonyService>` = None with no capability/health RPC (`local_service/mod.rs:85-95, 194-197` — `/watch` errors are equally opaque when feed runtime is None).
- **Steward**: subroutine errors increment a stat and are dropped; journal records only successes (`arawn-steward/src/runner.rs:174-182`).
- **Embedder**: failure to load degrades memory to FTS-only silently (`crates/arawn/src/main.rs:388-401`); no pending-embedding count is queryable anywhere.
- **Hooks**: load/parse failures are logged and skipped; background task hooks are fire-and-forget with errors swallowed (`arawn-engine/src/background.rs:225-234, 283-292`).
- **Permission audit**: populated but never exposed via RPC; broadcast buffer capped at 64 events, then drops oldest silently (`local_service/mod.rs:63-68`).
- **Server readiness**: accepts connections before feeds/ceremonies/memory finish initializing; no `/health` or `/ready`.
- **Timezone fallback**: failed TZ detection silently falls back to UTC for ceremony schedules (`arawn-ceremonies/src/runner.rs:291-315`) — visible only in startup logs.
**Fix (one coherent deliverable):** a `/status` RPC + TUI panel reporting per-subsystem state (each feed: enabled/last_run/last_error; ceremonies: engine availability + last run per ceremony; steward: last pass + error count; embedder: state + pending backlog; extraction: cursor positions per lens; permission audit tail), backed by persisted failure rows (ceremony run history table, steward error log) and a readiness gate. Protocol-first: structured RPC payload, TUI panel as thin renderer (ADR ARAWN-A-0005).

### P2-2 · CRITICAL · Feed auth lifecycle: broken feeds retry forever, token refresh can silently lose state
`crates/arawn-feeds/src/dispatch.rs:93-97, 168-182`, `crates/arawn-auth/src/oauth2.rs:178-180`, `crates/arawn-integrations/src/google_common.rs:137-143`
- `FeedError::Auth` becomes a generic `TaskError::ExecutionFailed`; cloacina retries indefinitely on schedule. An expired/revoked token means days of failed runs burning API quota, discoverable only by manually reading feed status.
- "Token revoked, reconnect needed" and "transient network blip" are not distinguished anywhere user-visible.
- When a refreshed token fails to persist (disk full/permissions), it's a log warning; the in-memory token works until restart, then the process reloads the stale token and fails confusingly.
**Fix:** auto-pause feeds on `AuthExpired` with a persisted "reconnect needed" reason (surfaced via P2-1); make token-persist failure an error that marks the integration degraded; map auth errors to a non-retryable task outcome.

### P2-3 · HIGH · Ceremony writes lack transactional integrity
`crates/arawn-ceremonies/src/engine.rs:124-133, 154-182, 235-279`
- The item-write loop is auto-commit per item: if item N+1 fails, items 1..N remain — a partially-written tablet is presented as a real tablet. Pipeline error handling deletes the tablet row but not pattern rows written before the LLM compose call (orphaned cruft, confusing audits).
- No transaction/locking against concurrent user `patch_item()` — ceremony writes can clobber user edits (TOCTOU).
- An `open` tablet cannot be force-regenerated; fixing a bad LLM output requires manual row deletion.
**Fix:** wrap detect→compose→write in a transaction or savepoint; track and roll back pattern rows on error; add `force` to dispatch; return item counts in DispatchOutcome so partial success is visible.

### P2-4 · HIGH (verified) · Session promotion does not exist; lens creation is non-atomic
`crates/arawn-storage/src/store.rs:65-81`, `crates/arawn-storage/src/jsonl.rs:157` — implements ARAWN-T-0012
- `JsonlMessageStore::move_session()` exists; `SessionStore` can update rows; but no `Store::promote_session(session_id, new_lens_id)` composes the SQLite update + JSONL file move atomically. The scratch→lens promotion workflow from the vision cannot be performed at all.
- `create_lens` inserts the SQLite row, then `create_dir_all` — a mkdir failure leaves a lens row with no directory and no rollback.
**Fix:** implement `promote_session` (SQLite txn + file move + rollback on failure); make lens creation mkdir-first or roll back the row; expose a `/lens promote` RPC + TUI command.

### P2-5 · HIGH · Embedding lifecycle: stale vectors and a stuck pass
`crates/arawn-projections/src/embed.rs:113-180, 147-154, 193-284`, `crates/arawn-projections/src/store.rs:465-488`
- `write_embedding()` never validates `body_hash` against the row's current body. If a body updates between the pending-rows fetch and the vector write, the row flips to `embedded` with a vector for stale text — and never re-embeds because it's no longer pending. Semantic search silently returns results based on superseded content.
- If `embed_batch()` errors, the batch loop makes zero progress; a persistently failing embedder spins the pass forever while the backlog grows unbounded.
**Fix:** compare-and-set on `body_hash` at vector write (mismatch → leave pending); on batch failure mark rows errored/backoff rather than retrying the same head-of-line batch; expose pending/error counts (via P2-1).

### P2-6 · HIGH · Memory store concurrency races corrupt dedup and confidence
`crates/arawn-memory/src/store.rs:446-461, 465-502`
- `store_fact()` is FTS-search-then-reinforce with no transaction: a concurrent delete between search and reinforce errors out and the incoming fact is lost without user-visible signal.
- `reinforce_entity()` is read-increment-write without a lock: concurrent reinforcement undercounts (two threads read 5, both write 6), silently corrupting confidence ranking that the extraction/ranking layers depend on.
**Fix:** wrap dedupe-or-insert in a transaction; make reinforcement a single SQL `UPDATE ... SET count = count + 1`.

### P2-7 · MEDIUM · Extraction is non-debuggable and not fully idempotent
`crates/arawn-extractor/src/cot.rs:255-259, 451, 494-501`, `crates/arawn-extractor/src/runner.rs:210-264`
- `link_by_name` resolution takes the first FTS hit; the `_floor: f32` confidence parameter is accepted but unused — typos can link signals to entirely wrong entities with no threshold.
- No per-run provenance (extraction run ID) on EXTRACTED_FROM edges → backfill re-runs can duplicate entities in the lens KB.
- Valid-but-empty LLM output advances the cursor (row never revisited), so extraction outcomes depend on model mood — re-running a lens over the same feed is not deterministic.
- Token filter `len() >= 3` drops short tokens ("AI") from global-fact context lookup — silent recall loss in classify.
- No user-facing way to see why a row was classified in/out of scope, re-run extraction on a single row, or mark a signal as garbage (current recourse: manual memory deletion).
- `in_flight` dedup gate is per-process; two processes can race the same cursor.
**Fix:** wire the floor parameter; stamp run IDs; distinguish "extracted empty" from "skipped"; add `signal_explain` / `extract_rerun <row>` / `signal_dismiss` tooling.

### P2-8 · HIGH · Integration test suite over the seams (currently hollow)
`crates/arawn-tests/src/lib.rs` (one comment, zero tests)
- Per-crate unit tests are real (300+, passing) but every bug class above lives in the *seams* that have zero coverage: session lifecycle end-to-end (create → append → load → compact → promote), store atomicity under injected failure (mkdir fails, JSONL write fails), startup ordering (feed runtime/ceremony engine absent), stream interruption mid-tool-call, JSONL corruption handling (currently skip-with-warn, `jsonl.rs:88-99`).
**Fix:** 10–20 integration tests in arawn-tests covering exactly these scenarios; wire into `angreal test integration`.

## Testing Strategy

- **The integration suite is a deliverable, not an afterthought** (P2-8): it lands alongside the data-integrity fixes (P2-3…P2-6), not after them, so each fix is pinned the day it merges. Lives in `crates/arawn-tests`, runs under `angreal test integration`.
- **Every correctness fix carries an inline regression test** in the owning crate (project convention: inline `#[cfg(test)]` modules). Failure-injection patterns: mkdir fails mid-lens-creation, JSONL write fails mid-promotion, embedder errors mid-pass, LLM fails mid-ceremony.
- **UAT acceptance gate**: induce a feed auth failure and a ceremony failure; both must be visible in `/status` with actionable reasons. UAT must be green before this initiative is called done.

## Alternatives Considered **[REQUIRED]**

- **Logs-only observability (skip the /status surface)** — rejected. Logs exist today and didn't prevent the problem; the user does not read logs during daily use. Also pre-rejected at the ADR level (ARAWN-A-0005 rationale).
- **Per-subsystem status bolted onto each command** (`/feeds list`, `ceremonies.get_today`, …) — rejected. The failure modes are cross-cutting; the value is one glanceable surface, and the GUI needs one RPC to render, not seven.
- **Fixing data races opportunistically during feature work** — rejected. The races (P2-5, P2-6) corrupt silently; they need dedicated, tested fixes, not drive-by patches.
- **Deferring the integration suite to a later initiative** — rejected. Without seam tests the fixes here regress invisibly; test-first is a vision principle and the reason the last attempt failed.

## Implementation Plan **[REQUIRED]**

Suggested task slicing (decomposition requires human review before tasks are created):
1. `/status` RPC + TUI panel + readiness gate (P2-1 core) — **first**; other items report into it. Protocol-first payload design per ADR ARAWN-A-0005.
2. Persisted failure history: ceremony run-history table + steward error log (P2-1 storage half).
3. Feed auth lifecycle: auto-pause, reconnect-needed states, token-persist hardening (P2-2).
4. Ceremony transactional integrity + force-regenerate (P2-3).
5. `Store::promote_session` + atomic lens creation + `/lens promote` (P2-4; implements and closes ARAWN-T-0012).
6. Embedding hash validation + stuck-pass handling (P2-5) and memory-store race fixes (P2-6) — small, surgical, can parallelize with 3-4.
7. Extraction debuggability: floor wiring, run IDs, explain/rerun/dismiss tools (P2-7).
8. Integration test suite over the seams (P2-8) — lands **alongside** items 3-6, not after.

**Exit criteria:**
- Every background failure mode in the P2-1 list is visible in `/status` at a glance.
- An expired Gmail token results in a paused feed with a "reconnect" instruction — not silent retry spam.
- A scratch session can be promoted to a lens from the TUI.
- Killing the embedder or LLM mid-ceremony leaves no partial tablets, stale vectors, or stuck passes.
- `angreal test integration` covers all of the above; UAT green.

**Sequencing notes:** P2-1 may begin as I-0067 Phase 1 wraps. ARAWN-I-0069 (Phase 3) runs after this initiative; the web GUI initiative is gated on both landing (ADR ARAWN-A-0005).