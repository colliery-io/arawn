---
id: engine-no-progress-breaker-stop-a
level: task
title: "Engine no-progress breaker: stop a model that narrates a tool call without ever emitting it"
short_code: "ARAWN-T-0475"
created_at: 2026-06-12T11:51:03.172995+00:00
updated_at: 2026-06-12T11:51:03.172995+00:00
parent: ARAWN-I-0069
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


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

*To be added during implementation*