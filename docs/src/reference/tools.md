# Built-in Tools Reference

Complete reference for all 14 built-in tools provided by the `arawn-agent-tools` crate.

## Tool Categories

| Category | Tools | Gate Required |
|----------|-------|---------------|
| File System | `file_read`, `file_write`, `glob`, `grep` | Yes (FsGate) |
| Execution | `shell` | Yes (FsGate) |
| Web | `web_fetch`, `web_search` | No |
| Memory | `memory_search`, `think`, `note` | No |
| Orchestration | `delegate`, `explore` | No |
| Pipeline | `catalog`, `workflow` | No |

Gated tools require an active `FsGate` in the `ToolContext`. Without one, execution returns an error: `"requires a filesystem gate"`.

---

## File System Tools

### file_read

Read the contents of a file. Returns file content as text.

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `path` | string | Yes | — | Path to the file to read |

**Return format:** Plain text containing file contents.

**Security restrictions:**
- Path traversal (`..` components) is rejected as a defense-in-depth check.
- When `base_dir` is configured, all paths are canonicalized and verified to remain within the allowed directory.
- FsGate validates read access before execution; the path is rewritten to its canonical form.

**Configuration options:**

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `base_dir` | string | None | Restrict reads to a specific directory tree |

**Output limit:** 500 KB (`OutputConfig::for_file_read()`).

---

### file_write

Write content to a file. Can create new files, overwrite existing files, or append.

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `path` | string | Yes | — | Path to the file to write |
| `content` | string | Yes | — | Content to write |
| `append` | boolean | No | `false` | Append to file instead of overwriting |

**Return format:** Plain text confirmation message (e.g., `"Successfully written to /path/to/file"`).

**Security restrictions:**
- Path traversal (`..` components) is rejected.
- When `base_dir` is configured, the parent directory is canonicalized and checked against the allowed base.
- For non-existent parent directories with `base_dir`, lexical normalization prevents traversal bypasses.
- FsGate validates write access before execution.
- Uses atomic `OpenOptions` to enforce create/overwrite policies (no TOCTOU races).

**Configuration options:**

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `base_dir` | string | None | Restrict writes to a specific directory tree |
| `allow_create` | boolean | `true` | Allow creating new files |
| `allow_overwrite` | boolean | `true` | Allow overwriting existing files |

Parent directories are created automatically when needed.

---

### glob

Find files matching a glob pattern. Returns a list of matching file paths with metadata.

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `pattern` | string | Yes | — | Glob pattern (e.g., `**/*.rs`, `src/*.txt`) |
| `directory` | string | No | Current directory | Directory to search in |

**Return format:** JSON object:

```json
{
  "pattern": "**/*.rs",
  "directory": "/project",
  "count": 42,
  "truncated": false,
  "files": [
    { "path": "src/main.rs", "is_dir": false, "size": 1234 }
  ]
}
```

**Limits and behavior:**

| Constraint | Value |
|------------|-------|
| Maximum results | 1000 |
| Maximum traversal depth | 20 |
| Symlink following | Disabled |

**Walk depth optimization:** The tool calculates the minimum walk depth needed for a pattern. Patterns without `**` only traverse the required number of directory levels (e.g., `*.txt` walks depth 1, `src/*.rs` walks depth 2). Patterns containing `**` use the full `max_depth`.

---

### grep

Search file contents using regular expressions. Returns matching lines with file paths and line numbers.

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `pattern` | string | Yes | — | Regex pattern to search for |
| `directory` | string | No | Current directory | Directory to search in |
| `file_pattern` | string | No | None | Glob pattern to filter files (e.g., `*.rs`) |
| `case_insensitive` | boolean | No | `false` | Ignore case when matching |

**Return format:** JSON object:

```json
{
  "pattern": "fn main",
  "directory": "/project",
  "files_searched": 150,
  "match_count": 3,
  "truncated": false,
  "matches": [
    { "file": "src/main.rs", "line_number": 5, "line": "fn main() {" }
  ]
}
```

**Limits and behavior:**

