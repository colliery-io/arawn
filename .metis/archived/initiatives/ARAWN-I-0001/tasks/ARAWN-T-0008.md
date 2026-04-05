---
id: integration-wire-binary-crate-run
level: task
title: "Integration — Wire binary crate, run headless conversation end-to-end"
short_code: "ARAWN-T-0008"
created_at: 2026-03-31T17:37:43.714535+00:00
updated_at: 2026-03-31T19:39:36.867732+00:00
parent: ARAWN-I-0001
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0001
---

# Integration — Wire binary crate, run headless conversation end-to-end

## Parent Initiative
[[ARAWN-I-0001]]

## Objective
Wire the binary crate to assemble all components and run a headless conversation against Groq. This is the proof that all crates integrate correctly — create a workstream, start a session, register tools, run the engine loop, get a real response from a real LLM.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] `main.rs` creates a scratch `Workstream` (temp dir)
- [ ] Creates a `Session` bound to that workstream
- [ ] Creates `GroqClient` with API key from env
- [ ] Creates `ToolRegistry`, registers ThinkTool + ShellTool + FileReadTool
- [ ] Creates `QueryEngine` with the client and registry
- [ ] Adds a user message to the session (hardcoded or from CLI arg)
- [ ] Runs `QueryEngine::run` and prints the result to stdout
- [ ] If the LLM calls tools, the tool results are visible in debug output
- [ ] `angreal build workspace` produces a working binary
- [ ] Running the binary with `GROQ_API_KEY` set produces a coherent response
- [ ] Integration test (gated `#[ignore]`) that runs the full pipeline against Groq

## Implementation Notes
- `crates/arawn/src/main.rs` — minimal, just wiring
- Use `clap` or just `std::env::args` for the initial prompt input — no fancy CLI yet
- Print tool calls and results to stderr for debugging, final response to stdout
- This is the "it works" milestone — the foundation is complete when this runs
- Depends on: all other tasks (T-0001 through T-0007)

## Status Updates
- **2026-03-31**: Binary crate wired. main.rs assembles all components: scratch workstream (CWD), session, GroqClient from env, ToolRegistry with 3 tools, QueryEngine with configurable model (GROQ_MODEL env or default llama-3.3-70b-versatile). CLI arg for prompt, tracing to stderr, response to stdout. Binary builds and shows usage. Live Groq test requires GROQ_API_KEY — deferred to manual verification. 64 total workspace tests, all passing, clippy clean.