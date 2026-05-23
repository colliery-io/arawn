---
id: refactor-monolithic-main-rs-and
level: task
title: "Refactor monolithic main.rs and send_message into composable functions"
short_code: "ARAWN-T-0155"
created_at: 2026-04-10T01:01:19.662996+00:00
updated_at: 2026-04-10T23:27:55.319706+00:00
parent: ARAWN-I-0024
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0024
---

# Refactor monolithic main.rs and send_message into composable functions

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0024]]

## Objective
Break down the ~650-line `main.rs` serve mode setup and ~220-line `LocalService::send_message` into composable helper functions. Add `#[instrument]` span annotations for tracing.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] `register_default_tools(registry, config, bg_manager)` extracted from main.rs
- [ ] `build_serve_context(config, store) -> ServeContext` extracted from main.rs
- [ ] `load_session_state(...)`, `build_session_context(...)`, `build_engine(...)` extracted from local_service.rs
- [ ] `#[instrument(skip_all, fields(session_id))]` on `send_message` and key engine methods
- [ ] No functionality changes — pure refactoring
- [ ] All tests pass

## Implementation Notes
- Files: `crates/arawn/src/main.rs`, `crates/arawn/src/local_service.rs`
- Easier after ARAWN-T-0153 (arawn-tool extraction makes boundaries clearer)

## Status Updates
- **COMPLETE**: All extractions done, all tests pass.

### main.rs extractions:
- `register_default_tools(registry, config, data_dir, bg_manager, plan_state)` — 20 tool registrations extracted from inline serve block
- `connect_mcp_servers(data_dir, plugin_result, registry) -> McpManager` — config + plugin MCP server connection
- `register_workflow_tools(registry, workflows_dir, shared_runner)` — 4 workflow tool registrations

### local_service.rs extractions:
- `load_session_state(session_id)` — metadata loading, workstream resolution (with `#[instrument]`)
- `build_session_context(session_id, workstream, ws_dir, workspace_dir, content)` — ToolContext + PromptContext construction with memory injection (with `#[instrument]`)
- `build_engine(prompt_context, event_tx)` — QueryEngine with compactor, skills, plugins, permissions (with `#[instrument]`)
- `#[instrument(skip_all, fields(%session_id))]` on `send_message`

### Notes:
- Skipped `build_serve_context` — the serve block initialization is heterogeneous (MCP, plugins, workflows, config watcher) and doesn't form a clean struct. The individual extractions (`register_default_tools`, `connect_mcp_servers`, `register_workflow_tools`) are more useful.
- No functionality changes — pure refactoring