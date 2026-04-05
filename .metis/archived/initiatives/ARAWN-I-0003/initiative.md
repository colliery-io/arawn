---
id: expanded-tool-suite-filewrite-grep
level: initiative
title: "Expanded Tool Suite — FileWrite, Grep, WebFetch, WebSearch"
short_code: "ARAWN-I-0003"
created_at: 2026-03-31T22:34:01.528428+00:00
updated_at: 2026-04-02T12:35:41.936506+00:00
parent: ARAWN-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: S
initiative_id: expanded-tool-suite-filewrite-grep
---

# Expanded Tool Suite + Fides Plugin Infrastructure

## Context

Arawn has 3 starter tools (Think, Shell, FileRead) that proved the Tool trait works. To be a genuinely useful agent, it needs tools for writing files, searching content, and accessing the web.

More importantly, we need a plugin architecture so tools can be developed, compiled, and loaded independently — without rebuilding the whole binary. **Fides** (fidius) is our plugin framework: it transforms Rust traits into dynamically loadable, type-safe plugins via procedural macros and C-compatible dylibs.

This initiative does two things:
1. Integrate fides so the Tool system supports dynamically loaded plugins
2. Add the first batch of new tools (some built-in, some as plugin examples)

### Reference
- ARAWN-I-0001: established Tool trait, ToolRegistry, ToolContext, and 3 starter tools
- Claude Code analysis: 42+ tools across file I/O, search, execution, web categories
- Fides docs at `../fides/docs/` — plugin framework with `#[plugin_interface]` / `#[plugin_impl]`

## Goals & Non-Goals

**Goals:**
- Define an `ArawnTool` fides interface (`#[plugin_interface]`) that mirrors our Tool trait
- `arawn-tool-interface` crate — the shared interface crate that plugins depend on
- Adapter layer: bridge between fides `PluginHandle` and arawn's `Tool` trait so loaded plugins register seamlessly in `ToolRegistry`
- `ToolRegistry::load_plugin(path)` — load a dylib tool at runtime
- Built-in tools: FileWriteTool, FileEditTool, GrepTool (stay in arawn-engine, no dylib needed)
- Plugin tools: WebFetchTool, WebSearchTool as example dylib plugins
- Plugin discovery: scan a `~/.arawn/plugins/` directory on startup
- Unit tests + TestHarness functional tests for all tools

**Non-Goals:**
- Plugin signing/verification (fides supports it, but not needed yet)
- Plugin marketplace or remote download
- MCP integration (separate initiative)
- Agent/sub-agent tools (separate initiative)

## Architecture

### Crate Structure

```
arawn/
├── crates/
│   ├── arawn-tool-interface/     # Fides interface crate
│   │   └── src/lib.rs            # #[plugin_interface] ArawnTool trait
│   ├── arawn-engine/             # Adapter: PluginHandle → Tool trait
│   │   └── src/
│   │       ├── plugin_adapter.rs # Wraps PluginHandle as Box<dyn Tool>
│   │       └── tools/            # Built-in tools (FileWrite, Grep, etc.)
│   └── arawn/                    # Binary: scans plugins dir on startup
├── plugins/                      # Example plugin crates
│   ├── arawn-plugin-web-fetch/   # WebFetchTool as dylib
│   └── arawn-plugin-web-search/  # WebSearchTool as dylib
```

### Fides Interface

```rust
// arawn-tool-interface/src/lib.rs
use fidius::plugin_interface;

#[plugin_interface(version = 1, buffer = PluginAllocated)]
pub trait ArawnTool: Send + Sync {
    fn name(&self) -> String;
    fn description(&self) -> String;
    fn parameters_schema(&self) -> String;  // JSON Schema as string
    fn execute(&self, context_json: String, params_json: String) -> Result<String, String>;
}
```

