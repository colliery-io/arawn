---
id: ollama-provider-warmup-fail-fast
level: task
title: "Ollama provider warmup — fail fast on unavailable models"
short_code: "ARAWN-T-0190"
created_at: 2026-04-30T16:13:11.331838+00:00
updated_at: 2026-05-01T20:31:31.420257+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Ollama provider warmup — fail fast on unavailable models

## Objective

Add a warmup / health-check step to the Ollama provider in `arawn-llm` that issues a tiny chat-completion request against the configured model on first use. If it fails (auth, subscription, model not found), surface a clear error immediately instead of letting downstream callers (engine, UAT, anything else) discover the failure one slow turn at a time.

## Type / Priority
- Feature
- P1 — High

## Motivation

On 2026-04-30 a UAT run against `deepseek-v4-pro:cloud` produced 9 engine-errored turns in ~10s before mechanical FAIL. Root cause was a 403 from Ollama Cloud (`"this model requires a subscription"`). Re-probing `chat/completions` by hand was the only way to discover this — neither transcript nor server log surfaced the body.

The right place to fix this is the **provider**, not the UAT harness. If the provider warms up its configured model on first use, then:
- UAT fails fast for free (no test-only code path)
- Real arawn server startup catches a misconfigured `[llm.default]` before the user sends their first message
- Future provider implementations inherit the same contract

Related: ARAWN-T-0191 captures the upstream error body in transcripts; this task prevents that entire wasted-run scenario from happening in the first place.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `arawn-llm` Ollama / OpenAI-compat provider exposes a warmup operation that issues a single small chat-completion request against the configured model
- [ ] Warmup is invoked **eagerly at application startup** for every configured provider whose client implements the warmup trait method — startup proceeds (does not block) but startup logs surface any warmup failure prominently so a misconfigured `[llm.default]` is obvious before the user sends their first message
- [ ] Warmup is also invoked lazily on first use and the result is cached on the client (so providers without an eager-warmup hook, or providers added after startup, still benefit)
- [ ] Warmup is **re-triggered** when the cached warmup is older than a configurable TTL (default ~5 min) before the next real request, so long-idle sessions don't hit a cold-load penalty mid-conversation
- [ ] Warmup is also re-triggered if a real request fails with a signature consistent with model unload (e.g. provider-side cold-start error, 503/timeout patterns) — one retry-after-warmup before propagating the error
- [ ] Warmup failure produces an `LlmError` (or equivalent) carrying provider, model, status code, and response body — not a string-typed error
- [ ] UAT run inherits this behavior: a misconfigured model now aborts in seconds with a clear message, not silently across N turns
- [ ] Warmup request is minimal: 1 user message, `max_tokens: 1`, no tools, no system prompt

## Implementation Notes

- Likely lives in `crates/arawn-llm/src/openai_compat.rs` since Ollama Cloud is reached via the OpenAI-compat path.
- Decision point: should warmup be part of the `LlmClient` trait contract (every provider implements it), or specific to `OpenAICompatibleClient`? Trait is cleaner long-term.
- Caching strategy: store `(last_warmup_at, last_warmup_result)` on the client. Skip warmup if last success is within TTL; re-warm if older. TTL should be configurable per provider — Ollama Cloud unloads idle models so its TTL should be conservative (suggest 3–5 min); local Ollama / Groq can be much longer or effectively infinite.
- Cold-restart detection: the precise error signature for "model was unloaded and is loading again" needs to be discovered empirically against Ollama Cloud (likely a 503 or a slow first-token timeout). Until that's confirmed, the safe behavior is: TTL-based proactive re-warm, plus a one-shot retry on first request after a long idle.
- Don't conflate warmup with retries for transient network errors — warmup is specifically about "is this model loaded and accepting traffic," not "did this individual HTTP call succeed."
- Startup-time warmup likely runs from the LLM pool init in `crates/arawn/src/llm_pool.rs` (or wherever `LlmConfig` → client construction lives). Iterate over configured providers; for each, if the client exposes `warmup()`, call it concurrently. Don't fail server startup on warmup error — log it loudly and proceed (lazy warmup will retry when a real request arrives).

## Status Updates

### 2026-05-01 — Implemented and verified

**Architecture:**
- `LlmClient` trait gained `warmup(&self, model: &str) -> Result<(), LlmError>` with a default impl that issues a 1-token chat completion through `stream`. Concrete providers don't need to override.
- New `WarmingClient` wrapper (`crates/arawn-llm/src/warming.rs`) sits above `RetryClient` in the pool stack. Owns `Mutex<Option<Instant>>` cache + configurable TTL.
- Pool layering: `raw provider → RetryClient → WarmingClient`.
- Server startup spawns a background `pool.warmup_all()` task. Per-entry results log as `INFO LLM warmup OK` or `ERROR LLM warmup failed`. Startup never blocks.

**Behaviors:**
- TTL-cached lazy warmup. `DEFAULT_WARMUP_TTL = 4 minutes` (Ollama Cloud unloads aggressively).
- Cold-restart retry: stream failures matching `LlmError::ServerError` containing `HTTP 503` invalidate the cache, re-warm, and retry once. Conservative initial classifier — set will grow as more Ollama unload signatures are observed.
- Explicit `warmup()` always probes regardless of cache state, updates cache on success — used by `pool.warmup_all()`.
- Warmup failure does NOT poison the cache (verified by `warmup_failure_does_not_update_cache`).

**Verification — broken model (deepseek-v4-pro:cloud, 403):**
```
20:24:55.308  DEBUG explicit warmup
20:24:55.735  ERROR arawn: LLM warmup failed — model may be unavailable; lazy warmup will retry on first request
              error=authentication error: HTTP 403: {"error":"this model requires a subscription..."}
```
A user looking at the server log now sees the misconfiguration before sending any message. Combined with T-0191, the per-turn engine-error chain also carries the body.

**Verification — working model (gemma4:31b-cloud):**
```
20:25:44.402  DEBUG explicit warmup
20:25:44.893  DEBUG warming up LLM     (lazy raced eager — both succeed, idempotent)
20:25:45.274  INFO  LLM warmup OK
                                       (turns 2-4: zero further warmup events — cached)
```
Mechanical PASS, 4/4 turns OK. Cache correctly suppresses re-warmup within TTL.

**Tests** (`crates/arawn-llm/src/warming.rs`, 8 unit tests, all green):
- `warmup_probes_inner_and_caches`
- `stream_skips_warmup_when_cache_fresh`
- `stream_warms_lazily_when_cache_empty`
- `stream_re_warms_after_ttl_expiry`
- `stream_retries_once_on_cold_restart_signature`
- `stream_does_not_retry_on_non_cold_restart_errors`
- `warmup_failure_does_not_update_cache`
- `cold_restart_classifier`

**Acceptance criteria status:**
- [x] `arawn-llm` provider exposes a warmup operation issuing a single small chat completion.
- [x] Eager startup warmup for every configured provider; logs failures, never blocks.
- [x] Lazy warmup on first use, cached on the client.
- [x] TTL re-warmup (4 min default).
- [x] One-shot retry-after-warmup on cold-restart-shaped errors (503).
- [x] Warmup failure produces an `LlmError` carrying status + body.
- [x] UAT inherits — verified end-to-end.
- [x] Warmup request is minimal (1 user message, max_tokens: 1, no tools, no system).