---
id: make-arawn-daily-drivable
level: initiative
title: "Make Arawn daily-drivable: correctness, observability, and last-mile gaps"
short_code: "ARAWN-I-0067"
created_at: 2026-06-11T10:52:34.227666+00:00
updated_at: 2026-06-12T12:01:03.218147+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: L
initiative_id: make-arawn-daily-drivable
---

# Make Arawn daily-drivable: correctness, observability, and last-mile gaps Initiative

## Context **[REQUIRED]**

A full-codebase review (2026-06-11, eight parallel subsystem reviewers + manual spot-verification of top claims) assessed Arawn (~118k lines, 20 crates) for correctness, ergonomics, and product-engineering readiness. The unit suite (300+ tests) passes clean and module-level architecture is sound: clean crate boundaries, provenance edges, idempotency keys, circuit breakers.

**What blocks daily use is not the foundations.** Three cross-cutting deficits dominate:

1. **A silent-failure epidemic** — nearly every background subsystem (feeds, ceremonies, steward, embedder, hooks, permission audit) can fail or be absent without any user-visible signal. The only evidence lives in logs or `meta.json` files the user never reads.
2. **A handful of real correctness bugs** — most critically in the OpenAI-compat streaming parser (parallel tool calls silently dropped), the embedding lifecycle (stale vectors), memory-store concurrency, and compaction validation.
3. **An unfinished last mile** — onboarding (`arawn init` advertised but absent, docs drift), session promotion (T-0012 never implemented), no health/readiness surface, lock-poisoning panics in service hot paths.

Findings marked **(verified)** were re-confirmed by direct source reading during synthesis; all others were grounded in code reading by the subsystem reviewers with file:line citations.

**Integration completeness at review time:** Gmail (3 templates), Slack (4), GitHub (4), Drive (2), Calendar (1), Jira/Confluence (3) — all partial but functional. Filesystem watch — stub. The feeds → projections → extraction → signals pipeline is wired end-to-end, but lacks a single "process this feed through everything" lever and any extraction-provenance debugging surface.

**Related existing work items:** ARAWN-T-0012 (Unified Store + session promotion, todo), ARAWN-T-0026 (LocalService — ArawnService impl, todo), T-0194 (`arawn init`, referenced from README), ARAWN-T-0270/T-0274 (token-budget tooling, adjacent to compaction findings).

**Related decisions:** ADR ARAWN-A-0005 (2026-06-11) — web GUI becomes the primary review/triage surface; TUI demoted to chat client. Phase 3 of this initiative was restructured accordingly, and the GUI is a separate initiative gated on Phases 1–2 landing.

## Goals & Non-Goals **[REQUIRED]**

**Goals:**
- Phase 1: make interactive chat trustworthy enough to daily-drive — fix streaming/tool-call correctness, lock safety, fail-fast configuration, first-run onboarding, and compaction degradation.
- Phase 2 — **spun out to [[ARAWN-I-0068]]** (2026-06-11): trustworthy background brain (status surface, failure history, data integrity, integration tests). That initiative is canonical for Phase 2 execution.
- Phase 3 — **spun out to [[ARAWN-I-0069]]** (2026-06-11): TUI correctness + GUI prerequisites per ADR ARAWN-A-0005. That initiative is canonical for Phase 3 execution.
- This initiative now tracks **Phase 1 execution only** (tasks ARAWN-T-0468 … ARAWN-T-0474) and remains the full review record.

**Non-Goals:**
- Building the web GUI itself — decided in ADR ARAWN-A-0005 as the primary review/triage surface, but scoped as its own initiative that starts only after Phases 1–2 land here.
- TUI experience features (session browser, dashboard-style views, richer triage UX) — superseded by the GUI direction; the TUI remains a maintained *chat client*.
- RemoteService / headless multi-client mode (tracked separately via ARAWN-T-0026; vision "Future Directions").
- New feed integrations or completing the filesystem-watch stub beyond marking it clearly unsupported.
- Tier-3 architectural drive-out (explicitly deferred per I-0048 decision — prefer user-visible capability).
- Web UI, multi-user (AWEN), or any scope beyond making the existing single-binary product reliable.

## Verification & caveat resolution (2026-06-12)

After the 7 Phase 1 tasks landed, the three known caveats were fixed and Phase 1 was validated end-to-end against a live LLM:

