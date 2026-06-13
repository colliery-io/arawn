---
id: engine-no-progress-breaker-stop-a
level: task
title: "Engine no-progress breaker: stop a model that narrates a tool call without ever emitting it"
short_code: "ARAWN-T-0475"
created_at: 2026-06-12T11:51:03.172995+00:00
updated_at: 2026-06-13T14:24:42.811757+00:00
parent: ARAWN-I-0069
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0069
---

# Engine no-progress breaker: stop a model that narrates a tool call without ever emitting it

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0069]] — engine-side hardening that benefits all clients (same "backend cleanup" bucket as P3-6). Surfaced by the live-LLM UAT during the ARAWN-I-0067 Phase 1 validation.

## Objective **[REQUIRED]**

Add a "no-progress" breaker to the agent loop so a model that keeps *narrating* a tool call it never actually emits can't burn the whole iteration budget producing unusable output. Terminate the turn with a clear, user-visible reason instead.

**Observed failure (UAT, 2026-06-12, `filesystem-watch-roundtrip` vs `gemma4:31b-cloud`):** after a `file_read` returned an error, the model emitted text mentioning "grep" **468 times across the turn but issued 0 actual `grep` tool calls** — a pure narration loop ("Wait, I'll just use grep" / "Actually, I will use grep" …). The run only stopped at the iteration cap, having produced "unusable garbage" as the final answer. The existing duplicate-call detector (`query_engine.rs`, `failed_call_counts`) does NOT catch this — it only counts repeated *emitted-and-failed* tool calls with identical args, not a model that promises a call without emitting one.

## Backlog Item Details

### Type
- [x] Tech Debt — agent-loop robustness / quality guard

### Priority
- [x] P2 - Medium (a model-quality safety net; degrades gracefully today via the iteration cap, but the output is garbage and tokens are wasted)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] The agent loop detects "no progress": N consecutive iterations that produce assistant text but **zero new tool calls** (and no final answer), where N is small (e.g. 2–3) and config-defaulted.
- [ ] On trip, the turn ends with a distinct, user-visible reason (e.g. an `EngineEvent::Warning` / surfaced error: "the model kept describing a tool call without issuing it and made no progress") — NOT a silent iteration-cap exhaustion that returns garbage as the final answer.
- [ ] The breaker does **not** fire on legitimate text-only turns (a normal final assistant message with no tool calls must still complete cleanly) or on interleaved think→tool→think patterns that do make progress.
- [ ] Distinct from the existing `failed_call_counts` duplicate-call detector (which handles repeated *emitted* failing calls) — this covers the *no-emission* loop.
- [ ] Inline regression test: a mock LLM that returns text-only deltas every iteration (never a tool call, never a stop) trips the breaker within N iterations instead of running to `max_iterations`.
- [ ] `angreal check all` and `angreal test unit` green.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
In `query_engine.rs`'s turn loop, track a `no_progress_streak` counter: increment when an iteration yields assistant text but no tool calls *and* the loop is about to continue (i.e. it wasn't treated as a final answer); reset to 0 whenever a tool call is emitted or real progress is made. When the streak reaches the threshold, break out with a surfaced reason (reuse the `ProgressEvent::Notice` → `EngineEvent::Warning` path added in ARAWN-T-0473, or an `EngineError`). Decide the right boundary carefully: a turn with no tool calls is normally the *terminal* assistant answer, so the breaker must only engage when the loop would otherwise iterate again (e.g. the model emitted text that the loop doesn't treat as final, which is exactly the pathological case).

### Dependencies
Builds on the `ProgressEvent::Notice` surfacing added in ARAWN-T-0473. Touches the same `query_engine.rs` turn loop as T-0468/T-0473/T-0474 — land after those (already complete).

### Risk Considerations
False positives are the main risk — a too-eager breaker could cut off a model mid-reasoning. Mitigate with a conservative default threshold and by only counting iterations that genuinely make no progress (text but no tool call AND not a clean end-of-turn). The iteration cap remains as the ultimate backstop.

## Status Updates **[REQUIRED]**

### 2026-06-13 — COMPLETE ✅
**Reframe (important):** the literal spec — "N consecutive iterations that produce text but **zero tool calls**" — can't actually occur in this loop. `query_engine.rs` already returns at the top of the loop when `response.tool_calls.is_empty()` (a no-tool-call response IS the terminal answer). So the realizable pathology is **iterations whose emitted tool calls ALL error** — the model keeps issuing failing/invalid calls (and narrating between them, the gemma "grep" case) and never converges. The breaker targets that.

**Implementation (`arawn-engine/query_engine.rs`):**
- New `no_progress_streak` on `QueryEngine` + `max_no_progress_iterations` on `QueryEngineConfig` (0 disables). After each round of tool execution: if any tool result succeeded → reset streak to 0; if every call errored → increment. On `streak >= threshold`, end the turn — record a final assistant message with the model's last text + a surfaced reason (via `ProgressEvent::Notice`) and fire the Stop hook (`stop_reason: "no_progress"`). Never silent iteration-cap garbage.
- Distinct from `failed_call_counts` (repeated *identical* failing calls); this catches a varied-but-fruitless loop. Config-exposed via `[engine] max_no_progress_iterations` (config.rs, serde default).

**Default threshold = 5, not 2–3.** The two harness tests `harness_permission_denial_cascade_then_success` and `harness_repeated_failure_circuit_breaker` empirically show legitimate recovery does ~3 consecutive failures then pivots (3 permission denials → switch to `think` → success). A default of 3 false-positived on exactly that. 5 lets real recovery through while still being ~40× tighter than the 200-iteration cap. Those two tests now pass unchanged → they double as regression tests proving no false-positive.

**Tests:** `no_progress_breaker_trips_on_all_errored_iterations` (unregistered-tool loop → trips at the threshold with the surfaced reason, not the cap); `no_progress_breaker_does_not_fire_when_a_tool_succeeds` (think→answer completes cleanly).

**Gates:** fmt + clippy -D warnings clean · arawn-engine lib 735 passed / 0 failed · arawn config tests green.

**Note on parent:** still filed under [[ARAWN-I-0069]] (the GUI initiative) — it's an engine-loop concern, arguably mis-parented, but left as-is.