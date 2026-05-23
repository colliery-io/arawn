---
id: plugin-adapter-loader-unpack-build
level: task
title: "Plugin adapter + loader — unpack, build, load .arawn_tool archives"
short_code: "ARAWN-T-0016"
created_at: 2026-04-01T01:16:44.096225+00:00
updated_at: 2026-04-01T01:35:50.979063+00:00
parent: ARAWN-I-0003
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0003
---

# Plugin adapter + loader — unpack, build, load .arawn_tool archives

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0003]]

## Objective

Build the plugin loader that handles the full lifecycle: scan `~/.arawn/plugins/tools/` for `.arawn_tool` archives, unpack via `fidius_host::unpack_fid`, build via `fidius_host::build_package`, load the compiled dylib via `PluginHost`, and wrap in a `PluginToolAdapter` that implements the engine's `Tool` trait.

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [ ] Bug - Production issue that needs fixing
- [ ] Feature - New functionality or enhancement  
- [ ] Tech Debt - Code improvement or refactoring
- [ ] Chore - Maintenance or setup work

### Priority
- [ ] P0 - Critical (blocks users/revenue)
- [ ] P1 - High (important for user experience)
- [ ] P2 - Medium (nice to have)
- [ ] P3 - Low (when time permits)

### Impact Assessment **[CONDITIONAL: Bug]**
- **Affected Users**: {Number/percentage of users affected}
- **Reproduction Steps**: 
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected vs Actual**: {What should happen vs what happens}

### Business Justification **[CONDITIONAL: Feature]**
- **User Value**: {Why users need this}
- **Business Value**: {Impact on metrics/revenue}
- **Effort Estimate**: {Rough size - S/M/L/XL}

### Technical Debt Impact **[CONDITIONAL: Tech Debt]**
- **Current Problems**: {What's difficult/slow/buggy now}
- **Benefits of Fixing**: {What improves after refactoring}
- **Risk Assessment**: {Risks of not addressing this}

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `PluginToolAdapter` struct wraps a fides `PluginHandle` and implements `Tool` trait
- [ ] Adapter caches `name`, `description`, `parameters_schema` on construction (call fides methods once)
- [ ] Adapter `execute` serializes `ToolContext` + params to JSON strings, calls fides `execute`, deserializes result to `ToolOutput`
- [ ] `PluginLoader` struct with `load_tools(tools_dir, build_dir) → Vec<Box<dyn Tool>>`
- [ ] Scans `tools_dir` for `*.arawn_tool` files
- [ ] For each archive: `unpack_fid` → `build_package` → `PluginHost::load` → `PluginToolAdapter`
- [ ] Build caching: skip rebuild if archive hasn't changed (compare file mtime or digest)
- [ ] Handles build failures gracefully — log warning, skip plugin, continue with others
- [ ] Handles load failures gracefully — same pattern
- [ ] Unit test: PluginToolAdapter wraps a mock handle correctly
- [ ] Integration test: full lifecycle with a real .arawn_tool archive (can use example from T-0018)

## Test Cases **[CONDITIONAL: Testing Task]**

{Delete unless this is a testing task}

### Test Case 1: {Test Case Name}
- **Test ID**: TC-001
- **Preconditions**: {What must be true before testing}
- **Steps**: 
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

### Test Case 2: {Test Case Name}
- **Test ID**: TC-002
- **Preconditions**: {What must be true before testing}
- **Steps**: 
  1. {Step 1}
  2. {Step 2}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

## Documentation Sections **[CONDITIONAL: Documentation Task]**

{Delete unless this is a documentation task}

### User Guide Content
- **Feature Description**: {What this feature does and why it's useful}
- **Prerequisites**: {What users need before using this feature}
- **Step-by-Step Instructions**:
  1. {Step 1 with screenshots/examples}
  2. {Step 2 with screenshots/examples}
  3. {Step 3 with screenshots/examples}

### Troubleshooting Guide
- **Common Issue 1**: {Problem description and solution}
- **Common Issue 2**: {Problem description and solution}
- **Error Messages**: {List of error messages and what they mean}

### API Documentation **[CONDITIONAL: API Documentation]**
- **Endpoint**: {API endpoint description}
- **Parameters**: {Required and optional parameters}
- **Example Request**: {Code example}
- **Example Response**: {Expected response format}

## Implementation Notes
- `plugin_adapter.rs` + `plugin_loader.rs` in `crates/arawn-engine/src/`
- arawn-engine gains deps on `fidius-host = "0.0.4"` and `arawn-tool-plugin` (for interface hash + method indices)
- The adapter bridges fides's sync `call_method` with engine's async `Tool::execute` — use `tokio::task::spawn_blocking` to avoid blocking the async runtime
- `PluginLoader` uses:
  - `fidius_host::unpack_fid(archive, build_dir)` → unpacked source dir
  - `fidius_host::build_package(dir, !cfg!(debug_assertions))` → dylib path
  - `PluginHost::builder().search_path(dylib_dir).build()?.load(name)?` → LoadedPlugin
  - `PluginHandle::from_loaded(loaded)` → handle for calling
- Interface hash from `arawn-tool-plugin` used in PluginHost builder for validation
- Depends on: ARAWN-T-0015 (interface crate)

## Status Updates
- **2026-04-01**: Complete. PluginToolAdapter wraps PluginHandle, caches metadata on construction, uses block_in_place for sync FFI. PluginLoader scans .arawn_tool files, unpacks via fidius_host::package::unpack_fid, builds, discovers + loads all plugins. Graceful error handling. Integration test deferred to T-0018. 110 workspace tests, clippy clean.