- **`angreal check all` → green.** Fixed 23 lib/bin clippy warnings (incl. a `clippy --fix` auto-applying a wrong `unnecessary_to_owned` suggestion → `E0521` borrow-escape in `main.rs`; the `.cloned()` is required because the value moves into a spawned task). Reformatted the whole repo under rust 1.93 (`cargo fmt --all`, 63 files) and added `rust-toolchain.toml` pinning 1.93.0 so fmt/clippy output can't silently drift again.
- **Sandbox-exec flaky tests → fixed.** Root cause in `vendor/sandbox-runtime`: the Seatbelt profile temp file was named only by PID, so concurrent calls collided and `reset()`'s sweep deleted in-flight profiles. Fix: unique per-call filenames (atomic counter) + self-cleaning wrapped command (`rm`s its own profile, preserving exit code) + age-based safety sweep. Stress-tested 5/5 clean (was flaking every run).
- **UAT run + judged against real Ollama Cloud (`gemma4:31b-cloud`).** Full 14-scenario suite: **14/14 mechanical PASS**, **13/14 LLM-judge PASS** (completion 4–5/5). The one judge FAIL (`filesystem-watch-roundtrip`) was diagnosed as a **model-quality issue, not a regression**: the model issued 4 real tool calls correctly (3× `feed_search` ✓, 1× `file_read` → correctly errored on a relative path "outside lens root"), then collapsed into a narration loop mentioning "grep" 468× while emitting **0 actual grep tool calls**. The T-0468 streaming/tool-call rewrite handled every emitted call flawlessly across all scenarios (e.g. signal-extraction-e2e: 8 turns, 0 tool errors) — strong end-to-end validation of Phase 1.

**Follow-ups surfaced by the UAT (orthogonal to Phase 1):** (a) `file_read` on a relative path fails "outside lens root" — the filesystem feed/path UX is rough (filesystem feed already flagged as a stub for I-0069/P3-5); (b) nothing guards a model that *narrates* a tool call repeatedly without ever emitting it (the dup-call detector only catches repeated emitted-and-failed calls) — candidate hardening for a future engine task.

## Detailed Design **[REQUIRED]**

The detailed design of this initiative IS the findings catalogue: each finding below is a concrete defect or gap with location, mechanism, and fix sketch. Findings are numbered `P<phase>-<n>` and grouped by the phase that closes them. Severity scale: critical / high / medium / low.

> **Spin-out note (2026-06-11):** Phases 2 and 3 were spun out to [[ARAWN-I-0068]] and [[ARAWN-I-0069]], which carry their findings and are **canonical for execution** (plans, exit criteria, status updates happen there). The Phase 2/3 sections below are retained as the original review record only — do not update them; update the spun-out initiatives.

---

### Phase 1 — Make interactive chat trustworthy (deep findings)

#### P1-1 · CRITICAL (verified) · Streaming parser drops parallel tool calls and loses completion intent
`crates/arawn-llm/src/openai_compat.rs:300-306, 445-460`
- The delta handler does `tool_calls.first()` — if a model emits two tool calls in one streaming delta, the second is silently discarded.
- `StreamToolCall` has no `index` field (OpenAI spec uses `index` to key interleaved multi-tool deltas), so even calls split across deltas cannot be assembled correctly.
- `StreamDelta` does not capture `finish_reason` — `tool_calls` vs `stop` vs `length` vs `content_filter` are indistinguishable; a token-limit truncation looks identical to a normal stop.
- Missing tool-call `id` falls back to `unwrap_or_default()` → empty-string IDs corrupt assembly keying downstream in the engine.
- Related: if a stream dies mid-tool-call, malformed/incomplete arguments silently parse to `{}` and the tool executes with empty args instead of surfacing "stream interrupted" (`query_engine.rs:376-384`).
**Fix:** add `index` + `finish_reason` to the stream structs; iterate all tool calls keyed by index; reject empty IDs; treat incomplete tool-call JSON at stream end as an error, not `{}`.

#### P1-2 · CRITICAL · RwLock `.unwrap()` poisoning can kill the whole service
`crates/arawn/src/local_service/mod.rs` (~10 occurrences: 171, 176, 191, 195, …), `crates/arawn/src/main.rs:69, 911-938`
- All service hot paths unwrap RwLock guards (`feed_runtime.read().unwrap()`, `ceremony_service.write().unwrap()`, the `notice_tx` broadcast wrapper). One panic in any writer poisons the lock; every subsequent request panics until restart. For a process meant to run unattended overnight, this is the difference between "one ceremony failed" and "the server is dead until morning."
**Fix:** switch to `parking_lot::RwLock` (no poisoning) or map poisons to recoverable errors.

