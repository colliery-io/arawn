---
id: llm-resource-gate-process-wide
level: task
title: "LLM resource gate — process-wide concurrency cap for local-model work"
short_code: "ARAWN-T-0275"
created_at: 2026-05-15T14:12:54.580759+00:00
updated_at: 2026-05-15T14:12:54.580759+00:00
parent: ARAWN-I-0044
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


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
