---
id: p3-5-cleanup-batch-triage-typed
level: task
title: "P3-5: Cleanup-batch triage — typed errors, MCP fidelity, tablet surfacing, fs file_read"
short_code: "ARAWN-T-0489"
created_at: 2026-06-13T14:43:32.878608+00:00
updated_at: 2026-06-13T15:30:11.896518+00:00
parent: ARAWN-I-0069
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0069
---

# P3-5: Cleanup-batch triage — typed errors, MCP fidelity, tablet surfacing, fs file_read

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0069]] — implements P3-5 (the accumulated-findings cleanup batch). **Every item is fixed or explicitly wont-fix'd with a rationale — no silent drops.**

## Objective **[REQUIRED]**

Triage the P3-5 cleanup batch from the I-0067 review. GUI-serving items first (they unblock a thin web client); the rest fixed or wont-fix'd with reasons recorded in this task's status.

### Type
- [x] Tech Debt — accumulated cleanup, GUI-serving error fidelity

### Priority
- [x] P2 - Medium (the GUI-serving sub-items); others P3

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

**GUI-serving (prioritize):**
- [ ] `ServiceError` no longer flattens `LlmError` to `{"kind":"llm"}` — clients can distinguish bad-key / model-not-found / rate-limited (`arawn-service/src/error.rs:41-54`).
- [ ] MCP adapter distinguishes transport/RPC failure from tool failure (not all `Ok(ToolOutput::error)`) and doesn't silently overwrite duplicate MCP tool names (`arawn-mcp/src/adapter.rs:26-29, 115-118`).
- [ ] Ceremony tablets can surface into chat/lenses (a "your brief is ready" signal), not poll-only `ceremonies.get_today()`.
- [ ] Filesystem `file_read`: agent-supplied relative paths resolve against the lens root (so `transcripts/foo.md` works) OR return a clear, actionable error; the filesystem feed template stub is implemented or removed from the registry. (UAT `filesystem-watch-roundtrip` regression.)

**Remaining items — each fixed OR wont-fix'd with rationale in Status Updates:**
- [ ] Usage tracking when provider omits final-chunk `usage` (`usage/tracking_client.rs:70-91`).
- [ ] Graceful-shutdown JSONL flush guarantee (`main.rs:1233`).
- [ ] Steward tick backoff/circuit-breaker on repeatedly-failing lenses + duplicate-proposal accumulation (`main.rs:759-778`, `arawn-steward/journal.rs:160-180`).
- [ ] Ceremony `match` arms that panic on unexpected event variants in prod code (`service.rs:1138`, `plugins/retro.rs:853`, `engine.rs:656`).
- [ ] Config hot-reload silent rejection of invalid TOML (`config_watcher.rs:124-161`).
- [ ] Retro sweep hardcoded 3600s + DST handling (`startup/ceremonies.rs:388-404`).
- [ ] Repo hygiene: root `test.log`; document the `vendor/sandbox-runtime` upstream-patch relationship (`Cargo.toml:92`).
- [ ] `angreal check all` + `angreal test unit` green; UAT green.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Work GUI-serving items first (typed `ServiceError` variants surfaced through the WS protocol; MCP adapter error typing + name-collision guard; a tablet-ready notice via the existing `ServerNotice`/`/status` plumbing; the fs file_read relative-path resolution). Then sweep the rest; for any item that's genuinely not worth fixing now, record an explicit wont-fix rationale here rather than dropping it. Land as a few focused commits, not one mega-commit.

### Dependencies
Builds on the T-0476 status/notice surface (tablet-ready, error fidelity) and the T-0477 failure history.

### Risk Considerations
Big surface area — keep each sub-fix small + tested, commit incrementally. The panicking `match` arms are a real crash risk; prioritize converting them to graceful handling.

## Status Updates **[REQUIRED]**

