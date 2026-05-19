---
id: mcp-server-config-var-env-var
level: task
title: "MCP server config: ${VAR} env-var substitution"
short_code: "ARAWN-T-0352"
created_at: 2026-05-19T12:07:50.622305+00:00
updated_at: 2026-05-19T12:07:50.622305+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


exit_criteria_met: false
initiative_id: NULL
---

# MCP server config: ${VAR} env-var substitution

## Objective

Support `${VAR}` substitution in `[[mcp.servers]].env` map values so plugin manifests and `arawn.toml` MCP entries can reference credentials from the parent process env without baking them in.

## Impact

- **Severity:** P3 — feature gap. Users have to inline secrets in `arawn.toml` or in a plugin's `plugin.json`.
- **Affected users:** anyone using MCP servers that need API tokens (GitHub MCP, OpenAI MCP, etc.).
- **Current behavior:** `env = { GITHUB_TOKEN = "${GITHUB_TOKEN}" }` passes the literal 6-character string `${GITHUB_TOKEN}` to the child process. The child sees `GITHUB_TOKEN=${GITHUB_TOKEN}` and fails.

## Implementation notes

- `crates/arawn-mcp/src/manager.rs:277-282` is where the `env` map is consumed: `cmd.env(key, val)`. Add a pre-process step that walks `val` looking for `${NAME}` patterns and substitutes from the parent env.
- Edge cases:
  - Missing env var: leave the placeholder, or fail-fast and log? Recommend fail-fast at config-load with a clear error.
  - Literal `${`: support a backslash escape (`\${`) for the unusual case.
  - Recursive substitution: not needed; one level is sufficient.
- Apply the same substitution to plugin-declared MCP servers (`crates/arawn-engine/src/plugins/manifest.rs::McpServerDef`).
- Update `docs/src/reference/mcp.md` (and the example block) once landed — the triple-check (commit 7037763) removed the substitution claim; this task restores it.

## Acceptance criteria

- [ ] `${VAR}` in `[[mcp.servers]].env` values is substituted from `std::env::var` at server-spawn time.
- [ ] Missing env var fails fast at config load with an actionable error.
- [ ] Unit test covering substitution + missing-var error path.
- [ ] `docs/src/reference/mcp.md` updated to document the substitution.

Surfaced during ARAWN-I-0051 doc triple-check.

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

## Acceptance Criteria **[REQUIRED]**

- [ ] {Specific, testable requirement 1}
- [ ] {Specific, testable requirement 2}
- [ ] {Specific, testable requirement 3}

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

## Implementation Notes **[CONDITIONAL: Technical Task]**

{Keep for technical tasks, delete for non-technical. Technical details, approach, or important considerations}

### Technical Approach
{How this will be implemented}

### Dependencies
{Other tasks or systems this depends on}

### Risk Considerations
{Technical risks and mitigation strategies}

## Status Updates **[REQUIRED]**

*To be added during implementation*