#### P1-3 · HIGH · Missing API keys fail late and with provider-mismatched messages
`crates/arawn/src/llm_pool.rs`, `crates/arawn/src/startup/helpers.rs:18-24`, `crates/arawn-llm/src/error.rs:84-87`
- Anthropic configs fail fast on missing key; OpenAI-compatible providers (Groq/Ollama/OpenAI) accept `None` at construction and only error at warmup/first message — the user types their first prompt before learning the env var was missing.
- The Auth `user_message()` hardcodes `GROQ_API_KEY` as the example regardless of actual provider.
**Fix:** validate key presence per provider at pool construction; parameterize the auth error message with the configured `api_key_env`.

#### P1-4 · HIGH · `arawn tui` doesn't check for a running server; port conflicts are cryptic
`crates/arawn/src/main.rs:1243` (TUI launch), `crates/arawn/src/startup/cli.rs:12-16` (CLI does this right), `crates/arawn/src/ws_server/mod.rs:299`
- CLI one-shot mode prints "Start the server first: arawn serve"; TUI mode just launches and surfaces a raw WebSocket connect error.
- `serve` on an occupied port propagates an undecorated bind error — no "port 3100 in use; is another arawn serve running? use --port".
- Stale `server.token` after a crash causes clients to retry with an invalid token (`ws_server/mod.rs:309-311`).
**Fix:** preflight connectivity check in TUI with actionable message; decorate bind errors; treat stale token as reconnect-with-refresh.

#### P1-5 · HIGH (verified) · Onboarding is broken: `arawn init` absent, docs drift
`README.md:17,22,44`, `crates/arawn/src/config.rs:42`
- README references `arawn init` (T-0194) — the subcommand does not exist in the CLI.
- README quickstart uses model `openai/gpt-oss-120b`; the config default is `openai/gpt-oss-20b`; the tutorial also says 120b.
- README links `docs/src/getting-started.md` — file does not exist (actual: `docs/src/tutorials/first-chat.md`).
**Fix:** ship a minimal `arawn init` (write config, prompt for provider/key env, validate); fix the model string and the dead link.

#### P1-6 · HIGH · Compaction is unvalidated and degrades silently with no recovery path
`crates/arawn-engine/src/compactor.rs:70-189`, `crates/arawn-engine/src/query_engine.rs:346, 354-413`
- After summarizing, the code logs `tokens_after` but never validates `tokens_after < tokens_before`. A verbose summary that grows the session is recorded as success and resets the circuit breaker.
- After 3 hard failures the breaker silently disables compaction for the session; the user discovers this 20 turns later as a raw context-overflow error. No alert, no "clear old messages?" recovery, no graceful save-and-abort.
- Microcompact hardcodes `keep_recent=6` (full compactor is configurable — inconsistent).
- Compaction retries have no backoff; transient LLM errors burn 3 rapid attempts.
**Fix:** validate shrinkage (treat non-shrinking summaries as failure); surface breaker state + context pressure to the client; add a recovery action; unify keep_recent config.

