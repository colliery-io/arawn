---
id: add-end-to-end-agent-tests
level: task
title: "Add end-to-end agent tests"
short_code: "ARAWN-T-0292"
created_at: 2026-03-08T20:21:14.234061+00:00
updated_at: 2026-03-08T23:44:47.005565+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Add end-to-end agent tests

## Objective

Agent tests currently mock everything — no test verifies the full flow of tool dispatch → execution → response assembly. Add end-to-end tests that exercise the agent loop with real (but lightweight) tool implementations.

## Backlog Item Details

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P1 - High (important for user experience)

### Technical Debt Impact
- **Current Problems**: 87 agent tests exist but all use mocks. Integration bugs between tool dispatch and response assembly are invisible.
- **Benefits of Fixing**: Catches integration bugs in the agent loop, validates tool results flow back correctly.
- **Risk Assessment**: Medium — mocked tests cover unit logic but miss wiring issues.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] At least one test exercises: user message → tool selection → tool execution → response with tool output
- [ ] Test covers multi-turn conversation (tool call followed by follow-up)
- [ ] Test covers tool error handling (tool returns error, agent handles gracefully)
- [ ] Tests use lightweight mock LLM backend (not real API calls)
- [ ] `cargo test -p arawn-agent` passes
- [ ] `angreal check clippy` clean

## Implementation Notes

### Technical Approach
- Create a test-only LLM backend that returns deterministic tool-call responses
- Wire it into the real Agent with real tool registry
- Register simple test tools (echo, fail, etc.)
- Verify the full round-trip through the agent loop

### Files
- `crates/arawn-agent/src/agent.rs` or `crates/arawn-agent/tests/`

## Status Updates

### Session 1 — Complete
- Added 7 end-to-end agent tests in `crates/arawn-agent/src/agent.rs` (e2e_tests submodule)
- Tests verify the FULL agent loop: user message → LLM → tool dispatch → tool execution → result fed back to LLM → final response
- **test_tool_output_flows_back_to_llm** — uses `Arc<MockBackend>` + `with_shared_backend` to inspect that tool output appears in the follow-up LLM request messages
- **test_tool_arguments_pass_through** — verifies tool receives exact args from LLM via `MockTool.calls()`
- **test_multi_turn_with_tool_then_followup** — tool in turn 1, text follow-up in turn 2, verifies full history in 3rd LLM request
- **test_session_records_tool_state** — verifies session Turn records tool_calls, tool_results, assistant_response correctly
- **test_tool_error_flows_back_to_llm** — verifies error tool results are sent back to LLM with `is_error: true`
- **test_multiple_sequential_tool_calls** — two tools called across iterations in one turn
- **test_usage_accumulates_across_iterations** — verifies token counts sum correctly
- All `cargo test -p arawn-agent` pass, clippy clean, fmt clean