---
id: t-b-tool-turn-lifecycle-fires
level: task
title: "T-B: Tool & turn lifecycle fires — PreToolUse, PostToolUse, PostToolUseFailure, Stop, StopFailure, UserPromptSubmit"
short_code: "ARAWN-T-0413"
created_at: 2026-05-23T03:31:01.000000+00:00
updated_at: 2026-05-23T03:31:01.000000+00:00
parent: ARAWN-I-0056
blocked_by: [ARAWN-T-0412]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: ARAWN-I-0056
---

# T-B: Tool & turn lifecycle fires

## Parent Initiative

[[ARAWN-I-0056]]

## Objective

Add `.fire_hook(...)` call sites in `QueryEngine` for the six tool-and-turn lifecycle events. Implement block semantics for the two events where it's meaningful.

## Acceptance Criteria

- [ ] `PreToolUse` fires in `QueryEngine::run` (or wherever a tool call is dispatched) BEFORE the tool's `execute()` is called.
  - Block aborts the tool call: agent sees a `ToolError::HookBlocked(reason)` (or equivalent) so the model can read why and adapt next turn.
  - Hook input carries: `tool_name`, `tool_args` (serialized JSON), `session_id`.
- [ ] `PostToolUse` fires in `QueryEngine::run` AFTER successful tool execution.
  - Non-blocking; hook stdout can be surfaced as a log line.
  - Hook input carries: `tool_name`, `tool_args`, `tool_result`, `session_id`.
- [ ] `PostToolUseFailure` fires AFTER a tool returns an error.
  - Non-blocking; same payload as PostToolUse but with the error string.
- [ ] `UserPromptSubmit` fires when a new user message is added to the session, BEFORE the model call.
  - Block aborts the turn; user sees the hook's block reason as the assistant response.
  - Hook input carries: `prompt_text`, `session_id`.
- [ ] `Stop` fires when the model produces a final response (no more tool calls).
  - Non-blocking for V1; the response has already been delivered.
  - Hook input carries: `final_text`, `session_id`.
- [ ] `StopFailure` fires when the model stream errors mid-turn.
  - Non-blocking.
  - Hook input carries: `error_string`, `session_id`.
- [ ] Each fire site is guarded by `if let Some(runner) = &self.hook_runner` — runs only when a runner is attached.
- [ ] Unit tests using the existing test harness's `with_hook_runner` to verify each fire site:
  - [ ] Configured PreToolUse hook + matching tool → hook fires, can block.
  - [ ] Configured PostToolUse hook → fires after success.
  - [ ] Configured PostToolUseFailure hook → fires after error.
  - [ ] Configured UserPromptSubmit hook → fires before LLM, can block turn.
  - [ ] Configured Stop hook → fires on final response.
  - [ ] Configured StopFailure hook → fires on stream error.
- [ ] `cargo test --workspace --lib` green.

## Implementation Notes

- The existing `HookInput` enum already has constructors for each event type — use them rather than introducing new constructors.
- Block result type: `AggregatedHookResult` exposes `.blocked` (bool) and `.block_reason` (Option<String>). Match Claude Code's semantics — block means "stop the workflow."
- For `PreToolUse` block: the cleanest surface is a new `ToolError::HookBlocked(String)` variant in arawn-tool. The engine's tool-dispatch loop maps this to a tool-result message visible to the agent.
- For `UserPromptSubmit` block: short-circuit the turn before the LLM is even called. The blocked user message becomes a synthetic assistant message with the hook's reason.
- The existing `arawn-tests/tests/hooks.rs` UAT scenarios validate the runner against canned events — confirm they still pass after the engine starts firing for real (they should — the harness fires events manually, decoupled from engine flow).

## Status Updates

*To be added during implementation*
