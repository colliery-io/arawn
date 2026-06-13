---
id: p3-4-retry-warmup-refinement-per
level: task
title: "P3-4: Retry/warmup refinement — per-provider TTL + broader cold-start detection"
short_code: "ARAWN-T-0488"
created_at: 2026-06-13T14:43:31.451863+00:00
updated_at: 2026-06-13T15:11:21.741684+00:00
parent: ARAWN-I-0069
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0069
---

# P3-4: Retry/warmup refinement — per-provider TTL + broader cold-start detection

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0069]] — implements P3-4 (LOW-MEDIUM).

## Objective **[REQUIRED]**

Make LLM warmup/cold-start handling provider-aware instead of one-size-fits-Ollama: a per-provider warmup TTL and broader cold-start detection.

**The defect** (`crates/arawn-llm/src/warming.rs:27, 90-92`): the warmup TTL is a global 4-minute constant tuned for Ollama Cloud — wasteful re-warming for Groq/OpenAI which don't go cold — and cold-restart detection hardcodes HTTP 503, missing other cold-start signals.

### Type
- [x] Tech Debt — provider-aware resilience tuning

### Priority
- [x] P3 - Low

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] Warmup TTL is configurable per provider (e.g. long for Ollama, effectively off for Groq/OpenAI) rather than a single global constant.
- [ ] Cold-start detection recognizes more than bare HTTP 503 (e.g. connection-refused / model-loading signals) where applicable.
- [ ] Sensible defaults preserve current Ollama behavior; non-cold providers stop needlessly re-warming.
- [ ] Inline tests for the TTL selection + cold-start classification.
- [ ] `angreal check all` + `angreal test unit` green.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
In `arawn-llm/warming.rs`, replace the global `DEFAULT_WARMUP_TTL` use with a per-provider TTL (the `WarmingClient` already carries a provider string — key off it). Broaden the cold-start predicate beyond `503`. Wire provider TTLs from config where it makes sense.

### Dependencies
`arawn-llm` warming + the pool/provider config. Lowest-priority slice.

### Risk Considerations
Don't disable warmup where it's actually needed (Ollama). Keep defaults conservative; this is tuning, not behavior change for the common path.

## Status Updates **[REQUIRED]**

**2026-06-13 — Implemented & complete.**
- **Per-provider TTL** (`warming.rs`): new `warmup_ttl_for_provider(provider) -> Duration` + `NEVER_COLD_WARMUP_TTL` (1 year, ~"warm once, never re-warm"). Hosted providers (groq/openai/anthropic/together/fireworks, substring + case-insensitive) get the long TTL; Ollama/local/unknown keep the short `DEFAULT_WARMUP_TTL` (4 min). `WarmingClient::new` now derives the TTL from the provider instead of the global constant; `with_ttl` retained for explicit override.
- **Broadened cold-start detection**: `looks_like_cold_restart(provider, err)` now matches HTTP 503 AND "model loading / not loaded / warming up" messages (any provider, via `mentions_model_loading`), PLUS connection-refused (`reqwest is_connect()`) ONLY for cold-capable providers (ollama/local/llamacpp/lmstudio) — a refused connection on a hosted provider is a real outage, not a cold model, so it isn't widened there. Timeouts stay with the retry layer.
- **Config override**: `LlmConfig.warmup_ttl_secs: Option<u64>` (serde default None) lets a profile pin its own TTL; the pool uses `with_ttl` when set, else provider-derived `new`.
- Defaults preserve current behavior: Ollama still re-warms every 4 min; the only change for hosted providers is they stop needlessly re-warming.
- **Tests (4 new + updated classifier):** `cold_restart_recognizes_model_loading_messages`, `cold_restart_connection_refused_only_for_cold_providers` (real refused connect to 127.0.0.1:1 — ollama yes, groq no), `ttl_is_short_for_cold_providers_and_long_for_hosted`, `hosted_provider_does_not_rewarm_within_session` (end-to-end through `new("groq")`). Updated `cold_restart_classifier` for the new signature.
- `cargo test -p arawn-llm --lib`: 85 passed; `arawn` pool tests 14 passed. Gate clippy clean (both crates); fmt clean.