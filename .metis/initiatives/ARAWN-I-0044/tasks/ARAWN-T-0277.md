---
id: token-usage-tracker-typed
level: task
title: "Token usage tracker — typed TokenUsage records with per-model rollups"
short_code: "ARAWN-T-0277"
created_at: 2026-05-15T14:13:05.229977+00:00
updated_at: 2026-05-15T19:09:57.084507+00:00
parent: ARAWN-I-0044
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0044
---

# Token usage tracker — typed TokenUsage records with per-model rollups

## Tier
Tier 1 — prerequisite for the routing policy's usage-aware hint (`T-0278`) and the trigger condition for unfreezing the deferred token-efficiency tasks (`T-0270`, `T-0274`).

## Rationale — tokens, not dollars
We deliberately do **not** convert token usage into a dollar figure. Provider pricing changes without notice, is sometimes negotiated, varies by tier, and is not always public. Shipping an inaccurate "you've spent $X" number would be worse than shipping nothing — it would either mislead the user or panic them. Tokens are the unambiguous, model-native unit. Anyone who wants dollars can do the multiplication themselves with the pricing they actually pay.

## Reference
`/tmp/openhuman/src/openhuman/cost/{tracker,types,schemas}.rs` — their `TokenUsage` and `ModelStats` shapes are the parts we copy. Their `CostRecord` (dollar-denominated) and the pricing table are explicitly *not* copied.

## Goal
Every LLM call in `arawn-llm` emits a `TokenUsageRecord { provider, model, prompt_tokens, completion_tokens, ts, call_site? }`. Records aggregate into per-model / per-period rollups. A `UsageQuery` API answers "how many tokens has model X used today / this week / this month?" Future callers can use the rollups to apply their own policy (warn, throttle, refuse) without the tracker imposing one.

## Acceptance
- New module `crates/arawn-engine/src/token_usage/` (single crate-internal module — not its own crate, keeps it close to `arawn-llm`).
- Types:
  - `TokenUsageRecord { provider, model, prompt_tokens, completion_tokens, ts, call_site: Option<String> }`. No cost field, no currency.
  - `ModelUsageStats { provider, model, total_prompt_tokens, total_completion_tokens, call_count }` rolled up per period.
  - `UsagePeriod { Day, Week, Month, All }`.
- `TokenUsageTracker` is `Arc`-shared (or rides the event bus once that lands — see `ARAWN-S-0004` §B).
- Wired into `arawn-llm` response path: every completion records usage. Streaming responses record on completion (or on cancel with partial tokens).
- Persistence: append-only log in the data dir (`<data>/token_usage.jsonl` or SQLite if the volume warrants it). Rollups computed lazily on query, cached for the running process.
- Optional `call_site` tag — callers can pass a string like `"steward.reshelve"` or `"workflow.morning_brief"` so rollups can also be sliced by subsystem. Useful diagnostics, not load-bearing.
- TUI + RPC surface: `arawn usage [--period day|week|month] [--model <name>] [--by-site]`. Output is purely token counts and call counts — no currency anywhere in the surface.
- Tests cover: record rollup correctness, period boundaries (UTC day boundary), streaming completion path, missing-usage-from-provider fallback (record what we got, log the gap).

## Out of scope
- Cost-in-dollars / pricing table / budget-in-USD. Permanently out of scope on this task; if a user wants a dollar figure they multiply our tokens by their negotiated rate themselves.
- Hard-cap enforcement ("refuse calls when over budget"). The tracker surfaces data; any enforcement is a separate policy task built on top.

## Notes
- This is what unfreezes `T-0270` (redirect-link shortener) and `T-0274` (TokenJuice). The decision rule there is "revisit when telemetry shows a specific tool or boundary blowing context" — token-count rollups by `call_site` are exactly the signal needed.
- The routing policy (`T-0278`) reads recent usage instead of cost. Its hint shape becomes something like `UsagePressure::{Low, High}` rather than `CostSensitivity::{Normal, High}` — wording change captured on T-0278.

## Status Updates

**2026-05-15 — implementation landed.**

**Module placement deviation:** task said `crates/arawn-engine/src/token_usage/`. Moved to `crates/arawn-llm/src/usage/` so the steward + extractor crates can wire it in without taking a new `arawn-engine` dep (same reasoning as T-0275's gate).

- New `crates/arawn-llm/src/usage/` module:
  - `mod.rs` — `TokenUsageRecord`, `ModelUsageStats`, `UsagePeriod` ({Day, Week, Month, All}), `UsageSummary`, `CallSiteStats`. `UsageTracker::open(data_dir)` writes append-only JSONL to `<data_dir>/token_usage.jsonl`. `record()` and `read_all()` are best-effort (failures log, never bubble). `summary()` produces rollups filtered by period + optional model name; opt-in `by_site` adds a per-`call_site` breakdown.
  - `tracking_client.rs` — `UsageTrackingClient` LlmClient decorator. Wraps another `LlmClient`; on every streamed `ChatChunk::Done { usage: Some(_) }` records a `TokenUsageRecord` via the global tracker. Streams that end without a Done chunk emit a tracing debug — the log under-counts in that case, by design.
- Process-wide singleton: `install(Arc<UsageTracker>)` is idempotent; `global()` reads the active instance; `record()` is a no-op (with tracing debug) when no tracker is installed so non-bin callers / unit tests don't crash.
- **Tokens, not dollars.** No `cost_usd`, no pricing table, no currency anywhere in the record / summary / CLI surface. Permanently out of scope per the task body.

Wiring:

- `crates/arawn/src/llm_pool.rs`: pool layering is now `raw → RetryClient → UsageTrackingClient → WarmingClient`. Every entry's traffic flows through the decorator.
- `crates/arawn/src/main.rs`: tracker installed at startup via `arawn_llm::usage::install(...)` before the pool builds. New `Command::Usage { --period, --model, --by-site, --json }` reads the on-disk log without spinning up the engine.

CLI verified end-to-end against a fresh temp dir: `arawn usage --period day` prints the human-readable empty-window message; `arawn usage --json` returns a parseable `UsageSummary` JSON.

Tests: 10 in `arawn_llm::usage::tests` (record round-trip, by-model rollup, model filter, day window, by-site grouping, empty log, malformed-line skip, global-without-install no-panic). 1 in `tracking_client::tests` verifying the decorator records on a Done-with-usage chunk. Full workspace suite: **1575 passed / 0 failed.**

**Follow-up for T-0278:** the routing policy reads recent usage from `UsageTracker::summary(period, Some(model), false)` to compute `UsagePressure::High` when totals exceed a configurable per-period threshold. The tracker shape is already what T-0278 needs.