---
id: test-harness-mockllmclient
level: task
title: "Test harness — MockLlmClient, TestHarness builder, functional tests"
short_code: "ARAWN-T-0007"
created_at: 2026-03-31T17:37:42.328239+00:00
updated_at: 2026-03-31T19:09:26.047977+00:00
parent: ARAWN-I-0001
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0001
---

# Test harness — MockLlmClient, TestHarness builder, functional tests

## Parent Initiative
[[ARAWN-I-0001]]

## Objective
Build the evaluation framework: a `MockLlmClient` for deterministic testing and a `TestHarness` builder for fixture-style functional tests. Then write functional tests that exercise the full engine loop end-to-end with mock LLM + real tools.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] `MockLlmClient` implements `LlmClient` — returns pre-scripted `ChatChunk` sequences
- [ ] Scripting API: `MockLlmClient::new(vec![Response::tool_use(...), Response::text(...)])` — each entry is one LLM turn
- [ ] `MockLlmClient` panics (or errors) if the engine makes more calls than scripted responses
- [ ] `TestHarness` builder: `.with_workstream_file(path, content)`, `.with_tools([...])`, `.with_script(vec![...])`, `.build()`
- [ ] `TestHarness::run(user_input) → HarnessResult` — runs the full engine loop
- [ ] `HarnessResult` exposes: `final_text()`, `tool_calls() → Vec<(name, params)>`, `session_messages()`
- [ ] Functional test: text-only response (no tools called)
- [ ] Functional test: single tool call (LLM calls FileReadTool, gets result, responds)
- [ ] Functional test: multi-step tool chain (LLM calls ThinkTool, then ShellTool, then responds)
- [ ] Functional test: tool-not-found (LLM calls nonexistent tool, engine returns error, LLM recovers)
- [ ] Functional test: max iteration guard triggers

## Implementation Notes
- `MockLlmClient` lives in `crates/arawn-llm/src/mock.rs` (behind `#[cfg(test)]` or a `test-support` feature)
- `TestHarness` lives in `crates/arawn-engine/src/testing.rs` (behind same gate)
- Functional tests in `crates/arawn-engine/src/` as inline test modules
- The harness creates a `tempdir`, populates files, builds workstream + session + registry + mock LLM, runs `QueryEngine::run`
- Depends on: ARAWN-T-0006 (query engine), ARAWN-T-0005 (starter tools), ARAWN-T-0003 (LLM types)

## Status Updates
- **2026-03-31**: Complete. MockLlmClient in arawn-llm/mock.rs with MockResponse enum (text/tool_call), call counting, exhaustion panic. TestHarness builder in arawn-engine/testing.rs with file population, tool registration, script injection, max_iterations config. HarnessResult with final_text(), tool_calls(), session_messages(). 6 functional tests + 4 mock unit tests + 1 bonus (real filesystem read). 64 total workspace tests, all passing, clippy clean.