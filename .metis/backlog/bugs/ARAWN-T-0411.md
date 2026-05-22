---
id: groq-backend-hardening-failed-generation
level: task
title: "Groq backend hardening — surface failed_generation, expand is_retryable, document model selection"
short_code: "ARAWN-T-0411"
created_at: 2026-05-22T16:50:00.000000+00:00
updated_at: 2026-05-22T16:50:00.000000+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#bug"


exit_criteria_met: false
initiative_id: NULL
---

# Groq backend hardening — surface failed_generation, expand is_retryable, document model selection

## Backlog Item Details

### Type
- [x] Bug — Active diagnostic blindspot (`failed_generation` discarded) + missing retry coverage for a known nondeterministic upstream failure mode.
- [ ] Feature
- [ ] Tech Debt
- [ ] Chore

### Priority
- [ ] P0
- [ ] P1
- [x] P2 — Quality / robustness; not blocking, but the diagnostic gap makes future Groq issues harder to debug and the retry gap leaves some 4xx edge cases exposed.
- [ ] P3

### Impact Assessment

- **Affected users**: anyone running arawn against Groq (or any OpenAI-compatible endpoint with strict server-side tool-call validation). Currently that's the secondary `[llm.*]` entries pointing at `https://api.groq.com/openai/v1`.
- **Reproduction**:
  1. Configure an `[llm.<name>]` entry pointing at Groq with a Llama/Qwen/gpt-oss model that emits native tool-call formats.
  2. Run a tool-heavy session (any UAT scenario will do).
  3. Observe intermittent errors like "Failed to call a function. Please adjust your prompt." with no actionable detail.
- **Expected vs actual**:
  - Expected: error message includes the raw model output (`failed_generation`) so the operator can diagnose whether it's a format-bias issue, a schema issue, or a real prompt issue.
  - Actual: arawn drops `failed_generation` on the floor in `error.rs::extract_api_message` (only pulls `error.message`).
- **Secondary impact**: today the 502 case already retries via `LlmError::ServerError`, but if Groq ever surfaces these failures as 4xx (we've seen muninn report unusual code paths), the `Api(_)` branch only retries on Anthropic's `"tool_use_failed"` string — not Groq's `"Failed to call a function"` / `"Failed to parse tool call arguments"`. Belt-and-suspenders gap.

### Source

Pattern lifted from a sister project (muninn — `../muninn/crates/muninn-rlm/src/{groq.rs,backend.rs,error.rs}`) that hardened its Groq path after hitting intermittent 502 `backend_error` failures during UAT. They wrote up the lessons portably; this task ports the applicable lessons to arawn.

## Objective

Bring arawn's Groq backend to parity with muninn's hardened path for the three lessons that apply (lessons #1, #2, and #4 from the muninn writeup; lesson #3 — `tool_choice` forwarding — doesn't apply because arawn doesn't use forced tool calls).

## Acceptance Criteria

### 1. Surface `failed_generation` in Groq error path

- [ ] `crates/arawn-llm/src/error.rs::extract_api_message` (or a sibling helper) deserializes the `error.failed_generation` field in addition to `error.message`.
- [ ] When `failed_generation` is present and non-empty, the surfaced error message is formatted as `"{message} | failed_generation: {raw_model_output}"`.
- [ ] Truncation policy: if `failed_generation` is over ~2KB, truncate to the first 2KB and append `"…[truncated]"` — long native-format dumps shouldn't blow up the log.
- [ ] Both `crates/arawn-llm/src/groq.rs` (dedicated Groq client) and `crates/arawn-llm/src/openai_compat.rs` (generic path, which Groq users also use) call through `from_status`, so the fix lands in one place.
- [ ] Unit test in `error.rs::mod tests`:
  - Body: `{"error":{"message":"Failed to call a function. Please adjust your prompt.","type":"backend_error","failed_generation":"<function=foo>{\"x\":1}</function>"}}`.
  - Assert: extracted message contains both the original message AND the `failed_generation` content.
- [ ] Unit test: body without `failed_generation` still parses correctly (backward compat).

### 2. Expand `is_retryable` to catch Groq tool-call patterns explicitly

- [ ] `LlmError::is_retryable` in the `Api(_)` branch additionally matches `"Failed to call a function"` (case-insensitive) and `"Failed to parse tool call arguments"`.
- [ ] Existing `"tool_use_failed"` (Anthropic) and `"overloaded"` patterns remain.
- [ ] Unit tests for the two new retryable patterns (Groq-style API error with the new strings → `is_retryable() == true`).
- [ ] Unit test: random `Api(_)` error message (`"400 Bad Request: invalid model"`) still NOT retryable.

### 3. Document model selection for Groq users

- [ ] Either `docs/src/llm-providers.md` (preferred, if it exists) or a new `docs/src/groq.md` (link from index) contains a model-stability table for Groq based on muninn's findings:

| Model | Stability with retry |
|---|---|
| `qwen/qwen3-32b` | Best. Occasional `<tool_call>` wrap, retry recovers. |
| `openai/gpt-oss-120b` | Sporadic empty content. Usable but slower. |
| `llama-3.3-70b-versatile` | Retry-dependent; deterministic XML emission on some prompts. |
| `openai/gpt-oss-20b` | **Avoid.** Deterministically leaks `<\|channel\|>` tokens into tool names. Retry can't recover. |

- [ ] Document recommends `qwen/qwen3-32b` as the Groq default for tool-call-heavy work.
- [ ] `angreal docs build` clean.

### Validation

- [ ] `cargo check --workspace` clean.
- [ ] `cargo test -p arawn-llm` green (new unit tests + existing).
- [ ] `cargo test --workspace --lib` ≥ 1,758 + new tests pass.
- [ ] Manual smoke: run a session against Groq with a model known to emit native formats (e.g., llama-3.3-70b); confirm error messages now include `failed_generation` content when failures occur.

## Implementation Notes

### File pointers

- `crates/arawn-llm/src/error.rs::extract_api_message` — currently extracts only `error.message`. Extend here.
- `crates/arawn-llm/src/error.rs::is_retryable` — extend the `Api(_)` match arm.
- `crates/arawn-llm/src/groq.rs:79-83` — error path calls `LlmError::from_status(status, text)`. No change needed if `extract_api_message` is upgraded — the fix lands transparently.
- `crates/arawn-llm/src/openai_compat.rs:156-157` — same pattern; same transparent fix.

### Out of scope

- `tool_choice` enum and forced-tool-call plumbing. arawn doesn't use this today; if it ever lands, the lesson #3 lookup applies (verify the field reaches the wire by logging one outbound body).
- Lowering temperature to 0. Muninn confirmed this doesn't help — output-bias issue, not sampling-jitter.
- Changing retry defaults (1000ms base → muninn's 500ms). Arawn's defaults are more conservative; not worth changing without evidence.

### Why this is a backlog bug not a feature

The `failed_generation` gap is an *active* diagnostic blindspot: when Groq fails, the operator gets no actionable info. That's a bug in the error surface, even if no symptom reproduces today.

## Status Updates

*To be added during implementation*
