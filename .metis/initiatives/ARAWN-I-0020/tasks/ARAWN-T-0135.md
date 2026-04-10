---
id: engine-loop-tests-parallel-tool
level: task
title: "Engine loop tests: parallel tool calls, mixed text+tool, streaming edge cases"
short_code: "ARAWN-T-0135"
created_at: 2026-04-09T16:57:05.848802+00:00
updated_at: 2026-04-09T17:18:03.407281+00:00
parent: ARAWN-I-0020
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0020
---

# Engine loop tests: parallel tool calls, mixed text+tool, streaming edge cases

## Parent Initiative

[[ARAWN-I-0020]]

## Objective

Add integration tests covering engine loop behaviors that have zero coverage: parallel tool calls in a single LLM turn, mixed text+tool responses, and streaming edge cases (no Done chunk, empty stream, interleaved text+tool deltas). All testable now with `MockResponse::Raw`.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] **Parallel tool calls**: Test with 2+ `ToolUseStart` chunks before `Done` — verify both tools execute, results appear in correct order, read-only tools run concurrently
- [ ] **Mixed text + tool call**: Test with `TextDelta` followed by `ToolUseStart` in same response — verify assistant message has both text and tool_uses
- [ ] **Stream without Done chunk**: Test `Raw` with `ToolUseStart` + `ToolUseInputDelta` but no `Done` — verify flush path assembles the tool call correctly
- [ ] **Empty stream (Done only)**: Test `Raw([Done])` — verify engine returns empty string, no crash
- [ ] **Empty text deltas**: Test `Raw([TextDelta(""), TextDelta("hello"), Done])` — verify correct text assembly
- [ ] **Interleaved text after tool start**: Test `TextDelta` appearing after `ToolUseStart` + `ToolUseInputDelta` — verify both captured
- [ ] All new tests pass, existing tests unaffected

## Implementation Notes

### Files to Modify
- `crates/arawn-engine/src/testing.rs` — inline tests

### Mock Script Examples

Parallel tool calls:
```rust
MockResponse::raw(vec![
    ChatChunk::ToolUseStart { id: "c1".into(), name: "file_read".into() },
    ChatChunk::ToolUseInputDelta { json: r#"{"path":"a.txt"}"#.into() },
    ChatChunk::ToolUseStart { id: "c2".into(), name: "file_read".into() },
    ChatChunk::ToolUseInputDelta { json: r#"{"path":"b.txt"}"#.into() },
    ChatChunk::Done { usage: None },
])
```

Mixed text + tool:
```rust
MockResponse::raw(vec![
    ChatChunk::TextDelta { text: "Let me check.".into() },
    ChatChunk::ToolUseStart { id: "c1".into(), name: "think".into() },
    ChatChunk::ToolUseInputDelta { json: r#"{"thought":"planning"}"#.into() },
    ChatChunk::Done { usage: None },
])
```

### Dependencies
None — fully testable with current mock infrastructure.

## Status Updates

- Added 6 new tests in `testing.rs`:
  - `harness_parallel_tool_calls_in_single_turn` — 2 file_read calls in one response, verifies both execute in order with correct content
  - `harness_mixed_text_and_tool_call_in_same_turn` — TextDelta + ToolUseStart, verifies assistant message has both text and tool_uses
  - `harness_stream_without_done_chunk` — no Done, verifies flush path assembles tool call correctly
  - `harness_empty_stream_done_only` — Done only, verifies empty string returned
  - `harness_empty_text_deltas_assembled_correctly` — empty + non-empty deltas, verifies "hello world"
  - `harness_text_after_tool_start_both_captured` — text after ToolUseStart, verifies both captured
- All 6 pass. 2 pre-existing shell sandbox failures unrelated to changes.