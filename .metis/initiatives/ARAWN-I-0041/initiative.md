---
id: tool-parameter-schema-enforcement
level: initiative
title: "Tool Parameter Schema Enforcement — validate inputs against declared schemas at every boundary"
short_code: "ARAWN-I-0041"
created_at: 2026-03-26T17:17:38.027233+00:00
updated_at: 2026-03-26T18:18:13.591445+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: M
initiative_id: tool-parameter-schema-enforcement
---

# Tool Parameter Schema Enforcement — validate inputs against declared schemas at every boundary Initiative

## Context

Every tool declares a JSON schema via `fn parameters(&self) -> Value`, but nobody validates incoming params against that schema before execution. The LLM sends arbitrary JSON, the tool parses what it wants, and anything unexpected is silently ignored.

This caused a real security bug: the FsGate hardcoded `params.get("path")` for glob/grep, but those tools use `"directory"`. The check silently passed (returned None → skipped), the tool executed unrestricted, and the test "passed" because it used the same wrong param name. The tool's own schema declared `"directory"` but nothing enforced it.

The problem is systemic — it's not just the gate. Every boundary where params cross from one component to another is vulnerable to the same mismatch:

1. **LLM → Tool execution**: LLM sends params, tool receives them. No schema validation.
2. **Gate enforcement**: Gate hardcodes param names instead of reading the tool's schema.
3. **Secret resolution**: Replaces `${{secrets.*}}` in string params, but doesn't know which params are strings.
4. **Output sanitization**: Applies per-tool limits by hardcoded tool name, not by schema metadata.
5. **Test assertions**: Tests construct params manually with no compile-time check against the schema.

## Goals

- Every tool call validates incoming params against the tool's declared JSON schema before execution
- The gate reads path/directory param names from the tool's schema or a trait method, not hardcoded strings
- Tools declare which params are filesystem paths (for gate enforcement) and which are secrets (for resolution)
- Invalid params return a structured error to the LLM with the schema, not a silent pass
- Tests validate params against schemas — a test using the wrong param name fails at the validation step, not silently

## Non-Goals

- Changing the Tool trait interface beyond what's needed for schema enforcement
- JSON Schema validation library (the schemas are simple enough for hand-rolled validation)
- Changing how MCP tools work (they have their own schema mechanism)

## Design

### Two separate concerns

**Concern 1: Schema validation** — "did the LLM send the right params?"
Owned by the tool. Validates param names, types, required fields against the declared schema. Prevents silent mismatches.

**Concern 2: Security enforcement** — "is this tool allowed to access this path?"
Owned by the gate. Enforced externally. The tool cannot opt out. Prevents filesystem escapes.

These run as separate steps in the execution pipeline. A tool validates its own schema. The gate enforces security policy. Neither depends on the other being correct.

### Tool trait — add `validate()` method

```rust
trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> Value;
    
    /// Validate params against this tool's declared schema.
    /// Default impl checks required fields, types, rejects unknown params.
    /// Tools can override to add domain-specific validation (URL format, etc).
    fn validate(&self, params: &Value) -> Result<(), ToolResult> {
        validate_against_schema(params, &self.parameters())
    }

    /// Declare which params are filesystem paths that the gate should enforce.
    /// Default: none. Tools that access the filesystem override this.
    fn gated_params(&self) -> Vec<GatedParam> { vec![] }
    
    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<ToolResult>;
}

enum GatedParam {
    ReadPath(&'static str),      // param name containing a read path
    WritePath(&'static str),     // param name containing a write path
    WorkingDir(&'static str),    // param name for cwd (defaults to gate working_dir if absent)
}
```

The tool declares its gated params. The gate reads them. No hardcoding on either side.

- `FileReadTool::gated_params()` → `[ReadPath("path")]`
- `FileWriteTool::gated_params()` → `[WritePath("path")]`
- `GlobTool::gated_params()` → `[ReadPath("directory")]`
- `GrepTool::gated_params()` → `[ReadPath("directory")]`
- `ShellTool::gated_params()` → `[WorkingDir("cwd")]`
- `WebFetchTool::gated_params()` → `[]` (SSRF is in validate(), not gate)
- Others → `[]`

### Execution flow

```
ToolRegistry::execute_with_config(name, params, ctx):
    1. tool = self.get(name)?
    2. params = resolve_secret_handles(params, ctx)
    3. tool.validate(&params)?                     // schema check — tool concern
    4. params = gate.enforce(tool, params, ctx)?    // security check — gate concern
    5. result = tool.execute(params, ctx).await?
    6. result.sanitize(output_config)
```

### Gate enforcement (revised)

`gate.enforce()` replaces the hardcoded match. It:
1. Reads `tool.gated_params()`
2. If empty → tool is not gated, pass through
3. For each `GatedParam`:
   - `ReadPath(name)` → get param value, call `ctx.fs_gate.validate_read(path)`
   - `WritePath(name)` → get param value, call `ctx.fs_gate.validate_write(path)`
   - `WorkingDir(name)` → if param present validate it, if absent set to `gate.working_dir()`
4. Shell tools: additionally run command validator + sandbox execution
5. If no `ctx.fs_gate` and `gated_params()` is non-empty → deny by default

This is the same security enforcement as today, but driven by the tool's declaration instead of hardcoded tool names. The gate reads `gated_params()`, the tool can't lie about them (the gate validates the actual param values), and a new tool that forgets to declare `gated_params()` defaults to ungated (same as today's behavior for non-gated tools).

### Default schema validation

`validate_against_schema(params, schema)` is a shared function that:
- Checks required fields are present
- Checks field types match (string, number, bool, array, object)
- Rejects unknown fields (not in schema)
- Returns structured error to the LLM with the expected schema on failure

## Phases

### Phase 1: Schema validation — `validate()` on Tool trait
- Add `fn validate(&self, params: &Value) -> Result<(), ToolResult>` to Tool trait with default impl
- Implement `validate_against_schema()` — checks required fields, types, rejects unknown params
- Wire `tool.validate()` into `execute_with_config()` as step 3 (before gate, before execute)
- MockTool gets default validate impl
- Existing tests should still pass (they send correct params)

### Phase 2: Gate reads `gated_params()` instead of hardcoded tool names
- Add `fn gated_params(&self) -> Vec<GatedParam>` to Tool trait with default `vec![]`
- Implement for filesystem tools: FileReadTool, FileWriteTool, GlobTool, GrepTool, ShellTool
- Rewrite `gate.enforce()` to iterate `tool.gated_params()` instead of matching tool names
- Delete `is_gated_tool()`, `GATED_TOOLS`, `validate_tool_paths()` — the hardcoded dispatch
- Keep FsGate trait, PathValidator, WorkstreamFsGate, SandboxManager — gate calls these
- Keep `execute_shell_sandboxed()` — gate calls it when tool has `WorkingDir` gated param
- Security behavior identical to today, just driven by tool declarations

### Phase 3: Test infrastructure and audit
- All existing tool tests go through `validate()` first — wrong param names now fail loudly
- Add negative tests: missing required field, wrong type, unknown field → structured error returned to LLM
- Gate tests verify deny-by-default: tool with `gated_params()` but no `ctx.fs_gate` → denied
- Gate tests verify param name correctness: gate reads the right param because tool declared it
- Audit all 14 built-in tools: every test constructs params matching the declared schema