| Constraint | Value |
|------------|-------|
| Maximum results | 500 |
| Maximum traversal depth | 20 |
| Maximum file size | 10 MB |
| Hidden files | Skipped |
| Binary files | Skipped |
| Symlink following | Disabled |

**Skipped binary extensions:** `exe`, `dll`, `so`, `dylib`, `bin`, `o`, `a`, `lib`, `png`, `jpg`, `jpeg`, `gif`, `bmp`, `ico`, `webp`, `mp3`, `mp4`, `avi`, `mov`, `mkv`, `wav`, `flac`, `zip`, `tar`, `gz`, `bz2`, `xz`, `7z`, `rar`, `pdf`, `doc`, `docx`, `xls`, `xlsx`, `ppt`, `pptx`, `wasm`, `pyc`, `class`.

---

## Execution Tools

### shell

Execute shell commands with safety controls. Supports both standard process execution and PTY mode.

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `command` | string | Yes | — | Shell command to execute |
| `timeout` | integer | No | 30 | Timeout in seconds |

**Return format:** Plain text containing stdout. If stderr is present, it is appended after a `--- stderr ---` separator. Non-zero exit codes return an error result with the exit code and output.

**Security restrictions:**

When an FsGate is present, the shell tool routes all commands through the OS-level sandbox (`sandbox_execute`), bypassing the tool's own `execute()` method entirely.

Before sandbox execution, the `CommandValidator` checks the command against blocked patterns (case-insensitive, whitespace-normalized):

| Blocked Pattern | Reason |
|-----------------|--------|
| `rm -rf /` | Filesystem destruction |
| `rm -rf /*` | Filesystem destruction |
| `:(){ :\|:& };:` | Fork bomb |
| `dd if=/dev` | Raw device access |
| `> /dev/sda` | Raw device write |
| `mkfs` | Filesystem formatting |
| `shutdown` | System shutdown |
| `reboot` | System reboot |
| `halt` | System halt |
| `poweroff` | System power off |

**Configuration options (ShellConfig):**

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `timeout` | Duration | 30s | Maximum execution time |
| `working_dir` | string | None | Working directory for commands |
| `allowed_commands` | Vec\<string\> | Empty (all allowed) | Whitelist of command prefixes |
| `blocked_commands` | Vec\<string\> | See table above | Blacklist of command prefixes |
| `max_output_size` | usize | 1 MB | Maximum captured output |
| `pty_size` | (u16, u16) | (24, 80) | Terminal dimensions for PTY mode |

**Output limit:** 100 KB (`OutputConfig::for_shell()`).

---

## Web Tools

### web_fetch

Fetch content from a URL. Supports all HTTP methods, custom headers, request bodies, and file downloads.

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `url` | string | Yes | — | URL to fetch (http/https only) |
| `method` | string | No | `"GET"` | HTTP method: GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS |
| `headers` | object | No | None | Custom request headers as key-value pairs |
| `body` | string | No | None | Request body (for POST, PUT, PATCH) |
| `timeout_secs` | integer | No | 30 | Request timeout in seconds (1-300) |
| `raw` | boolean | No | `false` | Return raw HTML instead of extracted text |
| `include_headers` | boolean | No | `false` | Include response headers in result |
| `download` | string | No | None | File path to save response body to disk |

**Return format:** JSON object containing status code, content type, title, description, and extracted text (or raw HTML). When `download` is specified, the response is streamed directly to disk and file metadata is returned instead.

**SSRF protection:** Before making any request, the tool resolves the URL's hostname and validates that none of the resolved IP addresses fall within restricted ranges:

| Range | Description |
|-------|-------------|
| `127.0.0.0/8` | Loopback |
| `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16` | Private networks |
| `169.254.0.0/16` | Link-local (includes cloud metadata) |
| `100.64.0.0/10` | CGNAT / Tailscale |
| `255.255.255.255` | Broadcast |
| `0.0.0.0` | Unspecified |
| `::1` | IPv6 loopback |
| `fc00::/7` | IPv6 unique local |
| `fe80::/10` | IPv6 link-local |

