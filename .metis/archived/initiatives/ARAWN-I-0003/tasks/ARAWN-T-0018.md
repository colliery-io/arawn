---
id: example-plugin-tools-webfetchtool
level: task
title: "Example plugin tools — WebFetchTool + WebSearchTool as .arawn_tool packages"
short_code: "ARAWN-T-0018"
created_at: 2026-04-01T01:16:46.349477+00:00
updated_at: 2026-04-01T02:50:05.245251+00:00
parent: ARAWN-I-0003
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0003
---

# Example plugin tools — WebFetchTool + WebSearchTool as .arawn_tool packages

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0003]]

## Objective

Create two example tool plugins as standalone crates that compile to `.arawn_tool` archives: WebFetchTool and WebSearchTool. These prove the full plugin pipeline works and serve as templates for future plugin authors.

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

- [ ] `plugins/arawn-plugin-web-fetch/` — standalone crate with `crate-type = ["cdylib"]`
- [ ] Implements `ArawnTool` via `#[plugin_impl]` for WebFetchTool
- [ ] WebFetchTool: fetches URL, strips HTML, respects max_bytes, 30s timeout
- [ ] `plugins/arawn-plugin-web-search/` — standalone crate
- [ ] Implements `ArawnTool` via `#[plugin_impl]` for WebSearchTool
- [ ] WebSearchTool: DuckDuckGo HTML search, returns title+URL+snippet, no API key
- [ ] Each has `package.toml` with `extension = "arawn_tool"` and valid metadata
- [ ] Each has `fidius::fidius_plugin_registry!()` call
- [ ] Both compile via `fidius package build`
- [ ] Both pack to `.arawn_tool` via `fidius package pack`
- [ ] Integration test: unpack + build + load + call execute via PluginHandle

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
- Plugin crates live in `plugins/` at repo root (NOT in `crates/` — they're standalone, not workspace members)
- Each depends on `arawn-tool-plugin` (for the interface) and `fidius = "0.0.4"`
- WebFetchTool: `reqwest` (blocking client, since fides shims are sync) + basic HTML tag stripping
- WebSearchTool: `reqwest` to DuckDuckGo `html.duckduckgo.com/html/?q=...`, parse result HTML
- `package.toml` metadata schema should include `category`, `description` at minimum
- Pack via `fidius package pack <dir> --output <name>.arawn_tool`
- These archives get committed to the repo as examples and can be copied to `~/.arawn/plugins/tools/` for testing
- Depends on: ARAWN-T-0015 (interface crate)

## Status Updates
- **2026-04-01**: Complete. Two plugin crates in plugins/ (excluded from workspace). WebFetchTool: reqwest blocking + HTML tag stripping + max_bytes truncation. WebSearchTool: DuckDuckGo HTML search with result parsing. Both use white-label pattern: `#[plugin_impl(ArawnTool, crate = "arawn_tool_plugin::fidius")]` with `arawn_tool_plugin::fidius::fidius_plugin_registry!()`. package.toml with extension = "arawn_tool". Both compile as cdylibs. fidius 0.0.5 with multi-arg execute(context_json, params_json). Pack/integration test deferred to T-0019 wiring.