Note: fides requires `Serialize + Deserialize` args, so we pass JSON strings across the FFI boundary rather than `serde_json::Value` (which doesn't work with bincode). The adapter handles ser/deser on both sides.

### Plugin Adapter

```rust
// arawn-engine/src/plugin_adapter.rs
struct PluginToolAdapter {
    handle: PluginHandle,
    cached_name: String,
    cached_description: String,
    cached_schema: serde_json::Value,
}

#[async_trait]
impl Tool for PluginToolAdapter {
    fn name(&self) -> &str { &self.cached_name }
    fn description(&self) -> &str { &self.cached_description }
    fn parameters_schema(&self) -> Value { self.cached_schema.clone() }
    async fn execute(&self, ctx: &ToolContext, params: Value) -> Result<ToolOutput, EngineError> {
        // Serialize context + params to JSON strings
        // Call handle.call_method(EXECUTE_INDEX, &(ctx_json, params_json))
        // Deserialize result back to ToolOutput
    }
}
```

### Plugin Lifecycle

`.arawn_tool` files are **compressed source archives** (tar.bz2) produced by `fidius package pack`. They contain the plugin source code, `package.toml` manifest, and optional `package.sig` signature.

**Installation flow:**
1. User places `web-fetch.arawn_tool` into `~/.arawn/plugins/tools/`
2. On startup, Arawn scans for `.arawn_tool` files
3. For each archive:
   - **Unpack** to a build directory (`~/.arawn/plugins/build/<name>-<version>/`)
   - **Verify** signature if configured
   - **Build** via `fidius package build` → compiles to cdylib
   - **Load** compiled dylib via `PluginHost`
   - **Wrap** in `PluginToolAdapter` and register in `ToolRegistry`
4. Build artifacts are cached — only rebuild if the archive changed (compare digest)

**The `package.toml` manifest** inside each archive uses `extension = "arawn_tool"` to identify the archive format.

This meshes with the existing hot-reload design — plugins can also be loaded/unloaded at runtime. Dropping a new `.arawn_tool` file and restarting (or triggering a reload) picks it up.

### Built-in vs Plugin Tools

| Tool | Location | Why |
|------|----------|-----|
| ThinkTool | built-in (arawn-engine) | No deps, core functionality |
| ShellTool | built-in | Needs ToolContext.working_dir directly |
| FileReadTool | built-in | Path traversal needs direct fs access |
| FileWriteTool | built-in | Same as FileRead |
| FileEditTool | built-in | Same |
| GrepTool | built-in | Shells to rg, needs working_dir |
| WebFetchTool | plugin dylib | Isolated deps (reqwest), good plugin example |
| WebSearchTool | plugin dylib | Isolated deps, good plugin example |

## Detailed Design

### FileWriteTool
- Params: `path: String`, `content: String`
- Creates parent directories if needed
- Path traversal protection (canonicalize + check prefix)
- Returns success message with bytes written

### FileEditTool
- Params: `path: String`, `old_string: String`, `new_string: String`, `replace_all: Option<bool>`
- Reads file, performs string replacement, writes back
- Fails if `old_string` not found or not unique (unless `replace_all`)
- Path traversal protection

### GrepTool
- Params: `pattern: String`, `path: Option<String>`, `glob: Option<String>`, `case_insensitive: Option<bool>`
- Shells out to `rg` (ripgrep) with appropriate flags
- Falls back to `grep -r` if rg not available
- CWD = workstream root, path is relative

### WebFetchTool (plugin)
- Params: `url: String`, `max_bytes: Option<usize>`
- Uses `reqwest` to fetch URL
- Strips HTML tags for HTML responses
- Timeout: 30s

### WebSearchTool (plugin)
- Params: `query: String`, `num_results: Option<u32>`
- DuckDuckGo HTML search (no API key)
- Returns title + URL + snippet

### DataLayout V2

Add `plugins/` to the filesystem layout:
```
~/.arawn/
├── arawn.db
├── plugins/
│   ├── tools/        # NEW — .arawn_tool source archives (user drops files here)
│   └── build/        # NEW — unpacked + compiled plugin artifacts (managed by Arawn)
├── workstreams/
└── scratch/sessions/
```

## Alternatives Considered

1. **All tools built-in, no plugins** — Rejected. The hot-reload design in ToolRegistry already anticipates runtime tool changes. Fides gives us a safe, typed plugin boundary. Web tools are a perfect first candidate — isolated deps, no need for direct ToolContext access.
2. **WASM plugins instead of dylibs** — Rejected. Fides is already built and tested. WASM adds sandboxing overhead and serde complexity. Can revisit for untrusted plugins later.
3. **Built-in grep instead of shelling to rg** — Rejected. rg is fast, handles binary files, respects .gitignore.

## Implementation Plan

Tasks will be decomposed after design approval. Rough ordering:
1. arawn-tool-interface crate (fides interface)
2. Plugin adapter in arawn-engine (PluginHandle → Tool)
3. Built-in tools: FileWriteTool, FileEditTool, GrepTool
4. Plugin tools: WebFetchTool, WebSearchTool as dylib crates
5. Plugin discovery + DataLayout V2 (add plugins/ dir)
6. Wire into binary — scan plugins on startup
7. Tests for everything