**2026-06-13 — Triage complete. Every item resolved (fixed or wont-fix'd with rationale). Landed as 5 focused commits.**

### GUI-serving items
1. **ServiceError LLM fidelity — FIXED** (commit 9bfa379). `details()` flattened every engine LLM failure to `{"kind":"llm"}`. Added `LlmError::kind()` and thread the inner classification as `llm_kind` (+ `retry_after_secs` for rate limits). 4 tests. Serves TUI + future web client.
2. **MCP adapter transport-vs-tool + name collision — FIXED** (commit 1aa66f8). Transport/RPC failures now return `Err(ToolError)` (engine routes through the failure path / PostToolUseFailure hook, no turn abort) while a tool's own `is_error` stays `Ok(ToolOutput::error)`. Added `ToolRegistry::contains()` + a guard in the MCP manager that logs-and-skips a duplicate tool name instead of silently shadowing; reconnect clears its own stale prefix first. Registry test added; live adapter/peer path needs a mock MCP server (not unit-testable here).
3. **Ceremony tablet-ready signal — ALREADY IMPLEMENTED (verified, no work needed).** `TabletGenerated` already broadcasts a `briefing_ready` ServerNotice (`startup/ceremonies.rs`), and the TUI's `apply_system_notice` both posts a visible toast AND refreshes the brief cache (`event_loop/notices.rs`). This is the push signal the item asks for — not poll-only. (I-0035 Phase 4 / T-0359.)
4. **fs file_read relative paths + filesystem feed stub — FIXED/verified** (commit 04877fa). `file_read` already resolves relative paths against the lens root (`working_dir().join(path)`); added a nested-path regression test (`transcripts/foo.md`). The `filesystem/folder` feed template's `run()` was implemented by T-0418 and is registered — it is **not** a stub; corrected the stale module doc that still claimed `unimplemented!()`.

### Remaining sweep
5. **Config hot-reload crash on invalid TOML — FIXED** (commit 97ac4e8). The cited "silent rejection" was actually worse: `reload()` called `ArawnConfig::load()` which `process::exit(1)`s on a parse error → a typo would **kill the running daemon**. Split out `try_load() -> Result`; `load()` keeps fail-fast startup, `reload()` now keeps the previous config and surfaces an actionable toast. 3 tests.
6. **Repo hygiene — FIXED** (commit 97ac4e8). Removed stray 0-byte root `test.log` (untracked, already gitignored via `*.log`). Documented the vendored `sandbox-runtime` patch relationship in the workspace `Cargo.toml` (points at `Cargo.toml.orig` + `upstream-sync.yml`).
7. **Ceremony panicking `match` arms — WONT-FIX (misidentified).** All three cited sites (`service.rs:1138`, `plugins/retro.rs:853`, `engine.rs:656`) are inside `#[cfg(test)]` modules (`assert_eq!` / `panic!("expected …")` test assertions), verified by locating the enclosing `#[cfg(test)]`. They are correct test-failure behavior, not production crash risk; converting them would weaken regression coverage. No genuine prod panic-on-event-variant exists at those locations.
8. **Usage tracking when provider omits final-chunk `usage` — WONT-FIX.** `UsageTrackingClient` already degrades gracefully: it records when `Done{usage}` arrives and logs a debug when a stream ends without it. The only "fix" would be local per-provider/model tokenization — a large effort for best-effort cost telemetry, and inherently approximate. The gap is already observable in logs.
9. **Graceful-shutdown JSONL flush — WONT-FIX (already guaranteed).** `JsonlStore::append` does `write_all` and the file handle closes before the `await` returns — there is no in-process `BufWriter`, so a graceful shutdown loses nothing (every appended message is already handed to the OS). Only power-loss `fsync` durability is skipped per-message, which is intentional for throughput (the batch-write path does `sync_all`).
10. **Retro sweep hardcoded 3600s + DST — WONT-FIX.** The sweep is a fixed-interval `interval(3600s)` poll, not a wall-clock-anchored schedule, so DST has no effect on it; `sweep_unreviewed_retros` decides correctness from timestamps each tick. The 3600s is a plain hourly cadence; only the "Sunday-night" comment is loosely worded. No behavior bug.
11. **Steward backoff/circuit-breaker + duplicate-proposal dedup — WONT-FIX (own ticket).** This is substantial resilience feature work (a per-lens backoff state machine + proposal dedup), explicitly P3. T-0477 already added `steward_error_log`; a full circuit-breaker should be its own scoped initiative rather than riding this cleanup batch. **Recommend filing a dedicated follow-up.**

### Verification
`angreal test unit`: green (exit 0, reached doc-tests phase, no failures). Gate clippy + fmt clean on every touched crate. **UAT not run in this environment** (needs a real LLM + the sops secrets bundle) — recommend the user run `angreal test uat` before relying on the `filesystem-watch-roundtrip` scenario; the file_read + filesystem-feed code paths both function in unit scope.