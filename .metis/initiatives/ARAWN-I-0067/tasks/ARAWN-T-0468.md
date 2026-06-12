---
id: p1-1-streaming-protocol
level: task
title: "P1-1: Streaming protocol correctness — parallel tool calls, index, finish_reason, IDs, interrupted streams"
short_code: "ARAWN-T-0468"
created_at: 2026-06-11T11:07:16.095235+00:00
updated_at: 2026-06-11T14:31:01.523077+00:00
parent: ARAWN-I-0067
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0067
---

# P1-1: Streaming protocol correctness — parallel tool calls, index, finish_reason, IDs, interrupted streams

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0067]] — implements finding P1-1 (CRITICAL, verified). First task of Phase 1.

## Objective **[REQUIRED]**

Fix the OpenAI-compat streaming parser so tool calls survive intact across all providers (Groq, Ollama, OpenAI): handle multiple/parallel tool calls, track per-call `index`, capture `finish_reason`, reject empty tool-call IDs, and surface interrupted streams as errors instead of silently executing tools with `{}` args.

**Current defects** (all in `crates/arawn-llm/src/openai_compat.rs` unless noted):
- `:300-306` — delta handler does `tool_calls.first()`; a second tool call in the same delta is silently discarded.
- `:445-460` — `StreamToolCall` has no `index` field, so multi-tool calls split across deltas cannot be assembled correctly (OpenAI spec keys interleaved deltas by `index`).
- `StreamDelta` has no `finish_reason` — `tool_calls` / `stop` / `length` / `content_filter` are indistinguishable; token-limit truncation looks like a normal stop.
- `:306` — missing tool-call `id` falls back to `unwrap_or_default()` → empty-string keys corrupt assembly downstream.
- `crates/arawn-engine/src/query_engine.rs:376-384` — a stream dying mid-tool-call leaves incomplete JSON args that silently parse to `{}` and the tool executes anyway.

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [x] `StreamToolCall` carries `index`; the parser iterates **all** tool calls in a delta, and the engine assembles concurrent tool calls keyed by index, not arrival order
- [x] `StreamDelta` captures `finish_reason` and it propagates through the chunk stream so the engine distinguishes `tool_calls` / `stop` / `length` / `content_filter`; `length` and `content_filter` are surfaced to the user, not treated as normal stops
- [x] Missing/empty tool-call `id` never produces an empty-string key: rejected with a clear error or assigned a synthesized deterministic ID
- [x] A stream that ends mid-tool-call surfaces an explicit "stream interrupted" error; the tool is never executed with fallback `{}` args
- [x] Inline regression tests (project convention: `#[cfg(test)]` modules in `openai_compat.rs` / `query_engine.rs`): multi-tool single delta, multi-tool interleaved across deltas by index, missing id, each `finish_reason` variant, interrupted stream — modeled on the existing Ollama tool-only-content regression test
- [x] `angreal test unit` green for the changed crates (arawn-llm 78/0, arawn-engine 729/0); `angreal check all` carries pre-existing repo-wide failures unrelated to this task (see status note)

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Add `index: Option<u32>` to `StreamToolCall` and `finish_reason: Option<String>` to `StreamChoice`/`StreamDelta` handling; change the delta handler to iterate all tool calls, emitting per-index start/delta chunks. In the engine (`query_engine.rs`), key pending tool-call assembly by index instead of "the one in flight." Propagate `finish_reason` on the `Done` chunk. Treat stream-end with a pending incomplete tool call as an error chunk, not a flush-with-`{}`.

### Dependencies
None — this is the first task of Phase 1. ARAWN-T-0473 and ARAWN-T-0474 also touch `query_engine.rs`; land this first to minimize conflicts.

### Risk Considerations
Provider variance: Ollama/Groq may omit `index` or `id` on single-tool streams — when `index` is absent, fall back to the current single-tool path rather than erroring. Verify delta shapes against all three providers (the existing inline tests show real captured shapes; extend that corpus).

## Status Updates **[REQUIRED]**

**2026-06-11 — COMPLETE.** Threaded `index` + `finish_reason` through the provider-neutral `ChatChunk` and rewrote tool-call assembly to be index-keyed. 17 files changed.

What landed:
- **`arawn-llm/types.rs`** — new `FinishReason` enum (`Stop`/`ToolCalls`/`Length`/`ContentFilter`/`Other`) with `from_openai`/`from_anthropic` wire mappers and `is_truncated()`. `ChatChunk::ToolUseStart`/`ToolUseInputDelta` gained `index: u32`; `ChatChunk::Done` gained `finish_reason: Option<FinishReason>`. Exported from `lib.rs`.
- **`arawn-llm/openai_compat.rs`** — `StreamToolCall.index`, `StreamChoice.finish_reason`. Parser now iterates **all** tool calls in a delta (was `.first()`), keys each by `index` (falling back to array position when omitted), and synthesizes a deterministic non-empty id (`call_<index>`) when the provider omits it. `finish_reason` is captured into the `SseParser` and attached to the emitted `Done`.
- **`arawn-llm/anthropic.rs`** — emits content-block `index` on tool-use start/delta; maps `stop_reason` → `FinishReason`.
- **`arawn-engine/query_engine.rs`** — assembly rewritten from the single "current tool" flush model to an index-keyed `BTreeMap<u32, PartialToolCall>`. Finalization surfaces an explicit **"stream interrupted"** `LlmError::Stream` when a tool call's non-empty args don't parse as JSON, or when a truncating `finish_reason` lands mid-tool-call — never the old silent `{}` fallback (dead `parse_arguments` removed). `Length`/`ContentFilter` on a text-only turn append a user-visible truncation notice.
- Mechanical `ChatChunk` updates across mock/test sites (mock.rs, retry.rs, warming.rs, tracking_client.rs, harness.rs, web_fetch.rs, memory_store.rs, cot.rs, steward {doorwatch,dust,map,reshelve}). Multi-tool harness fixtures got correct distinct indices (0/1).

Tests: new inline regression tests in `openai_compat.rs` (two parallel calls in one delta; interleaved arg deltas keyed by index; synthesized id; missing-index fallback; finish_reason on Done; wire mapping). Rewrote the obsolete `harness_malformed_json_args_falls_back_to_empty_object` → `..._surface_stream_interrupted_error` to assert the new contract. The three existing parallel-tool-call harness tests now exercise the index-keyed path.

Verification: workspace builds (`cargo build --workspace --all-targets`); **arawn-llm 78 passed / 0 failed**, **arawn-engine 729 passed / 0 failed**; all 17 changed files are `rustfmt --check` clean (edition 2024); no clippy findings in the changed code.

Caveats (pre-existing, verified on clean HEAD via `git stash`):
- `angreal check all` is red independent of this task: rust 1.93's rustfmt reformats much of the repo (HEAD committed under an older toolchain), and there is a pre-existing `error[E0521]` in a clippy `--all-targets` test build. Both reproduce on clean HEAD.
- Two harness shell tests (`harness_shell_tool_receives_arguments`, `harness_raw_chunks_split_arguments`) intermittently fail under full-suite parallelism with `sandbox-exec: srt-profile-*.sb: No such file or directory` — a vendored `sandbox-runtime` temp-profile race that also fails identically on clean HEAD; both pass in isolation.
- Mid-task, `angreal check all`'s clippy `--fix` auto-applied let-chain collapses to ~25 unrelated files; these were reverted so the diff is scoped to the 17 T-0468 files.