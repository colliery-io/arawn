---
id: tool-wall-clock-timeout-120s
level: task
title: "Tool wall-clock timeout — 120s default, agent-overridable per call"
short_code: "ARAWN-T-0269"
created_at: 2026-05-15T14:11:56.748379+00:00
updated_at: 2026-05-15T16:36:08.793967+00:00
parent: ARAWN-I-0044
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0044
---

# Tool wall-clock timeout — 120s default, agent-overridable per call

## Tier
Tier 1 — small win, low cost.

## Reference
`/tmp/openhuman/src/openhuman/tool_timeout/mod.rs` — global `OnceLock<Duration>` initialised from `OPENHUMAN_TOOL_TIMEOUT_SECS` (1–3600, default 120). We extend their model: the default is the same, but we let the agent override per-call without a ceiling. The agent has more context than a static config and we trust it to size the budget for the work it is about to do.

## Goal
Every tool execution in `arawn-engine` is wrapped in a wall-clock timeout enforced by `tokio::time::timeout`. The agent can pass a `timeout_secs` argument on any tool call to override; absent that, the default applies. There is no hard ceiling — the default exists to bound *unintended* hangs, not to second-guess the agent.

## Acceptance
- New `crates/arawn-engine/src/tool_timeout.rs` exposing:
  - `default_timeout() -> Duration` reading `ARAWN_TOOL_TIMEOUT_SECS` env var or `[engine] tool_timeout_secs` in `arawn.toml`, falling back to 120s.
  - `resolve(call_override: Option<u64>) -> Duration` — returns the call override if present, else the default.
- Every tool schema gains an optional `timeout_secs: u64` argument. The harness strips it before dispatching to the tool body and feeds it to `resolve(...)`.
- Tool dispatch wraps every tool future in `tokio::time::timeout(resolve(call_override), ...)`. On timeout, return a `ToolResult::Err` with a clear message naming the tool, the budget that fired, and whether the budget was the default or an override. Emit a hook event so the timeout is visible.
- Env var override + config field both honoured for the default; env wins.
- Agent system prompt gains a one-liner: *"Tools accept `timeout_secs`. Default 120s. Pass a larger value when you expect a long-running command (builds, large fetches); pass a smaller value to fail fast on operations that should be quick."*
- Tests cover: default resolution precedence (env > config > 120s), agent override honoured at any positive value, override of 0 rejected, regression test that a hanging `web_fetch` against a bad host fires the timeout under the default, and a second regression test that a 10s override fires before the 120s default would.

## Out of scope
Hard ceiling. We deliberately do not cap the override — the agent owns this. If misbehaviour shows up in practice, follow up with a soft warning at large values, not a cap.

## Notes
This is a strict timeout — the tool future is cancelled when it fires. Tools that need long-running work and *cannot* tolerate cancellation should already be using the `agent` or `workflow` paths, not the inline tool surface.

## Status Updates

**2026-05-15 — implementation landed (pending workspace test).**

- New module `crates/arawn-engine/src/tool_timeout.rs` exposes
  `DEFAULT_TIMEOUT_SECS`, `TIMEOUT_PARAM`, `TIMEOUT_ENV_VAR`,
  `default_timeout(config)`, `resolve(override, config)`,
  `extract_override(args)`. 14 unit tests pass.
- `crates/arawn-tool/src/registry.rs::tool_definitions()` now injects an
  optional `timeout_secs` integer property into every tool's parameters
  schema. Schemas with `additionalProperties: false` are relaxed since
  the field is added to `properties` first. 5 unit tests pass.
- `QueryEngineConfig` gained `tool_timeout_secs: Option<u64>`. The
  dispatch site in `execute_tool` now (1) strips the override from args
  before invoking the tool, (2) wraps `tool.execute()` in
  `tokio::time::timeout(...)`, (3) on elapsed returns an error result
  citing the budget and source (default vs override) and emits a
  `PostToolUseFailure` hook with `error: "timeout after Xs (source)"`.
- Agent system prompt gained the one-liner about `timeout_secs`.
- `crates/arawn/src/config.rs`: `[engine] tool_timeout_secs` added as
  `Option<u64>`. Plumbed through `main.rs::build_engine_config` and
  `local_service.rs`. Sub-agent path in `tools/agent.rs` leaves it at
  `None` for now (falls through to env / 120s) with a TODO to thread
  parent context if needed later.
- Integration tests in `query_engine.rs`:
  - `tool_completes_when_default_budget_is_large` — 5s budget vs 50ms
    sleep, completes.
  - `slow_tool_times_out_under_short_default` — 1s budget vs 2s sleep,
    tool result is an error citing timeout.
  - `agent_override_fires_before_default_would` — 60s default but
    agent passes `timeout_secs: 1` on a 3s sleep; elapsed < 2s and
    error mentions "override".
  - `invalid_override_surfaces_as_tool_error` — `timeout_secs: 0`
    rejected with a clear message.
- `cargo check workspace` clean; `arawn-engine` lib tests green (14 +
  9 in the two affected modules).

Next: run full workspace test suite, then commit + transition.

**2026-05-15 — completed.**

- Full workspace test run: 583 passed, 1 failure on the `snapshot_full_build` insta snapshot (expected — the system-prompt addition changed the snapshot). Snapshot accepted; re-ran and green.
- `angreal check all` (fmt + clippy + cargo check): clean. Pre-existing lints unchanged; no new lints introduced.
- Final targeted run of 18 timeout-related tests (14 in `tool_timeout`, 4 in `query_engine`): all pass.
- Transitioned to `completed`.