**Configuration (WebFetchConfig):**

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `timeout` | Duration | 30s | Request timeout |
| `max_size` | usize | 10 MB | Maximum response body size (in-memory) |
| `user_agent` | string | `Arawn/<version> (Research Agent)` | User-Agent header |
| `extract_text` | boolean | `true` | Extract readable text from HTML |
| `max_text_length` | usize | 50,000 chars | Maximum extracted text length |

**Output limit:** 200 KB (`OutputConfig::for_web_fetch()`).

---

### web_search

Search the web for information. Returns a list of results with titles, URLs, and snippets.

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `query` | string | Yes | — | Search query |

**Return format:** JSON object:

```json
{
  "query": "rust async trait",
  "count": 5,
  "results": [
    { "title": "...", "url": "...", "snippet": "..." }
  ]
}
```

**Search providers:** Configured at tool construction time. One provider is active per tool instance.

| Provider | API Key Required | Description |
|----------|-----------------|-------------|
| DuckDuckGo | No | Default provider, no API key needed |
| Brave | Yes | Brave Search API |
| Serper | Yes | Google Search via Serper API |
| Tavily | Yes | AI-optimized search API |

**Output limit:** 50 KB (`OutputConfig::for_search()`).

---

## Memory Tools

### memory_search

Query the agent's persistent memory store for relevant information.

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `query` | string | Yes | — | Search query |
| `type` | string | No | `"all"` | Memory type filter: `all`, `conversation`, `fact`, `preference`, `research` |
| `limit` | integer | No | 10 | Maximum results to return |
| `time_range` | string | No | `"all"` | Time filter: `all`, `today`, `week`, `month` |

**Return format:** JSON object:

```json
{
  "status": "ok",
  "query": "user preferences",
  "type": "all",
  "time_range": "all",
  "count": 3,
  "results": [
    {
      "id": "...",
      "content_type": "fact",
      "content": "...",
      "score": 0.85,
      "created_at": "2026-03-24T10:00:00Z",
      "session_id": "..."
    }
  ]
}
```

**Type filter mapping:**

| Filter Value | Content Types Searched |
|-------------|----------------------|
| `all` | All types (no filter) |
| `conversation` | UserMessage, AssistantMessage |
| `fact` | Fact |
| `preference` | Note |
| `research` | WebContent, FileContent |

Results also include matching notes (supplemented up to the limit). When the memory store is not connected, returns `{"status": "disconnected"}`.

---

### think

Record internal reasoning as a persistent thought. Thoughts are stored in memory with the current `session_id` and available for recall in future turns but not shown to the user.

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `thought` | string | Yes | — | Reasoning or observation to record |

**Return format:** Plain text: `"Thought recorded."`

**Behavior:** The thought is stored as a `ContentType::Thought` memory entry with the session ID from the tool context. Empty or whitespace-only thoughts are rejected.

---

### note

Create, update, retrieve, list, or delete session-scoped notes. Notes persist throughout the conversation in shared in-memory storage.

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `action` | string | Yes | — | Action: `create`, `update`, `get`, `list`, `delete` |
| `title` | string | Conditional | — | Note title (required for create, update, get, delete) |
| `content` | string | Conditional | — | Note content (required for create, update) |

**Return format by action:**

| Action | Success Return | Error Conditions |
|--------|---------------|-----------------|
| `create` | `"Created note '<title>'"` | Title already exists |
| `update` | `"Updated note '<title>'"` | Note not found |
| `get` | JSON with title, content, created_at, updated_at | Note not found |
| `list` | JSON with count and notes array (content previewed at 50 chars) | — |
| `delete` | `"Deleted note '<title>'"` | Note not found |

---

## Orchestration Tools

### delegate

Delegate a task to a specialized subagent. Subagents have constrained tool sets and custom system prompts.

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `agent` | string | Yes | — | Name of the subagent to delegate to |
| `task` | string | Yes | — | Task description for the subagent |
| `context` | string | No | None | Additional context from the current conversation |
| `background` | boolean | No | `false` | Run asynchronously and return immediately |
| `max_turns` | integer | No | None | Override maximum conversation turns |

