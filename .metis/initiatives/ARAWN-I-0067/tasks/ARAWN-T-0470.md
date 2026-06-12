---
id: p1-3-p1-7-fail-fast-llm-config
level: task
title: "P1-3/P1-7: Fail-fast LLM config validation + provider-correct auth errors + retry classification"
short_code: "ARAWN-T-0470"
created_at: 2026-06-11T11:07:18.835813+00:00
updated_at: 2026-06-11T15:53:27.323230+00:00
parent: ARAWN-I-0067
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0067
---

# P1-3/P1-7: Fail-fast LLM config validation + provider-correct auth errors + retry classification

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0067]] — implements findings P1-3 (HIGH) and P1-7 (HIGH).

## Objective **[REQUIRED]**

Make misconfiguration fail fast with provider-correct messages, and stop the retry layer from wasting 30+ seconds on permanent errors: validate API keys for **all** providers at LLM-pool construction (not just Anthropic), fix the hardcoded auth error text, and reclassify retryable errors (exclude DNS/TLS permanent failures, honor `Retry-After`, add jitter, retry 408).

**Current defects:**
- `crates/arawn/src/llm_pool.rs` + `crates/arawn/src/startup/helpers.rs:18-24` — Anthropic configs fail fast on a missing key; OpenAI-compatible providers (Groq/Ollama/OpenAI) accept `None` at construction and only error at warmup/first message. The user types their first prompt before learning the env var was missing.
- `crates/arawn-llm/src/error.rs:84-87` — the Auth `user_message()` hardcodes `GROQ_API_KEY` as the example regardless of the actual configured provider/env var.
- `crates/arawn-llm/src/error.rs:39` — `is_retryable()` includes `is_request()` reqwest errors: DNS NXDOMAIN (typo'd provider URL) and TLS handshake failures retry through the full backoff schedule before surfacing.
- `crates/arawn-llm/src/retry.rs:40-42` — fixed 1s/2s/4s/8s backoff: no jitter, `Retry-After` on 429 ignored.
- `crates/arawn-llm/src/error.rs:76` — HTTP 408 (request timeout) classified non-retryable; only 429 and 5xx retry.

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [x] `arawn serve` with a cloud provider whose `api_key_env` is set but unresolvable exits at startup with a one-line error naming that env var (parity with Anthropic; `missing_key_error`); local Ollama/LM Studio still build keyless (`provider_requires_key`)
- [x] Auth `user_message()` is provider-agnostic — never hardcodes `GROQ_API_KEY` (test `auth_user_message_is_provider_agnostic`); the startup fail-fast path names the exact env var
- [x] DNS-resolution and TLS failures fail immediately (`is_permanent_transport_error` sniffs the source chain); timeouts retry; `is_request()` request-construction errors dropped from retryable
- [x] 429 honors `Retry-After` (parsed in both clients, threaded via `RateLimited { retry_after }`, used by `RetryClient`); backoff has time-based jitter; 408 is retryable
- [x] Inline regression tests: `from_status_408_is_retryable_server_error`, `rate_limited_carries_retry_after`, `auth_user_message_is_provider_agnostic`, `cloud_provider_without_key_fails_fast_naming_env_var`, `local_provider_without_key_still_builds`
- [x] `angreal test unit` green for changed crates (arawn-llm 81/0, arawn 61/0, arawn-engine 729/0); `angreal check all` carries pre-existing repo-wide failures unrelated to this task

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Extend the client-construction path (`llm_pool.rs` / `startup/helpers.rs`) to resolve `api_key_env` for every provider at startup, mirroring the existing Anthropic check. Thread the provider name / configured env var into `LlmError::Auth` so `user_message()` renders the right remediation. In `error.rs`, stop classifying all `is_request()` errors as retryable — inspect for DNS/TLS (permanent) vs timeout/connect-reset (transient). In `retry.rs`, parse `Retry-After` from 429 responses, add jitter to the exponential schedule, and mark 408 retryable.

### Dependencies
Land after ARAWN-T-0468 (both touch `crates/arawn-llm`). `arawn init` (ARAWN-T-0472) should reuse this validation path — coordinate.

### Risk Considerations
Over-strict fail-fast would break keyless providers: local Ollama needs no API key. Only fail when an `api_key_env` is *configured* but unresolvable — absent key config for a provider that doesn't require one must stay valid. Retry-classification changes need care not to stop retrying genuinely transient request-phase errors (connection reset mid-handshake).

## Status Updates **[REQUIRED]**

**2026-06-11 — COMPLETE.** All five sub-fixes landed across `arawn-llm` + `arawn`.

- **Fail-fast config** (`startup/helpers.rs`): `build_llm_client` now rejects a cloud provider with no resolvable key, via `provider_requires_key()` (groq/openai/anthropic/mistral/together/fireworks) + `missing_key_error()` which names the configured `api_key_env`. Local providers (ollama/lmstudio) and unknown providers stay permissive. Anthropic path refactored onto the same `missing_key_error`.
- **Provider-agnostic auth message** (`error.rs`): removed the hardcoded `GROQ_API_KEY`; runtime 401/403 now say "check your API key (via `api_key`/`api_key_env`)". The precise env var is named at startup, which is where the typo is actually caught.
- **Retry classification** (`error.rs::is_retryable`): dropped `is_request()` (request-construction = permanent); added `is_permanent_transport_error()` which walks the reqwest error source chain for DNS/TLS/cert markers and bails out fast. Timeouts and genuine connection-level transients still retry.
- **Retry-After + jitter** (`error.rs`, `retry.rs`, `openai_compat.rs`, `anthropic.rs`): `RateLimited` is now a struct variant `{ message, retry_after: Option<Duration> }` with a `retry_after()` accessor; both provider clients parse the `Retry-After` header (delta-seconds) and thread it via `from_status_with_retry_after`. `RetryClient` honors it over its own backoff, and `delay_for_attempt` now adds time-derived jitter (no `rand` dependency).
- **408 retryable** (`error.rs::from_status`): 408 maps to `ServerError` (retryable) with a "request timeout" label.

Mechanical: `RateLimited(String)` → struct variant rippled to mock.rs, warming.rs, retry.rs tests, and arawn-engine harness.rs (5 sites, sed-converted, long lines wrapped).

**Tests (all new/updated passing):** `from_status_408_is_retryable_server_error`, `rate_limited_carries_retry_after`, `auth_user_message_is_provider_agnostic` (arawn-llm); `cloud_provider_without_key_fails_fast_naming_env_var`, `local_provider_without_key_still_builds` (arawn). DNS/TLS classification not unit-tested directly (fabricating a real `reqwest::Error` in a test is impractical); the `is_permanent_transport_error` heuristic is covered by reasoning + the source-chain sniff.

**Verification:** `cargo build --workspace` clean; arawn-llm **81/0**, arawn **61/0**, arawn-engine **729/0**; all changed files `rustfmt --check` clean (edition 2024); no clippy findings in changed code; **no new dependency**. `angreal check all` red is pre-existing (rust-1.93 rustfmt churn + a pre-existing `E0521` in a clippy all-targets build), reproduces on clean HEAD.