#### P1-7 · HIGH · Retry layer retries permanent errors and ignores Retry-After
`crates/arawn-llm/src/error.rs:39, 76`, `crates/arawn-llm/src/retry.rs:40-42`
- `is_retryable()` includes `is_request()` — DNS NXDOMAIN (typo'd provider URL) and TLS failures retry through the full 1s/2s/4s/8s schedule before the user sees the real error (30+ seconds of nothing).
- 429 `Retry-After` headers are ignored; no jitter; 408 is classed non-retryable.
**Fix:** exclude permanent request-phase errors; honor Retry-After; add jitter; retry 408.

#### P1-8 · MEDIUM · Tool filtering can drop a tool the agent just used
`crates/arawn-engine/src/query_engine.rs:1115-1200`
- After the first turn, tools are filtered by keyword/category match against the new user message. A tool used successfully last turn can vanish from the next turn's list if the message doesn't keyword-match, producing "tool not registered" rejections for a tool that was available moments ago.
**Fix:** always include tools used earlier in the session (sticky set), or filter only on token pressure.

#### P1-9 · MEDIUM · Ask-mode permission checks with no prompter silently deny
`crates/arawn-engine/src/permissions/checker.rs:448-488`
- When a rule resolves to Ask and no prompter is attached, the checker logs a warning and returns Denied; the agent (and user) see an ordinary denial with no hint the real cause is a missing prompter wiring.
**Fix:** return a distinct error string ("ask required but no prompter attached") so misconfiguration is diagnosable.

#### P1-10 · MEDIUM · Duplicate-failing-call detection is fragile
`crates/arawn-engine/src/query_engine.rs:485-500`
- Detection keys on exact argument hashes (whitespace variation resets the counter) and `failed_call_counts` accumulates without session-scoped clearing if the engine is reused.
**Fix:** normalize args before hashing; scope/clear counts per session.

---

### Phase 2 — Make the background brain trustworthy (deep findings)

#### P2-1 · CRITICAL · No unified health/status surface; failures are invisible (the keystone finding)
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
**Fix (one coherent deliverable):** a `/status` RPC + TUI panel reporting per-subsystem state (each feed: enabled/last_run/last_error; ceremonies: engine availability + last run per ceremony; steward: last pass + error count; embedder: state + pending backlog; extraction: cursor positions per lens; permission audit tail), backed by persisted failure rows (ceremony run history table, steward error log) and a readiness gate.

#### P2-2 · CRITICAL · Feed auth lifecycle: broken feeds retry forever, token refresh can silently lose state
`crates/arawn-feeds/src/dispatch.rs:93-97, 168-182`, `crates/arawn-auth/src/oauth2.rs:178-180`, `crates/arawn-integrations/src/google_common.rs:137-143`
- `FeedError::Auth` becomes a generic `TaskError::ExecutionFailed`; cloacina retries indefinitely on schedule. An expired/revoked token means days of failed runs burning API quota, discoverable only by manually reading feed status.
- "Token revoked, reconnect needed" and "transient network blip" are not distinguished anywhere user-visible.
- When a refreshed token fails to persist (disk full/permissions), it's a log warning; the in-memory token works until restart, then the process reloads the stale token and fails confusingly.
**Fix:** auto-pause feeds on `AuthExpired` with a persisted "reconnect needed" reason (surfaced via P2-1); make token-persist failure an error that marks the integration degraded; map auth errors to a non-retryable task outcome.

#### P2-3 · HIGH · Ceremony writes lack transactional integrity
`crates/arawn-ceremonies/src/engine.rs:124-133, 154-182, 235-279`
- The item-write loop is auto-commit per item: if item N+1 fails, items 1..N remain — a partially-written tablet is presented as a real tablet. Pipeline error handling deletes the tablet row but not pattern rows written before the LLM compose call (orphaned cruft, confusing audits).
- No transaction/locking against concurrent user `patch_item()` — ceremony writes can clobber user edits (TOCTOU).
- An `open` tablet cannot be force-regenerated; fixing a bad LLM output requires manual row deletion.
**Fix:** wrap detect→compose→write in a transaction or savepoint; track and roll back pattern rows on error; add `force` to dispatch; return item counts in DispatchOutcome so partial success is visible.

#### P2-4 · HIGH (verified) · Session promotion does not exist; lens creation is non-atomic
`crates/arawn-storage/src/store.rs:65-81`, `crates/arawn-storage/src/jsonl.rs:157` — implements ARAWN-T-0012
- `JsonlMessageStore::move_session()` exists; `SessionStore` can update rows; but no `Store::promote_session(session_id, new_lens_id)` composes the SQLite update + JSONL file move atomically. The scratch→lens promotion workflow from the vision cannot be performed at all.
- `create_lens` inserts the SQLite row, then `create_dir_all` — a mkdir failure leaves a lens row with no directory and no rollback.
**Fix:** implement `promote_session` (SQLite txn + file move + rollback on failure); make lens creation mkdir-first or roll back the row; expose a `/lens promote` RPC + TUI command.

#### P2-5 · HIGH · Embedding lifecycle: stale vectors and a stuck pass
`crates/arawn-projections/src/embed.rs:113-180, 147-154, 193-284`, `crates/arawn-projections/src/store.rs:465-488`
- `write_embedding()` never validates `body_hash` against the row's current body. If a body updates between the pending-rows fetch and the vector write, the row flips to `embedded` with a vector for stale text — and never re-embeds because it's no longer pending. Semantic search silently returns results based on superseded content.
- If `embed_batch()` errors, the batch loop makes zero progress; a persistently failing embedder spins the pass forever while the backlog grows unbounded.
**Fix:** compare-and-set on `body_hash` at vector write (mismatch → leave pending); on batch failure mark rows errored/backoff rather than retrying the same head-of-line batch; expose pending/error counts (via P2-1).

#### P2-6 · HIGH · Memory store concurrency races corrupt dedup and confidence
`crates/arawn-memory/src/store.rs:446-461, 465-502`
- `store_fact()` is FTS-search-then-reinforce with no transaction: a concurrent delete between search and reinforce errors out and the incoming fact is lost without user-visible signal.
- `reinforce_entity()` is read-increment-write without a lock: concurrent reinforcement undercounts (two threads read 5, both write 6), silently corrupting confidence ranking that the extraction/ranking layers depend on.
**Fix:** wrap dedupe-or-insert in a transaction; make reinforcement a single SQL `UPDATE ... SET count = count + 1`.

#### P2-7 · MEDIUM · Extraction is non-debuggable and not fully idempotent
`crates/arawn-extractor/src/cot.rs:255-259, 451, 494-501`, `crates/arawn-extractor/src/runner.rs:210-264`
- `link_by_name` resolution takes the first FTS hit; the `_floor: f32` confidence parameter is accepted but unused — typos can link signals to entirely wrong entities with no threshold.
- No per-run provenance (extraction run ID) on EXTRACTED_FROM edges → backfill re-runs can duplicate entities in the lens KB.
- Valid-but-empty LLM output advances the cursor (row never revisited), so extraction outcomes depend on model mood — re-running a lens over the same feed is not deterministic.
- Token filter `len() >= 3` drops short tokens ("AI") from global-fact context lookup — silent recall loss in classify.
- No user-facing way to see why a row was classified in/out of scope, re-run extraction on a single row, or mark a signal as garbage (current recourse: manual memory deletion).
- `in_flight` dedup gate is per-process; two processes can race the same cursor.
**Fix:** wire the floor parameter; stamp run IDs; distinguish "extracted empty" from "skipped"; add `signal_explain` / `extract_rerun <row>` / `signal_dismiss` tooling.

#### P2-8 · HIGH · Integration test suite over the seams (currently hollow)
`crates/arawn-tests/src/lib.rs` (one comment, zero tests)
- Per-crate unit tests are real (300+, passing) but every bug class above lives in the *seams* that have zero coverage: session lifecycle end-to-end (create → append → load → compact → promote), store atomicity under injected failure (mkdir fails, JSONL write fails), startup ordering (feed runtime/ceremony engine absent), stream interruption mid-tool-call, JSONL corruption handling (currently skip-with-warn, `jsonl.rs:88-99`).
**Fix:** 10–20 integration tests in arawn-tests covering exactly these scenarios; wire into `angreal test integration`.

---

### Phase 3 — TUI correctness + GUI-prerequisite cleanups (deep findings)

> **Restructured 2026-06-11 per ADR ARAWN-A-0005** (web GUI becomes the primary review/triage surface; TUI demoted to chat client). Phase 3 now contains only (a) TUI *correctness* fixes — bugs stay bugs in a maintained chat client — and (b) backend/protocol cleanups, several of which are prerequisites for any thin GUI client. TUI *experience* findings are recorded below as **deferred to the GUI initiative** so they aren't lost; they must shape the GUI's requirements, not new ratatui work.

#### Deferred to the GUI initiative (do NOT build in the TUI)
- **Session visibility / quick-jump** (was P3-1): only indicator of the active session is an 8-char ID in the status bar (`crates/arawn-tui/src/app/mod.rs:44-46`, sessions sub-view removed per I-0035 T-0356). After switching lenses the user can't tell resumed-vs-fresh. → GUI session browser requirement; also depends on P2-4's promotion RPC.
- **Permission/modal context richness** (was part of P3-3): UserInputRequest modals show only title+options; overlapping requests indistinguishable (`event_loop/mod.rs:1096-1126`). → GUI approval-surface requirement.
- **Brief/ceremony/action-item review UX**: tablets render as markdown in a chat pane and are poll-only (see P3-5 item "tablets never surface into lenses/chat"). → this is the GUI's core screen.

*(Finding IDs are stable: P3-1 is retired to the deferred list above; P3-2…P3-5 keep their original numbers; P3-6 is new.)*

#### P3-2 · MEDIUM · Markdown/table wrapping measures codepoints, not display cells (TUI correctness — keep)
`crates/arawn-tui/src/markdown.rs:573-615` (esp. 583, 594), `render/input.rs:87-96`
- `wrap_text()` uses `chars().count()` for word width and `take(width)` for hard breaks — CJK/emoji (2-cell glyphs) wrap wrong or overflow. Same width-vs-codepoint bug class as the previously-fixed table wrapping issue; the fix didn't reach these paths.
- Autocomplete dropdown clamps to min 20 cells and overflows terminals narrower than that.
**Fix:** use unicode-width display cells in wrap and hard-break paths; clamp dropdown to terminal width.

#### P3-3 · MEDIUM · Stale modal state survives reconnect (TUI correctness — keep; scope reduced)
`crates/arawn-tui/src/event_loop/mod.rs:1109-1125`
- Pending modal oneshot channels aren't cleaned up on disconnect — stale pending-response state can survive into the next connection.
- (The other half of the original P3-3 — modal context richness — is deferred to the GUI initiative, see list above.)
**Fix:** clear pending modal state on `WsEvent::Closed`.

#### P3-6 · MEDIUM · Permission grant shapes too narrow (engine-side — new, benefits all clients)
`crates/arawn-engine/src/permissions/checker.rs:138-144`
- Session grants match exact shapes with wildcard fallback only — approving `~/x/foo.rs` doesn't help with `~/x/bar.rs`; users re-approve near-identical operations repeatedly, in every client.
**Fix:** directory-scoped grant shapes (grant covers the parent dir or a glob), engine-side so TUI and GUI both benefit.

#### P3-4 · LOW-MEDIUM · Retry/warmup refinement
`crates/arawn-llm/src/warming.rs:27, 90-92`
- Warmup TTL is a global 4-minute constant tuned for Ollama Cloud, wasteful for Groq/OpenAI; cold-restart detection hardcodes HTTP 503.
**Fix:** per-provider TTL config; broaden cold-start detection.

#### P3-5 · Accumulated smaller findings (batch as cleanup tasks)
- MCP adapter converts transport/RPC failures into `Ok(ToolOutput::error(...))` — engine can't distinguish "MCP server crashed" from "tool failed" (`arawn-mcp/src/adapter.rs:115-118`); duplicate MCP tool names silently overwrite in the registry (`adapter.rs:26-29`).
- `ServiceError` flattens `LlmError` to `{"kind":"llm"}` — clients can't distinguish bad-key / model-not-found / rate-limited (`arawn-service/src/error.rs:41-54`).
- Usage tracking records nothing if the provider omits `usage` in the final chunk — silent accounting gaps (`arawn-llm/src/usage/tracking_client.rs:70-91`).
- Graceful shutdown doesn't guarantee JSONL append flush (`crates/arawn/src/main.rs:1233`).
- Steward runner: fixed 1-hour tick, no backoff/circuit-breaker on repeatedly failing lenses (`crates/arawn/src/main.rs:759-778`); journal accumulates duplicate unapplied proposals across passes (`arawn-steward/src/journal.rs:160-180`).
- Ceremony `match` arms panic on unexpected event variants in production code (`arawn-ceremonies/src/service.rs:1138`, `plugins/retro.rs:853`, `engine.rs:656`).
- Ceremony tablets never surface into lenses/chat — "your daily brief is ready" requires polling `ceremonies.get_today()`.
- Config hot-reload rejects invalid TOML silently (logged only) (`crates/arawn/src/config_watcher.rs:124-161`).
- Retro sweep interval hardcoded 3600s (`startup/ceremonies.rs:388-404`); DST transitions untested for scheduled ceremonies.
- Filesystem feed template is a stub — either implement or remove from the registry so it can't be configured into a no-op.
- Repo hygiene: `test.log` at root (gitignore/delete), `vendor/sandbox-runtime` patch relationship to upstream undocumented (`Cargo.toml:92`).

## Testing Strategy

- **Every correctness fix lands with a regression test** in the owning crate's inline test module (project convention: inline `#[cfg(test)]`, not separate files). The streaming-parser fixes (P1-1) in particular need table-driven tests over multi-tool deltas, missing IDs, interrupted streams, and `finish_reason` variants — modeled on the existing Ollama tool-only-content regression test in `openai_compat.rs`.
- **P2-8 is the structural deliverable**: a real integration suite in `crates/arawn-tests` exercising the seams (session lifecycle incl. promotion, store atomicity under injected failure, startup with absent subsystems, JSONL corruption). Runs under `angreal test integration`.
- **UAT stays the acceptance gate** (`angreal test uat` + `uat-judge`) for the user-visible phases: first-run flow after `arawn init`, status surface accuracy after induced feed/ceremony failure. UAT must be green before any phase is called done.

## Alternatives Considered **[REQUIRED]**

- **Big-bang hardening rewrite of the service layer** — rejected. The review showed module-level code is sound; the defects are in seams and feedback paths. Targeted fixes with regression tests are cheaper and lower-risk.
- **Fix-on-touch (no initiative, just fold fixes into feature work)** — rejected. The dominant failure mode is cross-cutting (silent failures spanning feeds/ceremonies/steward/embedder); piecemeal fixes would never produce the unified status surface, which is the single highest-leverage deliverable.
- **Observability via logs only (skip the /status surface)** — rejected. Logs already exist and didn't prevent the problem; the user does not read logs during daily use. The status surface is what converts "mysteriously stopped working" into a glance.
- **Deferring correctness fixes until after RemoteService (T-0026)** — rejected. Daily-drivability on the existing LocalService path is the adoption gate; remote mode multiplies the value of a service that is already trustworthy, not the other way around.

## Implementation Plan **[REQUIRED]**

Phases are sequential adoption gates, not strict dependency chains — Phase 1 unblocks "I use the chat every day"; Phase 2 unblocks "I trust it running unattended"; Phase 3 makes it pleasant.

### Phase 1 — Trustworthy interactive chat (findings P1-1 … P1-10)
Suggested task slicing at decomposition time:
1. Streaming protocol correctness: index/finish_reason/IDs/interrupted-stream (P1-1) — the highest-value single fix.
2. Lock-poisoning elimination via parking_lot or poison-recovery (P1-2).
3. Fail-fast config + provider-correct auth errors (P1-3) + retry classification/Retry-After/jitter (P1-7).
4. Server-lifecycle ergonomics: TUI preflight, bind-error decoration, stale token handling (P1-4).
5. `arawn init` + README/docs drift fixes (P1-5; absorbs/implements T-0194).
6. Compaction validation + user-visible context-pressure/recovery (P1-6) — coordinate with ARAWN-T-0270/T-0274 token-budget work.
7. Small-batch: sticky tool set (P1-8), ask-without-prompter diagnosability (P1-9), duplicate-call detection (P1-10).
**Exit criteria:** parallel tool calls round-trip correctly under streaming; a panic in one subsystem cannot brick the service; missing API key / absent server / occupied port each produce a one-line actionable message at the moment of the mistake; a brand-new user reaches first chat following README alone; compaction failure is visible and recoverable in-session.

### Phase 2 — spun out to [[ARAWN-I-0068]]
Task slicing, exit criteria, and execution tracking live in ARAWN-I-0068 (canonical). Findings P2-1 … P2-8 are carried there verbatim.

### Phase 3 — spun out to [[ARAWN-I-0069]]
Task slicing, exit criteria, the deferred-to-GUI requirements list, and the GUI-readiness checklist live in ARAWN-I-0069 (canonical). Findings P3-2 … P3-6 are carried there verbatim.

### Sequencing notes
- P1-1 (ARAWN-T-0468) and P1-2 (ARAWN-T-0469) are the first two tasks, full stop — both are silent-corruption/total-outage class.
- ARAWN-I-0068's status-surface work (P2-1) may start as Phase 1 wraps, since several Phase 2 fixes report into it. Per ADR ARAWN-A-0005, it is designed protocol-first.
- ARAWN-I-0069's completion is the gate for scaffolding the web GUI initiative (ADR ARAWN-A-0005 review trigger).
- This initiative completes when the seven Phase 1 tasks (ARAWN-T-0468 … ARAWN-T-0474) meet the Phase 1 exit criteria above.