# Tool Execution Pipeline

When the LLM decides to call a tool, a lot happens between "the model said to run `ls -la`" and "the output appears in the conversation." This page traces the complete journey of a tool call through the execution pipeline, explaining why each step exists and how the pieces fit together.

## The Big Picture

Tool execution is the most security-sensitive part of Arawn. The LLM can request arbitrary tool calls -- file reads, file writes, shell commands, web requests -- and the system must decide what to allow, how to execute it safely, and what to return. Getting this wrong means the LLM could delete files, exfiltrate data, or compromise the host system.

The pipeline is designed so that security checks compose. Each step narrows what is allowed, and no step can widen what a previous step restricted.

## Step 1: The Agent Loop Calls the Tool

The agent loop in `arawn-agent` receives an LLM response containing tool use blocks. Each block specifies a tool name and JSON parameters. The loop calls `ToolRegistry.execute_with_config()` for each tool call.

If the tool name does not exist in the registry, the agent returns a correction message to the LLM: "Tool 'X' does not exist. Available tools are: [list]." This handles the common case where the LLM hallucinates a tool name. Rather than a cryptic error, the model gets actionable feedback and can retry with a valid tool.

## Step 2: Secret Resolution

Before execution, the parameters are scanned for `${{secrets.*}}` handles. If a `SecretResolver` is present in the tool context, matching handles are replaced with their actual values.

```json
// Before resolution
{"header": "Bearer ${{secrets.api_key}}"}

// After resolution (internal only - never logged)
{"header": "Bearer sk-ant-real-key-value"}
```

This happens before any other processing because tools need the actual secret values to function. The critical property is that the **original parameters** (with handles) are what the caller retains for logging and conversation history. The resolved values exist only in the execution path and never reach persistent storage.

If no resolver is present, or if the parameters contain no handles, this step is a no-op. Unknown handles (e.g., `${{secrets.nonexistent}}`) are left as-is rather than causing an error. This graceful degradation means a typo in a secret name does not crash the tool -- it just passes the literal handle string, which will likely cause an authentication failure that the agent can diagnose.

## Step 3: Gated Tool Check

Five tools interact with the filesystem and require special security treatment: `file_read`, `file_write`, `glob`, `grep`, and `shell`. These are "gated tools" -- they cannot execute unless a `FsGate` is present in the tool context.

### No Gate: Default Deny

