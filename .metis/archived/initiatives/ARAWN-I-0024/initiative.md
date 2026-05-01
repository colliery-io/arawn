---
id: crate-extraction-and-service-layer
level: initiative
title: "Crate Extraction and Service Layer Completion"
short_code: "ARAWN-I-0024"
created_at: 2026-04-09T23:59:37.387469+00:00
updated_at: 2026-04-10T23:28:03.381139+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: L
initiative_id: crate-extraction-and-service-layer
---

# Crate Extraction and Service Layer Completion Initiative

## Context

Architecture review identified `arawn-engine` as a ~21K-line mega-crate housing 6+ subsystems (query engine, tools, permissions, hooks, plugins, skills, plan mode), forcing full recompilation on any change and creating an unwieldy public API. The `ArawnService` trait covers only 7 of 16 RPC methods. The TUI depends on engine despite being a WebSocket client. The legacy plugin system coexists with the new one without migration path.

**Review findings addressed:** R-15, R-16, R-17, R-18

## Goals & Non-Goals

**Goals:**
- Extract `arawn-tool` interface crate with Tool, ToolOutput, ToolRegistry, ToolContext (EVO-01, EVO-02, EVO-03)
- Complete ArawnService trait to cover all 16 RPC methods with typed responses (API-002, EVO-05)
- Refactor monolithic main.rs and send_message into composable functions (LEG-003, LEG-004)
- Deprecate legacy (WASM/fidius) plugin system behind feature flag (LEG-005, EVO-08)

**Non-Goals:**
- Splitting arawn-engine further beyond the tool crate extraction
- Rewriting the WebSocket handler (just completing the trait)
- Removing the legacy plugin system entirely (just deprecation)

## Detailed Design

### arawn-tool Extraction (R-15)
Create `crates/arawn-tool/` containing: `Tool` trait, `ToolOutput`, `ToolRegistry`, `ToolContext`, `EngineError`. Update `arawn-engine` and `arawn-mcp` to depend on `arawn-tool`. Move modal prompt types to `arawn-service`. Drop `arawn-engine` dependency from `arawn-tui`. Add `Tool::category() -> ToolCategory` to replace string-based `tool_category()`.

### Service Trait Completion (R-16)
Add 9 missing methods to `ArawnService`. Define typed response structs. Map `ServiceError` variants to distinct RPC error codes. Update WS handler to call through trait for all methods.

### Orchestration Refactoring (R-17)
Extract `register_default_tools()`, `build_serve_context()` from main.rs. Extract `load_session_state()`, `build_session_context()`, `build_engine()` from local_service.rs. Add `#[instrument]` span annotations.

### Legacy Plugin Deprecation (R-18)
Add `#[deprecated]` to legacy plugin modules. Feature-gate fidius dependency behind `legacy-plugins`. Log deprecation warning on legacy plugin load.

## Implementation Plan

1. arawn-tool extraction (Weeks) — independent, highest leverage
2. Service trait completion (Days) — independent
3. Orchestration refactoring (Days) — easier after arawn-tool extraction
4. Legacy plugin deprecation (Hours) — independent