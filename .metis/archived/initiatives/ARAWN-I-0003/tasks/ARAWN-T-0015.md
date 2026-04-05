---
id: arawn-tool-interface-crate-fides
level: task
title: "arawn-tool-interface crate — fides plugin interface for tools"
short_code: "ARAWN-T-0015"
created_at: 2026-04-01T01:16:28.429310+00:00
updated_at: 2026-04-01T01:30:13.784901+00:00
parent: ARAWN-I-0003
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0003
---

# arawn-tool-interface crate — fides plugin interface for tools

## Parent Initiative
[[ARAWN-I-0003]]

## Objective
Create the `arawn-tool-plugin` crate — the shared fides interface that defines the `ArawnTool` trait. This is the contract between Arawn and any tool plugin. Plugin crates depend on this; the host loads plugins that implement it. Also includes a `fidius.toml` with `extension = "arawn_tool"`.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] `crates/arawn-tool-plugin/` added to workspace
- [ ] Dependencies: `fidius = "0.0.4"`, `serde`, `serde_json`
- [ ] `#[plugin_interface(version = 1, buffer = PluginAllocated)]` trait `ArawnTool`
- [ ] Trait methods: `name() → String`, `description() → String`, `parameters_schema() → String`, `execute(context_json: String, params_json: String) → Result<String, String>`
- [ ] `fidius.toml` at crate root with `extension = "arawn_tool"`
- [ ] Generated constants exported: interface hash, method indices, capability bits
- [ ] Crate compiles and interface hash is stable (test)
- [ ] Workspace compiles, all existing tests pass

## Implementation Notes
- `crates/arawn-tool-plugin/src/lib.rs` — the `#[plugin_interface]` trait + any shared types
- JSON strings across FFI boundary: `serde_json::Value` doesn't work with bincode, so we serialize context/params to JSON strings on both sides of the boundary. The adapter (T-0016) handles conversion.
- The `execute` return type is `Result<String, String>` — the Ok string is JSON-serialized `ToolOutput`, the Err string is an error message. This keeps the FFI boundary simple.
- This crate is what external plugin authors depend on — keep it minimal.
- Depends on: nothing (new standalone crate)

## Status Updates
- **2026-04-01**: Complete. arawn-tool-plugin crate with `#[plugin_interface(version = 1, buffer = PluginAllocated)]` ArawnTool trait. Methods: name(), description(), parameters_schema(), execute(ToolExecuteInput) → ToolExecuteOutput. Used struct args instead of multiple strings for cleaner FFI. ToolExecuteOutput with success/error helpers. fidius.toml with extension = "arawn_tool". fidius 0.0.4 from crates.io. 110 workspace tests passing, clippy clean.