**Return format:**
- **Blocking mode:** Text containing the subagent's result, prefixed with `## Result from '<agent>'`.
- **Background mode:** Text confirming delegation: `"Delegated to '<agent>' in background. You'll be notified when complete."`
- **Unknown agent:** Error listing available agent names.

---

### explore

Spawn an isolated RLM (Recursive Language Model) exploration sub-agent to research a query. The sub-agent has read-only tool access and uses iterative compaction to work beyond context limits.

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `query` | string | Yes | — | Research question or topic to explore |

**Return format:** Plain text summary of findings, followed by a metadata footer:

```
<summary text>

---
Exploration: 5 iterations, 12000 tokens (8000in/4000out), 2 compactions
```

**Available tools in exploration context:** `file_read`, `grep`, `glob`, `web_search` (read-only subset). Empty queries are rejected.

---

## Pipeline Tools

### catalog

Manage WASM runtimes in the pipeline runtime catalog. Requires the `pipeline` feature.

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `action` | string | Yes | — | Action: `list`, `compile`, `register`, `inspect`, `remove` |
| `name` | string | Conditional | — | Runtime name (required for compile, register, inspect, remove) |
| `source_path` | string | Conditional | — | Path to `.rs` source file (required for compile) |
| `wasm_path` | string | Conditional | — | Path to `.wasm` file (required for register) |
| `description` | string | No | None | Human-readable runtime description |

**Name validation:** Names must not be empty, contain path separators (`/`, `\`), contain `..`, start with `.`, or contain control characters.

**Return format by action:**

| Action | Return |
|--------|--------|
| `list` | JSON with runtimes array (name, description, path, category) and count |
| `compile` | Compile `.rs` source to WASM and register; returns path to compiled module |
| `register` | Register an existing `.wasm` file in the catalog |
| `inspect` | Runtime details (name, path, description, category) |
| `remove` | Confirmation of removal |

---

### workflow

Manage and execute workflows through the pipeline engine. Requires the `pipeline` feature.

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `action` | string | Yes | — | Action: `create`, `run`, `schedule`, `list`, `cancel`, `status` |
| `name` | string | Conditional | — | Workflow name (required for create, run, schedule, cancel, status) |
| `definition` | string | Conditional | — | Workflow definition in TOML format (required for create) |
| `context` | string | No | None | Context data passed to workflow execution |
| `cron` | string | No | None | Cron expression for scheduling |
| `timezone` | string | No | None | IANA timezone for cron schedule |

**Name validation:** Same rules as catalog tool names.

Workflow definitions are validated as TOML and checked for structural correctness before being written to the workflow directory and registered with the engine.

---

## Output Configuration

Tool output is sanitized before being returned to the LLM. The `OutputConfig` controls size limits, truncation, and content cleaning.

**Global defaults:**

| Setting | Default Value |
|---------|---------------|
| `max_size_bytes` | 102,400 (100 KB) |
| `strip_control_chars` | `true` (preserves `\n`, `\t`, `\r`) |
| `strip_null_bytes` | `true` |
| `validate_json` | `true` (max nesting depth: 50) |

**Per-tool output size overrides:**

| Tool | Max Output Size |
|------|----------------|
| `shell` | 100 KB |
| `file_read` | 500 KB |
| `web_fetch` | 200 KB |
| `grep`, `web_search` | 50 KB |

When output exceeds the size limit, it is truncated at a UTF-8-safe boundary and the message `[Output truncated - exceeded size limit]` is appended. Binary content (>1% null bytes in the first 8 KB) is rejected.

---

## Tool Result Types

All tools return one of the following result variants:

```rust
pub enum ToolResult {
    Text(String),
    Json(Value),
    Error(String),
    Binary { path: PathBuf, mime: String },
}
```

Error results from parameter validation or tool execution are returned as `ToolResult::Error`, not as Rust `Err` values. Only infrastructure failures (missing parameters for required fields) propagate as `Err`.
