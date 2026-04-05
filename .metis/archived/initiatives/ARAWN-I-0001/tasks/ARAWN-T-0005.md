---
id: arawn-engine-starter-tools
level: task
title: "arawn-engine — Starter tools: ThinkTool, ShellTool, FileReadTool"
short_code: "ARAWN-T-0005"
created_at: 2026-03-31T17:37:40.085391+00:00
updated_at: 2026-03-31T19:03:52.918421+00:00
parent: ARAWN-I-0001
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0001
---

# arawn-engine — Starter tools: ThinkTool, ShellTool, FileReadTool

## Parent Initiative
[[ARAWN-I-0001]]

## Objective
Implement three starter tools that exercise different aspects of the tool system: a no-op reasoning tool, a shell execution tool, and a file read tool. These prove the Tool trait works and give the query engine real tools to call.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] `ThinkTool` — accepts `thought: String`, returns the thought as output. No side effects. Used as LLM reasoning scratchpad.
- [ ] `ShellTool` — accepts `command: String`, `timeout_ms: Option<u64>`. Executes via `tokio::process::Command` with CWD set to `ToolContext::working_dir`. Returns stdout + stderr. Respects timeout.
- [ ] `FileReadTool` — accepts `path: String`, `offset: Option<usize>`, `limit: Option<usize>`. Reads file relative to `ToolContext::working_dir`. Returns file contents. Rejects paths that escape workstream root (path traversal protection).
- [ ] Each tool returns valid JSON Schema from `parameters_schema()`
- [ ] Each tool registers successfully in `ToolRegistry`
- [ ] Unit tests per tool: ThinkTool roundtrip, ShellTool runs `echo`, FileReadTool reads a temp file
- [ ] FileReadTool test: path traversal attempt (`../../etc/passwd`) returns error

## Implementation Notes
- `tools/` module in `crates/arawn-engine/src/` with `think.rs`, `shell.rs`, `file_read.rs`
- ShellTool: use `tokio::process::Command`, capture stdout/stderr, enforce timeout with `tokio::time::timeout`
- FileReadTool: canonicalize path, check it starts with workstream root before reading
- Keep tools simple — no streaming, no fancy output. Just prove the trait works.
- Depends on: ARAWN-T-0004 (Tool trait + registry)

## Status Updates
- **2026-03-31**: All 3 tools implemented. 15 new tests (29 total in engine crate). ThinkTool: roundtrip + empty. ShellTool: echo, nonzero exit, timeout, missing param. FileReadTool: read, offset+limit, nonexistent, path traversal rejection, missing param, schema validation. Plus path normalization helper with unit test.