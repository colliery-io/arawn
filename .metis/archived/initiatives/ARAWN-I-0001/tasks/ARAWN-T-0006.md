---
id: arawn-engine-queryengine-agentic
level: task
title: "arawn-engine — QueryEngine agentic loop (stream, detect tool_use, execute, loop)"
short_code: "ARAWN-T-0006"
created_at: 2026-03-31T17:37:41.402658+00:00
updated_at: 2026-03-31T19:06:44.788727+00:00
parent: ARAWN-I-0001
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0001
---

# arawn-engine — QueryEngine agentic loop (stream, detect tool_use, execute, loop)

## Parent Initiative
[[ARAWN-I-0001]]

## Objective
Implement the `QueryEngine` — the heart of Arawn. This is the agentic loop that sends prompts to the LLM, streams responses, detects tool_use blocks, executes tools via the registry, feeds results back, and loops until the LLM produces a final text response.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] `QueryEngine::new(llm: Arc<dyn LlmClient>, registry: Arc<ToolRegistry>)` constructor
- [ ] `QueryEngine::run(&self, session: &mut Session, ctx: &ToolContext) → Result<String>` — main entry point
- [ ] Builds `ChatRequest` from session history + tool definitions from registry (queried fresh each turn)
- [ ] Streams response via `LlmClient::stream`, collects chunks into complete response
- [ ] Detects tool_use in response — extracts tool name + params
- [ ] Executes each tool via `ToolRegistry::get(name)` + `Tool::execute(ctx, params)`
- [ ] Appends assistant message (with tool_uses) and tool_result messages to session
- [ ] Loops: sends updated history back to LLM for next turn
- [ ] Terminates when LLM response contains only text (no tool_use)
- [ ] Returns the final text response
- [ ] Handles tool-not-found gracefully (returns error tool_result to LLM, doesn't panic)
- [ ] Max iteration guard (configurable, default 20) to prevent infinite loops
- [ ] Unit tests with mock LLM: text-only response, single tool call, multi-turn tool chain

## Implementation Notes
- `query_engine.rs` in `crates/arawn-engine/src/`
- The loop mirrors Claude Code's QueryEngine: stream → detect → execute → feed back → stream
- Chunk reassembly: accumulate `TextDelta` into text, `ToolUseStart` + `ToolUseInputDelta` into complete tool calls
- The engine converts between `arawn-llm` types (`ChatChunk`) and `arawn-core` types (`Message`) — this is the seam between the two crates
- System prompt is part of `ChatRequest` — for now, a simple hardcoded prompt. System prompt building is a future concern.
- Depends on: ARAWN-T-0003 (LLM client), ARAWN-T-0004 (tool trait/registry), ARAWN-T-0002 (session/message types)

## Status Updates
- **2026-03-31**: Complete. QueryEngine implements the full agentic loop with streaming chunk reassembly, tool execution, session history management, and max iteration guard. 5 unit tests with inline MockLlm: text-only, single tool call, tool-not-found, max iterations exceeded, multi-turn tool chain. 34 total tests in engine crate, all passing, clippy clean.