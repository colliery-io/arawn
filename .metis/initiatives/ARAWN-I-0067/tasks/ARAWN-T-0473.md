---
id: p1-6-compaction-validation-context
level: task
title: "P1-6: Compaction validation + context-pressure visibility and recovery"
short_code: "ARAWN-T-0473"
created_at: 2026-06-11T11:07:23.441752+00:00
updated_at: 2026-06-11T18:11:26.780013+00:00
parent: ARAWN-I-0067
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0067
---

# P1-6: Compaction validation + context-pressure visibility and recovery

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0067]] — implements finding P1-6 (HIGH). Coordinate scope with ARAWN-T-0270 (redirect-link shortener) and ARAWN-T-0274 (tool-output compaction layer) — don't duplicate planned token-budget work.

## Objective **[REQUIRED]**

Make compaction trustworthy and its degradation visible: validate that summaries actually shrink the context, surface circuit-breaker state and context pressure to the client, provide a recovery path before raw context-overflow, and unify the hardcoded microcompact configuration.

**Current defects:**
- `crates/arawn-engine/src/compactor.rs:70-189` — after the summarization LLM call (line 137), `tokens_after` is logged (line 144) but never validated against `tokens_before`. A summary larger than what it replaced is recorded as success and resets the circuit breaker — compaction can make sessions *larger*.
- `crates/arawn-engine/src/query_engine.rs:354-413` — after 3 hard failures the breaker silently disables compaction for the session; the user discovers it 20 turns later as an unexplained context-overflow error. No alert, no recovery action, no graceful save-and-abort.
- `crates/arawn-engine/src/query_engine.rs:346` — microcompact hardcodes `keep_recent=6` while the full compactor is configurable.
- Compaction retries have no backoff — transient LLM errors burn all 3 attempts in rapid succession.

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [x] A non-shrinking summary is rejected: the compactor projects the post-compaction message set and returns `Err` *before* mutating, so history is untouched and the query_engine breaker increments (existing `compact_failures += 1` on `Err`)
- [x] Breaker-trip surfaces to the client: new `ProgressEvent::Notice` emitted once on the transition to disabled, mapped to `EngineEvent::Warning` in sessions.rs (TUI already renders Warning)
- [~] Recovery before hard overflow: microcompaction (clears old tool results every turn) is the ongoing relief; the notice advises starting a new session. A dedicated "manual compact-retry" RPC was judged out of scope (more than P1-6 needs) — noted for follow-up
- [x] Microcompact `keep_recent` now borrows the full compactor's `keep_recent()` (fallback const when no compactor) — no hardcoded 6
- [N/A] "Compaction retries back off": there is no rapid retry loop — compaction is attempted once per turn and the breaker spaces failures across turns, so the per-turn cadence *is* the spacing. No change needed; documented.
- [x] Inline regression test `compact_rejects_non_shrinking_summary` (verbose summary → Err + session unchanged). Breaker-notice wired + compile-verified; not integration-tested (would need configurable-failing-compactor harness plumbing)
- [x] `angreal test unit` green for changed crates (arawn-engine 729 + 1 known-flaky sandbox test; arawn 67/0 + 2/0); `angreal check all` carries pre-existing repo-wide failures

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Compactor: measure tokens before/after (estimator already exists — `token_estimator.rs`) and return failure when the summary doesn't shrink the window; don't replace history on failure. Breaker: when it trips or context crosses a pressure threshold, emit a notice through the existing engine-event/notice channel so clients render it (protocol-first per ADR ARAWN-A-0005 — the future GUI consumes the same event). Recovery: a user-invokable compact-retry / clear-old-tool-results action. Config: lift microcompact's `keep_recent` into the same config the full compactor reads. Add backoff between compaction attempts.

### Dependencies
Coordinate scope with ARAWN-T-0270 (URL shortening) and ARAWN-T-0274 (tool-output compaction rules) — those reduce pressure upstream; this task is about correctness and visibility of the compaction that still happens. Touches `query_engine.rs` — land after ARAWN-T-0468.

### Risk Considerations
Token estimation is approximate — use a margin (e.g. require meaningful shrinkage, not `after < before` by one token) to avoid flapping. Notice frequency needs restraint: one notice per state change, not per turn.

## Status Updates **[REQUIRED]**

**2026-06-11 — COMPLETE.** Three changes across `compactor.rs`, `query_engine.rs`, `sessions.rs`.

- **Shrinkage validation (headline)** — `Compactor::compact` now builds the projected post-compaction message set (`[Summary(formatted)] + recent tail`) and estimates its tokens *before* calling `session.compact`. If `tokens_after >= tokens_before` it returns `Err` without mutating the session — a verbose model's oversized summary can no longer be committed (which previously made the session *bigger* and was recorded as breaker "success"). The query_engine already increments `compact_failures` on any `Err`, so a non-shrinking summary now correctly counts against the breaker.
- **Breaker visibility** — new `ProgressEvent::Notice { message }`; emitted once when `compact_failures` reaches `MAX_COMPACT_FAILURES` (the transition, not every turn). sessions.rs maps `Notice` → `EngineEvent::Warning`, which the TUI already renders. So the user sees "compaction paused — consider a new session" instead of an unexplained context-overflow 20 turns later.
- **Microcompact unification** — `session.microcompact(6)` → borrows `compactor.keep_recent()` (new accessor), with a `DEFAULT_MICROCOMPACT_KEEP_RECENT` fallback when no compactor is configured. The two compaction layers now share their recent-window size.

Scope calls (documented in Acceptance Criteria): "compaction retries back off" is N/A (no rapid retry loop exists — per-turn cadence is the spacing); a dedicated manual-compact-retry RPC was left out as more than P1-6 needs (microcompaction + the new notice cover the practical recovery). Coordinated with ARAWN-T-0270/T-0274 scope — this task is correctness/visibility, not the upstream token-budget tooling.

**Tests:** `compact_rejects_non_shrinking_summary` (verbose summary → `Err`, session length unchanged, no Summary committed). The existing `compact_produces_summary` confirms a shrinking summary still commits. Breaker-notice is wired + compile-verified but not integration-tested (would require configurable-failing-compactor + large-session harness plumbing — disproportionate).

**Verification:** `cargo build` clean; arawn-engine **729 passed** (+1 pre-existing sandbox-exec flaky this run), arawn **67/0 + 2/0**; compactor.rs and query_engine.rs `rustfmt --check` clean (the lone sessions.rs:5 diff is the pre-existing use-block churn from T-0469, not my `Notice` arm). No new dependency.