If no `FsGate` is configured (the tool context's `fs_gate` field is `None`), gated tools are immediately denied:

```
Tool 'file_read' requires a filesystem gate but none is configured.
This tool can only be used within a workstream context.
```

This is a deliberate default-deny posture. A gated tool running without a gate would have unrestricted filesystem access -- exactly the scenario the security model exists to prevent. The error message is explicit about why and how to fix it (use a workstream), so the LLM can adjust its approach.

### Gate Present: Route by Tool Type

When a gate is present, the execution path splits based on the tool type:

- **Shell tools** go through the CommandValidator and then the OS sandbox (Steps 4a-4b below).
- **File and search tools** go through path validation (Step 5 below).

This split exists because shell commands and file operations have fundamentally different security properties. A file read is a single path access that can be validated statically. A shell command is arbitrary code that can do anything, requiring process-level isolation.

## Step 4: Shell Tool Execution

Shell execution has two sub-steps: command validation and sandbox execution.

### Step 4a: CommandValidator

The `CommandValidator` normalizes the command string and checks it against a regex blocklist. The normalization defeats common bypass techniques:

1. **Lowercase**: Catches `RM -RF /`
2. **Quote removal**: Catches `"rm" "-rf" "/"`
3. **Backslash removal**: Catches `r\m -rf /`
4. **Whitespace collapse**: Catches `rm   -rf   /`
5. **Basename extraction**: Catches `/usr/bin/rm -rf /`

If the command matches a blocked pattern, execution stops immediately with a human-readable error: "Command not allowed: rm -rf /". The error is returned as a `ToolResult::error`, which the LLM sees and can respond to ("I'll use a safer approach to clean up the build directory").

The blocklist includes destructive filesystem operations, fork bombs, device access, system control commands, sandbox escape attempts, kernel module manipulation, and process tracing. Importantly, it does **not** block commands that might fail harmlessly in the sandbox. `cat /etc/shadow` is not blocked because the sandbox will deny access -- the CommandValidator catches only commands that should never be attempted regardless of sandbox state.

### Step 4b: OS Sandbox Execution

Commands that pass validation are executed through `FsGate.sandbox_execute()`. The `WorkstreamFsGate` implementation delegates to the `SandboxManager`, which:

1. Builds a `SandboxRuntimeConfig` from the allowed paths and network configuration.
2. Initializes the sandbox runtime with this configuration.
3. Wraps the command with platform-specific sandbox restrictions.
4. Executes the wrapped command with a timeout.
5. Returns stdout, stderr, exit code, and success status.

On macOS, this means running the command under `sandbox-exec` with a profile that denies all writes except to the workstream directories. On Linux, it means running under `bubblewrap` with namespace isolation and restricted mount points.

The sandbox is the primary security boundary. Even if the CommandValidator misses something, the sandbox constrains what the command can do at the OS level.

### Why Both Validator and Sandbox?

If the sandbox is the real security boundary, why have the CommandValidator at all?

**User experience.** When the LLM tries `rm -rf /`, the CommandValidator returns "Command not allowed: rm -rf /" instantly. Without the validator, the command would enter the sandbox, the sandbox would deny the write to `/`, and the error would be "Operation not permitted" -- which is correct but unhelpful. The validator catches obvious problems with clear, actionable messages.

**Defense-in-depth.** If the sandbox has a vulnerability, the validator is still there. If the validator has a gap, the sandbox is still there. Neither layer alone is sufficient; together they provide robust protection.

## Step 5: File and Search Tool Execution

For `file_read`, `file_write`, `glob`, and `grep`, the execution path validates file paths before calling the tool implementation.

### Path Validation

`ToolRegistry.validate_tool_paths()` extracts the `path` parameter from the tool's JSON parameters and passes it through the gate:

- **Read tools** (file_read, glob, grep): Call `gate.validate_read(path)`. The gate returns a canonicalized path on success or an access denied error.
- **Write tools** (file_write): Call `gate.validate_write(path)`. Same pattern.

Path canonicalization is important. If the LLM requests `/workstream/work/../../../etc/passwd`, canonicalization resolves the `..` components to `/etc/passwd`, which the gate then correctly denies. Without canonicalization, the gate might see the path as starting with the allowed `/workstream/work/` prefix and allow it.

The validated (canonicalized) path replaces the original in the parameters before the tool executes. This means the tool implementation receives a clean, absolute path and does not need to handle path traversal attacks itself.

### If Validation Fails

If the gate denies access, a `ToolResult::error` is returned immediately:

```
Access denied: path is outside the workstream sandbox
```

The tool implementation never runs. The file is never read. The LLM sees the error and can adjust: "I can't read that file because it's outside the workspace. Let me try a different approach."

## Step 6: Tool Execution

Once all validation passes, the actual tool implementation runs:

```rust
let result = tool.execute(params, ctx).await?;
```

Each tool implementation is a struct that implements the `Tool` trait with an `execute` method. The implementations live in `arawn-agent-tools`:

- `file_read`: Reads file contents, respects line limits.
- `file_write`: Writes content to a file.
- `glob`: Pattern-based file discovery.
- `grep`: Content search with regex support.
- `shell`: Direct execution (only reached when no gate is present for non-gated contexts).
- `web_search`: Search the web via API.
- `web_fetch`: Fetch a URL's content.
- `memory_search`: Query the agent's persistent memory store.
- `note`: Create, update, retrieve, list, or delete session-scoped notes.
- `think`: Scratchpad for agent reasoning (stored in memory but not shown to user).
- `delegate`: Spawn a sub-agent for a specialized task.
- `explore`: Spawn an isolated RLM exploration sub-agent for research.
- `catalog`: Manage WASM runtimes in the pipeline runtime catalog.
- `workflow`: Manage and execute workflows through the pipeline engine.

Tools receive a `ToolContext` that includes the session ID, working directory, config, optional memory store, optional filesystem gate, and optional secret resolver. This context-passing design means tools do not need global state -- everything they need is in the context.

## Step 7: Output Sanitization

After the tool returns a `ToolResult`, the output is sanitized:

```rust
Ok(result.sanitize(output_config))
```

Sanitization removes null bytes from the output text. Null bytes can cause problems in JSON serialization, terminal display, and LLM tokenization. They should never appear in tool output, but defensive sanitization catches any that slip through.

Per-tool output limits can also truncate excessively long output. A `grep` search that matches 10,000 lines would consume too much of the LLM's context window. Truncation with a clear marker ("... [truncated, showing first N lines]") preserves the most useful information while staying within bounds.

The `execute_raw` method skips sanitization for internal use cases where the raw output is needed (e.g., for programmatic processing rather than LLM consumption).

## Pre/Post Tool Hooks

The plugin system can register hooks that fire before and after tool execution:

### PreToolUse Hooks

Before a tool executes, the `HookDispatcher` checks for matching PreToolUse hooks. Each hook specifies:
- A **tool_match** glob pattern (e.g., `"Bash"`, `"file_*"`)
- An optional **match_pattern** regex for parameter matching

If the tool name matches the glob and the parameters match the regex, the hook runs as a shell subprocess. The subprocess receives JSON context on stdin and has a 10-second timeout.

If the subprocess exits with a non-zero code, the tool execution is **blocked**. The hook's output becomes the error message returned to the LLM. This allows plugins to implement custom security policies, audit logging, or approval workflows.

### PostToolUse Hooks

After a tool executes, PostToolUse hooks fire with the same matching logic. These hooks are informational -- they cannot block execution (it already happened). They are used for logging, metrics, or triggering side effects.

### Why Shell Subprocesses?

Hooks run as shell subprocesses rather than in-process callbacks for two reasons:

1. **Language independence.** Hooks can be written in any language -- bash, Python, Node, Go. The only contract is stdin/stdout and exit codes. This makes the hook system accessible to anyone, not just Rust developers.

2. **Isolation.** A buggy hook cannot crash the Arawn process. It runs in its own process with a timeout. If it hangs, it gets killed after 10 seconds. If it crashes, the parent process is unaffected.

The trade-off is latency -- spawning a subprocess for every tool call adds overhead. The matching system (glob + regex) ensures hooks only fire when relevant, and the 10-second timeout bounds the worst case.

## Error Handling

The pipeline handles errors at every level:

| Error | Handling |
|-------|----------|
| Tool name does not exist | Return correction message listing available tools |
| Secret handle not found | Leave handle as-is (graceful degradation) |
| FsGate not configured | Return clear error explaining workstream requirement |
| Path validation denied | Return "Access denied" with reason |
| Command blocked | Return "Command not allowed" with blocked pattern name |
| Sandbox execution error | Return "Sandbox error" with details |
| Tool execution failure | Return error message to LLM for self-correction |
| Hook blocks execution | Return hook's output as error |
| Timeout | Cancel and return timeout message |

In every case, errors are returned as `ToolResult::error` messages visible to the LLM. The agent loop does not crash on tool failures -- it feeds the error back to the model, which can try a different approach. This self-correction capability is essential for an agentic system where tool calls are generated by inference rather than hardcoded.

## Design Trade-offs

**Gate enforcement in execute_raw.** Even the `execute_raw` method (which skips sanitization) enforces the filesystem gate. This prevents internal callers from accidentally bypassing security. The downside is that internal tools that need unrestricted access must construct their own execution path outside the registry.

**CommandValidator as blocklist, not allowlist.** The validator blocks known-dangerous patterns rather than allowing only known-safe commands. An allowlist would be more secure but impractical -- the set of useful shell commands is unbounded. The blocklist catches the most dangerous patterns while relying on the sandbox for comprehensive protection.

**Synchronous path validation, async sandbox execution.** Path validation for file tools is synchronous (no I/O needed beyond `canonicalize`). Shell sandbox execution is async because it spawns a subprocess and waits for it. This split keeps file tool execution fast while allowing shell commands their natural asynchronous lifecycle.
