---
id: llm-resource-gate-process-wide
level: task
title: "LLM resource gate — process-wide concurrency cap for local-model work"
short_code: "ARAWN-T-0275"
created_at: 2026-05-15T14:12:54.580759+00:00
updated_at: 2026-05-15T18:14:48.845799+00:00
parent: ARAWN-I-0044
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0044
---

# LLM resource gate — process-wide concurrency cap for local-model work

## Tier
Tier 1 — independent today; usage-aware policy can be added later once `T-0277` lands.

## Context
Arawn has several subsystems that independently decide to invoke an LLM: `arawn-workflow` (cron DAG jobs), `arawn-steward` (KB subroutines), `arawn-feeds` (ingestion), the agent loop (TUI input), and future triage / ceremony work. None of them coordinate today. When the configured LLM is **local Ollama**, Ollama is effectively serial — concurrent requests stack memory and the laptop has crashed twice in practice (see `feedback_local_llm_load.md`). When the configured LLM is **cloud**, concurrent calls are fine; the bottleneck is bandwidth and the provider's own throttling, not local RAM.

This task is a *resource-protection layer*, not a scheduler. The existing schedulers keep deciding *when* to run; the gate decides *whether enough local capacity exists right now*.

## Reference
`/tmp/openhuman/src/openhuman/scheduler_gate/{gate,policy,signals}.rs`. Process-wide `Arc<Semaphore>` (their `LLM_SLOTS=1` for laptop RAM); 30s sampler refreshes signals; workers call `wait_for_capacity`. We adopt the shape but rename to make the purpose clearer.

## Goal
A process-wide gate that every would-be LLM caller passes through. For local-bound work it enforces a small slot cap (default 1, matching Ollama's serial reality). For cloud-bound work it issues a non-counting permit so cloud calls aren't artificially serialised. The gate samples live host signals (free RAM, on-battery) and can degrade to `Pause(reason)` when the host can't safely run more local inference.

## Acceptance
- New module `crates/arawn-engine/src/llm_gate/{mod,signals,policy}.rs` (the `llm_gate` name picked deliberately to avoid the misleading "scheduler" framing).
- Two permit types:
  - `LocalPermit` — counts against the slot budget. Default cap = 1 (Ollama is serial).
  - `RemotePermit` — cheap to acquire, does not count against the slot budget.
- API:
  - `gate::acquire_local().await -> Result<LocalPermit, Paused>` — blocks while `Pause(_)` is in effect, else returns a permit.
  - `gate::acquire_remote() -> RemotePermit` — non-blocking.
  - `gate::current_policy() -> Policy` — cheap read for telemetry.
- Signals sampler refreshes every 30s: free RAM, on-battery, current concurrent local-slot usage. Configurable interval.
- Policy: `Capacity::Available | Pause(reason)`. Pause when free RAM drops below configurable threshold; resume when it recovers. On-battery is a soft signal that *can* tighten the cap (configurable, off by default).
- Routing into the gate: the routing policy task (`T-0278`) is the natural call site — when it picks `RoutingTarget::Local`, the caller acquires a `LocalPermit`; when it picks `Remote`, a `RemotePermit`. Until `T-0278` ships, wire the four current LLM-consuming paths directly: workflow runner, steward subroutines, feed dispatcher, agent loop.
- Tests cover: "two concurrent local acquires serialise behind a 1-slot cap", "pause-when-RAM-low blocks new local acquires until RAM recovers", "remote acquires are unaffected by local pause", "fresh tokio runtime gets a fresh slot" (mirrors openhuman's test-state pattern so parallel cargo test runs don't deadlock each other).

## Notes
- The 1-slot local default is non-negotiable for the laptop-RAM contract. Anything that wants to relax it must change the config; the code default stays at 1.
- If/when the routing policy (`T-0278`) lands, the call sites move there — but the gate itself stays in `arawn-engine` because workflow/steward/feeds also need it independently of routing.
- Cost-aware policy (back off when daily budget is near cap) is a follow-up once `T-0277` exists; out of scope here.

## Status Updates

**2026-05-15 — implementation landed.**

**Deviation from acceptance spec:** task said module location was `crates/arawn-engine/src/llm_gate/`. Moved to `crates/arawn-llm/src/gate/` because two of the four callers (steward, extractor) don't depend on `arawn-engine` — putting the gate in arawn-engine would have forced new upstream deps on lower-level crates. `arawn-llm` is the LLM-domain crate that *every* LLM consumer already depends on.

- New `crates/arawn-llm/src/gate/` module:
  - `mod.rs` — public API: `acquire_local() -> Future<Result<LocalPermit, AcquireError>>`, `try_acquire_local() -> Result<LocalPermit, AcquireError>`, `acquire_remote() -> RemotePermit`, `set_policy(Policy)`, `set_signals(Signals)`, `current_policy()`, `current_signals()`.
  - `policy.rs` — `Policy { local_slots: usize, free_ram_pause_bytes: Option<u64>, on_battery_extra_pause_bytes: Option<u64> }`, `Signals { free_ram_bytes, on_battery }`, `Capacity { Available | Pause(reason) }`, `decide(&policy, &signals)`.
  - `signals.rs` — `sample()` stub returning `Signals::default()`. The real RAM/battery probe lands as a follow-up; the gate API + tests are shaped to slot it in without touching call sites.
- Defaults: `Policy { local_slots: 1, free_ram_pause_bytes: None, on_battery_extra_pause_bytes: None }`. The 1-slot cap is the laptop-RAM safety contract from `feedback_local_llm_load.md`.
- `LocalPermit` is RAII (drop returns the slot). `RemotePermit` is a marker type, never blocks.
- State: single process-wide `GateState` behind `OnceLock`. `set_policy` mints a fresh semaphore at the new size; outstanding permits drain into the old one. Tests use a `TEST_LOCK` mutex + `reset_for_test()` to serialise gate-mutating tests.

**Call sites wired (4 direct + 2 incidental):**

| Site | Local/Remote | Justification |
|---|---|---|
| `arawn-engine::query_engine::stream_response` | Local | Main agent loop. Will switch to Remote once T-0278 routes. |
| `arawn-engine::compactor::call_llm` | Local | Compaction runs on the same backend. |
| `arawn-engine::tools::web_fetch::summarize_with_llm` | Local | Web summary path. |
| `arawn-engine::tools::workstream::propose_llm_call` | Local | WorkstreamProposeOntology tool. |
| `arawn-steward::llm_text::complete_text` | Local | Steward subroutines. |
| `arawn-extractor::llm_text::complete_text` | Local | Memory extraction (latent — not yet live but wired now). |

Every site acquires a `LocalPermit`. Once T-0278 lands the agent loop and any cloud-bound caller switch to `RemotePermit`; the gate API is already shaped for that.

**Tests:** 11 in `arawn_llm::gate` (4 policy decide-table tests + 7 acquire/serialise/pause tests). Full workspace suite remains green.

**Documented follow-ups:**
- Real RAM/battery probe in `signals::sample()` (needs `sysinfo` dep). Tracked in this task body as a note; can land standalone since the API is stable.
- Switch agent loop to `RemotePermit` when configured LLM is cloud — part of T-0278 routing policy.