---
id: wire-plugins-into-binary-startup
level: task
title: "Wire plugins into binary — startup scan, DataLayout V2, registration"
short_code: "ARAWN-T-0019"
created_at: 2026-04-01T01:16:47.990002+00:00
updated_at: 2026-04-01T03:00:13.132175+00:00
parent: ARAWN-I-0003
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: ARAWN-I-0003
---

# Wire plugins into binary — startup scan, DataLayout V2, registration

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[ARAWN-I-0003]]

## Objective

Wire plugin loading into the binary crate. On startup: scan `~/.arawn/plugins/tools/` for `.arawn_tool` archives, unpack/build/load via `PluginLoader`, register all plugin tools in `ToolRegistry` alongside built-in tools. Update `DataLayout` to V2 with `plugins/tools/` and `plugins/build/`.

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

- [ ] `DataLayout::v1()` updated to include `plugins/tools/` and `plugins/build/` directories
- [ ] On startup: calls `PluginLoader::load_tools(plugins/tools/, plugins/build/)`
- [ ] All returned plugin tools registered in `ToolRegistry`
- [ ] Plugin tools show up in tool definitions sent to LLM
- [ ] Built-in tools still work (not affected by plugin loading)
- [ ] No `.arawn_tool` files present → starts normally with only built-in tools
- [ ] Failed plugin build → warning logged, other plugins still load
- [ ] E2E test: drop example `.arawn_tool` into plugins/tools/, run arawn, verify tool is available

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
- Update `DataLayout::v1()` in arawn-storage to include `plugins/tools/` and `plugins/build/`
- `main.rs` changes: after opening Store, call `PluginLoader::load_tools()`, register results
- Plugin loading happens before the engine runs — tools are available for the first turn
- First run with no plugins should behave identically to current behavior
- Depends on: ARAWN-T-0016 (plugin loader), ARAWN-T-0017 (built-in tools), ARAWN-T-0018 (example plugins)

## Status Updates
- **2026-04-01**: Complete. DataLayout::v1() updated with plugins/tools/ and plugins/build/. main.rs calls PluginLoader::load_tools() after built-in tool registration. No plugins = starts normally with 6 built-in tools. Verified ~/.arawn/plugins/{tools,build}/ created. Layout tests updated (4 dirs now). 126 workspace tests, clippy clean.