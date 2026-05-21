---
id: t-f-tier-1-small-crate-cleanup
level: task
title: "T-F: Tier 1 — small-crate cleanup (steward, extractor, llm)"
short_code: "ARAWN-T-0383"
created_at: 2026-05-21T14:53:23.121729+00:00
updated_at: 2026-05-21T15:48:58.492303+00:00
parent: ARAWN-I-0053
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0053
---

# T-F: Tier 1 — small-crate cleanup (steward, extractor, llm)

## Parent Initiative

[[ARAWN-I-0053]]

## Objective

Mop-up task: delete `_ts` fake-use in arawn-steward, delete `push_classify`
dead test helper in arawn-extractor, and fix the pre-existing
`error_type` dead-field warning in arawn-llm's retry test fixture.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

### arawn-steward

- [ ] Delete `fn _ts() -> DateTime<Utc>` at `crates/arawn-steward/src/dust.rs:309-312` and its `#[allow(dead_code)]`.
- [ ] After deletion, verify `chrono::Utc` (or `DateTime<Utc>`) is still used elsewhere in `dust.rs`. If not, remove the chrono imports from this file.

### arawn-extractor

- [ ] Delete `fn push_classify(&self, v: Value)` at `crates/arawn-extractor/src/cot.rs:629-632` and its `#[allow(dead_code)]`. This is in a `#[cfg(test)]` module; verified zero test callers.

### arawn-llm

- [ ] Fix pre-existing dead-code warning at `crates/arawn-llm/src/retry.rs:96-98`: the `error_type: LlmError` field on `struct FailThenSucceed` is never read. Either use it in the test's behavior assertion (e.g., to vary which error type is returned) or remove the field.

### Validation

- [ ] `cargo check --workspace` clean.
- [ ] `cargo build --workspace --release` clean.
- [ ] `cargo test --workspace --no-run` clean.
- [ ] `angreal test unit` passes (specifically arawn-extractor and arawn-llm tests).
- [ ] `angreal test integration` passes.

## Implementation Notes

### Technical Approach

Three independent small deletions/fixes. Order doesn't matter.

### Dependencies

None internal to the initiative.

### Risk Considerations

Minimal. The `_ts` function is a `_`-prefixed no-op; deleting it is safe.
`push_classify` is in a test mod and confirmed unused. The `error_type`
field fix is small.

## Verification

Per the initiative's dead-code methodology:
- `angreal check workspace`
- `cargo build --workspace --release`
- `cargo test --workspace --no-run`
- `angreal test unit`
- `angreal test integration`

## Status Updates

### 2026-05-21 — landed

- **arawn-steward/dust.rs:** deleted `fn _ts() -> DateTime<Utc>` + its `#[allow(dead_code)]` at lines 307-312. `DateTime` was only used here, so dropped it from the chrono import (line 14: `use chrono::{DateTime, Duration, Utc}` → `use chrono::{Duration, Utc}`). `Duration` and `Utc` stay (used at lines 105/311/366/476).
- **arawn-extractor/cot.rs:** deleted `fn push_classify(&self, v: Value)` + its `#[allow(dead_code)]` at lines 629-632. The method was inside a `#[cfg(test)]` `KeyedMockLlm` fixture but had zero callers.
- **arawn-llm/retry.rs:** deleted the `error_type: LlmError` field from `struct FailThenSucceed` (line 98 in original). The field was set in 4 construction sites (lines 147, 170, 215, 251) but never read — the `stream()` impl always returned a hardcoded `ServerError`. Adding a Clone-based real usage would have required adding Clone to LlmError, which propagates through `reqwest::Error`/`serde_json::Error` (not worth it for a dead field). Removed all 4 construction lines via sed.

**Validation:**
- `cargo check --workspace`: ✅ clean.
- `cargo test --workspace --no-run`: ✅ clean.
- `cargo test -p arawn-steward -p arawn-extractor -p arawn-llm`: ✅ all targets pass (28 + 103 + 45 = 176 tests, 0 fail).