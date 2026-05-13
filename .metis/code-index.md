# Code Index

> Generated: 2026-05-13T11:25:32Z | 296 files | Python, Rust

## Project Structure

```
├── crates/
│   ├── arawn/
│   │   ├── build.rs
│   │   └── src/
│   │       ├── channel_prompt.rs
│   │       ├── config.rs
│   │       ├── config_watcher.rs
│   │       ├── lib.rs
│   │       ├── llm_pool.rs
│   │       ├── local_service.rs
│   │       ├── main.rs
│   │       ├── plugin_cmd.rs
│   │       └── ws_server.rs
│   ├── arawn-auth/
│   │   └── src/
│   │       ├── error.rs
│   │       ├── lib.rs
│   │       ├── oauth2.rs
│   │       ├── server.rs
│   │       └── token_store.rs
│   ├── arawn-core/
│   │   └── src/
│   │       ├── error.rs
│   │       ├── lib.rs
│   │       ├── message.rs
│   │       ├── session.rs
│   │       ├── session_stats.rs
│   │       └── workstream.rs
│   ├── arawn-embed/
│   │   └── src/
│   │       ├── api.rs
│   │       ├── config.rs
│   │       ├── error.rs
│   │       ├── lib.rs
│   │       └── local.rs
│   ├── arawn-engine/
│   │   └── src/
│   │       ├── agent_defs.rs
│   │       ├── background.rs
│   │       ├── compact_prompt.rs
│   │       ├── compactor.rs
│   │       ├── context.rs
│   │       ├── diff.rs
│   │       ├── error.rs
│   │       ├── hooks/
│   │       │   ├── config.rs
│   │       │   ├── events.rs
│   │       │   ├── executor.rs
│   │       │   ├── file_watcher.rs
│   │       │   ├── loader.rs
│   │       │   ├── matcher.rs
│   │       │   ├── mod.rs
│   │       │   └── runner.rs
│   │       ├── lib.rs
│   │       ├── permissions/
│   │       │   ├── checker.rs
│   │       │   ├── config.rs
│   │       │   ├── mod.rs
│   │       │   ├── prompt.rs
│   │       │   └── rules.rs
│   │       ├── plan.rs
│   │       ├── plugins/
│   │       │   ├── builtin.rs
│   │       │   ├── components.rs
│   │       │   ├── installer.rs
│   │       │   ├── loader.rs
│   │       │   ├── manifest.rs
│   │       │   ├── marketplace.rs
│   │       │   ├── mod.rs
│   │       │   ├── runtime.rs
│   │       │   └── settings.rs
│   │       ├── query_engine.rs
│   │       ├── skills/
│   │       │   ├── definition.rs
│   │       │   ├── loader.rs
│   │       │   └── mod.rs
│   │       ├── system_prompt.rs
│   │       ├── testing.rs
│   │       ├── token_estimator.rs
│   │       ├── tool.rs
│   │       ├── tool_result_limiter.rs
│   │       ├── tools/
│   │       │   ├── agent.rs
│   │       │   ├── ask_user.rs
│   │       │   ├── enter_plan_mode.rs
│   │       │   ├── exit_plan_mode.rs
│   │       │   ├── feed_search.rs
│   │       │   ├── file_edit.rs
│   │       │   ├── file_read.rs
│   │       │   ├── file_write.rs
│   │       │   ├── glob.rs
│   │       │   ├── grep.rs
│   │       │   ├── memory_search.rs
│   │       │   ├── memory_store.rs
│   │       │   ├── mod.rs
│   │       │   ├── safe_env.rs
│   │       │   ├── sensitive_paths.rs
│   │       │   ├── shell.rs
│   │       │   ├── signal.rs
│   │       │   ├── skill.rs
│   │       │   ├── sleep.rs
│   │       │   ├── steward.rs
│   │       │   ├── task_list.rs
│   │       │   ├── task_output.rs
│   │       │   ├── task_stop.rs
│   │       │   ├── think.rs
│   │       │   ├── web_fetch.rs
│   │       │   ├── web_search.rs
│   │       │   └── workstream.rs
│   │       └── workstream_router.rs
│   ├── arawn-extractor/
│   │   └── src/
│   │       ├── chain.rs
│   │       ├── cot.rs
│   │       ├── error.rs
│   │       ├── lib.rs
│   │       ├── llm_text.rs
│   │       └── runner.rs
│   ├── arawn-feeds/
│   │   ├── src/
│   │   │   ├── cadence.rs
│   │   │   ├── clients/
│   │   │   │   ├── atlassian.rs
│   │   │   │   ├── calendar.rs
│   │   │   │   ├── drive.rs
│   │   │   │   ├── gmail.rs
│   │   │   │   ├── mod.rs
│   │   │   │   └── slack.rs
│   │   │   ├── dispatch.rs
│   │   │   ├── error.rs
│   │   │   ├── layout.rs
│   │   │   ├── lib.rs
│   │   │   ├── meta.rs
│   │   │   ├── registry.rs
│   │   │   ├── runtime.rs
│   │   │   ├── store.rs
│   │   │   ├── template.rs
│   │   │   ├── templates/
│   │   │   │   ├── calendar/
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   └── upcoming_archive.rs
│   │   │   │   ├── confluence/
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   └── space_archive.rs
│   │   │   │   ├── drive/
│   │   │   │   │   ├── common.rs
│   │   │   │   │   ├── folder_sync.rs
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   └── recent.rs
│   │   │   │   ├── gmail/
│   │   │   │   │   ├── common.rs
│   │   │   │   │   ├── inbox_archive.rs
│   │   │   │   │   ├── label_archive.rs
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   └── sender_filter.rs
│   │   │   │   ├── jira/
│   │   │   │   │   ├── assignee_tracker.rs
│   │   │   │   │   ├── common.rs
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   └── project_tracker.rs
│   │   │   │   ├── mod.rs
│   │   │   │   ├── slack/
│   │   │   │   │   ├── channel_archive.rs
│   │   │   │   │   ├── common.rs
│   │   │   │   │   ├── dm_archive.rs
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   └── my_mentions.rs
│   │   │   │   └── stub.rs
│   │   │   └── types.rs
│   │   └── tests/
│   │       ├── calendar_upcoming_archive.rs
│   │       ├── cloacina_fire.rs
│   │       ├── confluence_space_archive.rs
│   │       ├── discovery.rs
│   │       ├── drive_folder_sync.rs
│   │       ├── drive_recent.rs
│   │       ├── dynamic_register.rs
│   │       ├── gmail_archive.rs
│   │       ├── jira_trackers.rs
│   │       ├── slack_channel_archive.rs
│   │       ├── slack_dm_archive.rs
│   │       └── slack_my_mentions.rs
│   ├── arawn-integrations/
│   │   └── src/
│   │       ├── atlassian/
│   │       │   ├── adf.rs
│   │       │   ├── client.rs
│   │       │   ├── confluence.rs
│   │       │   ├── integration.rs
│   │       │   ├── jira.rs
│   │       │   └── mod.rs
│   │       ├── calendar/
│   │       │   ├── client.rs
│   │       │   ├── integration.rs
│   │       │   ├── mod.rs
│   │       │   └── tools.rs
│   │       ├── credential_store.rs
│   │       ├── drive/
│   │       │   ├── client.rs
│   │       │   ├── integration.rs
│   │       │   ├── mod.rs
│   │       │   └── tools.rs
│   │       ├── error.rs
│   │       ├── gmail/
│   │       │   ├── client.rs
│   │       │   ├── integration.rs
│   │       │   ├── mod.rs
│   │       │   └── tools.rs
│   │       ├── google_common.rs
│   │       ├── integration.rs
│   │       ├── lib.rs
│   │       ├── oauth_flow.rs
│   │       ├── retry_after.rs
│   │       └── slack/
│   │           ├── client.rs
│   │           ├── integration.rs
│   │           ├── mod.rs
│   │           └── tools.rs
│   ├── arawn-llm/
│   │   └── src/
│   │       ├── anthropic.rs
│   │       ├── client.rs
│   │       ├── error.rs
│   │       ├── groq.rs
│   │       ├── lib.rs
│   │       ├── mock.rs
│   │       ├── openai_compat.rs
│   │       ├── retry.rs
│   │       ├── types.rs
│   │       └── warming.rs
│   ├── arawn-mcp/
│   │   └── src/
│   │       ├── adapter.rs
│   │       ├── config.rs
│   │       ├── lib.rs
│   │       └── manager.rs
│   ├── arawn-memory/
│   │   ├── src/
│   │   │   ├── cypher_schema.rs
│   │   │   ├── error.rs
│   │   │   ├── inject.rs
│   │   │   ├── lib.rs
│   │   │   ├── manager.rs
│   │   │   ├── shortcodes.rs
│   │   │   ├── stack.rs
│   │   │   ├── store.rs
│   │   │   ├── types.rs
│   │   │   └── vector.rs
│   │   └── tests/
│   │       ├── longmemeval_bench.rs
│   │       └── recall_eval.rs
│   ├── arawn-projections/
│   │   ├── src/
│   │   │   ├── atlassian.rs
│   │   │   ├── calendar.rs
│   │   │   ├── dispatch.rs
│   │   │   ├── drive.rs
│   │   │   ├── embed.rs
│   │   │   ├── error.rs
│   │   │   ├── gmail.rs
│   │   │   ├── lib.rs
│   │   │   ├── schema.rs
│   │   │   ├── slack.rs
│   │   │   ├── store.rs
│   │   │   └── types.rs
│   │   └── tests/
│   │       ├── embed_pass.rs
│   │       ├── gmail_e2e.rs
│   │       └── hybrid_search.rs
│   ├── arawn-service/
│   │   └── src/
│   │       ├── error.rs
│   │       ├── lib.rs
│   │       └── types.rs
│   ├── arawn-steward/
│   │   └── src/
│   │       ├── cursor.rs
│   │       ├── doorwatch.rs
│   │       ├── error.rs
│   │       ├── journal.rs
│   │       ├── lib.rs
│   │       ├── llm_text.rs
│   │       ├── map.rs
│   │       ├── reshelve.rs
│   │       ├── rollback.rs
│   │       ├── runner.rs
│   │       └── subroutine.rs
│   ├── arawn-storage/
│   │   └── src/
│   │       ├── database.rs
│   │       ├── error.rs
│   │       ├── extractor_cursor_store.rs
│   │       ├── jsonl.rs
│   │       ├── layout.rs
│   │       ├── lib.rs
│   │       ├── session_store.rs
│   │       ├── store.rs
│   │       └── workstream_store.rs
│   ├── arawn-tests/
│   │   ├── build.rs
│   │   ├── src/
│   │   │   └── lib.rs
│   │   └── tests/
│   │       ├── compaction.rs
│   │       ├── engine_persistence.rs
│   │       ├── full_pipeline.rs
│   │       ├── hooks.rs
│   │       ├── hot_reload.rs
│   │       ├── local_service.rs
│   │       ├── memory_stack.rs
│   │       ├── memory_tools.rs
│   │       ├── permissions.rs
│   │       ├── plugin_components.rs
│   │       ├── skills.rs
│   │       ├── tool_artifacts.rs
│   │       ├── uat.rs
│   │       ├── websocket.rs
│   │       └── workflows.rs
│   ├── arawn-tool/
│   │   └── src/
│   │       ├── context.rs
│   │       ├── error.rs
│   │       ├── lib.rs
│   │       ├── llm_preference.rs
│   │       ├── registry.rs
│   │       └── tool.rs
│   ├── arawn-tui/
│   │   └── src/
│   │       ├── action.rs
│   │       ├── app.rs
│   │       ├── command.rs
│   │       ├── event.rs
│   │       ├── event_loop.rs
│   │       ├── lib.rs
│   │       ├── markdown.rs
│   │       ├── modal.rs
│   │       ├── render.rs
│   │       ├── snapshot.rs
│   │       ├── snapshot_tests.rs
│   │       ├── theme.rs
│   │       ├── tui_prompt.rs
│   │       ├── width.rs
│   │       ├── wrap.rs
│   │       └── ws_client.rs
│   └── arawn-workflow/
│       ├── build.rs
│       └── src/
│           ├── agent_executor.rs
│           ├── lib.rs
│           ├── runner.rs
│           ├── scaffold.rs
│           └── tools.rs
├── examples/
│   └── workflows/
│       ├── daily-pr-summary/
│       │   ├── build.rs
│       │   └── src/
│       │       └── lib.rs
│       ├── issue-triage/
│       │   └── lib.rs
│       └── work-signal-pipeline/
│           └── lib.rs
└── scripts/
    └── functional_test.py
```

## Modules

### crates/arawn

**Role**: The binary crate that owns `main`, CLI argument parsing, startup orchestration, and the WebSocket server — it wires all subsystem crates into a running process.

**Key abstractions**:
- `main` — Parses the CLI (clap), builds the `LlmClientPool`, opens the `Store`, calls `register_default_tools`, loads plugins/skills/hooks, constructs `LocalService`, and either runs the WebSocket server (`Serve`), the TUI (`Tui`), or a single prompt via WebSocket (`run_cli_via_server`). The serve path also spawns the `ConfigWatcher` and plugin hot-reload watcher.
- `Command` — Three modes: `Serve` (start the JSON-RPC WebSocket server), `Tui` (attach the terminal UI), `Plugin` (delegated to `plugin_cmd`).
- `build_llm_client` — Instantiates either an `AnthropicClient` or `OpenAICompatibleClient` from a config entry; called once per named LLM in `arawn.toml`.
- `register_default_tools` — Creates and registers all engine tools (file I/O, shell, grep, glob, memory, web, plan mode, workflows, etc.) against the shared `ToolRegistry`. This is the canonical list of which tools are active.
- `connect_mcp_servers` — Launches configured MCP server subprocesses and registers each tool they advertise.

**Internal flow**: Startup creates `LlmClientPool` → `Store` → tool `ToolRegistry` → `LocalService`. If `Serve`, `run_server` is called which starts the Axum WebSocket listener. The `ConfigWatcher` watches `arawn.toml` and calls `PermissionChecker::update_rules` / `update_mode` on change without restart.

**Dependencies**: Depends on virtually every other crate. Acts as the composition root.

#### crates/arawn/build.rs

-  `main` function L1-3 — `()`

### crates/arawn/src

**Role**: Source modules for the binary crate — configuration, runtime service implementation, WebSocket server, and supporting utilities that tie engine and storage together.

**Key abstractions**:
- `LocalService` — The concrete `ArawnService` impl. Holds the `Store`, `LlmClientPool`, `ToolRegistry`, permission state, plan state, background task manager, and memory manager. Each call to `send_message` builds a fresh `QueryEngine`, runs it against the stored session, streams `EngineEvent`s back through an mpsc channel, and persists messages to JSONL. This is the only `ArawnService` impl in the system.
- `LlmClientPool` — Name-keyed map of `Arc<dyn LlmClient>` instances built from `ArawnConfig`. Separates the engine LLM from the compactor LLM. Exposes `resolve(&LlmPreference)` which tools and agents call to pick the best match; falls back gracefully when a preference cannot be satisfied.
- `ArawnConfig` — Top-level config deserialized from `arawn.toml`. Contains named `[llm.*]`, `[engine]`, `[compactor]`, `[server]`, `[storage]`, `[sandbox]`, and `[embeddings]` sections. `load()` merges env var overrides on top of the file.
- `ConfigWatcher` — Uses `notify` to watch `arawn.toml` with debouncing. On change it calls `ArawnConfig::load`, diffs permissions, and hot-updates `PermissionChecker` without a restart.
- `ChannelModalPrompt` — Implements `ModalPrompt` by sending a `ModalRequest` through the engine-event mpsc channel to the WebSocket server, which relays it to the client. The response flows back through a `oneshot` channel keyed in `PendingModals`.
- `ws_server` — Axum-based JSON-RPC over WebSocket. Each connection gets an independent `handle_connection` task. Methods map directly to `LocalService` methods. `from_service_error` converts `ServiceError` to structured wire responses, preserving the `kind` tag from `ServiceError::details()`.
- `plugin_cmd` — CLI dispatch for `arawn plugin install/uninstall/enable/disable/list/marketplace`. Delegates to the plugin installer and settings JSON.

**Internal flow**: `send_message` in `LocalService` acquires the session from the store, calls `build_session_context` (which assembles `EngineToolContext` and `PromptContext`), calls `build_engine` (which wires compactor, permissions, hooks, skills, plugins, plan state), then runs the engine and streams results. Messages are appended to JSONL inside the stream loop.

**Mixed concerns / gotchas**: `LocalService` carries a `std::sync::Mutex<Store>` (not async) because `rusqlite::Connection` is not `Send`. The mutex is acquired briefly for each DB operation and released before any async await points. `active_sessions` prevents concurrent `send_message` calls to the same session.

**Dependencies**: `arawn-engine`, `arawn-storage`, `arawn-llm`, `arawn-service`, `arawn-core`, `arawn-memory`, `arawn-workflow`, `arawn-mcp`, `arawn-tool`, `arawn-embed`; uses `axum` + `tokio-tungstenite` for the WebSocket server.

#### crates/arawn/src/channel_prompt.rs

- pub `PendingModals` type L23 — `= Arc<Mutex<HashMap<String, oneshot::Sender<Option<usize>>>>>` — Shared map of pending modal responses.
- pub `new_pending_modals` function L26-28 — `() -> PendingModals` — Create a new empty pending modals map.
- pub `ChannelModalPrompt` struct L31-34 — `{ tx: mpsc::Sender<EngineEvent>, pending: PendingModals }` — ModalPrompt that sends via an EngineEvent channel and waits for response.
- pub `new` function L37-39 — `(tx: mpsc::Sender<EngineEvent>, pending: PendingModals) -> Self` — 6.
-  `ChannelModalPrompt` type L36-40 — `= ChannelModalPrompt` — 6.
-  `ChannelModalPrompt` type L43-84 — `impl ModalPrompt for ChannelModalPrompt` — 6.
-  `prompt` function L44-83 — `(&self, request: ModalRequest) -> Option<usize>` — 6.

#### crates/arawn/src/config.rs

- pub `LlmConfig` struct L9-34 — `{ provider: String, model: String, api_key: Option<String>, api_key_env: String,...` — A named LLM provider configuration.
- pub `to_resolved_info` function L68-76 — `(&self) -> arawn_tool::ResolvedLlmInfo` — Project this config into the capability metadata used by
- pub `EngineConfig` struct L80-87 — `{ llm: String, max_iterations: usize, max_result_size: usize }`
- pub `CompactorConfig` struct L110-118 — `{ llm: Option<String>, compaction_threshold: f32, keep_recent: usize }`
- pub `ExtractionConfig` struct L144-148 — `{ llm: Option<String> }` — Configuration for the per-workstream extractor (I-0040 phase 4).
- pub `ServerConfig` struct L151-156 — `{ host: String, port: u16 }`
- pub `StorageConfig` struct L175-178 — `{ data_dir: String }`
- pub `PromptsConfig` struct L193-196 — `{ token_budget: u32 }`
- pub `SandboxConfig` struct L212-218 — `{ network_tools: Vec<String> }` — Sandbox configuration for shell command execution.
- pub `IntegrationCredentials` struct L270-275 — `{ client_id: String, client_secret: String }` — OAuth client credentials for one integration.
- pub `IntegrationsConfig` struct L282-305 — `{ slack: IntegrationCredentials, google: IntegrationCredentials, gmail: Integrat...` — Per-integration credential blocks.
- pub `ArawnConfig` struct L309-328 — `{ llm: HashMap<String, LlmConfig>, engine: EngineConfig, compactor: CompactorCon...` — Top-level configuration.
- pub `load` function L354-387 — `(data_dir: &Path) -> Self` — Load config from `data_dir/arawn.toml`, merging with env var overrides and defaults.
- pub `engine_llm` function L410-415 — `(&self) -> &LlmConfig` — Resolve the LLM config for the engine.
- pub `compactor_llm` function L418-425 — `(&self) -> &LlmConfig` — Resolve the LLM config for the compactor.
- pub `extraction_llm` function L430-437 — `(&self) -> &LlmConfig` — Resolve the LLM config for the per-workstream extractor.
- pub `extraction_llm_name` function L442-447 — `(&self) -> &str` — The configured name of the extraction LLM (or the engine's
- pub `data_dir` function L450-452 — `(&self) -> PathBuf` — Resolve the data directory with ~ expansion.
- pub `prompts_dir` function L455-457 — `(&self) -> PathBuf` — Resolve the prompts directory.
- pub `resolve_api_key` function L461-468 — `(llm: &LlmConfig) -> Option<String>` — Resolve API key for an LLM config.
- pub `generate_default_toml` function L471-562 — `() -> String` — Generate a default config file string with comments.
-  `default_api_key_env` function L36-38 — `() -> String`
-  `default_context_window` function L39-41 — `() -> u32`
-  `default_max_tokens` function L42-44 — `() -> u32`
-  `default_tool_use` function L45-47 — `() -> bool`
-  `LlmConfig` type L49-63 — `impl Default for LlmConfig`
-  `default` function L50-62 — `() -> Self`
-  `LlmConfig` type L65-77 — `= LlmConfig`
-  `default_engine_llm` function L89-91 — `() -> String`
-  `default_max_iterations` function L92-94 — `() -> usize`
-  `default_max_result_size` function L95-97 — `() -> usize`
-  `EngineConfig` type L99-107 — `impl Default for EngineConfig`
-  `default` function L100-106 — `() -> Self`
-  `default_compaction_threshold` function L120-122 — `() -> f32`
-  `default_keep_recent` function L123-125 — `() -> usize`
-  `CompactorConfig` type L127-135 — `impl Default for CompactorConfig`
-  `default` function L128-134 — `() -> Self`
-  `default_host` function L158-160 — `() -> String`
-  `default_port` function L161-163 — `() -> u16`
-  `ServerConfig` type L165-172 — `impl Default for ServerConfig`
-  `default` function L166-171 — `() -> Self`
-  `default_data_dir` function L180-182 — `() -> String`
-  `StorageConfig` type L184-190 — `impl Default for StorageConfig`
-  `default` function L185-189 — `() -> Self`
-  `default_prompt_token_budget` function L198-200 — `() -> u32`
-  `PromptsConfig` type L202-208 — `impl Default for PromptsConfig`
-  `default` function L203-207 — `() -> Self`
-  `default_network_tools` function L220-256 — `() -> Vec<String>`
-  `SandboxConfig` type L258-264 — `impl Default for SandboxConfig`
-  `default` function L259-263 — `() -> Self`
-  `default_llm_configs` function L330-334 — `() -> HashMap<String, LlmConfig>`
-  `ArawnConfig` type L336-350 — `impl Default for ArawnConfig`
-  `default` function L337-349 — `() -> Self`
-  `ArawnConfig` type L352-563 — `= ArawnConfig`
-  `apply_env_overrides` function L389-407 — `(&mut self)`
-  `expand_tilde` function L565-572 — `(path: &str) -> PathBuf`
-  `tests` module L575-702 — `-`
-  `default_config_has_working_values` function L579-588 — `()`
-  `load_from_toml_string` function L591-611 — `()`
-  `compactor_falls_back_to_engine_llm` function L614-619 — `()`
-  `compactor_uses_own_llm_when_specified` function L622-641 — `()`
-  `missing_llm_name_falls_back_to_default_via_load` function L644-660 — `()`
-  `load_missing_file_uses_defaults` function L663-667 — `()`
-  `load_from_tempdir` function L670-688 — `()`
-  `generate_default_toml_is_parseable` function L691-695 — `()`
-  `tilde_expansion` function L698-701 — `()`

#### crates/arawn/src/config_watcher.rs

- pub `ConfigWatcher` struct L21-31 — `{ config_path: PathBuf, data_dir: PathBuf, permission_rules: Arc<std::sync::RwLo...` — Watches config files and dispatches live updates to running subsystems.
- pub `new` function L34-49 — `( config_path: PathBuf, data_dir: PathBuf, permission_rules: Arc<std::sync::RwLo...` — with debouncing.
- pub `with_notify` function L52-55 — `(mut self, notify: Arc<dyn Fn(bool, String) + Send + Sync>) -> Self` — Attach a notify callback fired after each reload completes.
- pub `spawn` function L58-64 — `(self) -> tokio::task::JoinHandle<()>` — Spawn the file watcher as a background tokio task.
-  `ConfigWatcher` type L33-168 — `= ConfigWatcher` — with debouncing.
-  `run` function L66-125 — `(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>` — with debouncing.
-  `reload` function L127-167 — `(&self)` — with debouncing.

#### crates/arawn/src/lib.rs

- pub `channel_prompt` module L1 — `-`
- pub `config` module L2 — `-`
- pub `config_watcher` module L3 — `-`
- pub `llm_pool` module L4 — `-`
- pub `local_service` module L5 — `-`
- pub `plugin_cmd` module L6 — `-`
- pub `ws_server` module L7 — `-`

#### crates/arawn/src/llm_pool.rs

- pub `LlmClientPool` struct L21-26 — `{ clients: HashMap<String, Arc<dyn LlmClient>>, configs: HashMap<String, LlmConf...` — A pool of named LLM clients built from an [`ArawnConfig`].
- pub `from_config` function L42-71 — `(config: &ArawnConfig, build: F) -> Result<Self>` — Build the pool from the given config.
- pub `from_clients` function L75-86 — `( clients: HashMap<String, Arc<dyn LlmClient>>, configs: HashMap<String, LlmConf...` — Construct a pool from a pre-built map of clients.
- pub `single` function L90-102 — `(client: Arc<dyn LlmClient>, model: impl Into<String>) -> Self` — Build a single-entry pool wrapping `client` as both engine and
- pub `get` function L105-107 — `(&self, name: &str) -> Option<Arc<dyn LlmClient>>` — Look up a client by name (e.g., "default", "cheap", "judge").
- pub `config` function L110-112 — `(&self, name: &str) -> Option<&LlmConfig>` — Get the [`LlmConfig`] for a named entry.
- pub `engine` function L115-117 — `(&self) -> Arc<dyn LlmClient>` — Engine LLM — never fails; falls back to whatever `engine_llm()` resolved.
- pub `engine_config` function L119-121 — `(&self) -> &LlmConfig` — surfaces here, not mid-session.
- pub `engine_name` function L123-125 — `(&self) -> &str` — surfaces here, not mid-session.
- pub `compactor` function L129-131 — `(&self) -> Arc<dyn LlmClient>` — Compactor LLM — never fails; falls back to engine LLM if `[compactor]`
- pub `compactor_config` function L133-135 — `(&self) -> &LlmConfig` — surfaces here, not mid-session.
- pub `compactor_name` function L137-139 — `(&self) -> &str` — surfaces here, not mid-session.
- pub `entries` function L142-144 — `(&self) -> impl Iterator<Item = (&String, &LlmConfig)>` — Iterator over (name, config) pairs.
- pub `warmup_all` function L149-168 — `( &self, ) -> Vec<(String, Result<(), arawn_llm::LlmError>)>` — Warm up every entry concurrently.
- pub `resolve` function L178-239 — `(&self, preference: &LlmPreference) -> LlmResolution` — Resolve an [`LlmPreference`] against the pool.
- pub `len` function L241-243 — `(&self) -> usize` — surfaces here, not mid-session.
- pub `is_empty` function L245-247 — `(&self) -> bool` — surfaces here, not mid-session.
-  `LlmClientPool` type L28-36 — `= LlmClientPool` — surfaces here, not mid-session.
-  `fmt` function L29-35 — `(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` — surfaces here, not mid-session.
-  `LlmClientPool` type L38-248 — `= LlmClientPool` — surfaces here, not mid-session.
-  `resolve_engine_name` function L250-264 — `( config: &ArawnConfig, clients: &HashMap<String, Arc<dyn LlmClient>>, ) -> Resu...` — surfaces here, not mid-session.
-  `resolve_compactor_name` function L266-274 — `(config: &ArawnConfig, engine_name: &str) -> String` — surfaces here, not mid-session.
-  `tests` module L277-537 — `-` — surfaces here, not mid-session.
-  `mock_builder` function L281-283 — `(_cfg: &LlmConfig) -> Result<Arc<dyn LlmClient>>` — surfaces here, not mid-session.
-  `cfg_from_toml` function L285-287 — `(toml_str: &str) -> ArawnConfig` — surfaces here, not mid-session.
-  `pool_builds_every_named_entry` function L290-310 — `()` — surfaces here, not mid-session.
-  `engine_and_compactor_resolve_distinct_clients_when_configured` function L313-337 — `()` — surfaces here, not mid-session.
-  `compactor_falls_back_to_engine_when_unconfigured` function L340-352 — `()` — surfaces here, not mid-session.
-  `compactor_falls_back_to_engine_when_pointing_at_missing_entry` function L355-368 — `()` — surfaces here, not mid-session.
-  `resolve_named_exact_match` function L371-387 — `()` — surfaces here, not mid-session.
-  `resolve_named_missing_falls_back` function L390-402 — `()` — surfaces here, not mid-session.
-  `resolve_provider_model_exact` function L405-424 — `()` — surfaces here, not mid-session.
-  `resolve_capability_match_when_no_exact` function L427-452 — `()` — surfaces here, not mid-session.
-  `resolve_capability_too_strict_falls_back` function L455-474 — `()` — surfaces here, not mid-session.
-  `resolve_empty_preference_is_fallback` function L477-488 — `()` — surfaces here, not mid-session.
-  `resolve_provider_only_uses_capability_path` function L491-511 — `()` — surfaces here, not mid-session.
-  `pool_construction_fails_fast_when_builder_errors` function L514-536 — `()` — surfaces here, not mid-session.

#### crates/arawn/src/local_service.rs

- pub `LocalService` struct L31-86 — `{ store: Arc<Mutex<Store>>, data_dir: PathBuf, llm_pool: Arc<LlmClientPool>, reg...` — In-process implementation of ArawnService.
- pub `new` function L89-118 — `( store: Store, data_dir: PathBuf, llm_pool: Arc<LlmClientPool>, registry: Arc<T...`
- pub `with_active_workstream` function L123-126 — `(mut self, ws: arawn_engine::SessionWorkstream) -> Self` — Wire the shared `SessionWorkstream` shim.
- pub `set_feed_runtime` function L131-133 — `(&self, runtime: Arc<arawn_feeds::FeedRuntime>)` — Hand the live feed runtime to the service so `/watch` and
- pub `register_integration` function L149-153 — `(&self, integration: Arc<dyn arawn_integrations::Integration>)` — Register an external integration.
- pub `shared_integrations` function L157-161 — `( &self, ) -> Arc<std::sync::RwLock<HashMap<String, Arc<dyn arawn_integrations::...` — Shared reference to the integration registry — for tools that want
- pub `subscribe_notices` function L167-169 — `(&self) -> tokio::sync::broadcast::Receiver<arawn_service::ServerNotice>` — Subscribe to server-wide notices (plugin/config hot-reload, etc.).
- pub `notice_sender` function L173-175 — `(&self) -> tokio::sync::broadcast::Sender<arawn_service::ServerNotice>` — Get a sender clone — used to wire watchers (plugin runtime, config
- pub `with_permission_rules` function L177-180 — `(self, rules: Vec<PermissionRule>) -> Self`
- pub `shared_store` function L184-186 — `(&self) -> Arc<Mutex<Store>>` — Get a reference to the shared permission rules for hot-reload.
- pub `shared_llm` function L188-190 — `(&self) -> Arc<dyn LlmClient>`
- pub `shared_compactor_llm` function L194-196 — `(&self) -> Arc<dyn LlmClient>` — Compactor LLM (separate client when `[compactor]` config selects a
- pub `compactor_model` function L199-201 — `(&self) -> &str` — Model name used by the compactor.
- pub `shared_llm_pool` function L205-207 — `(&self) -> Arc<LlmClientPool>` — Shared reference to the LLM pool — used by tools/agents that resolve
- pub `shared_registry` function L209-211 — `(&self) -> Arc<ToolRegistry>`
- pub `engine_config` function L213-215 — `(&self) -> &QueryEngineConfig`
- pub `shared_permission_rules` function L217-219 — `(&self) -> Arc<std::sync::RwLock<Vec<PermissionRule>>>`
- pub `shared_permission_mode` function L221-223 — `(&self) -> Arc<std::sync::RwLock<arawn_engine::permissions::PermissionMode>>`
- pub `with_skill_registry` function L225-228 — `(mut self, registry: Arc<arawn_engine::skills::SkillRegistry>) -> Self`
- pub `with_plugin_registry` function L230-233 — `(mut self, registry: Arc<arawn_engine::plugins::PluginRegistry>) -> Self`
- pub `with_plan_state` function L235-238 — `(mut self, state: Arc<PlanModeState>) -> Self`
- pub `with_background_tasks` function L240-243 — `(mut self, manager: Arc<BackgroundTaskManager>) -> Self`
- pub `with_memory_manager` function L245-248 — `(mut self, mgr: Arc<arawn_memory::MemoryManager>) -> Self`
-  `LocalService` type L88-458 — `= LocalService`
-  `feed_runtime_or_err` function L135-145 — `(&self) -> Result<Arc<arawn_feeds::FeedRuntime>, ServiceError>`
-  `load_session_state` function L252-293 — `( &self, session_id: Uuid, ) -> Result<(arawn_storage::SessionMeta, Workstream, ...` — Load session metadata, resolve workstream, and load message history.
-  `build_session_context` function L297-404 — `( &self, session_id: Uuid, workstream: &Workstream, ws_dir: &str, workspace_dir:...` — Build a ToolContext and per-session PromptContext for the engine.
-  `build_engine` function L408-457 — `( &self, prompt_context: Option<arawn_engine::PromptContext>, event_tx: &mpsc::S...` — Build a QueryEngine configured with compactor, skills, plugins, and plan state.
-  `infer_entity_type` function L462-475 — `(text: &str) -> (arawn_memory::EntityType, String)` — Infer entity type from text patterns.
-  `LocalService` type L480-1593 — `impl ArawnService for LocalService`
-  `list_workstreams` function L481-496 — `(&self) -> Result<Vec<WorkstreamInfo>, ServiceError>`
-  `create_workstream` function L498-515 — `( &self, name: String, root_dir: PathBuf, ) -> Result<WorkstreamInfo, ServiceErr...`
-  `list_sessions` function L517-536 — `( &self, workstream_id: Option<Uuid>, ) -> Result<Vec<SessionInfo>, ServiceError...`
-  `create_session` function L538-559 — `( &self, workstream_id: Option<Uuid>, ) -> Result<SessionInfo, ServiceError>`
-  `load_session` function L561-588 — `(&self, id: Uuid) -> Result<SessionDetail, ServiceError>`
-  `truncate_session_at_user_message` function L590-638 — `( &self, id: Uuid, user_message_index: usize, ) -> Result<SessionDetail, Service...`
-  `send_message` function L641-837 — `( &self, session_id: Uuid, content: String, ) -> Result<Pin<Box<dyn futures::Str...`
-  `cancel` function L839-852 — `(&self, session_id: Uuid) -> Result<(), ServiceError>`
-  `promote_session` function L854-905 — `( &self, session_id: Uuid, workstream_name: &str, ) -> Result<PromotionResult, S...`
-  `resolve_user_input` function L907-921 — `( &self, request_id: &str, selected_index: Option<usize>, ) -> Result<(), Servic...`
-  `query_inventory` function L923-988 — `(&self, kind: &str) -> Result<Vec<InventoryItem>, ServiceError>`
-  `list_available_commands` function L990-1002 — `(&self) -> Result<Vec<CommandInfo>, ServiceError>`
-  `list_workflows` function L1004-1035 — `(&self) -> Result<Vec<WorkflowInfo>, ServiceError>`
-  `remember_fact` function L1037-1083 — `(&self, text: &str) -> Result<MemoryStoreResult, ServiceError>`
-  `memory_summary` function L1085-1132 — `(&self) -> Result<MemorySummary, ServiceError>`
-  `forget_entity` function L1134-1184 — `(&self, query: &str) -> Result<ForgetResult, ServiceError>`
-  `get_permission_mode` function L1186-1194 — `(&self) -> Result<PermissionModeInfo, ServiceError>`
-  `set_permission_mode` function L1196-1208 — `(&self, mode_str: &str) -> Result<PermissionModeInfo, ServiceError>`
-  `get_capabilities` function L1210-1220 — `(&self) -> Result<arawn_service::ServerCapabilities, ServiceError>`
-  `get_permissions_status` function L1222-1271 — `(&self) -> Result<arawn_service::PermissionsStatus, ServiceError>`
-  `list_integrations` function L1273-1291 — `(&self) -> Result<Vec<arawn_service::IntegrationStatus>, ServiceError>`
-  `start_oauth_flow` function L1293-1421 — `( &self, service: &str, ) -> Result<arawn_service::OAuthFlowStarted, ServiceErro...`
-  `disconnect_integration` function L1423-1446 — `(&self, service: &str) -> Result<(), ServiceError>`
-  `feed_register` function L1448-1481 — `( &self, spec: arawn_service::FeedRegisterSpec, ) -> Result<arawn_service::FeedS...`
-  `feed_list` function L1483-1487 — `(&self) -> Result<Vec<arawn_service::FeedSummaryDto>, ServiceError>`
-  `feed_pause` function L1489-1503 — `( &self, feed_id: &str, ) -> Result<arawn_service::FeedSummaryDto, ServiceError>`
-  `feed_resume` function L1505-1519 — `( &self, feed_id: &str, ) -> Result<arawn_service::FeedSummaryDto, ServiceError>`
-  `feed_run` function L1521-1542 — `( &self, feed_id: &str, ) -> Result<arawn_service::FeedSummaryDto, ServiceError>`
-  `feed_discover` function L1544-1569 — `( &self, template: &str, ) -> Result<arawn_service::FeedDiscoverDto, ServiceErro...`
-  `feed_remove` function L1571-1592 — `( &self, feed_id: &str, ) -> Result<arawn_service::FeedRemoveDto, ServiceError>`
-  `default_feed_for_service` function L1600-1609 — `(service: &str) -> Option<(&'static str, &'static str)>` — Personal default feed registered automatically the first time
-  `current_summary` function L1611-1621 — `( runtime: &arawn_feeds::FeedRuntime, feed_id: &str, ) -> Result<arawn_service::...`
-  `feed_err` function L1623-1632 — `(e: arawn_feeds::FeedError) -> ServiceError`
-  `feed_summary_to_dto` function L1634-1648 — `(s: arawn_feeds::FeedSummary) -> arawn_service::FeedSummaryDto`
-  `OAuthFlowCtx` struct L1653-1657 — `{ service: String, url_tx: tokio::sync::Mutex<Option<tokio::sync::oneshot::Sende...` — Glue that lets `LocalService::start_oauth_flow` bridge the integration's
-  `OAuthFlowCtx` type L1660-1682 — `= OAuthFlowCtx`
-  `service` function L1661-1663 — `(&self) -> &str`
-  `publish_auth_url` function L1665-1672 — `(&self, url: &url::Url)`
-  `publish_progress` function L1674-1681 — `(&self, message: &str)`
-  `resolve_ws_dir_from_store` function L1685-1696 — `(store: &Store, ws_id: Option<Uuid>) -> Result<String, ServiceError>` — Resolve workstream directory name from store.
-  `first_sentence` function L1700-1711 — `(s: &str) -> String` — Extract the first sentence and sanitize for use in a markdown table cell.
-  `feed_default_tests` module L1714-1751 — `-`
-  `known_services_each_have_a_default_feed` function L1718-1744 — `()`
-  `unknown_service_has_no_default_feed` function L1747-1750 — `()`

#### crates/arawn/src/main.rs

-  `EmbedderBridge` struct L12-14 — `{ inner: Arc<dyn arawn_embed::Embedder> }` — Adapter from `arawn_embed::Embedder` to the trait
-  `EmbedderBridge` type L16-32 — `= EmbedderBridge`
-  `embed_batch` function L17-31 — `( &'a self, texts: &'a [&'a str], ) -> std::pin::Pin< Box<dyn std::future::Futur...`
-  `DEFAULT_MODEL` variable L39 — `: &str`
-  `FILE_LOG_FILTER` variable L42 — `: &str` — Default file log filter: debug for arawn crates, warn for third-party.
-  `main` function L45-1152 — `() -> Result<()>`
-  `Cli` struct L51-70 — `{ command: Option<Command>, data_dir: Option<String>, session: Option<Uuid>, lis...`
-  `Command` enum L73-92 — `Serve | Tui | Plugin`
-  `ExtractorBindHook` struct L672-675 — `{ runner: Arc<arawn_extractor::ExtractorRunner>, store: Arc<std::sync::Mutex<ara...`
-  `ExtractorBindHook` type L676-710 — `= ExtractorBindHook`
-  `on_bind` function L677-709 — `(&self, workstream_name: &str, feed_id: &str)`
-  `run_cli_via_server` function L1155-1260 — `( url: &str, prompt: &str, session_id: Option<Uuid>, ) -> Result<()>` — Run a CLI prompt by connecting to the running server via WebSocket.
-  `build_llm_client` function L1263-1286 — `( config: &arawn_bin::LlmConfig, ) -> Result<Arc<dyn arawn_llm::LlmClient>>` — Build the appropriate LLM client based on provider config.
-  `register_default_tools` function L1289-1335 — `( registry: &Arc<arawn_engine::ToolRegistry>, config: &arawn_bin::ArawnConfig, d...` — Register all default tools into the registry.
-  `connect_mcp_servers` function L1338-1386 — `( data_dir: &str, plugin_result: &arawn_engine::plugins::PluginLoadResult, regis...` — Connect to MCP servers from config and plugins.
-  `register_workflow_tools` function L1389-1406 — `( registry: &Arc<arawn_engine::ToolRegistry>, workflows_dir: std::path::PathBuf,...` — Register workflow management tools.
-  `build_engine_config` function L1408-1443 — `( config: &arawn_bin::ArawnConfig, workstream: &arawn_core::Workstream, data_dir...`
-  `dirs_path` function L1445-1454 — `() -> Option<String>`

#### crates/arawn/src/plugin_cmd.rs

- pub `run_plugin_command` function L12-27 — `(args: &[String], plugins_root: &Path) -> Result<(), String>` — Handle the `arawn plugin` subcommand.
-  `cmd_install` function L29-45 — `(args: &[String], plugins_root: &Path) -> Result<(), String>` — Plugin CLI commands — arawn plugin install/uninstall/enable/disable/list/marketplace.
-  `cmd_uninstall` function L47-60 — `(args: &[String], plugins_root: &Path) -> Result<(), String>` — Plugin CLI commands — arawn plugin install/uninstall/enable/disable/list/marketplace.
-  `cmd_enable` function L62-71 — `(args: &[String], plugins_root: &Path) -> Result<(), String>` — Plugin CLI commands — arawn plugin install/uninstall/enable/disable/list/marketplace.
-  `cmd_disable` function L73-81 — `(args: &[String], plugins_root: &Path) -> Result<(), String>` — Plugin CLI commands — arawn plugin install/uninstall/enable/disable/list/marketplace.
-  `cmd_list` function L83-104 — `(plugins_root: &Path) -> Result<(), String>` — Plugin CLI commands — arawn plugin install/uninstall/enable/disable/list/marketplace.
-  `cmd_marketplace` function L106-116 — `(args: &[String], plugins_root: &Path) -> Result<(), String>` — Plugin CLI commands — arawn plugin install/uninstall/enable/disable/list/marketplace.
-  `cmd_marketplace_add` function L118-138 — `(args: &[String], plugins_root: &Path) -> Result<(), String>` — Plugin CLI commands — arawn plugin install/uninstall/enable/disable/list/marketplace.
-  `cmd_marketplace_list` function L140-160 — `(plugins_root: &Path) -> Result<(), String>` — Plugin CLI commands — arawn plugin install/uninstall/enable/disable/list/marketplace.
-  `parse_scope` function L163-175 — `(args: &[String]) -> Result<InstallScope, String>` — Parse --scope flag from args.
-  `parse_marketplace_source` function L182-229 — `(s: &str) -> Result<(String, MarketplaceSource), String>` — Parse a marketplace source string.
-  `update_enabled_plugins` function L232-268 — `( plugins_root: &Path, identifier: &str, enabled: bool, ) -> Result<(), String>` — Update enabledPlugins in settings.json at the plugins root.
-  `print_plugin_help` function L270-296 — `() -> Result<(), String>` — Plugin CLI commands — arawn plugin install/uninstall/enable/disable/list/marketplace.
-  `tests` module L299-348 — `-` — Plugin CLI commands — arawn plugin install/uninstall/enable/disable/list/marketplace.
-  `parse_github_source` function L303-307 — `()` — Plugin CLI commands — arawn plugin install/uninstall/enable/disable/list/marketplace.
-  `parse_url_source` function L310-315 — `()` — Plugin CLI commands — arawn plugin install/uninstall/enable/disable/list/marketplace.
-  `parse_directory_source` function L318-322 — `()` — Plugin CLI commands — arawn plugin install/uninstall/enable/disable/list/marketplace.
-  `parse_relative_directory` function L325-329 — `()` — Plugin CLI commands — arawn plugin install/uninstall/enable/disable/list/marketplace.
-  `parse_scope_default` function L332-335 — `()` — Plugin CLI commands — arawn plugin install/uninstall/enable/disable/list/marketplace.
-  `parse_scope_project` function L338-341 — `()` — Plugin CLI commands — arawn plugin install/uninstall/enable/disable/list/marketplace.
-  `parse_scope_invalid` function L344-347 — `()` — Plugin CLI commands — arawn plugin install/uninstall/enable/disable/list/marketplace.

#### crates/arawn/src/ws_server.rs

- pub `read_token_file` function L151-162 — `() -> Option<String>` — Read the auth token from {data_dir}/server.token.
- pub `run_server` function L165-200 — `(service: LocalService, port: u16) -> anyhow::Result<()>` — Start the WebSocket server on the given port.
- pub `handle_connection_public` function L286-288 — `(socket: WebSocket, service: Arc<LocalService>)` — Handle a single WebSocket connection.
-  `PROTOCOL_VERSION` variable L24 — `: &str` — Protocol version reported by the `hello` handshake.
-  `RPC_METHODS` variable L27-59 — `: &[&str]` — Canonical RPC method names (returned by `hello`).
-  `Request` struct L63-68 — `{ id: u64, method: String, params: Value }` — JSON-RPC style request from client.
-  `Response` struct L72-78 — `{ id: u64, result: Option<Value>, error: Option<ErrorBody> }` — JSON-RPC style response to client.
-  `ErrorBody` struct L81-86 — `{ code: String, message: String, details: Option<Value> }`
-  `Response` type L88-124 — `= Response`
-  `success` function L89-95 — `(id: u64, result: Value) -> Self`
-  `error` function L97-107 — `(id: u64, code: &str, message: String) -> Self`
-  `from_service_error` function L113-123 — `(id: u64, e: &arawn_service::ServiceError) -> Self` — Build an error response from a [`ServiceError`].
-  `AppState` struct L128-133 — `{ service: Arc<LocalService>, auth_token: Option<String> }` — Shared app state for the WebSocket server.
-  `generate_auth_token` function L136-139 — `() -> String` — Generate a random auth token for WebSocket connections.
-  `write_token_file` function L142-147 — `(data_dir: &std::path::Path, token: &str) -> std::io::Result<std::path::PathBuf>` — Write the auth token to {data_dir}/server.token for clients to read.
-  `shutdown_signal` function L203-225 — `()` — Wait for a shutdown signal (Ctrl-C / SIGTERM).
-  `decision_handler` function L230-249 — `( State(AppState { service, .. }): State<AppState>, Json(req): Json<arawn_workfl...` — HTTP endpoint for workflow decision tasks.
-  `WsQueryParams` struct L253-255 — `{ token: Option<String> }` — Query parameters for WebSocket connection.
-  `ws_handler` function L257-283 — `( ws: WebSocketUpgrade, Query(params): Query<WsQueryParams>, State(state): State...`
-  `handle_connection` function L290-1175 — `(socket: WebSocket, service: Arc<LocalService>)`
-  `tests` module L1178-1228 — `-`
-  `from_service_error_preserves_structured_detail_for_typed_variants` function L1185-1195 — `()` — Typed Storage error should round-trip through the wire payload with
-  `from_service_error_omits_details_for_string_only_variants` function L1201-1212 — `()` — String-only variants (NotFound, InvalidOperation, Internal) keep
-  `from_service_error_preserves_engine_error_kind` function L1218-1227 — `()` — Engine errors surface a `kind` that identifies the inner variant —

### crates/arawn-auth/src

**Role**: Provider-agnostic OAuth2 PKCE flow and encrypted on-disk token persistence for authenticating Arawn against external services.

**Key abstractions**:
- `OAuthClient` — Drives the browser-based PKCE authorization flow. `start_flow()` generates a PKCE verifier+challenge, CSRF state, and the authorization URL the caller must open. `exchange_code()` POSTs the code to the token endpoint. `refresh()` uses a refresh token to mint a new access token. Uses `sha2` + `base64` for the challenge and `reqwest` for HTTP.
- `Token` — The credential stored per provider: access token, optional refresh token, optional expiry, and scopes. `is_expired()` checks the clock against `expires_at`.
- `CallbackServer` — A one-shot HTTP listener on a random port that waits for the OAuth redirect. `listen_with_timeout()` accepts one connection, parses `?code=&state=`, serves an HTML success page, and returns. Shuts itself down after the first redirect.
- `TokenStore` — Encrypts tokens with `ChaCha20Poly1305` and persists them under `{data_dir}/tokens/{provider}.enc`. The master key is stored in `tokens/master.key` at mode 600. `open()` creates or reads the master key; `save`/`load`/`delete` handle individual provider tokens.

**Internal flow**: A caller calls `OAuthClient::start_flow`, opens the authorization URL in a browser, binds a `CallbackServer`, waits for the redirect via `listen()`, then calls `exchange_code` with the returned code. The resulting `Token` is persisted via `TokenStore::save`.

**Dependencies**: `reqwest` (HTTP), `sha2` + `base64ct` (PKCE challenge), `chacha20poly1305` (token encryption), `tokio` (async runtime for the HTTP stub in tests).

#### crates/arawn-auth/src/error.rs

- pub `AuthError` enum L5-26 — `AuthExpired | ApiError | Network | InvalidConfig | Decode` — Errors raised by the auth primitives.

#### crates/arawn-auth/src/lib.rs

- pub `error` module L12 — `-` — Provides a provider-agnostic OAuth2 client (`OAuthClient`), a local
- pub `oauth2` module L13 — `-` — nothing else.
- pub `server` module L14 — `-` — nothing else.
- pub `token_store` module L15 — `-` — nothing else.

#### crates/arawn-auth/src/oauth2.rs

- pub `OAuthProviderConfig` struct L22-38 — `{ auth_url: Url, token_url: Url, client_id: String, client_secret: String, scope...` — Static configuration for an OAuth2 provider — not the user's credentials.
- pub `Token` struct L42-54 — `{ access: String, refresh: Option<String>, expires_at: Option<DateTime<Utc>>, sc...` — A user's OAuth credential — what `TokenStore` persists.
- pub `is_expired` function L61-66 — `(&self) -> bool` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.
- pub `AuthRequest` struct L71-78 — `{ authorization_url: Url, csrf_state: String, pkce_verifier: String }` — What `OAuthClient::start_flow` hands back.
- pub `OAuthClient` struct L80-83 — `{ config: OAuthProviderConfig, http: reqwest::Client }` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.
- pub `new` function L86-94 — `(config: OAuthProviderConfig) -> Self` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.
- pub `with_http` function L96-98 — `(config: OAuthProviderConfig, http: reqwest::Client) -> Self` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.
- pub `start_flow` function L106-137 — `(&self, redirect_uri: &Url) -> AuthRequest` — Generate a PKCE verifier + challenge + CSRF state and build the
- pub `exchange_code` function L140-157 — `( &self, code: &str, redirect_uri: &Url, pkce_verifier: &str, ) -> Result<Token,...` — Exchange an authorization code for a [`Token`].
- pub `refresh` function L160-183 — `(&self, refresh_token: &str) -> Result<Token, AuthError>` — Use a refresh token to mint a new access token.
-  `default_token_type` function L56-58 — `() -> String` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.
-  `Token` type L60-67 — `= Token` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.
-  `OAuthClient` type L85-220 — `= OAuthClient` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.
-  `post_token` function L185-219 — `(&self, form: &[(&str, &str)]) -> Result<Token, AuthError>` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.
-  `TokenResponse` struct L223-237 — `{ access_token: String, refresh_token: Option<String>, expires_in: Option<u64>, ...` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.
-  `generate_pkce_verifier` function L244-251 — `() -> String` — 64-character URL-safe random string.
-  `pkce_challenge_s256` function L253-256 — `(verifier: &str) -> String` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.
-  `generate_state` function L258-265 — `() -> String` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.
-  `tests` module L268-447 — `-` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.
-  `pkce_challenge_matches_rfc_7636_example` function L272-277 — `()` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.
-  `pkce_verifier_length` function L280-284 — `()` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.
-  `state_length` function L287-290 — `()` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.
-  `start_flow_includes_required_params` function L293-313 — `()` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.
-  `spawn_token_stub` function L318-362 — `( status: u16, body: &'static str, ) -> (Url, tokio::task::JoinHandle<Vec<u8>>)` — Tiny in-process HTTP stub for the OAuth token endpoint.
-  `client_with_token_url` function L364-373 — `(token_url: Url) -> OAuthClient` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.
-  `exchange_code_decodes_token_response` function L376-392 — `()` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.
-  `refresh_failure_with_400_returns_auth_expired` function L395-402 — `()` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.
-  `refresh_preserves_refresh_token_when_provider_omits_it` function L405-413 — `()` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.
-  `token_is_expired_respects_expiration_time` function L416-446 — `()` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.

#### crates/arawn-auth/src/server.rs

- pub `CallbackResult` struct L26-29 — `{ code: String, state: String }` — What the callback yielded.
- pub `CallbackServer` struct L31-34 — `{ listener: TcpListener, redirect_uri: Url }` — responds with a small HTML success page, then shuts down.
- pub `bind` function L39-41 — `(path: &str) -> Result<Self, AuthError>` — Bind to an OS-assigned port on `127.0.0.1`.
- pub `bind_with_port` function L46-48 — `(path: &str, port: u16) -> Result<Self, AuthError>` — Bind to a specific port on `127.0.0.1`.
- pub `redirect_uri` function L70-72 — `(&self) -> &Url` — responds with a small HTML success page, then shuts down.
- pub `listen` function L76-78 — `(self) -> Result<CallbackResult, AuthError>` — Wait up to [`DEFAULT_TIMEOUT`] for a single redirect, parse it, and
- pub `listen_with_timeout` function L80-176 — `( self, timeout: Duration, ) -> Result<CallbackResult, AuthError>` — responds with a small HTML success page, then shuts down.
-  `DEFAULT_TIMEOUT` variable L20 — `: Duration` — responds with a small HTML success page, then shuts down.
-  `SUCCESS_PAGE` variable L22 — `: &str` — responds with a small HTML success page, then shuts down.
-  `CallbackServer` type L36-177 — `= CallbackServer` — responds with a small HTML success page, then shuts down.
-  `bind_inner` function L50-68 — `(path: &str, port: u16) -> Result<Self, AuthError>` — responds with a small HTML success page, then shuts down.
-  `tests` module L180-249 — `-` — responds with a small HTML success page, then shuts down.
-  `simulate_browser` function L185-197 — `(server_url: &Url, query: &str)` — responds with a small HTML success page, then shuts down.
-  `happy_path_returns_code_and_state` function L200-208 — `()` — responds with a small HTML success page, then shuts down.
-  `missing_code_yields_invalid_config_error` function L211-221 — `()` — responds with a small HTML success page, then shuts down.
-  `provider_error_propagates` function L224-234 — `()` — responds with a small HTML success page, then shuts down.
-  `timeout_returns_error` function L237-241 — `()` — responds with a small HTML success page, then shuts down.
-  `redirect_uri_normalizes_path_with_or_without_slash` function L244-248 — `()` — responds with a small HTML success page, then shuts down.

#### crates/arawn-auth/src/token_store.rs

- pub `TokenStore` struct L30-33 — `{ tokens_dir: PathBuf, cipher: ChaCha20Poly1305 }` — System spec's security contract and the sensitive-paths deny list.
- pub `open` function L38-64 — `(data_dir: &Path) -> Result<Self, AuthError>` — Open or initialise the token store under `{data_dir}/tokens/`.
- pub `save` function L67-93 — `(&self, provider: &str, token: &Token) -> Result<(), AuthError>` — Persist `token` for the named `provider`.
- pub `load` function L96-124 — `(&self, provider: &str) -> Result<Option<Token>, AuthError>` — Load the token for `provider`, returning `Ok(None)` when absent.
- pub `delete` function L126-135 — `(&self, provider: &str) -> Result<(), AuthError>` — System spec's security contract and the sensitive-paths deny list.
- pub `tokens_dir` function L137-139 — `(&self) -> &Path` — System spec's security contract and the sensitive-paths deny list.
-  `KEY_LEN` variable L26 — `: usize` — System spec's security contract and the sensitive-paths deny list.
-  `NONCE_LEN` variable L27 — `: usize` — System spec's security contract and the sensitive-paths deny list.
-  `KEY_FILENAME` variable L28 — `: &str` — System spec's security contract and the sensitive-paths deny list.
-  `TokenStore` type L35-182 — `= TokenStore` — System spec's security contract and the sensitive-paths deny list.
-  `path_for` function L141-148 — `(&self, provider: &str) -> PathBuf` — System spec's security contract and the sensitive-paths deny list.
-  `write_key` function L150-155 — `(path: &Path, bytes: &[u8]) -> Result<(), AuthError>` — System spec's security contract and the sensitive-paths deny list.
-  `set_file_mode` function L158-164 — `(path: &Path, mode: u32) -> Result<(), AuthError>` — System spec's security contract and the sensitive-paths deny list.
-  `set_file_mode` function L167-171 — `(_path: &Path, _mode: u32) -> Result<(), AuthError>` — System spec's security contract and the sensitive-paths deny list.
-  `set_dir_mode` function L174-176 — `(path: &Path) -> Result<(), AuthError>` — System spec's security contract and the sensitive-paths deny list.
-  `set_dir_mode` function L179-181 — `(_path: &Path) -> Result<(), AuthError>` — System spec's security contract and the sensitive-paths deny list.
-  `tests` module L185-302 — `-` — System spec's security contract and the sensitive-paths deny list.
-  `sample_token` function L190-199 — `() -> Token` — System spec's security contract and the sensitive-paths deny list.
-  `save_then_load_round_trip` function L202-210 — `()` — System spec's security contract and the sensitive-paths deny list.
-  `load_missing_returns_none` function L213-217 — `()` — System spec's security contract and the sensitive-paths deny list.
-  `delete_then_load_returns_none` function L220-226 — `()` — System spec's security contract and the sensitive-paths deny list.
-  `delete_nonexistent_is_idempotent` function L229-233 — `()` — System spec's security contract and the sensitive-paths deny list.
-  `tampered_ciphertext_fails_decrypt` function L236-251 — `()` — System spec's security contract and the sensitive-paths deny list.
-  `second_open_reuses_master_key` function L254-263 — `()` — System spec's security contract and the sensitive-paths deny list.
-  `missing_master_key_after_save_fails_clearly` function L266-280 — `()` — System spec's security contract and the sensitive-paths deny list.
-  `provider_name_sanitization_rejects_path_chars` function L283-290 — `()` — System spec's security contract and the sensitive-paths deny list.
-  `master_key_has_restrictive_permissions` function L294-301 — `()` — System spec's security contract and the sensitive-paths deny list.

### crates/arawn-core/src

**Role**: Foundational domain types shared across all crates — the conversation model, session lifecycle, workstream concept, and session statistics. Has no upstream arawn dependencies.

**Key abstractions**:
- `Session` — Owns the in-memory message history for one conversation. Created via `new(workstream_id)` or `scratch()` (unbound). `promote()` binds a scratch session to a workstream (panics if already bound). `compact()` replaces old messages with a `Message::Summary`, keeping the last N verbatim — this is the LLM-backed compaction path. `microcompact()` is a cheaper in-process pass that stubs out large tool results from non-recent turns without an LLM call.
- `Message` — Four variants: `User` (text), `Assistant` (text + optional tool_use list), `ToolResult` (content + error flag), `Summary` (replaces compacted history). The `Summary` variant is the sentinel that `load_compacted()` uses to discard messages before it on resume.
- `Workstream` — Represents a named project directory binding: `id`, `name`, `root_dir`, `created_at`. `scratch()` creates the default scratch workstream.
- `SessionStats` — Accumulates `input_tokens`, `output_tokens`, `turns`, and `tool_calls` across a session. `record_turn()` adds one LLM call's usage.

**Internal flow**: The engine appends messages to an in-memory `Session` during each loop iteration. After each turn the storage layer appends the new messages to JSONL. On resume, the storage layer calls `Session::load_compacted()` which skips messages before any `Summary` marker.

**Mixed concerns / gotchas**: `microcompact` only stubs results from a hardcoded `TARGETED_TOOLS` list (large-output tools like file_read, shell, grep) and only if they exceed `STUB_THRESHOLD` bytes. Error results are always preserved verbatim regardless of size.

#### crates/arawn-core/src/error.rs

- pub `CoreError` enum L4-10 — `Workstream | Session`

#### crates/arawn-core/src/lib.rs

- pub `error` module L1 — `-`
- pub `message` module L2 — `-`
- pub `session` module L3 — `-`
- pub `session_stats` module L4 — `-`
- pub `workstream` module L5 — `-`

#### crates/arawn-core/src/message.rs

- pub `ToolUse` struct L6-10 — `{ id: String, name: String, input: Value }` — A tool invocation requested by the assistant.
- pub `Message` enum L15-42 — `User | Assistant | ToolResult | Summary` — A message in a conversation session.
-  `tests` module L45-130 — `-`
-  `user_message_serialization_roundtrip` function L50-60 — `()`
-  `assistant_message_with_tool_uses` function L63-82 — `()`
-  `assistant_message_without_tool_uses_omits_field` function L85-92 — `()`
-  `tool_result_message_roundtrip` function L95-114 — `()`
-  `tool_result_error_flag` function L117-129 — `()`

#### crates/arawn-core/src/session.rs

- pub `Session` struct L12-22 — `{ id: Uuid, workstream_id: Option<Uuid>, workstream_name: String, messages: Vec<...` — A conversation session.
- pub `new` function L26-35 — `(workstream_id: Uuid) -> Self` — Create a session bound to a workstream.
- pub `new_with_workstream` function L39-43 — `(workstream_id: Uuid, workstream_name: impl Into<String>) -> Self` — Create a session bound to a workstream by name.
- pub `from_parts` function L46-60 — `( id: Uuid, workstream_id: Option<Uuid>, created_at: DateTime<Utc>, messages: Ve...` — Reconstruct a session from persisted parts (DB load path).
- pub `from_parts_with_stats` function L63-78 — `( id: Uuid, workstream_id: Option<Uuid>, created_at: DateTime<Utc>, messages: Ve...` — Reconstruct a session with stats from persisted parts.
- pub `scratch` function L81-90 — `() -> Self` — Create a scratch session (no workstream binding yet).
- pub `workstream_id` function L92-94 — `(&self) -> Option<Uuid>`
- pub `workstream_name` function L98-100 — `(&self) -> &str` — Current workstream slug for this session.
- pub `set_workstream` function L105-108 — `(&mut self, name: impl Into<String>, id: Uuid)` — Update the active workstream binding.
- pub `is_scratch` function L111-113 — `(&self) -> bool` — Returns true if this is a scratch session (not yet promoted).
- pub `promote` function L116-123 — `(&mut self, workstream_id: Uuid)` — Promote a scratch session to a workstream.
- pub `add_message` function L125-127 — `(&mut self, msg: Message)`
- pub `messages` function L129-131 — `(&self) -> &[Message]`
- pub `microcompact` function L137-201 — `(&mut self, keep_recent: usize) -> usize` — Clear old tool results to save context space without an LLM call.
- pub `compact` function L205-238 — `(&mut self, summary_content: String, keep_recent: usize) -> usize` — Replace old messages with a Summary, keeping the last `keep_recent` messages verbatim.
- pub `load_compacted` function L242-252 — `(messages: Vec<Message>) -> Vec<Message>` — Load messages with compaction awareness — if a Summary exists, use the
-  `Session` type L24-253 — `= Session`
-  `TARGETED_TOOLS` variable L138-144 — `: &[&str]`
-  `STUB_THRESHOLD` variable L145 — `: usize`
-  `tests` module L256-562 — `-`
-  `session_bound_to_workstream` function L262-267 — `()`
-  `scratch_session_has_no_workstream` function L270-274 — `()`
-  `promote_scratch_session` function L277-283 — `()`
-  `promote_already_bound_panics` function L287-290 — `()`
-  `session_starts_with_no_messages` function L293-296 — `()`
-  `session_message_ordering_preserved` function L299-326 — `()`
-  `session_ids_are_unique` function L329-334 — `()`
-  `compact_replaces_old_with_summary` function L337-366 — `()`
-  `compact_too_few_messages_noop` function L369-381 — `()`
-  `load_compacted_skips_before_summary` function L384-409 — `()`
-  `load_compacted_no_summary_returns_all` function L412-424 — `()`
-  `microcompact_clears_old_tool_results` function L427-467 — `()`
-  `microcompact_preserves_recent_results` function L470-489 — `()`
-  `microcompact_skips_small_results` function L492-513 — `()`
-  `microcompact_skips_errors` function L516-537 — `()`
-  `microcompact_skips_non_targeted_tools` function L540-561 — `()`

#### crates/arawn-core/src/session_stats.rs

- pub `SessionStats` struct L5-10 — `{ input_tokens: u64, output_tokens: u64, turns: u32, tool_calls: u32 }` — Accumulated token usage and activity stats for a session.
- pub `new` function L13-15 — `() -> Self`
- pub `record_turn` function L18-23 — `(&mut self, input_tokens: u32, output_tokens: u32, tool_call_count: u32)` — Record usage from a single LLM call.
- pub `total_tokens` function L26-28 — `(&self) -> u64` — Total tokens (input + output).
- pub `estimated_cost_usd` function L31-35 — `(&self, cost_per_1k_input: f64, cost_per_1k_output: f64) -> f64` — Estimate cost in USD given per-1k-token rates.
-  `SessionStats` type L12-36 — `= SessionStats`
-  `tests` module L39-82 — `-`
-  `default_stats_are_zero` function L43-49 — `()`
-  `record_turn_accumulates` function L52-62 — `()`
-  `cost_calculation` function L65-74 — `()`
-  `zero_rates_zero_cost` function L77-81 — `()`

#### crates/arawn-core/src/workstream.rs

- pub `SCRATCH_NAME` variable L14 — `: &str` — Reserved workstream slug — auto-created on first boot and undeletable.
- pub `validate_name` function L18-36 — `(name: &str) -> Result<(), WorkstreamNameError>` — Validation for workstream slugs.
- pub `WorkstreamNameError` enum L39-44 — `Empty | TooLong | BadLeading | BadChar` — feeds extractor prompts in Phase 4.
- pub `Workstream` struct L66-90 — `{ id: Uuid, name: String, display_name: String, description: String, root_dir: P...` — A workstream — the primary organizational unit.
- pub `new` function L93-107 — `(name: impl Into<String>, root_dir: impl Into<PathBuf>) -> Self` — feeds extractor prompts in Phase 4.
- pub `scratch` function L110-112 — `(root_dir: impl Into<PathBuf>) -> Self` — Create the default scratch workstream for ad-hoc sessions.
- pub `is_scratch` function L114-116 — `(&self) -> bool` — feeds extractor prompts in Phase 4.
-  `WorkstreamNameError` type L46-60 — `= WorkstreamNameError` — feeds extractor prompts in Phase 4.
-  `fmt` function L47-59 — `(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` — feeds extractor prompts in Phase 4.
-  `WorkstreamNameError` type L62 — `= WorkstreamNameError` — feeds extractor prompts in Phase 4.
-  `Workstream` type L92-117 — `= Workstream` — feeds extractor prompts in Phase 4.
-  `tests` module L120-164 — `-` — feeds extractor prompts in Phase 4.
-  `workstream_creation_uses_name_as_display_by_default` function L124-131 — `()` — feeds extractor prompts in Phase 4.
-  `scratch_workstream` function L134-138 — `()` — feeds extractor prompts in Phase 4.
-  `workstream_ids_are_unique` function L141-145 — `()` — feeds extractor prompts in Phase 4.
-  `name_validation_accepts_valid_slugs` function L148-152 — `()` — feeds extractor prompts in Phase 4.
-  `name_validation_rejects_invalid_slugs` function L155-163 — `()` — feeds extractor prompts in Phase 4.

### crates/arawn-embed/src

**Role**: Text-to-vector embedding with two interchangeable backends: a local ONNX model (no external service needed) and an OpenAI-compatible HTTP API.

**Key abstractions**:
- `Embedder` trait — Single contract: `embed(text) -> Vec<f32>`, `embed_batch(texts) -> Vec<Vec<f32>>`, `dimensions() -> usize`. The default `embed_batch` implementation calls `embed` in a loop; both backends override it for efficiency.
- `LocalEmbedder` — Loads an ONNX sentence-transformer model from `~/.arawn/models/` (downloading from HuggingFace on first use). Runs inference synchronously inside `Mutex<Session>` (hence the manual `Send`+`Sync` impl), processing up to `CHUNK_SIZE` texts per ONNX call. Tokenizes with the HF `tokenizers` crate, truncates at `MAX_TOKENS`.
- `ApiEmbedder` — POSTs to any OpenAI-compatible `/v1/embeddings` endpoint. Default base URL targets OpenAI. Batches all texts in a single request.
- `create_embedder(config)` — Factory function: reads `provider` field from `EmbeddingConfig` and creates the appropriate backend. Used at startup; the result is wrapped in `Arc<dyn Embedder>` and passed to `MemoryManager`.

**Dependencies**: `ort` (ONNX Runtime), `tokenizers` (HuggingFace tokenization), `reqwest` (API backend), `serde`/`serde_json`.

#### crates/arawn-embed/src/api.rs

- pub `ApiEmbedder` struct L14-20 — `{ client: reqwest::Client, model: String, dimensions: usize, api_key: String, ba...` — Embedder that calls an OpenAI-compatible embedding API.
- pub `new` function L23-47 — `(config: &EmbeddingConfig) -> Result<Self, EmbedError>` — API-based embedder using OpenAI-compatible embedding endpoints.
-  `DEFAULT_API_BASE` variable L11 — `: &str` — API-based embedder using OpenAI-compatible embedding endpoints.
-  `ApiEmbedder` type L22-48 — `= ApiEmbedder` — API-based embedder using OpenAI-compatible embedding endpoints.
-  `EmbeddingRequest` struct L51-54 — `{ model: String, input: Vec<String> }` — API-based embedder using OpenAI-compatible embedding endpoints.
-  `EmbeddingResponse` struct L57-59 — `{ data: Vec<EmbeddingData> }` — API-based embedder using OpenAI-compatible embedding endpoints.
-  `EmbeddingData` struct L62-64 — `{ embedding: Vec<f32> }` — API-based embedder using OpenAI-compatible embedding endpoints.
-  `ApiEmbedder` type L67-137 — `impl Embedder for ApiEmbedder` — API-based embedder using OpenAI-compatible embedding endpoints.
-  `embed` function L68-74 — `(&self, text: &str) -> Result<Vec<f32>, EmbedError>` — API-based embedder using OpenAI-compatible embedding endpoints.
-  `embed_batch` function L76-132 — `(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedError>` — API-based embedder using OpenAI-compatible embedding endpoints.
-  `dimensions` function L134-136 — `(&self) -> usize` — API-based embedder using OpenAI-compatible embedding endpoints.
-  `tests` module L140-156 — `-` — API-based embedder using OpenAI-compatible embedding endpoints.
-  `api_embedder_requires_key` function L144-155 — `()` — API-based embedder using OpenAI-compatible embedding endpoints.

#### crates/arawn-embed/src/config.rs

- pub `EmbeddingConfig` struct L6-31 — `{ provider: String, model: String, dimensions: usize, api_key_env: Option<String...` — Configuration for the embedding provider.
-  `EmbeddingConfig` type L33-44 — `impl Default for EmbeddingConfig`
-  `default` function L34-43 — `() -> Self`
-  `default_provider` function L46-48 — `() -> String`
-  `default_model` function L50-52 — `() -> String`
-  `default_dimensions` function L54-56 — `() -> usize`
-  `tests` module L59-105 — `-`
-  `default_config` function L63-69 — `()`
-  `deserialize_local` function L72-81 — `()`
-  `deserialize_api` function L84-96 — `()`
-  `deserialize_minimal` function L99-104 — `()`

#### crates/arawn-embed/src/error.rs

- pub `EmbedError` enum L4-19 — `Config | ModelLoad | Inference | Api | Tokenization`

#### crates/arawn-embed/src/lib.rs

- pub `Embedder` interface L26-42 — `{ fn embed(), fn embed_batch(), fn dimensions() }` — Trait for embedding text into dense vectors.
- pub `create_embedder` function L46-60 — `(config: &EmbeddingConfig) -> Result<Arc<dyn Embedder>, EmbedError>` — Create an embedder from configuration.
-  `api` module L9 — `-` — Provides a trait-based embedding system with two backends:
-  `config` module L10 — `-` — Configuration lives in `arawn.toml` under `[embeddings]`.
-  `error` module L11 — `-` — Configuration lives in `arawn.toml` under `[embeddings]`.
-  `local` module L12 — `-` — Configuration lives in `arawn.toml` under `[embeddings]`.
-  `embed_batch` function L32-38 — `(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedError>` — Embed multiple texts in a batch.

#### crates/arawn-embed/src/local.rs

- pub `LocalEmbedder` struct L26-30 — `{ session: Mutex<Session>, tokenizer: tokenizers::Tokenizer, dimensions: usize }` — Local ONNX-based embedder.
- pub `new` function L37-70 — `(config: &EmbeddingConfig) -> Result<Self, EmbedError>` — Model files are downloaded to ~/.arawn/models/ on first use.
-  `MAX_TOKENS` variable L19 — `: usize` — Model files are downloaded to ~/.arawn/models/ on first use.
-  `HF_REPO_BASE` variable L22 — `: &str` — HuggingFace repo base for downloading model files.
-  `LocalEmbedder` type L33 — `impl Send for LocalEmbedder` — Model files are downloaded to ~/.arawn/models/ on first use.
-  `LocalEmbedder` type L34 — `impl Sync for LocalEmbedder` — Model files are downloaded to ~/.arawn/models/ on first use.
-  `LocalEmbedder` type L36-175 — `= LocalEmbedder` — Model files are downloaded to ~/.arawn/models/ on first use.
-  `run_batch` function L73-174 — `(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedError>` — Run inference on a batch of texts.
-  `LocalEmbedder` type L178-199 — `impl Embedder for LocalEmbedder` — Model files are downloaded to ~/.arawn/models/ on first use.
-  `embed` function L179-185 — `(&self, text: &str) -> Result<Vec<f32>, EmbedError>` — Model files are downloaded to ~/.arawn/models/ on first use.
-  `embed_batch` function L187-194 — `(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedError>` — Model files are downloaded to ~/.arawn/models/ on first use.
-  `CHUNK_SIZE` variable L188 — `: usize` — Model files are downloaded to ~/.arawn/models/ on first use.
-  `dimensions` function L196-198 — `(&self) -> usize` — Model files are downloaded to ~/.arawn/models/ on first use.
-  `resolve_model_dir` function L201-214 — `(config: &EmbeddingConfig) -> Result<PathBuf, EmbedError>` — Model files are downloaded to ~/.arawn/models/ on first use.
-  `download_model_files` function L216-248 — `(model_dir: &Path, model_name: &str) -> Result<(), EmbedError>` — Model files are downloaded to ~/.arawn/models/ on first use.
-  `tests` module L251-270 — `-` — Model files are downloaded to ~/.arawn/models/ on first use.
-  `resolve_default_dir` function L255-259 — `()` — Model files are downloaded to ~/.arawn/models/ on first use.
-  `resolve_custom_dir` function L262-269 — `()` — Model files are downloaded to ~/.arawn/models/ on first use.

### crates/arawn-engine/src

**Role**: The agentic loop and all subsystems the loop depends on: tool execution, permission checking, context compaction, hooks, skills, plugins, plan mode, and the system prompt builder.

**Key abstractions**:
- `QueryEngine` — The core agent loop. `run()` builds the request, streams the LLM response, collects tool calls, executes each (in parallel for independent calls), feeds results back, and repeats until the LLM produces a text-only response or `max_iterations` is hit. Checks the `CancellationToken` before each iteration and before each tool call. Fires `PreToolUse`/`PostToolUse` hooks and calls `PermissionChecker` on each tool invocation. `stream_response_with_retry` handles mid-stream failures (distinct from `RetryClient` which handles connection-time failures).
- `Compactor` — Decides whether to compact (`should_compact`) based on estimated token count vs. model limits, then summarizes old messages via an LLM call using the `compact_prompt` templates, and calls `Session::compact()`. Called at the start of each iteration inside `QueryEngine::run`.
- `EngineToolContext` — Implements `ToolContext` with session-scoped state: validated working dir, allowed paths, read-file tracking (required before `file_edit`/`file_write`), sub-agent depth counter, and an `Option<Arc<LlmResolverFn>>` closure for LLM preference resolution. `for_sub_agent()` clones the context with depth+1 for sub-agent spawning.
- `SystemPromptBuilder` — Assembles the system prompt from 7 static sections (identity, system, doing_tasks, work_protocol, actions, using_tools, tone) each overridable from a user's `prompts/` directory, plus dynamic sections (environment, workstream, context files, memories, plugin prompts, tool list). Token budget enforcement drops low-priority sections first. Rebuilt each turn to stay fresh.
- `PlanModeState` — Guards plan mode: `enter()` saves the pre-plan `PermissionMode` and creates a plan file with a slug, `exit()` restores the mode and returns the prior mode. The plan file path is exposed so `EngineToolContext::validate_path` can allow writes to it while blocking all other writes in plan mode.
- `AgentTool` — Spawns a sub-agent by creating a new `QueryEngine` with a filtered `ToolRegistry`, a fresh `Session`, and an incremented agent depth. Supports `run_in_background` which hands the agent off to `BackgroundTaskManager`.
- `BackgroundTaskManager` — Tracks running `JoinHandle`s keyed by `bg_XXXXXXXX` IDs. Completed tasks queue `TaskNotification` messages that the engine drains and injects into the next LLM request.
- `filter_tools_for_context` — Decides which tool definitions to include in each turn's request based on session state (plan mode active, has background tasks, etc.) and `ToolCategory`. Core tools are always included; category-specific tools are added based on context signals.

**Internal flow**: `QueryEngine::run` loop: 1) drain background task notifications, 2) `should_compact` → compact if needed, 3) `build_request` (system prompt + messages + filtered tools), 4) `stream_response_with_retry` → `AssembledResponse`, 5) for each tool call: fire `PreToolUse` hook → `PermissionChecker::check` → `execute_tool` → fire `PostToolUse` hook, 6) push assistant message + tool results to session, 7) persist. Loop until text-only response.

**Mixed concerns / gotchas**: `stream_response_with_retry` retries the entire request-build-and-stream cycle (for mid-stream SSE errors), while the `RetryClient` wrapper on the `LlmClient` only retries at connection time. Both are needed. `token_estimator` uses a chars/4 heuristic — not exact but fast enough for compaction threshold decisions.

**Dependencies**: `arawn-tool` (Tool trait, ToolRegistry), `arawn-llm` (LlmClient, ChatRequest), `arawn-core` (Session, Message), `arawn-memory`, `arawn-embed`; `tokio` for async, `globwalk`/`ignore` for file tools, `sandbox` crate for shell sandboxing.

#### crates/arawn-engine/src/agent_defs.rs

- pub `AgentDefinition` struct L10-27 — `{ name: String, when_to_use: String, system_prompt: String, tools: Option<Vec<St...` — An agent definition — controls system prompt, tool access, and behavior.
- pub `AgentSource` enum L30-33 — `BuiltIn | User`
- pub `built_in_agents` function L36-130 — `() -> Vec<AgentDefinition>` — Returns the built-in agent definitions.
- pub `load_agents_dir` function L143-169 — `(dir: &Path) -> Vec<AgentDefinition>` — Load agent definitions from markdown files in a directory.
- pub `get_all_agents` function L249-265 — `(agents_dir: Option<&Path>) -> Vec<AgentDefinition>` — Get all agent definitions: built-in + user-defined from a directory.
- pub `find_agent` function L268-280 — `(agents: &[AgentDefinition], name: &str) -> AgentDefinition` — Look up an agent definition by name.
- pub `build_agent_registry` function L283-324 — `( parent_registry: &ToolRegistry, definition: &AgentDefinition, ) -> Arc<ToolReg...` — Build a filtered ToolRegistry based on an agent definition's tool constraints.
-  `parse_agent_markdown` function L171-202 — `(path: &Path) -> Result<AgentDefinition, String>`
-  `split_frontmatter` function L204-216 — `(content: &str) -> Option<(String, String)>`
-  `extract_field` function L218-236 — `(frontmatter: &str, key: &str) -> Option<String>`
-  `parse_list` function L238-246 — `(s: &str) -> Vec<String>`
-  `tests` module L327-496 — `-`
-  `built_in_agents_exist` function L332-338 — `()`
-  `find_agent_by_name` function L341-345 — `()`
-  `find_agent_case_insensitive` function L348-352 — `()`
-  `find_agent_unknown_falls_back` function L355-359 — `()`
-  `parse_agent_markdown_file` function L362-398 — `()`
-  `parse_agent_with_disallowed_tools` function L401-423 — `()`
-  `user_agents_override_builtin` function L426-444 — `()`
-  `load_empty_dir` function L447-451 — `()`
-  `load_nonexistent_dir` function L454-457 — `()`
-  `split_frontmatter_works` function L460-464 — `()`
-  `split_frontmatter_no_delimiters` function L467-469 — `()`
-  `extract_field_quoted` function L472-477 — `()`
-  `extract_field_unquoted` function L480-482 — `()`
-  `parse_list_wildcard` function L485-487 — `()`
-  `parse_list_comma_separated` function L490-495 — `()`

#### crates/arawn-engine/src/background.rs

- pub `TaskNotification` struct L48-53 — `{ task_id: String, description: String, status: String, summary: String }` — A notification about a completed background task, ready for injection
- pub `to_message` function L57-66 — `(&self) -> String` — Format as the XML structure the LLM expects.
- pub `BackgroundTaskKind` enum L71-74 — `Shell | Agent` — What kind of background task this is.
- pub `BackgroundTaskStatus` enum L78-83 — `Running | Completed | Failed | Killed` — Current status of a background task.
- pub `is_terminal` function L86-88 — `(&self) -> bool` — conversation so the LLM knows what finished.
- pub `label` function L90-97 — `(&self) -> &str` — conversation so the LLM knows what finished.
- pub `BackgroundTask` struct L101-119 — `{ id: String, kind: BackgroundTaskKind, description: String, status: BackgroundT...` — A single background task being tracked.
- pub `read_output` function L134-136 — `(&self) -> String` — Read the current output buffer.
- pub `output_handle` function L139-141 — `(&self) -> Arc<RwLock<String>>` — Get a shared handle to the output buffer (for the writer task).
- pub `append_output` function L146-156 — `(buf: &Arc<RwLock<String>>, text: &str)` — Append text to a bounded output buffer.
- pub `BackgroundTaskManager` struct L159-163 — `{ tasks: RwLock<HashMap<String, BackgroundTask>>, notifications: Mutex<Vec<TaskN...` — Session-scoped manager for background tasks.
- pub `new` function L166-171 — `() -> Self` — conversation so the LLM knows what finished.
- pub `register` function L175-201 — `( &self, kind: BackgroundTaskKind, description: String, handle: JoinHandle<()>, ...` — Register a new background task.
- pub `complete` function L204-245 — `(&self, task_id: &str, status: BackgroundTaskStatus)` — Mark a task as completed and queue a notification.
- pub `drain_notifications` function L248-251 — `(&self) -> Vec<TaskNotification>` — Drain all pending notifications (called by the engine at each iteration).
- pub `status` function L254-256 — `(&self, task_id: &str) -> Option<BackgroundTaskStatus>` — Get a task's current status.
- pub `read_output` function L259-261 — `(&self, task_id: &str) -> Option<String>` — Read a task's captured output.
- pub `cancel` function L264-273 — `(&self, task_id: &str) -> bool` — Cancel a running task.
- pub `list` function L276-288 — `(&self) -> Vec<TaskSummary>` — List all tasks (for inventory/status display).
- pub `running_count` function L291-298 — `(&self) -> usize` — Number of currently running tasks.
- pub `TaskSummary` struct L309-314 — `{ id: String, description: String, status: String, elapsed_secs: u64 }` — Lightweight summary for listing/display.
-  `MAX_OUTPUT_BYTES` variable L18 — `: usize` — Maximum output buffer size per task (100 KB).
-  `generate_task_id` function L21-30 — `() -> String` — Generates a background task ID: "bg_" + 8 hex chars.
-  `rand_bytes` function L32-43 — `() -> [u8; 4]` — conversation so the LLM knows what finished.
-  `TaskNotification` type L55-67 — `= TaskNotification` — conversation so the LLM knows what finished.
-  `BackgroundTaskStatus` type L85-98 — `= BackgroundTaskStatus` — conversation so the LLM knows what finished.
-  `BackgroundTask` type L121-130 — `= BackgroundTask` — conversation so the LLM knows what finished.
-  `fmt` function L122-129 — `(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` — conversation so the LLM knows what finished.
-  `BackgroundTask` type L132-142 — `= BackgroundTask` — conversation so the LLM knows what finished.
-  `BackgroundTaskManager` type L165-299 — `= BackgroundTaskManager` — conversation so the LLM knows what finished.
-  `BackgroundTaskManager` type L301-305 — `impl Default for BackgroundTaskManager` — conversation so the LLM knows what finished.
-  `default` function L302-304 — `() -> Self` — conversation so the LLM knows what finished.
-  `tests` module L317-501 — `-` — conversation so the LLM knows what finished.
-  `generate_task_id_format` function L322-326 — `()` — conversation so the LLM knows what finished.
-  `task_status_labels` function L329-343 — `()` — conversation so the LLM knows what finished.
-  `task_status_is_terminal` function L346-351 — `()` — conversation so the LLM knows what finished.
-  `notification_to_message_format` function L354-364 — `()` — conversation so the LLM knows what finished.
-  `register_and_complete` function L367-399 — `()` — conversation so the LLM knows what finished.
-  `cancel_running_task` function L402-422 — `()` — conversation so the LLM knows what finished.
-  `output_buffer_bounded` function L425-434 — `()` — conversation so the LLM knows what finished.
-  `output_buffer_small_writes` function L437-443 — `()` — conversation so the LLM knows what finished.
-  `list_tasks` function L446-465 — `()` — conversation so the LLM knows what finished.
-  `complete_unknown_task_is_safe` function L468-472 — `()` — conversation so the LLM knows what finished.
-  `cancel_nonexistent_returns_false` function L475-478 — `()` — conversation so the LLM knows what finished.
-  `duplicate_complete_only_notifies_once` function L481-500 — `()` — conversation so the LLM knows what finished.

#### crates/arawn-engine/src/compact_prompt.rs

- pub `get_compact_prompt` function L38-48 — `() -> String` — Get the full compaction prompt (summarize entire conversation).
- pub `get_partial_compact_prompt` function L51-61 — `() -> String` — Get the partial compaction prompt (summarize only old messages, recent are kept).
- pub `format_compact_summary` function L64-92 — `(raw: &str) -> String` — Strip the `<analysis>` drafting scratchpad and extract `<summary>` content.
- pub `get_compact_user_summary_message` function L95-109 — `(summary: &str, recent_preserved: bool) -> String` — Wrap a formatted summary with continuation framing for the LLM.
-  `NO_TOOLS_PREAMBLE` variable L4-10 — `: &str` — See: claude-code/src/services/compact/prompt.ts
-  `ANALYSIS_INSTRUCTION` variable L12-21 — `: &str` — See: claude-code/src/services/compact/prompt.ts
-  `SUMMARY_TEMPLATE` variable L23-33 — `: &str` — See: claude-code/src/services/compact/prompt.ts
-  `NO_TOOLS_TRAILER` variable L35 — `: &str` — See: claude-code/src/services/compact/prompt.ts
-  `tests` module L112-180 — `-` — See: claude-code/src/services/compact/prompt.ts
-  `compact_prompt_contains_key_sections` function L116-123 — `()` — See: claude-code/src/services/compact/prompt.ts
-  `partial_prompt_mentions_recent` function L126-130 — `()` — See: claude-code/src/services/compact/prompt.ts
-  `format_strips_analysis_extracts_summary` function L133-155 — `()` — See: claude-code/src/services/compact/prompt.ts
-  `format_handles_no_tags` function L158-162 — `()` — See: claude-code/src/services/compact/prompt.ts
-  `format_handles_analysis_only` function L165-170 — `()` — See: claude-code/src/services/compact/prompt.ts
-  `user_summary_message_has_framing` function L173-179 — `()` — See: claude-code/src/services/compact/prompt.ts

#### crates/arawn-engine/src/compactor.rs

- pub `CompactionResult` struct L19-23 — `{ messages_summarized: usize, tokens_before: u32, tokens_after: u32 }` — Result of a compaction operation.
- pub `Compactor` struct L26-30 — `{ llm: Arc<dyn LlmClient>, keep_recent: usize, model: String }` — Orchestrates context compaction via LLM summarization.
- pub `new` function L33-39 — `(llm: Arc<dyn LlmClient>, model: impl Into<String>) -> Self`
- pub `with_keep_recent` function L41-51 — `( llm: Arc<dyn LlmClient>, model: impl Into<String>, keep_recent: usize, ) -> Se...`
- pub `should_compact` function L54-67 — `( &self, session: &Session, limits: &ModelLimits, tool_tokens: u32, system_token...` — Check if the session needs compaction based on token estimates.
- pub `compact` function L70-159 — `( &self, session: &mut Session, _limits: &ModelLimits, ) -> Result<CompactionRes...` — Compact the session by summarizing old messages via LLM.
-  `DEFAULT_KEEP_RECENT` variable L15 — `: usize`
-  `Compactor` type L32-185 — `= Compactor`
-  `call_llm` function L161-184 — `(&self, request: ChatRequest) -> Result<String, EngineError>`
-  `tests` module L188-308 — `-`
-  `make_session_with_messages` function L193-210 — `(count: usize) -> Session`
-  `should_compact_false_under_threshold` function L213-220 — `()`
-  `should_compact_true_over_threshold` function L223-230 — `()`
-  `should_compact_false_too_few_messages` function L233-240 — `()`
-  `compact_produces_summary` function L243-262 — `()`
-  `compact_preserves_recent_messages` function L265-295 — `()`
-  `compact_noop_when_few_messages` function L298-307 — `()`

#### crates/arawn-engine/src/context.rs

- pub `EngineToolContext` struct L22-46 — `{ session_id: Uuid, working_dir: PathBuf, workstream_name: String, allowed_paths...` — Concrete execution context provided to tools within the engine.
- pub `new` function L62-76 — `(workstream: &Workstream, session_id: Uuid) -> Self`
- pub `with_llm_resolver` function L81-84 — `(mut self, resolver: Arc<LlmResolverFn>) -> Self` — Attach an LLM resolver closure (typically wrapping `arawn-bin`'s
- pub `with_allowed_paths` function L87-90 — `(mut self, paths: Vec<PathBuf>) -> Self` — Set allowed paths that file tools can access outside the sandbox.
- pub `with_llm` function L93-97 — `(mut self, llm: Arc<dyn LlmClient>, model: String) -> Self` — Attach an LLM client and model for tools that need sub-queries.
- pub `with_model_limits` function L100-103 — `(mut self, limits: ModelLimits) -> Self` — Set model limits for sub-agent compaction.
- pub `with_data_dir` function L106-109 — `(mut self, dir: PathBuf) -> Self` — Set data directory for persisting large tool results.
-  `MAX_AGENT_DEPTH` variable L13 — `: u8` — Maximum sub-agent nesting depth.
-  `EngineToolContext` type L48-59 — `= EngineToolContext`
-  `fmt` function L49-58 — `(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result`
-  `EngineToolContext` type L61-110 — `= EngineToolContext`
-  `EngineToolContext` type L116-211 — `= EngineToolContext`
-  `working_dir` function L117-119 — `(&self) -> &Path`
-  `session_id` function L121-123 — `(&self) -> Uuid`
-  `validate_path` function L125-148 — `(&self, path_str: &str) -> Result<PathBuf, String>`
-  `is_allowed_path` function L150-159 — `(&self, path: &Path) -> bool`
-  `mark_file_read` function L161-163 — `(&self, path: PathBuf)`
-  `has_read_file` function L165-167 — `(&self, path: &Path) -> bool`
-  `llm` function L169-171 — `(&self) -> Option<&Arc<dyn LlmClient>>`
-  `model` function L173-175 — `(&self) -> Option<&str>`
-  `model_limits` function L177-179 — `(&self) -> &ModelLimits`
-  `data_dir` function L181-183 — `(&self) -> Option<&PathBuf>`
-  `agent_depth` function L185-187 — `(&self) -> u8`
-  `can_spawn_agent` function L189-191 — `(&self) -> bool`
-  `for_sub_agent` function L193-198 — `(&self) -> Box<dyn arawn_tool::ToolContext>`
-  `workstream_name` function L200-202 — `(&self) -> &str`
-  `allowed_paths` function L204-206 — `(&self) -> &[PathBuf]`
-  `resolve_llm` function L208-210 — `(&self, preference: &LlmPreference) -> Option<LlmResolution>`
-  `tests` module L214-237 — `-`
-  `context_from_workstream` function L219-227 — `()`
-  `context_is_clone` function L230-236 — `()`
-  `normalize_path_components` function L240-253 — `(path: &Path) -> PathBuf` — Normalize a path by resolving .

#### crates/arawn-engine/src/diff.rs

- pub `unified_diff` function L17-34 — `(path: &str, old: &str, new: &str) -> Option<String>` — Generate a unified diff between `old` and `new` content for the given file path.
- pub `diff_to_markdown` function L37-39 — `(diff: &str) -> String` — Format a diff as a fenced markdown code block.
- pub `creation_diff` function L43-58 — `(path: &str, content: &str, max_lines: usize) -> String` — Generate a creation diff (all lines added) for a new file.
- pub `diff_summary` function L61-80 — `(old: &str, new: &str) -> String` — Compute a summary line: "N lines added, M lines removed"
-  `CONTEXT_LINES` variable L10 — `: usize` — Number of context lines to show around each change.
-  `tests` module L83-166 — `-` — a fenced ```diff code block for TUI rendering.
-  `identical_returns_none` function L87-89 — `()` — a fenced ```diff code block for TUI rendering.
-  `simple_edit` function L92-101 — `()` — a fenced ```diff code block for TUI rendering.
-  `context_collapses_unchanged` function L104-127 — `()` — a fenced ```diff code block for TUI rendering.
-  `diff_to_markdown_wraps` function L130-135 — `()` — a fenced ```diff code block for TUI rendering.
-  `creation_diff_shows_lines` function L138-143 — `()` — a fenced ```diff code block for TUI rendering.
-  `creation_diff_truncates` function L146-151 — `()` — a fenced ```diff code block for TUI rendering.
-  `summary_counts` function L154-160 — `()` — a fenced ```diff code block for TUI rendering.
-  `summary_no_changes` function L163-165 — `()` — a fenced ```diff code block for TUI rendering.

#### crates/arawn-engine/src/error.rs

- pub `EngineError` enum L5-23 — `Tool | ToolNotFound | Llm | MaxIterations | Other`
- pub `user_message` function L38-60 — `(&self) -> String` — Return a user-facing error message with actionable guidance.
-  `EngineError` type L25-34 — `= EngineError`
-  `from` function L26-33 — `(err: arawn_tool::ToolError) -> Self`
-  `EngineError` type L36-61 — `= EngineError`

#### crates/arawn-engine/src/lib.rs

- pub `agent_defs` module L1 — `-`
- pub `background` module L2 — `-`
- pub `compact_prompt` module L3 — `-`
- pub `diff` module L4 — `-`
- pub `compactor` module L5 — `-`
- pub `context` module L6 — `-`
- pub `error` module L7 — `-`
- pub `hooks` module L8 — `-`
- pub `permissions` module L9 — `-`
- pub `plan` module L10 — `-`
- pub `plugins` module L11 — `-`
- pub `query_engine` module L12 — `-`
- pub `skills` module L13 — `-`
- pub `system_prompt` module L14 — `-`
- pub `testing` module L15 — `-`
- pub `token_estimator` module L16 — `-`
- pub `tool` module L17 — `-`
- pub `tool_result_limiter` module L18 — `-`
- pub `tools` module L19 — `-`
- pub `workstream_router` module L20 — `-`

#### crates/arawn-engine/src/plan.rs

- pub `PlanModeState` struct L21-23 — `{ inner: RwLock<PlanModeInner> }` — State for plan mode within a session.
- pub `PlanModeSnapshot` struct L42-46 — `{ active: bool, plan_file: Option<PathBuf>, plan_slug: Option<String> }` — Snapshot of plan mode state for tools to read without holding a lock.
- pub `new` function L49-59 — `() -> Self` — keeping them contextual to the work being done.
- pub `is_active` function L62-64 — `(&self) -> bool` — Whether plan mode is currently active.
- pub `snapshot` function L67-74 — `(&self) -> PlanModeSnapshot` — Get a snapshot of the current state.
- pub `enter` function L79-100 — `( &self, current_mode: PermissionMode, slug: &str, working_dir: &Path, ) -> std:...` — Enter plan mode.
- pub `exit` function L103-112 — `(&self) -> Option<PermissionMode>` — Exit plan mode.
- pub `plan_file` function L115-117 — `(&self) -> Option<PathBuf>` — Get the current plan file path (if in plan mode).
- pub `read_plan` function L120-123 — `(&self) -> Option<String>` — Read the current plan content from disk.
- pub `write_plan` function L126-133 — `(&self, content: &str) -> std::io::Result<()>` — Write plan content to disk.
- pub `is_plan_file` function L136-143 — `(&self, path: &Path) -> bool` — Check if a given file path is the current plan file (for write exceptions).
- pub `generate_slug` function L154-183 — `(description: &str) -> String` — Generate a human-friendly slug from a task description.
-  `PlanModeInner` struct L26-38 — `{ active: bool, pre_plan_mode: Option<PermissionMode>, stripped_rules: Vec<Permi...` — keeping them contextual to the work being done.
-  `PlanModeState` type L48-144 — `= PlanModeState` — keeping them contextual to the work being done.
-  `PlanModeState` type L146-150 — `impl Default for PlanModeState` — keeping them contextual to the work being done.
-  `default` function L147-149 — `() -> Self` — keeping them contextual to the work being done.
-  `tests` module L186-270 — `-` — keeping them contextual to the work being done.
-  `generate_slug_basic` function L191-193 — `()` — keeping them contextual to the work being done.
-  `generate_slug_strips_stop_words` function L196-201 — `()` — keeping them contextual to the work being done.
-  `generate_slug_max_four_words` function L204-209 — `()` — keeping them contextual to the work being done.
-  `generate_slug_empty` function L212-215 — `()` — keeping them contextual to the work being done.
-  `generate_slug_special_chars` function L218-220 — `()` — keeping them contextual to the work being done.
-  `plan_mode_lifecycle` function L223-248 — `()` — keeping them contextual to the work being done.
-  `exit_when_not_active_returns_none` function L251-254 — `()` — keeping them contextual to the work being done.
-  `snapshot_reflects_state` function L257-269 — `()` — keeping them contextual to the work being done.

#### crates/arawn-engine/src/query_engine.rs

- pub `ProgressEvent` enum L24-41 — `AssistantText | ToolCallStart | ToolCallResult` — Live progress events emitted during the engine loop.
- pub `IntegrationCapabilitiesFn` type L54 — `= std::sync::Arc<dyn Fn() -> Vec<String> + Send + Sync>` — Provider for dynamic integration capability summaries.
- pub `PromptContext` struct L58-73 — `{ prompts_dir: Option<std::path::PathBuf>, os: String, shell: String, cwd: std::...` — Cached context for building system prompts per-turn.
- pub `QueryEngineConfig` struct L77-88 — `{ model: String, max_iterations: usize, system_prompt: String, max_tokens: Optio...` — Configuration for the query engine.
- pub `QueryEngine` struct L105-126 — `{ llm: Arc<dyn LlmClient>, registry: Arc<ToolRegistry>, config: QueryEngineConfi...` — The agentic loop: prompt → LLM → tool_use → execute → feed result → loop.
- pub `new` function L129-146 — `(llm: Arc<dyn LlmClient>, registry: Arc<ToolRegistry>) -> Self`
- pub `with_config` function L148-169 — `( llm: Arc<dyn LlmClient>, registry: Arc<ToolRegistry>, config: QueryEngineConfi...`
- pub `with_compactor` function L171-174 — `(mut self, compactor: Compactor) -> Self`
- pub `with_permission_checker` function L176-179 — `(mut self, checker: Arc<PermissionChecker>) -> Self`
- pub `with_hook_runner` function L181-184 — `(mut self, runner: Arc<HookRunner>) -> Self`
- pub `with_skill_registry` function L186-189 — `(mut self, registry: Arc<crate::skills::SkillRegistry>) -> Self`
- pub `with_plugin_registry` function L191-194 — `(mut self, registry: Arc<crate::plugins::PluginRegistry>) -> Self`
- pub `with_plan_state` function L196-199 — `(mut self, plan_state: Arc<PlanModeState>) -> Self`
- pub `plan_state` function L202-204 — `(&self) -> Option<&Arc<PlanModeState>>` — Get the plan mode state (if configured).
- pub `with_background_tasks` function L206-209 — `(mut self, manager: Arc<BackgroundTaskManager>) -> Self`
- pub `with_progress_sender` function L212-215 — `(mut self, tx: tokio::sync::mpsc::Sender<ProgressEvent>) -> Self` — Set a channel for live progress events during the engine loop.
- pub `with_cancel_token` function L218-221 — `(mut self, token: tokio_util::sync::CancellationToken) -> Self` — Set a cancellation token — checked at each loop iteration and before tool execution.
- pub `fire_hook` function L240-246 — `(&self, input: &HookInput) -> Option<crate::hooks::AggregatedHookResult>` — Fire a hook event.
- pub `run` function L249-568 — `( &mut self, session: &mut Session, ctx: &dyn arawn_tool::ToolContext, ) -> Resu...` — Run the agentic loop for a session.
-  `DEFAULT_MAX_ITERATIONS` variable L18 — `: usize`
-  `MAX_COMPACT_FAILURES` variable L19 — `: u32`
-  `DEFAULT_SYSTEM_PROMPT` variable L42 — `: &str`
-  `QueryEngineConfig` type L90-102 — `impl Default for QueryEngineConfig`
-  `default` function L91-101 — `() -> Self`
-  `QueryEngine` type L128-916 — `= QueryEngine`
-  `is_cancelled` function L224-226 — `(&self) -> bool` — Check if cancellation has been requested.
-  `emit_progress` function L229-233 — `(&self, event: ProgressEvent)` — Emit a progress event if a sender is configured.
-  `build_request` function L570-669 — `(&self, session: &Session) -> ChatRequest`
-  `stream_response_with_retry` function L689-723 — `( &self, session: &Session, _ctx: &dyn arawn_tool::ToolContext, ) -> Result<Asse...` — Retry the request-build-and-stream cycle when the stream fails mid-flight.
-  `MAX_RETRIES` variable L694 — `: u32`
-  `BASE_DELAY_MS` variable L695 — `: u64`
-  `stream_response` function L725-785 — `( &self, request: ChatRequest, ) -> Result<AssembledResponse, EngineError>`
-  `execute_tool` function L787-915 — `( &self, ctx: &dyn arawn_tool::ToolContext, tool_use_id: &str, name: &str, argum...`
-  `parse_arguments` function L918-927 — `(raw: &str) -> serde_json::Value`
-  `AssembledResponse` struct L930-934 — `{ text: String, tool_calls: Vec<AssembledToolCall>, usage: Option<arawn_llm::Usa...`
-  `AssembledToolCall` struct L936-940 — `{ id: String, name: String, arguments: serde_json::Value }`
-  `ToolResult` struct L942-945 — `{ content: String, is_error: bool }`
-  `filter_tools_for_context` function L950-1062 — `( all_tools: &[arawn_llm::ToolDefinition], session: &Session, registry: &ToolReg...` — Filter tool definitions to only contextually relevant ones for this turn.
-  `tests` module L1065-1253 — `-`
-  `MockLlm` struct L1077-1079 — `{ responses: Mutex<Vec<Vec<ChatChunk>>> }` — Mock LLM that returns pre-scripted responses.
-  `MockLlm` type L1081-1111 — `= MockLlm`
-  `new` function L1082-1086 — `(responses: Vec<Vec<ChatChunk>>) -> Self`
-  `text` function L1089-1096 — `(text: &str) -> Vec<ChatChunk>` — Convenience: text-only response
-  `tool_call` function L1099-1110 — `(id: &str, name: &str, args: &str) -> Vec<ChatChunk>` — Convenience: tool call then done
-  `MockLlm` type L1114-1130 — `impl LlmClient for MockLlm`
-  `stream` function L1115-1129 — `( &self, _request: ChatRequest, ) -> Result< Pin<Box<dyn futures::Stream<Item = ...`
-  `setup` function L1132-1137 — `() -> (Workstream, Session, EngineToolContext)`
-  `text_only_response` function L1140-1153 — `()`
-  `single_tool_call` function L1156-1174 — `()`
-  `tool_not_found` function L1177-1199 — `()`
-  `max_iterations_exceeded` function L1202-1229 — `()`
-  `multi_turn_tool_chain` function L1232-1251 — `()`

#### crates/arawn-engine/src/system_prompt.rs

- pub `SystemPromptBuilder` struct L151-154 — `{ sections: Vec<PromptSection>, token_budget: u32 }` — Builds a system prompt from static defaults (overridable) + dynamic context.
- pub `new` function L157-162 — `() -> Self`
- pub `with_token_budget` function L165-168 — `(mut self, budget: u32) -> Self` — Set a custom token budget.
- pub `load_static_sections` function L172-184 — `(mut self, prompts_dir: Option<&Path>) -> Self` — Load all 7 static sections, checking for user overrides in `prompts_dir`.
- pub `environment` function L187-198 — `(mut self, os: &str, shell: &str, cwd: &Path, model: &str) -> Self` — Add the environment section.
- pub `workstream` function L201-211 — `(mut self, name: &str, root_dir: &Path) -> Self` — Add the workstream section.
- pub `tools` function L221-236 — `(mut self, tool_defs: &[ToolDefinition]) -> Self` — Acknowledge tool availability in the system prompt.
- pub `context_files` function L239-262 — `(mut self, files: &[ContextFile]) -> Self` — Add context files (arawn.md at workstream and global levels).
- pub `memories` function L265-280 — `(mut self, memories: &[String]) -> Self` — Add relevant memories (future — currently a no-op if empty).
- pub `session_context` function L283-294 — `(mut self, summary: &str) -> Self` — Add session context (for resumed sessions).
- pub `integrations` function L302-321 — `(mut self, summaries: &[String]) -> Self` — Add a section listing connected integrations and their granted
- pub `plugin_prompts` function L324-340 — `(mut self, prompts: &[String]) -> Self` — Add plugin-contributed prompt fragments.
- pub `build` function L343-365 — `(mut self) -> String` — Build the final system prompt string, enforcing token budget.
- pub `ContextFile` struct L378-382 — `{ path: std::path::PathBuf, content: String, truncated: bool }` — A context file loaded from disk.
- pub `find_context_files` function L385-401 — `(workstream_root: &Path, global_dir: &Path) -> Vec<ContextFile>` — Load context files from workstream root and global config dir.
-  `DEFAULT_TOKEN_BUDGET` variable L6 — `: u32` — Default token budget for the system prompt (~24k chars).
-  `MAX_CONTEXT_FILE_CHARS` variable L9 — `: usize` — Max chars for a context file before truncation.
-  `DEFAULT_IDENTITY` variable L13 — `: &str`
-  `DEFAULT_SYSTEM` variable L15-20 — `: &str`
-  `DEFAULT_DOING_TASKS` variable L22-46 — `: &str`
-  `DEFAULT_WORK_PROTOCOL` variable L48-60 — `: &str`
-  `DEFAULT_ACTIONS` variable L62-70 — `: &str`
-  `DEFAULT_USING_TOOLS` variable L72-82 — `: &str`
-  `DEFAULT_TONE` variable L84-88 — `: &str`
-  `DEFAULT_OUTPUT_EFFICIENCY` variable L90-104 — `: &str`
-  `STATIC_SECTION_NAMES` variable L107-116 — `: &[&str]` — Names of the overridable static sections.
-  `STATIC_SECTION_DEFAULTS` variable L119-128 — `: &[&str]` — Compiled-in defaults for each static section.
-  `STATIC_SECTION_PRIORITIES` variable L131-140 — `: &[u8]` — Priority levels for sections.
-  `PromptSection` struct L144-148 — `{ name: String, content: String, priority: u8 }` — A section in the assembled prompt.
-  `SystemPromptBuilder` type L156-366 — `= SystemPromptBuilder`
-  `SystemPromptBuilder` type L368-372 — `impl Default for SystemPromptBuilder`
-  `default` function L369-371 — `() -> Self`
-  `load_context_file` function L403-422 — `(path: &Path, max_chars: usize) -> Option<ContextFile>`
-  `truncate_70_20` function L425-448 — `(content: &str, max_chars: usize) -> String` — Truncate keeping 70% from the head and 20% from the tail, with a marker in between.
-  `load_section` function L452-460 — `(name: &str, default: &str, prompts_dir: Option<&Path>) -> String`
-  `tests` module L463-778 — `-`
-  `default_assembly_includes_all_static_sections` function L470-486 — `()`
-  `sections_have_headers` function L490-501 — `()`
-  `empty_optional_sections_omitted` function L505-516 — `()`
-  `single_section_override` function L520-531 — `()`
-  `partial_overrides_other_sections_use_defaults` function L535-547 — `()`
-  `missing_override_dir_uses_defaults` function L551-557 — `()`
-  `empty_override_file_produces_empty_section` function L561-571 — `()`
-  `under_budget_all_sections_included` function L575-586 — `()`
-  `over_budget_drops_low_priority_sections` function L590-600 — `()`
-  `identity_survives_budget_cuts` function L604-613 — `()`
-  `truncation_produces_clean_sections` function L617-629 — `()`
-  `context_file_injected` function L633-644 — `()`
-  `context_file_missing_section_omitted` function L648-655 — `()`
-  `large_context_file_truncated` function L659-670 — `()`
-  `tools_section_reflects_tool_list` function L674-693 — `()`
-  `per_turn_freshness_different_tools` function L697-721 — `()`
-  `environment_section_contains_info` function L725-734 — `()`
-  `workstream_section_contains_info` function L738-745 — `()`
-  `snapshot_full_build` function L749-777 — `()`

#### crates/arawn-engine/src/testing.rs

- pub `HarnessResult` struct L16-19 — `{ final_text: String, session: Session }` — Result from running the test harness.
- pub `final_text` function L22-24 — `(&self) -> &str`
- pub `tool_calls` function L26-38 — `(&self) -> Vec<(&str, &serde_json::Value)>`
- pub `session_messages` function L40-42 — `(&self) -> &[Message]`
- pub `message_count` function L44-46 — `(&self) -> usize`
- pub `TestHarness` struct L50-62 — `{ _temp_dir: TempDir, workstream: Workstream, registry: Arc<ToolRegistry>, mock_...` — Builder for assembling a full engine test fixture.
- pub `TestHarnessBuilder` struct L65-76 — `{ temp_dir: TempDir, files: Vec<(String, String)>, tools: Vec<Box<dyn Tool>>, sc...` — Builder for constructing a TestHarness.
- pub `new` function L79-92 — `() -> Self`
- pub `with_workstream_file` function L95-102 — `( mut self, path: impl Into<String>, content: impl Into<String>, ) -> Self` — Pre-populate a file in the workstream directory.
- pub `with_tool` function L105-108 — `(mut self, tool: Box<dyn Tool>) -> Self` — Register a tool in the registry.
- pub `with_tools` function L111-114 — `(mut self, tools: impl IntoIterator<Item = Box<dyn Tool>>) -> Self` — Register multiple tools.
- pub `with_script` function L117-120 — `(mut self, script: Vec<MockResponse>) -> Self` — Set the scripted LLM responses.
- pub `with_max_iterations` function L123-126 — `(mut self, max: usize) -> Self` — Set max iterations for the engine.
- pub `with_permission_checker` function L129-132 — `(mut self, checker: Arc<PermissionChecker>) -> Self` — Wire a permission checker into the engine.
- pub `with_hook_runner` function L135-138 — `(mut self, runner: Arc<HookRunner>) -> Self` — Wire a hook runner into the engine.
- pub `with_skill_registry` function L141-144 — `(mut self, registry: Arc<SkillRegistry>) -> Self` — Wire a skill registry into the engine.
- pub `with_plan_active` function L147-150 — `(mut self) -> Self` — Enable plan mode on the engine (blocks write tools, allows read-only).
- pub `with_progress_channel` function L154-157 — `(mut self) -> Self` — Enable progress event capture.
- pub `build` function L160-222 — `(self) -> TestHarness` — Build the harness.
- pub `builder` function L232-234 — `() -> TestHarnessBuilder`
- pub `mock_llm` function L237-239 — `(&self) -> &Arc<MockLlmClient>` — Access the underlying mock LLM client for assertions (call_count, captured_requests).
- pub `take_progress_rx` function L242-244 — `(&self) -> Option<tokio::sync::mpsc::Receiver<ProgressEvent>>` — Take the progress event receiver.
- pub `run` function L247-266 — `(&self, user_input: impl Into<String>) -> HarnessResult` — Run the engine with the given user input and return results.
- pub `run_expect_error` function L269-286 — `( &self, user_input: impl Into<String>, ) -> crate::error::EngineError` — Run expecting an error (e.g., max iterations).
-  `HarnessResult` type L21-47 — `= HarnessResult`
-  `TestHarnessBuilder` type L78-223 — `= TestHarnessBuilder`
-  `TestHarnessBuilder` type L225-229 — `impl Default for TestHarnessBuilder`
-  `default` function L226-228 — `() -> Self`
-  `TestHarness` type L231-316 — `= TestHarness`
-  `build_engine` function L289-315 — `(&self) -> QueryEngine` — Build a QueryEngine with all configured subsystems wired in.
-  `tests` module L319-1924 — `-`
-  `harness_text_only` function L325-334 — `()`
-  `harness_single_tool_call` function L337-353 — `()`
-  `harness_multi_step_tool_chain` function L356-374 — `()`
-  `harness_tool_not_found` function L377-399 — `()`
-  `harness_max_iterations` function L402-418 — `()`
-  `harness_shell_tool_receives_arguments` function L421-448 — `()`
-  `harness_raw_chunks_split_arguments` function L451-494 — `()`
-  `harness_tool_arguments_passed_correctly` function L497-520 — `()`
-  `harness_permission_checker_blocks_tool` function L523-559 — `()`
-  `harness_permission_checker_allows_tool` function L562-592 — `()`
-  `harness_file_read_with_real_filesystem` function L595-619 — `()`
-  `harness_parallel_tool_calls_in_single_turn` function L622-687 — `()`
-  `harness_mixed_text_and_tool_call_in_same_turn` function L690-729 — `()`
-  `harness_stream_without_done_chunk` function L732-770 — `()`
-  `harness_empty_stream_done_only` function L773-786 — `()`
-  `harness_empty_text_deltas_assembled_correctly` function L789-808 — `()`
-  `harness_text_after_tool_start_both_captured` function L811-858 — `()`
-  `harness_malformed_json_args_falls_back_to_empty_object` function L861-891 — `()`
-  `harness_non_object_json_args_rejected` function L894-929 — `()`
-  `harness_string_json_args_rejected` function L932-963 — `()`
-  `harness_empty_tool_args_no_delta` function L966-993 — `()`
-  `harness_repeated_failure_circuit_breaker` function L996-1053 — `()`
-  `harness_empty_text_response_returns_cleanly` function L1056-1068 — `()`
-  `harness_token_usage_accumulation` function L1071-1114 — `()`
-  `harness_fatal_llm_error_no_retry` function L1117-1139 — `()`
-  `harness_transient_error_then_success` function L1142-1159 — `()`
-  `harness_transient_error_exhausts_retries` function L1162-1184 — `()`
-  `harness_mid_stream_error_during_text` function L1187-1214 — `()`
-  `harness_mid_stream_error_during_tool_call` function L1217-1246 — `()`
-  `harness_server_error_is_transient` function L1249-1263 — `()`
-  `harness_model_not_found_is_not_transient` function L1266-1283 — `()`
-  `harness_permission_denial_then_llm_recovery` function L1286-1336 — `()`
-  `harness_plan_mode_blocks_write_tool` function L1339-1367 — `()`
-  `harness_plan_mode_allows_read_only_tool` function L1370-1394 — `()`
-  `harness_hook_and_permission_both_wired` function L1397-1452 — `()`
-  `harness_long_tool_chain_five_steps` function L1457-1501 — `()`
-  `harness_tool_error_recovery_mid_chain` function L1504-1553 — `()`
-  `harness_parallel_reads_then_sequential_think` function L1556-1607 — `()`
-  `harness_narration_text_across_multiple_tool_turns` function L1610-1689 — `()`
-  `harness_retry_recovery_mid_conversation` function L1692-1720 — `()`
-  `harness_large_argument_reassembly_many_deltas` function L1723-1771 — `()`
-  `harness_alternating_success_and_failure_chain` function L1774-1805 — `()`
-  `harness_permission_denial_cascade_then_success` function L1808-1856 — `()`
-  `harness_plan_mode_parallel_mixed_tools` function L1859-1923 — `()`

#### crates/arawn-engine/src/token_estimator.rs

- pub `TokenEstimator` struct L6 — `-` — Fast, approximate token estimation using chars/4 heuristic.
- pub `estimate_message` function L10-26 — `(msg: &Message) -> u32` — Estimate tokens for a single message.
- pub `estimate_messages` function L29-31 — `(messages: &[Message]) -> u32` — Estimate total tokens for all messages in a session.
- pub `estimate_tools` function L34-40 — `(tools: &[ToolDefinition]) -> u32` — Estimate tokens for tool definitions (JSON schemas sent with each request).
- pub `estimate_system_prompt` function L43-45 — `(prompt: &str) -> u32` — Estimate tokens for a system prompt string.
-  `TokenEstimator` type L8-46 — `= TokenEstimator`
-  `tests` module L52-162 — `-`
-  `estimate_user_message` function L58-65 — `()`
-  `estimate_assistant_with_tool_uses` function L68-79 — `()`
-  `estimate_tool_result` function L82-90 — `()`
-  `estimate_messages_sums` function L93-109 — `()`
-  `estimate_tools` function L112-120 — `()`
-  `model_limits_for_known_models` function L123-140 — `()`
-  `should_compact_under_threshold` function L143-147 — `()`
-  `should_compact_over_threshold` function L150-153 — `()`
-  `available_for_messages` function L156-161 — `()`

#### crates/arawn-engine/src/tool.rs

-  `tests` module L9-209 — `-`
-  `DummyTool` struct L16-18 — `{ tool_name: String }` — A minimal test tool for unit testing the registry.
-  `DummyTool` type L20-26 — `= DummyTool`
-  `new` function L21-25 — `(name: &str) -> Self`
-  `DummyTool` type L29-49 — `impl Tool for DummyTool`
-  `name` function L30-32 — `(&self) -> &str`
-  `description` function L34-36 — `(&self) -> &str`
-  `parameters_schema` function L38-40 — `(&self) -> Value`
-  `execute` function L42-48 — `( &self, _ctx: &dyn arawn_tool::ToolContext, _params: Value, ) -> Result<ToolOut...`
-  `registry_starts_empty` function L52-56 — `()`
-  `register_and_get_tool` function L59-69 — `()`
-  `get_nonexistent_tool_returns_none` function L72-75 — `()`
-  `unregister_tool` function L78-87 — `()`
-  `unregister_nonexistent_returns_none` function L90-93 — `()`
-  `hot_reload_register_unregister_cycle` function L96-114 — `()`
-  `tool_definitions_reflects_registered_tools` function L117-128 — `()`
-  `tool_definitions_updates_after_unregister` function L131-140 — `()`
-  `registry_is_send_sync` function L143-146 — `()`
-  `assert_send_sync` function L144 — `()`
-  `concurrent_access` function L149-167 — `()`
-  `unregister_by_prefix_removes_matching` function L170-185 — `()`
-  `unregister_by_prefix_no_match` function L188-194 — `()`
-  `tool_output_success` function L197-201 — `()`
-  `tool_output_error` function L204-208 — `()`

#### crates/arawn-engine/src/tool_result_limiter.rs

- pub `DEFAULT_MAX_RESULT_SIZE_CHARS` variable L9 — `: usize` — Default maximum characters per tool result before persisting to disk.
- pub `limit_tool_result` function L18-57 — `( output: ToolOutput, session_id: Uuid, data_dir: &Path, max_chars: usize, ) -> ...` — Check if a tool output exceeds the size threshold.
-  `PREVIEW_SIZE` variable L12 — `: usize` — Truncation preview size — how much of the original to keep inline.
-  `truncate_output` function L59-88 — `( output: ToolOutput, _max_chars: usize, persisted_path: Option<&Path>, ) -> Too...`
-  `tests` module L91-181 — `-`
-  `small_output_passes_through` function L96-110 — `()`
-  `large_output_gets_truncated_and_persisted` function L113-144 — `()`
-  `truncated_output_contains_preview` function L147-157 — `()`
-  `error_flag_preserved` function L160-168 — `()`
-  `custom_threshold` function L171-180 — `()`

#### crates/arawn-engine/src/workstream_router.rs

- pub `WorkstreamMemoryRouter` struct L21-27 — `{ data_dir: PathBuf, embedding_dims: Option<usize>, embedder: Option<Arc<dyn Emb...` — Lazy + cached map of workstream-name → `MemoryManager`.
- pub `new` function L30-43 — `( data_dir: impl Into<PathBuf>, embedding_dims: Option<usize>, embedder: Option<...` — existing fixed-manager tests continue working unchanged.
- pub `current` function L47-50 — `(&self) -> Result<Arc<MemoryManager>, MemoryError>` — Resolve the active workstream's memory manager.
- pub `current_name` function L54-56 — `(&self) -> String` — Name of the active workstream — useful for tools that need to
- pub `for_workstream` function L58-72 — `(&self, name: &str) -> Result<Arc<MemoryManager>, MemoryError>` — existing fixed-manager tests continue working unchanged.
- pub `MemoryHandle` enum L79-82 — `Fixed | Routed` — Memory tools depend on one of these.
- pub `manager` function L87-92 — `(&self) -> Result<Arc<MemoryManager>, MemoryError>` — Resolve the active manager.
-  `WorkstreamMemoryRouter` type L29-73 — `= WorkstreamMemoryRouter` — existing fixed-manager tests continue working unchanged.
-  `MemoryHandle` type L84-93 — `= MemoryHandle` — existing fixed-manager tests continue working unchanged.
-  `MemoryHandle` type L95-99 — `= MemoryHandle` — existing fixed-manager tests continue working unchanged.
-  `from` function L96-98 — `(m: Arc<MemoryManager>) -> Self` — existing fixed-manager tests continue working unchanged.
-  `MemoryHandle` type L101-105 — `= MemoryHandle` — existing fixed-manager tests continue working unchanged.
-  `from` function L102-104 — `(r: Arc<WorkstreamMemoryRouter>) -> Self` — existing fixed-manager tests continue working unchanged.
-  `tests` module L108-133 — `-` — existing fixed-manager tests continue working unchanged.
-  `router_caches_per_workstream` function L112-124 — `()` — existing fixed-manager tests continue working unchanged.
-  `fixed_handle_dispatches` function L127-132 — `()` — existing fixed-manager tests continue working unchanged.

### crates/arawn-engine/src/hooks

**Role**: Lifecycle event interception — allows user-defined shell commands to observe or block engine actions (tool calls, permission requests, session events) by hooking into named event types with optional tool-name/content matchers.

**Key abstractions**:
- `HookEvent` — 25 event types matching Claude Code's hook surface: `PreToolUse`, `PostToolUse`, `PostToolUseFailure`, `PermissionRequest`, `PermissionDenied`, `SessionStart`, `SessionEnd`, `FileChanged`, etc. Only `PreToolUse`, `PermissionRequest`, and `UserPromptSubmit` can block execution (`can_block()`).
- `HookMatcher` — Filters a hook group by a field value (tool name, source, notification type). Supports exact strings, pipe-separated alternatives (`Bash|Edit`), and glob patterns (`File*`). An empty matcher matches everything. `matches(field_value, content)` handles both field matching and optional content-pattern matching.
- `HookConfig` — Maps event key strings to `Vec<HookGroup>`. Each `HookGroup` has an optional `HookMatcher` and a list of `CommandHookDef`. `matching_hooks()` returns only the defs whose group matcher fires for the current event/value. `merge()` combines user-level and project-level configs.
- `CommandHookExecutor` — Runs a hook command as a subprocess, sending `HookInput` JSON on stdin. Interprets exit codes: 0 = allow, 1 = warn (stdout becomes the warning message), 2 = block (stderr becomes the block reason). Default timeout 10 seconds.
- `HookRunner` — Ties matching, execution, and aggregation together. `run(&HookInput)` finds all matching commands, executes them, and returns an `AggregatedHookResult` where any block from any hook wins.
- `HookFileWatcher` — Watches a list of paths with debouncing and fires `FileChanged` hooks via `HookRunner` when changes are detected.

**Internal flow**: `QueryEngine` calls `fire_hook()` before tool use (passing `PreToolUse` input), checks the result for `blocked`, and returns a tool error if blocked. The `HookRunner` is loaded from merged user+project settings JSON by `load_merged_hooks()` at startup and re-loaded on config change.

**Dependencies**: `notify` (file watching), `serde_json` (stdin payload), standard library process spawning.

#### crates/arawn-engine/src/hooks/config.rs

- pub `HookConfig` struct L20-25 — `{ events: HashMap<String, Vec<HookGroup>> }` — Top-level hook configuration: maps event types to lists of hook groups.
- pub `groups_for` function L29-35 — `(&self, event: HookEvent) -> Vec<&HookGroup>` — Get all hook groups for a given event type.
- pub `matching_hooks` function L38-54 — `( &self, event: HookEvent, field_value: &str, content: &str, ) -> Vec<&CommandHo...` — Get all command hook definitions that match a given event and field value.
- pub `merge` function L57-64 — `(&mut self, other: HookConfig)` — Merge another config into this one.
- pub `is_empty` function L67-69 — `(&self) -> bool` — Returns true if this config has no hooks defined.
- pub `HookGroup` struct L83-90 — `{ matcher: Option<HookMatcher>, hooks: Vec<CommandHookDef> }` — A group of hooks sharing a common matcher.
- pub `CommandHookDef` struct L94-105 — `{ hook_type: String, command: String, timeout: Option<u64> }` — Definition of a command hook: a shell command to execute when the event fires.
- pub `HookResult` enum L109-124 — `Allow | Block | Warn` — The result of executing a single hook.
- pub `is_block` function L127-129 — `(&self) -> bool`
- pub `AggregatedHookResult` struct L134-141 — `{ blocked: bool, block_reason: Option<String>, warnings: Vec<String> }` — Aggregated result from running all matching hooks for an event.
- pub `add` function L145-158 — `(&mut self, result: HookResult)` — Merge a single hook result into the aggregate.
-  `HookConfig` type L27-70 — `= HookConfig`
-  `HookResult` type L126-130 — `= HookResult`
-  `AggregatedHookResult` type L143-159 — `= AggregatedHookResult`
-  `event_to_key` function L162-190 — `(event: HookEvent) -> &'static str` — Map a HookEvent to its config key string.
-  `tests` module L193-348 — `-`
-  `sample_config` function L196-221 — `() -> HookConfig`
-  `deserialize_config` function L224-229 — `()`
-  `matching_hooks_by_tool_name` function L232-244 — `()`
-  `session_start_no_matcher` function L247-252 — `()`
-  `merge_configs` function L255-282 — `()`
-  `empty_config` function L285-289 — `()`
-  `hook_result_aggregation` function L292-314 — `()`
-  `first_block_wins` function L317-328 — `()`
-  `command_hook_def_timeout` function L331-347 — `()`

#### crates/arawn-engine/src/hooks/events.rs

- pub `HookEvent` enum L11-83 — `PreToolUse | PostToolUse | PostToolUseFailure | PermissionRequest | PermissionDe...` — All 25 hook event types matching Claude Code's surface area.
- pub `ALL` variable L87-113 — `: &'static [HookEvent]` — All event variants, for iteration.
- pub `can_block` function L116-121 — `(&self) -> bool` — Whether this event can block execution (PreToolUse, PermissionRequest, UserPromptSubmit).
- pub `matcher_field` function L124-142 — `(&self) -> &'static str` — The field name that matchers filter on for this event type.
- pub `summary` function L145-173 — `(&self) -> &'static str` — Human-readable summary of when this event fires.
- pub `HookInput` enum L182-306 — `PreToolUse | PostToolUse | PostToolUseFailure | PermissionRequest | PermissionDe...` — Input data passed to hooks when they fire.
- pub `event` function L310-338 — `(&self) -> HookEvent` — Get the event type for this input.
- pub `matcher_value` function L341-354 — `(&self) -> &str` — Get the matcher field value for this input (the value that matchers filter on).
-  `HookEvent` type L85-174 — `= HookEvent`
-  `HookInput` type L308-355 — `= HookInput`
-  `tests` module L358-419 — `-`
-  `all_events_count` function L362-364 — `()`
-  `blocking_events` function L367-374 — `()`
-  `hook_input_event_roundtrip` function L377-384 — `()`
-  `hook_input_serialization` function L387-399 — `()`
-  `session_start_matcher_value` function L402-410 — `()`
-  `non_matchable_event_returns_empty` function L413-418 — `()`

#### crates/arawn-engine/src/hooks/executor.rs

- pub `CommandHookExecutor` struct L21 — `-` — Executes command hooks as shell subprocesses.
- pub `execute` function L27-130 — `( hook: &CommandHookDef, input: &HookInput, cwd: &Path, ) -> HookResult` — Execute a command hook with the given input.
-  `DEFAULT_TIMEOUT_SECS` variable L12 — `: u64` — Default timeout for hook execution (10 seconds).
-  `CommandHookExecutor` type L23-131 — `= CommandHookExecutor`
-  `tests` module L134-256 — `-`
-  `make_hook` function L137-143 — `(command: &str, timeout: Option<u64>) -> CommandHookDef`
-  `sample_input` function L145-150 — `() -> HookInput`
-  `cwd` function L152-154 — `() -> std::path::PathBuf`
-  `exit_code_0_allows` function L157-161 — `()`
-  `exit_code_2_blocks` function L164-173 — `()`
-  `exit_code_1_warns` function L176-185 — `()`
-  `captures_stdout` function L188-197 — `()`
-  `receives_json_on_stdin` function L200-211 — `()`
-  `timeout_blocks` function L214-223 — `()`
-  `spawn_failure_warns` function L226-243 — `()`
-  `block_with_empty_stderr_uses_default_message` function L246-255 — `()`

#### crates/arawn-engine/src/hooks/file_watcher.rs

- pub `HookFileWatcher` struct L16-19 — `{ paths: Vec<PathBuf>, hook_runner: Arc<HookRunner> }` — Watches file paths and fires `FileChanged` hooks when changes are detected.
- pub `new` function L22-24 — `(paths: Vec<PathBuf>, hook_runner: Arc<HookRunner>) -> Self`
- pub `spawn` function L27-33 — `(self) -> tokio::task::JoinHandle<()>` — Spawn the file watcher as a background tokio task.
-  `HookFileWatcher` type L21-139 — `= HookFileWatcher`
-  `run` function L35-138 — `(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>`

#### crates/arawn-engine/src/hooks/loader.rs

- pub `load_hooks_from_file` function L27-45 — `(path: &Path) -> HookConfig` — Load hook configuration from a JSON settings file.
- pub `load_merged_hooks` function L53-67 — `( user_settings_path: Option<&Path>, project_settings_path: Option<&Path>, ) -> ...` — Load and merge hook configs from user-level and project-level settings.
-  `SettingsFile` struct L18-21 — `{ hooks: HookConfig }` — Wrapper for the hooks section in settings.json.
-  `tests` module L70-249 — `-`
-  `write_json` function L75-78 — `(file: &std::fs::File, json: &str)` — Helper to write raw bytes to a temp file (avoids write! macro brace escaping).
-  `load_from_json_file` function L81-114 — `()`
-  `load_missing_file_returns_defaults` function L117-120 — `()`
-  `load_file_without_hooks_key` function L123-129 — `()`
-  `load_malformed_json_returns_defaults` function L132-138 — `()`
-  `merge_user_and_project` function L141-190 — `()`
-  `merge_missing_user_config` function L193-212 — `()`
-  `merge_both_missing` function L215-218 — `()`
-  `dedup_identical_hooks_across_sources` function L221-248 — `()`

#### crates/arawn-engine/src/hooks/matcher.rs

- pub `HookMatcher` struct L16-19 — `{ raw: String }` — Matches hook events by a filterable field value (tool name, source, notification type, etc.)
- pub `new` function L35-37 — `(raw: impl Into<String>) -> Self`
- pub `matches` function L43-62 — `(&self, field_value: &str, content: &str) -> bool` — Check if this matcher matches a given field value and optional content string.
-  `HookMatcher` type L21-25 — `impl Serialize for HookMatcher`
-  `serialize` function L22-24 — `(&self, serializer: S) -> Result<S::Ok, S::Error>`
-  `HookMatcher` type L27-32 — `= HookMatcher`
-  `deserialize` function L28-31 — `(deserializer: D) -> Result<Self, D::Error>`
-  `HookMatcher` type L34-72 — `= HookMatcher`
-  `matches_alternatives` function L65-71 — `(&self, spec: &str, value: &str) -> bool` — Check pipe-separated alternatives: "Bash|Edit|Write"
-  `glob_match` function L76-80 — `(pattern: &str, text: &str) -> bool` — Simple glob matching supporting `*` (any chars) and `?` (single char).
-  `glob_match_inner` function L82-110 — `(pat: &[char], txt: &[char]) -> bool`
-  `tests` module L113-210 — `-`
-  `glob_exact` function L119-122 — `()`
-  `glob_star` function L125-129 — `()`
-  `glob_question_mark` function L132-135 — `()`
-  `empty_matcher_matches_everything` function L140-145 — `()`
-  `exact_tool_match` function L148-152 — `()`
-  `pipe_separated_alternatives` function L155-161 — `()`
-  `glob_tool_match` function L164-169 — `()`
-  `content_pattern` function L172-178 — `()`
-  `content_pattern_with_pipes` function L181-188 — `()`
-  `session_source_matching` function L191-195 — `()`
-  `wildcard_matches_any_tool` function L198-203 — `()`
-  `nested_parens_in_content` function L206-209 — `()`

#### crates/arawn-engine/src/hooks/mod.rs

-  `config` module L8 — `-` — The hooks system intercepts lifecycle events (tool execution, session
-  `events` module L9 — `-` — event type + optional tool name / content patterns.
-  `executor` module L10 — `-` — event type + optional tool name / content patterns.
-  `file_watcher` module L11 — `-` — event type + optional tool name / content patterns.
-  `loader` module L12 — `-` — event type + optional tool name / content patterns.
-  `matcher` module L13 — `-` — event type + optional tool name / content patterns.
-  `runner` module L14 — `-` — event type + optional tool name / content patterns.

#### crates/arawn-engine/src/hooks/runner.rs

- pub `HookRunner` struct L15-19 — `{ config: HookConfig, cwd: PathBuf }` — Orchestrates hook matching, execution, and result aggregation.
- pub `new` function L22-24 — `(config: HookConfig, cwd: PathBuf) -> Self`
- pub `run` function L27-69 — `(&self, input: &HookInput) -> AggregatedHookResult` — Run all matching hooks for the given input and return the aggregated result.
- pub `has_hooks` function L72-74 — `(&self) -> bool` — Check if any hooks are configured (useful for fast-path skipping).
-  `HookRunner` type L21-90 — `= HookRunner`
-  `extract_content` function L77-89 — `(&self, input: &HookInput) -> String` — Extract the content string used for content-pattern matching.
-  `tests` module L93-228 — `-`
-  `config_with_blocking_hook` function L96-108 — `() -> HookConfig`
-  `config_with_allowing_hook` function L110-130 — `() -> HookConfig`
-  `cwd` function L132-134 — `() -> PathBuf`
-  `no_hooks_returns_default` function L137-145 — `()`
-  `blocking_hook_blocks` function L148-157 — `()`
-  `allowing_hook_allows` function L160-168 — `()`
-  `non_matching_tool_skips_hooks` function L171-179 — `()`
-  `post_tool_use_runs` function L182-191 — `()`
-  `has_hooks_true_when_configured` function L194-197 — `()`
-  `has_hooks_false_when_empty` function L200-203 — `()`
-  `multiple_hooks_any_block_wins` function L206-227 — `()`

### crates/arawn-engine/src/permissions

**Role**: Guards tool execution by evaluating explicit allow/deny/ask rules and a permission mode fallback, optionally prompting the user interactively for undecided cases.

**Key abstractions**:
- `PermissionMode` — Four modes controlling fallback behavior when no rule matches: `Default` (read-only auto-allowed, others ask), `AcceptEdits` (file ops auto-allowed, shell asks), `BypassPermissions` (everything allowed), `Plan` (only read-only allowed; plan mode tools `enter_plan_mode`/`exit_plan_mode` always allowed). The mode is serializable for wire transport.
- `PermissionCategory` — Risk class declared by each `Tool`: `ReadOnly`, `FileWrite`, `Shell`, `Other`. The `Tool` trait's default `permission_category()` returns `ReadOnly` when `is_read_only()` is true, otherwise `Other`. `FileEditTool`, `FileWriteTool`, and `ShellTool` explicitly override to their respective categories.
- `PermissionRule` — A parsed rule with `kind` (Allow/Deny/Ask), a `tool_pattern` (glob), and optional `content_pattern` (substring/glob on the tool's input JSON). Parsed from the compact string format `"ToolName(content)"`. `RuleMatcher::evaluate` scans rules in order: Deny beats Allow; first matching kind wins.
- `PermissionChecker` — The session-scoped gate. `check(tool_name, tool_input, category)` runs: 1) evaluate explicit rules, 2) check `SessionGrants` (from prior AllowAlways responses), 3) apply `PermissionMode::fallback`. If the result is `Ask`, calls `prompt_user` which delegates to the `ModalPrompt` impl. Returns `Allowed` or `Denied`. Supports hot-reload via `update_rules` / `update_mode` (both take a write lock).
- `ModalPrompt` — Trait for presenting a multiple-choice modal to the user. `CliModalPrompt` blocks stdin; `ChannelModalPrompt` (in arawn-bin) routes through the WS server to the client.

**Internal flow**: `QueryEngine::execute_tool` calls `PermissionChecker::check` before dispatching any tool. The checker looks up `registry.get(name).permission_category()` at the call site (not a string-switch table); this is how the per-tool `PermissionCategory` reaches the checker. If `Denied`, the tool is not executed and an error result is fed back to the LLM.

**Dependencies**: `arawn-tool` (PermissionCategory, ToolRegistry), `async-trait`.

#### crates/arawn-engine/src/permissions/checker.rs

- pub `PermissionMode` enum L12-27 — `Default | AcceptEdits | BypassPermissions | Plan` — Permission mode — controls fallback behavior when no explicit rule matches.
- pub `fallback` function L36-61 — `(&self, category: PermissionCategory, tool_name: &str) -> PermissionDecision` — Determine the fallback decision for a tool when no explicit rule
- pub `PermissionResponse` enum L66-70 — `AllowOnce | AllowAlways | Deny` — Response from a user when prompted for permission.
- pub `ModalOption` struct L74-77 — `{ label: String, description: Option<String> }` — A single option displayed in a modal prompt.
- pub `new` function L80-85 — `(label: impl Into<String>) -> Self`
- pub `with_description` function L87-90 — `(mut self, desc: impl Into<String>) -> Self`
- pub `ModalRequest` struct L95-99 — `{ title: String, subtitle: Option<String>, options: Vec<ModalOption> }` — A request to show a modal to the user and get a selection.
- pub `ModalPrompt` interface L105-107 — `{ fn prompt() }` — Generic trait for prompting the user with a modal dialog.
- pub `SessionGrants` struct L113-115 — `{ grants: std::collections::HashSet<String> }` — In-memory store for session-scoped permission grants.
- pub `new` function L118-120 — `() -> Self`
- pub `grant` function L123-125 — `(&mut self, tool_name: String)` — Record a session grant for a tool name.
- pub `is_granted` function L128-130 — `(&self, tool_name: &str) -> bool` — Check if a tool has been granted for this session.
- pub `clear` function L133-135 — `(&mut self)` — Clear all session grants.
- pub `DecisionReason` enum L142-154 — `MatchedRule | SessionGrant | ModeFallback | Prompted | NoChecker` — Why a permission decision came out the way it did.
- pub `display` function L158-174 — `(&self) -> String` — One-line human-readable form for error messages and audit display.
- pub `AuditEntry` struct L179-185 — `{ timestamp: std::time::SystemTime, tool_name: String, tool_input_summary: Strin...` — One row of the audit log — what was checked, when, and how it was decided.
- pub `PermissionSnapshot` struct L191-197 — `{ mode: PermissionMode, allow_rules: Vec<String>, deny_rules: Vec<String>, ask_r...` — Read-only snapshot of the current permission state — exposed via the
- pub `SharedAudit` type L207 — `= std::sync::Arc<std::sync::Mutex<std::collections::VecDeque<AuditEntry>>>` — Shareable audit buffer — held in an Arc so callers (e.g.
- pub `new_shared_audit` function L210-212 — `() -> SharedAudit` — Construct a fresh shared audit buffer with the standard cap.
- pub `PermissionChecker` struct L216-222 — `{ rules: std::sync::RwLock<Vec<PermissionRule>>, mode: std::sync::RwLock<Permiss...` — The central permission checker.
- pub `new` function L227-235 — `(rules: Vec<PermissionRule>) -> Self` — Create a new permission checker with the given rules and default mode.
- pub `with_audit` function L240-243 — `(mut self, audit: SharedAudit) -> Self` — Wire an externally-owned audit buffer so per-message checkers can
- pub `snapshot` function L248-275 — `(&self) -> PermissionSnapshot` — Capture a read-only snapshot of the current rules, mode, and recent
- pub `with_mode` function L294-300 — `(self, mode: PermissionMode) -> Self` — Set the permission mode (Default, AcceptEdits, BypassPermissions).
- pub `with_prompter` function L303-306 — `(mut self, prompter: Box<dyn ModalPrompt>) -> Self` — Set the modal prompter for interactive permission requests.
- pub `update_rules` function L309-312 — `(&self, rules: Vec<PermissionRule>)` — Hot-reload: replace the current rules with new ones.
- pub `update_mode` function L315-318 — `(&self, mode: PermissionMode)` — Hot-reload: update the permission mode.
- pub `check` function L331-338 — `( &self, tool_name: &str, tool_input: &str, category: PermissionCategory, ) -> P...` — Check if a tool call is permitted.
- pub `check_explained` function L343-416 — `( &self, tool_name: &str, tool_input: &str, category: PermissionCategory, ) -> (...` — Same as [`check`] but also returns *why* the decision was made.
- pub `mode` function L452-454 — `(&self) -> PermissionMode` — Get the current permission mode.
- pub `clear_grants` function L457-459 — `(&self)` — Clear all session grants.
-  `PermissionMode` type L30-62 — `= PermissionMode`
-  `ModalOption` type L79-91 — `= ModalOption`
-  `SessionGrants` type L117-136 — `= SessionGrants`
-  `DecisionReason` type L156-175 — `= DecisionReason`
-  `AUDIT_CAP` variable L202 — `: usize` — Cap on the audit ring buffer — newest decisions evict oldest.
-  `PermissionChecker` type L224-460 — `= PermissionChecker`
-  `record_audit` function L277-291 — `(&self, tool_name: &str, tool_input: &str, decision: PermissionDecision, reason:...`
-  `prompt_user` function L419-449 — `(&self, tool_name: &str, tool_input: &str) -> PermissionDecision` — Prompt the user for permission (or deny if no prompter is configured).
-  `truncate_input` function L462-470 — `(input: &str, max_len: usize) -> String`
-  `tests` module L473-919 — `-`
-  `MockPrompter` struct L478-480 — `{ index: Option<usize> }` — Mock prompter that returns a fixed index (0=AllowOnce, 1=AllowAlways, 2/None=Deny).
-  `MockPrompter` type L482-486 — `= MockPrompter`
-  `allow_once` function L483 — `() -> Self`
-  `allow_always` function L484 — `() -> Self`
-  `deny` function L485 — `() -> Self`
-  `MockPrompter` type L489-493 — `impl ModalPrompt for MockPrompter`
-  `prompt` function L490-492 — `(&self, _request: ModalRequest) -> Option<usize>`
-  `allowed_by_rule` function L496-503 — `()`
-  `denied_by_rule` function L506-513 — `()`
-  `ask_without_prompter_denies` function L516-523 — `()`
-  `ask_with_allow_once` function L526-535 — `()`
-  `ask_with_allow_always_grants_session` function L538-551 — `()`
-  `ask_with_deny` function L554-561 — `()`
-  `default_mode_allows_read_only` function L564-583 — `()`
-  `default_mode_asks_for_writes` function L586-601 — `()`
-  `accept_edits_mode_allows_file_ops` function L604-624 — `()`
-  `bypass_mode_allows_everything` function L627-645 — `()`
-  `explicit_rules_override_mode` function L648-656 — `()`
-  `deny_rules_override_session_grants` function L659-668 — `()`
-  `session_grant_works_for_non_denied_tools` function L671-680 — `()`
-  `clear_grants_resets` function L683-692 — `()`
-  `truncate_input_short` function L695-697 — `()`
-  `truncate_input_long` function L700-704 — `()`
-  `truncate_input_multibyte_utf8_no_panic` function L707-715 — `()`
-  `update_rules_hot_reload` function L718-739 — `()`
-  `update_mode_hot_reload` function L742-764 — `()`
-  `permission_mode_serde` function L767-776 — `()`
-  `plan_mode_allows_read_only` function L779-797 — `()`
-  `plan_mode_denies_writes` function L800-818 — `()`
-  `plan_mode_allows_plan_meta_tools` function L821-831 — `()`
-  `check_explained_attributes_deny_to_matching_rule` function L838-851 — `()`
-  `check_explained_attributes_no_match_to_mode_fallback` function L854-863 — `()`
-  `audit_log_records_decisions_in_order_and_caps` function L866-882 — `()`
-  `shared_audit_aggregates_across_checkers` function L885-901 — `()`
-  `snapshot_partitions_rules_by_kind_with_display_specs` function L904-918 — `()`

#### crates/arawn-engine/src/permissions/config.rs

- pub `PermissionConfig` struct L10-20 — `{ allow: Vec<String>, deny: Vec<String>, ask: Vec<String> }` — Permission configuration — holds allow/deny/ask rule lists.
- pub `into_rules` function L25-39 — `(&self) -> Vec<PermissionRule>` — Parse the string-based config into typed `PermissionRule` values.
- pub `merge` function L46-52 — `(self, other: PermissionConfig) -> PermissionConfig` — Merge two configs: `self` is higher priority (e.g., user-level),
- pub `PermissionsSection` struct L58-61 — `{ permissions: PermissionConfig }` — Wrapper for the permissions section in the top-level config.
- pub `load_permissions_from_file` function L65-83 — `(path: &std::path::Path) -> PermissionConfig` — Load permission config from a TOML file, returning defaults if the file
- pub `load_merged_permissions` function L88-101 — `( user_config_path: Option<&std::path::Path>, project_config_path: Option<&std::...` — Load and merge permission configs from user-level and project-level files.
-  `PermissionConfig` type L22-53 — `= PermissionConfig`
-  `tests` module L104-266 — `-`
-  `empty_config_produces_no_rules` function L110-113 — `()`
-  `config_parses_rules` function L116-135 — `()`
-  `merge_preserves_priority` function L138-163 — `()`
-  `load_from_toml_file` function L166-183 — `()`
-  `load_missing_file_returns_defaults` function L186-191 — `()`
-  `load_file_without_permissions_section` function L194-207 — `()`
-  `load_merged_both_sources` function L210-246 — `()`
-  `load_merged_missing_user_config` function L249-265 — `()`

#### crates/arawn-engine/src/permissions/mod.rs

-  `checker` module L7 — `-` — The permission system sits between the engine and tool execution, evaluating
-  `config` module L8 — `-` — (exact or glob) with optional content patterns.
-  `prompt` module L9 — `-` — (exact or glob) with optional content patterns.
-  `rules` module L10 — `-` — (exact or glob) with optional content patterns.

#### crates/arawn-engine/src/permissions/prompt.rs

- pub `CliModalPrompt` struct L9 — `-` — CLI-based modal prompt.
- pub `new` function L18-20 — `() -> Self`
- pub `MockModalPrompt` struct L68-71 — `{ responses: std::sync::Mutex<std::collections::VecDeque<Option<usize>>>, defaul...` — Mock modal prompt for tests.
- pub `always` function L75-80 — `(index: Option<usize>) -> Self` — Create a mock that always returns the given index.
- pub `with_responses` function L83-88 — `(responses: Vec<Option<usize>>, default: Option<usize>) -> Self` — Create a mock with queued responses.
-  `CliModalPrompt` type L11-15 — `impl Default for CliModalPrompt`
-  `default` function L12-14 — `() -> Self`
-  `CliModalPrompt` type L17-21 — `= CliModalPrompt`
-  `CliModalPrompt` type L24-65 — `impl ModalPrompt for CliModalPrompt`
-  `prompt` function L25-64 — `(&self, request: ModalRequest) -> Option<usize>`
-  `MockModalPrompt` type L73-89 — `= MockModalPrompt`
-  `MockModalPrompt` type L92-97 — `impl ModalPrompt for MockModalPrompt`
-  `prompt` function L93-96 — `(&self, _request: ModalRequest) -> Option<usize>`
-  `tests` module L100-141 — `-`
-  `test_request` function L104-114 — `() -> ModalRequest`
-  `mock_always_returns_index` function L117-121 — `()`
-  `mock_always_cancel` function L124-127 — `()`
-  `mock_queued_responses` function L130-140 — `()`

#### crates/arawn-engine/src/permissions/rules.rs

- pub `RuleKind` enum L6-10 — `Allow | Deny | Ask` — The kind of permission rule — what happens when it matches.
- pub `PermissionRule` struct L22-28 — `{ kind: RuleKind, tool_pattern: String, content_pattern: Option<String> }` — A single permission rule: a kind (allow/deny/ask), a tool name pattern,
- pub `new` function L31-37 — `(kind: RuleKind, tool_pattern: impl Into<String>) -> Self`
- pub `with_content` function L39-42 — `(mut self, pattern: impl Into<String>) -> Self`
- pub `parse` function L45-59 — `(kind: RuleKind, spec: &str) -> Self` — Parse a rule from the compact string format: `"ToolName"` or `"ToolName(content pattern)"`.
- pub `matches` function L62-70 — `(&self, tool_name: &str, tool_input: &str) -> bool` — Check if this rule matches a given tool name and input.
- pub `PermissionDecision` enum L75-84 — `Allowed | Denied | Ask | NoMatch` — The result of evaluating permission rules against a tool call.
- pub `RuleMatcher` struct L90 — `-` — Evaluates a list of permission rules against a tool call.
- pub `evaluate` function L96-102 — `( rules: &[PermissionRule], tool_name: &str, tool_input: &str, ) -> PermissionDe...` — Evaluate rules against a tool call.
- pub `evaluate_with_match` function L107-134 — `( rules: &[PermissionRule], tool_name: &str, tool_input: &str, ) -> (PermissionD...` — Evaluate rules and also return the rule that matched, when any did.
- pub `display_spec` function L140-145 — `(&self) -> String` — Compact human-readable form: `"shell(rm -rf *)"` or `"file_write"`.
-  `PermissionRule` type L30-71 — `= PermissionRule`
-  `RuleMatcher` type L92-135 — `= RuleMatcher`
-  `PermissionRule` type L137-146 — `= PermissionRule`
-  `glob_match` function L150-154 — `(pattern: &str, text: &str) -> bool` — Simple glob matching supporting `*` (any chars) and `?` (single char).
-  `glob_match_inner` function L156-184 — `(pat: &[char], txt: &[char]) -> bool`
-  `tests` module L187-392 — `-`
-  `glob_exact_match` function L193-196 — `()`
-  `glob_star_match` function L199-204 — `()`
-  `glob_question_mark` function L207-210 — `()`
-  `glob_complex_patterns` function L213-218 — `()`
-  `glob_content_patterns` function L221-226 — `()`
-  `rule_exact_tool_match` function L231-235 — `()`
-  `rule_glob_tool_match` function L238-243 — `()`
-  `rule_with_content_pattern` function L246-251 — `()`
-  `rule_parse_simple` function L254-258 — `()`
-  `rule_parse_with_content` function L261-265 — `()`
-  `rule_parse_nested_parens` function L268-273 — `()`
-  `matcher_deny_takes_priority` function L278-287 — `()`
-  `matcher_allow_before_ask` function L290-299 — `()`
-  `matcher_ask_when_only_ask_rule` function L302-308 — `()`
-  `matcher_no_match_when_no_rules` function L311-316 — `()`
-  `matcher_no_match_when_rules_dont_apply` function L319-325 — `()`
-  `matcher_content_pattern_deny` function L328-343 — `()`
-  `matcher_mixed_rules_realistic` function L346-391 — `()`

### crates/arawn-engine/src/plugins

**Role**: Plugin lifecycle management — discovery, manifest parsing, component loading (agents/skills/hooks/MCP servers), installation from marketplaces, enable/disable, and hot-reload.

**Key abstractions**:
- `PluginManifest` — Deserialized from `plugin.json`. Declares a plugin's name, version, author, component directories (agents, skills, commands, tools), MCP server definitions, inline or path-referenced hooks, and user-configurable fields with defaults.
- `LoadedPlugin` — A discovered, validated plugin ready for component extraction. Carries the manifest, the plugin directory path, `ResolvedPaths` (absolute paths for each component directory), and `PluginSource` (Cache, Inline, or BuiltIn). `enabled` defaults true; toggled by `apply_enable_disable` from settings.
- `PluginRegistry` — Concurrently-accessible `RwLock<HashMap<String, LoadedPlugin>>` keyed by `name@marketplace`. The key format is also how `unregister_by_prefix` removes all tools from a disconnecting plugin.
- `PluginRuntime` — The stateful coordinator for a running arawn instance. `load_all()` calls `discover_plugins`, registers builtins, loads components from each enabled plugin's directories, and merges them into the engine's skill registry, hook config, and MCP server list. `watch()` spawns a `notify` watcher on the cache directory that calls `load_all` again on any change.
- `load_plugin_components(plugin)` — Reads agents from the agents dir, skills from the skills dir, hooks from a JSON file or inline manifest, and extracts MCP server defs from the manifest. Returns a `PluginComponents` struct.
- `BuiltinPluginDef` / `builtin_plugins()` — Code-defined plugins that ship with the binary. The "core" built-in plugin contributes the default built-in skills. `register_builtin_plugins()` inserts them into the registry before disk plugins, allowing disk plugins to override by the same name.
- `InstalledPluginsRegistry` — Persists `installed_plugins.json` with install records (scope, path, version, timestamp). `install_plugin` fetches from a marketplace, clones into the versioned cache, and updates this file. `uninstall_plugin` removes and optionally deletes the cache.
- `PluginSettings` — Reads `settings.json` for per-plugin enabled/disabled flags and user config values. `apply_enable_disable` mutates loaded plugins. `validate_user_config` / `resolve_user_config` / `config_to_env_vars` handle the manifest `userConfig` schema.

**Internal flow**: Startup calls `PluginRuntime::load_all` which returns a `PluginLoadResult` containing the merged agents, skills, hooks, and MCP server defs. The main binary wires these into the skill registry, hook runner, and MCP manager. The `watch()` task repeats this on filesystem changes to the plugin cache.

**Dependencies**: `notify` (hot-reload), `serde_json` (manifests), `globwalk` (component directory scanning), git CLI (marketplace fetch).

#### crates/arawn-engine/src/plugins/builtin.rs

- pub `BuiltinPluginDef` struct L16-23 — `{ name: String, description: String, version: Option<String>, skills: Vec<SkillD...` — Definition for a built-in plugin (registered in code, not from disk).
- pub `into_loaded_plugin` function L27-42 — `(self) -> LoadedPlugin` — Convert this definition into a `LoadedPlugin` for the registry.
- pub `builtin_plugins` function L49-51 — `() -> Vec<(LoadedPlugin, BuiltinComponents)>` — Returns all built-in plugins.
- pub `BuiltinComponents` struct L54-58 — `{ skills: Vec<SkillDefinition>, hooks: Option<HookConfig>, agents: Vec<AgentDefi...` — Components from a built-in plugin (already loaded, no disk I/O needed).
- pub `register_builtin_plugins` function L85-98 — `( registry: &super::loader::PluginRegistry, ) -> Vec<BuiltinComponents>` — Register built-in plugins into the plugin registry alongside disk plugins.
-  `BuiltinPluginDef` type L25-43 — `= BuiltinPluginDef` — in the PluginRegistry.
-  `core_plugin` function L61-78 — `() -> (LoadedPlugin, BuiltinComponents)` — The "core" built-in plugin — ships default skills.
-  `tests` module L101-186 — `-` — in the PluginRegistry.
-  `builtin_plugin_converts_to_loaded` function L106-124 — `()` — in the PluginRegistry.
-  `builtin_plugins_exist` function L127-134 — `()` — in the PluginRegistry.
-  `register_into_registry` function L137-145 — `()` — in the PluginRegistry.
-  `disk_plugin_overrides_builtin` function L148-173 — `()` — in the PluginRegistry.
-  `disable_builtin_via_settings` function L176-185 — `()` — in the PluginRegistry.

#### crates/arawn-engine/src/plugins/components.rs

- pub `PluginComponents` struct L15-26 — `{ agents: Vec<AgentDefinition>, skills: Vec<SkillDefinition>, hooks: Option<Hook...` — Result of loading components from a single plugin.
- pub `load_plugin_components` function L32-123 — `(plugin: &LoadedPlugin) -> PluginComponents` — Load all components from a plugin into a `PluginComponents` struct.
- pub `register_plugin_skills` function L126-130 — `(registry: &SkillRegistry, skills: Vec<SkillDefinition>)` — Register a plugin's skills into a SkillRegistry.
- pub `merge_plugin_hooks` function L133-135 — `(target: &mut HookConfig, plugin_hooks: HookConfig)` — Merge a plugin's hooks into an existing HookConfig.
-  `tests` module L138-388 — `-` — from a plugin's declared directories into the engine's registries.
-  `make_plugin` function L145-157 — `(dir: &TempDir, name: &str, paths: ResolvedPaths) -> LoadedPlugin` — from a plugin's declared directories into the engine's registries.
-  `load_agents_from_plugin` function L160-189 — `()` — from a plugin's declared directories into the engine's registries.
-  `load_skills_from_plugin` function L192-223 — `()` — from a plugin's declared directories into the engine's registries.
-  `load_hooks_from_file_path` function L226-264 — `()` — from a plugin's declared directories into the engine's registries.
-  `load_inline_hooks` function L267-294 — `()` — from a plugin's declared directories into the engine's registries.
-  `mcp_servers_extracted` function L297-322 — `()` — from a plugin's declared directories into the engine's registries.
-  `missing_dir_produces_error_not_panic` function L325-341 — `()` — from a plugin's declared directories into the engine's registries.
-  `empty_plugin_loads_nothing` function L344-354 — `()` — from a plugin's declared directories into the engine's registries.
-  `register_skills_into_registry` function L357-372 — `()` — from a plugin's declared directories into the engine's registries.
-  `merge_hooks_into_config` function L375-387 — `()` — from a plugin's declared directories into the engine's registries.

#### crates/arawn-engine/src/plugins/installer.rs

- pub `InstallScope` enum L18-21 — `User | Project` — Installation scope — where the enablement is recorded.
- pub `InstallRecord` struct L26-33 — `{ scope: InstallScope, install_path: String, version: String, installed_at: Stri...` — A single installation record for a plugin at a specific scope.
- pub `InstalledPluginsRegistry` struct L37-40 — `{ version: u32, plugins: HashMap<String, Vec<InstallRecord>> }` — The installed_plugins.json registry.
- pub `load` function L53-61 — `(path: &Path) -> Self` — Load from a JSON file.
- pub `save` function L64-70 — `(&self, path: &Path) -> Result<(), String>` — Save to a JSON file.
- pub `add` function L73-77 — `(&mut self, id: &str, record: InstallRecord)` — Add an installation record.
- pub `remove` function L81-90 — `(&mut self, id: &str, scope: &InstallScope) -> bool` — Remove all records for a plugin at a specific scope.
- pub `get` function L93-95 — `(&self, id: &str) -> Option<&Vec<InstallRecord>>` — Get records for a plugin.
- pub `install_plugin` function L105-183 — `( identifier: &PluginIdentifier, scope: InstallScope, plugins_root: &Path, proje...` — Install a plugin from a marketplace into the versioned cache.
- pub `uninstall_plugin` function L186-212 — `( identifier: &PluginIdentifier, scope: InstallScope, plugins_root: &Path, remov...` — Uninstall a plugin — remove from registry, optionally remove cache.
-  `InstalledPluginsRegistry` type L42-49 — `impl Default for InstalledPluginsRegistry` — and track installations in installed_plugins.json.
-  `default` function L43-48 — `() -> Self` — and track installations in installed_plugins.json.
-  `InstalledPluginsRegistry` type L51-96 — `= InstalledPluginsRegistry` — and track installations in installed_plugins.json.
-  `clone_plugin_to_cache` function L215-322 — `( plugin: &MarketplacePlugin, market_source: &super::marketplace::MarketplaceSou...` — Clone a plugin's source into the cache directory.
-  `copy_dir_recursive` function L325-343 — `(src: &Path, dst: &Path) -> Result<(), String>` — Recursively copy a directory's contents.
-  `tests` module L346-508 — `-` — and track installations in installed_plugins.json.
-  `registry_roundtrip` function L351-375 — `()` — and track installations in installed_plugins.json.
-  `registry_replace_same_scope` function L378-404 — `()` — and track installations in installed_plugins.json.
-  `registry_multiple_scopes` function L407-432 — `()` — and track installations in installed_plugins.json.
-  `registry_remove_one_scope` function L435-461 — `()` — and track installations in installed_plugins.json.
-  `registry_remove_last_scope` function L464-480 — `()` — and track installations in installed_plugins.json.
-  `registry_load_missing` function L483-487 — `()` — and track installations in installed_plugins.json.
-  `copy_dir_skips_git` function L490-507 — `()` — and track installations in installed_plugins.json.

#### crates/arawn-engine/src/plugins/loader.rs

- pub `PluginIdentifier` struct L15-18 — `{ name: String, marketplace: String }` — Plugin identifier in `name@marketplace` format.
- pub `new` function L21-26 — `(name: impl Into<String>, marketplace: impl Into<String>) -> Self` — Plugin discovery and loading — scans directories for plugin.json manifests.
- pub `parse` function L29-38 — `(s: &str) -> Option<Self>` — Parse from `name@marketplace` string.
- pub `inline` function L41-46 — `(name: impl Into<String>) -> Self` — For inline/session plugins loaded via --plugin-dir.
- pub `PluginSource` enum L57-64 — `Cache | Inline | BuiltIn` — Source of a loaded plugin.
- pub `LoadedPlugin` struct L68-81 — `{ id: PluginIdentifier, manifest: PluginManifest, plugin_dir: PathBuf, source: P...` — A discovered and validated plugin ready for component loading.
- pub `ResolvedPaths` struct L85-91 — `{ agents: Option<PathBuf>, skills: Option<PathBuf>, commands: Option<PathBuf>, t...` — Resolved absolute paths for plugin component directories.
- pub `name` function L95-97 — `(&self) -> &str` — Plugin name (convenience accessor).
- pub `discover_plugins` function L104-163 — `(plugins_root: &Path) -> Vec<LoadedPlugin>` — Discover plugins from the versioned cache directory.
- pub `load_plugin_dir` function L168-174 — `(dir: &Path) -> Option<LoadedPlugin>` — Load a single plugin from a directory (for --plugin-dir flag).
- pub `PluginRegistry` struct L267-269 — `{ plugins: RwLock<HashMap<String, LoadedPlugin>> }` — Registry of loaded plugins, queryable by identifier string.
- pub `new` function L278-282 — `() -> Self` — Plugin discovery and loading — scans directories for plugin.json manifests.
- pub `register` function L285-288 — `(&self, plugin: LoadedPlugin)` — Register a loaded plugin (keyed by id string: `name@marketplace`).
- pub `get` function L292-307 — `(&self, key: &str) -> Option<LoadedPlugin>` — Get a plugin by identifier string (e.g.
- pub `all` function L310-312 — `(&self) -> Vec<LoadedPlugin>` — Get all registered plugins.
- pub `enabled` function L315-323 — `(&self) -> Vec<LoadedPlugin>` — Get only enabled plugins.
- pub `len` function L325-327 — `(&self) -> usize` — Plugin discovery and loading — scans directories for plugin.json manifests.
- pub `is_empty` function L329-331 — `(&self) -> bool` — Plugin discovery and loading — scans directories for plugin.json manifests.
- pub `set_enabled` function L334-338 — `(&self, key: &str, enabled: bool)` — Set enable/disable state by identifier string.
-  `PluginIdentifier` type L20-47 — `= PluginIdentifier` — Plugin discovery and loading — scans directories for plugin.json manifests.
-  `PluginIdentifier` type L49-53 — `= PluginIdentifier` — Plugin discovery and loading — scans directories for plugin.json manifests.
-  `fmt` function L50-52 — `(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` — Plugin discovery and loading — scans directories for plugin.json manifests.
-  `LoadedPlugin` type L93-98 — `= LoadedPlugin` — Plugin discovery and loading — scans directories for plugin.json manifests.
-  `load_plugin_from_dir` function L177-218 — `( dir: &Path, default_name: &str, marketplace: &str, source: PluginSource, ) -> ...` — Load a plugin from a directory, reading .claude-plugin/plugin.json or plugin.json.
-  `resolve_paths` function L226-264 — `(manifest: &PluginManifest, plugin_dir: &Path) -> ResolvedPaths` — Resolve relative component paths against the plugin directory.
-  `PluginRegistry` type L271-275 — `impl Default for PluginRegistry` — Plugin discovery and loading — scans directories for plugin.json manifests.
-  `default` function L272-274 — `() -> Self` — Plugin discovery and loading — scans directories for plugin.json manifests.
-  `PluginRegistry` type L277-339 — `= PluginRegistry` — Plugin discovery and loading — scans directories for plugin.json manifests.
-  `tests` module L342-467 — `-` — Plugin discovery and loading — scans directories for plugin.json manifests.
-  `write_cached_plugin` function L347-352 — `(root: &Path, marketplace: &str, name: &str, version: &str, extra: &str)` — Create a cache-structured plugin: cache/{marketplace}/{plugin}/{version}/plugin.json
-  `write_claude_plugin` function L355-361 — `(root: &Path, marketplace: &str, name: &str, version: &str)` — Create a .claude-plugin/plugin.json style plugin.
-  `discover_from_cache` function L364-375 — `()` — Plugin discovery and loading — scans directories for plugin.json manifests.
-  `latest_version_wins` function L378-386 — `()` — Plugin discovery and loading — scans directories for plugin.json manifests.
-  `claude_plugin_path_discovered` function L389-397 — `()` — Plugin discovery and loading — scans directories for plugin.json manifests.
-  `missing_cache_dir_returns_empty` function L400-403 — `()` — Plugin discovery and loading — scans directories for plugin.json manifests.
-  `load_plugin_dir_inline` function L406-414 — `()` — Plugin discovery and loading — scans directories for plugin.json manifests.
-  `identifier_parse_display` function L417-422 — `()` — Plugin discovery and loading — scans directories for plugin.json manifests.
-  `identifier_parse_invalid` function L425-429 — `()` — Plugin discovery and loading — scans directories for plugin.json manifests.
-  `registry_keyed_by_id` function L432-448 — `()` — Plugin discovery and loading — scans directories for plugin.json manifests.
-  `registry_enable_disable` function L451-466 — `()` — Plugin discovery and loading — scans directories for plugin.json manifests.

#### crates/arawn-engine/src/plugins/manifest.rs

- pub `PluginManifest` struct L15-60 — `{ name: String, version: Option<String>, description: Option<String>, author: Op...` — A plugin manifest loaded from `plugin.json`.
- pub `PluginAuthor` struct L64-70 — `{ name: String, email: Option<String>, url: Option<String> }` — Author information for a plugin.
- pub `McpServerDef` struct L77-83 — `{ command: String, args: Vec<String>, env: HashMap<String, String> }` — MCP server definition within a plugin manifest.
- pub `UserConfigField` struct L87-103 — `{ field_type: String, title: Option<String>, description: Option<String>, requir...` — A user-configurable field declared in the plugin manifest.
- pub `HooksField` enum L107-112 — `Inline | Path` — The `hooks` field can be either an inline HookConfig or a path string.
- pub `PluginError` enum L136-143 — `MissingField | InvalidPath | ParseError` — Structured error from manifest validation.
- pub `from_json` function L159-161 — `(json: &str) -> Result<Self, PluginError>` — Load a manifest from a JSON string.
- pub `from_file` function L164-168 — `(path: &std::path::Path) -> Result<Self, PluginError>` — Load a manifest from a file path.
- pub `from_dir` function L174-187 — `(dir: &std::path::Path) -> Result<Self, PluginError>` — Load a manifest from a plugin directory.
- pub `validate` function L190-217 — `(&self) -> Vec<PluginError>` — Validate the manifest and return any errors found.
-  `deserialize_hooks_field` function L114-132 — `(deserializer: D) -> Result<Option<HooksField>, D::Error>` — Plugin manifest — deserialization and validation of plugin.json.
-  `PluginError` type L145-155 — `= PluginError` — Plugin manifest — deserialization and validation of plugin.json.
-  `fmt` function L146-154 — `(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` — Plugin manifest — deserialization and validation of plugin.json.
-  `PluginManifest` type L157-236 — `= PluginManifest` — Plugin manifest — deserialization and validation of plugin.json.
-  `component_paths` function L220-235 — `(&self) -> Vec<(&str, &str)>` — Get all component path fields that are set.
-  `tests` module L239-423 — `-` — Plugin manifest — deserialization and validation of plugin.json.
-  `parse_full_manifest` function L243-285 — `()` — Plugin manifest — deserialization and validation of plugin.json.
-  `parse_minimal_manifest` function L288-296 — `()` — Plugin manifest — deserialization and validation of plugin.json.
-  `parse_hooks_inline` function L299-318 — `()` — Plugin manifest — deserialization and validation of plugin.json.
-  `parse_hooks_path` function L321-325 — `()` — Plugin manifest — deserialization and validation of plugin.json.
-  `validate_missing_name` function L328-335 — `()` — Plugin manifest — deserialization and validation of plugin.json.
-  `validate_invalid_paths` function L338-348 — `()` — Plugin manifest — deserialization and validation of plugin.json.
-  `validate_invalid_hooks_path` function L351-360 — `()` — Plugin manifest — deserialization and validation of plugin.json.
-  `validate_valid_manifest` function L363-373 — `()` — Plugin manifest — deserialization and validation of plugin.json.
-  `parse_error_on_invalid_json` function L376-379 — `()` — Plugin manifest — deserialization and validation of plugin.json.
-  `mcp_server_with_env` function L382-401 — `()` — Plugin manifest — deserialization and validation of plugin.json.
-  `user_config_with_default` function L404-422 — `()` — Plugin manifest — deserialization and validation of plugin.json.

#### crates/arawn-engine/src/plugins/marketplace.rs

- pub `MarketplaceSource` enum L16-34 — `GitHub | Git | Directory` — Source type for a marketplace.
- pub `git_url` function L38-46 — `(&self) -> Option<String>` — Get the git clone URL for this source.
- pub `git_ref` function L49-55 — `(&self) -> Option<&str>` — Get the git ref (branch/tag) to checkout.
- pub `MarketplaceManifest` struct L60-69 — `{ name: String, plugins: Vec<MarketplacePlugin>, metadata: Option<MarketplaceMet...` — A marketplace manifest (marketplace.json) — lists available plugins.
- pub `MarketplacePlugin` struct L73-87 — `{ name: String, version: Option<String>, description: Option<String>, source: Op...` — A plugin entry in a marketplace manifest.
- pub `PluginSourceRef` enum L92-115 — `RelativePath | GitHub | Git` — Reference to a plugin's source within a marketplace.
- pub `GithubSourceTag` enum L119-121 — `Github` — available plugins with their sources and versions.
- pub `GitSourceTag` enum L125-127 — `Git` — available plugins with their sources and versions.
- pub `relative_path` function L131-136 — `(&self) -> Option<&str>` — Get the relative path within the marketplace repo, if this is a relative path source.
- pub `MarketplaceMetadata` struct L157-162 — `{ version: Option<String>, description: Option<String> }` — Marketplace metadata.
- pub `MarketplaceEntry` struct L166-172 — `{ source: MarketplaceSource, install_location: Option<String>, last_updated: Opt...` — Entry in known_marketplaces.json.
- pub `KnownMarketplaces` struct L176-179 — `{ entries: HashMap<String, MarketplaceEntry> }` — Known marketplaces registry — read/write `known_marketplaces.json`.
- pub `load` function L183-191 — `(path: &Path) -> Self` — Load from a JSON file.
- pub `save` function L194-200 — `(&self, path: &Path) -> Result<(), String>` — Save to a JSON file.
- pub `add` function L203-205 — `(&mut self, name: String, entry: MarketplaceEntry)` — Add or update a marketplace entry.
- pub `get` function L208-210 — `(&self, name: &str) -> Option<&MarketplaceEntry>` — Get a marketplace entry by name.
- pub `names` function L213-215 — `(&self) -> Vec<&str>` — List all marketplace names.
- pub `fetch_marketplace` function L221-248 — `( source: &MarketplaceSource, name: &str, marketplaces_dir: &Path, ) -> Result<M...` — Fetch a marketplace manifest by cloning/pulling a git repo.
- pub `add_marketplace` function L251-279 — `( name: &str, source: MarketplaceSource, plugins_root: &Path, ) -> Result<Market...` — Add a marketplace source: fetch it and register in known_marketplaces.json.
- pub `list_marketplaces` function L282-297 — `( plugins_root: &Path, ) -> Vec<(String, MarketplaceEntry, Option<MarketplaceMan...` — List all marketplaces and their available plugins.
- pub `resolve_plugin` function L300-305 — `( manifest: &'a MarketplaceManifest, plugin_name: &str, ) -> Option<&'a Marketpl...` — Find a plugin entry in a marketplace manifest by name.
-  `MarketplaceSource` type L36-56 — `= MarketplaceSource` — available plugins with their sources and versions.
-  `PluginSourceRef` type L129-137 — `= PluginSourceRef` — available plugins with their sources and versions.
-  `deserialize_plugin_source` function L139-153 — `(deserializer: D) -> Result<Option<PluginSourceRef>, D::Error>` — available plugins with their sources and versions.
-  `KnownMarketplaces` type L181-216 — `= KnownMarketplaces` — available plugins with their sources and versions.
-  `read_marketplace_manifest` function L310-327 — `(dir: &Path) -> Result<MarketplaceManifest, String>` — Read a marketplace manifest from a directory.
-  `git_clone` function L330-348 — `(url: &str, target: &Path, git_ref: Option<&str>) -> Result<(), String>` — Clone a git repo to a directory.
-  `git_pull` function L351-375 — `(dir: &Path, git_ref: Option<&str>) -> Result<(), String>` — Pull latest changes in an existing clone.
-  `tests` module L378-552 — `-` — available plugins with their sources and versions.
-  `write_marketplace` function L382-385 — `(dir: &Path, json: &str)` — available plugins with their sources and versions.
-  `sample_manifest_json` function L387-408 — `() -> &'static str` — available plugins with their sources and versions.
-  `parse_marketplace_manifest` function L411-419 — `()` — available plugins with their sources and versions.
-  `read_manifest_from_root` function L422-428 — `()` — available plugins with their sources and versions.
-  `read_manifest_from_claude_plugin_dir` function L431-438 — `()` — available plugins with their sources and versions.
-  `read_manifest_missing` function L441-445 — `()` — available plugins with their sources and versions.
-  `resolve_plugin_found` function L448-454 — `()` — available plugins with their sources and versions.
-  `resolve_plugin_not_found` function L457-461 — `()` — available plugins with their sources and versions.
-  `fetch_from_directory_source` function L464-474 — `()` — available plugins with their sources and versions.
-  `known_marketplaces_roundtrip` function L477-499 — `()` — available plugins with their sources and versions.
-  `known_marketplaces_missing_file` function L502-505 — `()` — available plugins with their sources and versions.
-  `marketplace_source_git_url` function L508-532 — `()` — available plugins with their sources and versions.
-  `plugin_source_ref_deserialization` function L535-551 — `()` — available plugins with their sources and versions.

#### crates/arawn-engine/src/plugins/mod.rs

-  `builtin` module L7 — `-` — Plugins are directories with a `plugin.json` manifest that declares what
-  `components` module L8 — `-` — loads plugins from `~/.arawn/plugins/` and `.arawn/plugins/`.
-  `installer` module L9 — `-` — loads plugins from `~/.arawn/plugins/` and `.arawn/plugins/`.
-  `loader` module L10 — `-` — loads plugins from `~/.arawn/plugins/` and `.arawn/plugins/`.
-  `manifest` module L11 — `-` — loads plugins from `~/.arawn/plugins/` and `.arawn/plugins/`.
-  `marketplace` module L12 — `-` — loads plugins from `~/.arawn/plugins/` and `.arawn/plugins/`.
-  `runtime` module L13 — `-` — loads plugins from `~/.arawn/plugins/` and `.arawn/plugins/`.
-  `settings` module L14 — `-` — loads plugins from `~/.arawn/plugins/` and `.arawn/plugins/`.

#### crates/arawn-engine/src/plugins/runtime.rs

- pub `PluginMcpServer` struct L26-32 — `{ name: String, command: String, args: Vec<String>, env: std::collections::HashM...` — An MCP server config extracted from a plugin manifest, ready for connection.
- pub `PluginLoadResult` struct L35-40 — `{ agents: Vec<AgentDefinition>, skills: Vec<SkillDefinition>, hooks: HookConfig,...` — Result of loading all plugins — the components ready to wire into the engine.
- pub `PluginRuntime` struct L43-52 — `{ plugins_root: PathBuf, settings_path: Option<PathBuf>, plugin_dirs: Vec<PathBu...` — Plugin runtime — manages plugin lifecycle for a running arawn instance.
- pub `new` function L55-62 — `(plugins_root: PathBuf) -> Self` — to hot-reload when plugins are installed or changed.
- pub `with_settings` function L64-67 — `(mut self, path: PathBuf) -> Self` — to hot-reload when plugins are installed or changed.
- pub `with_plugin_dir` function L69-72 — `(mut self, dir: PathBuf) -> Self` — to hot-reload when plugins are installed or changed.
- pub `load_all` function L75-162 — `(&self, skill_registry: &Arc<SkillRegistry>) -> PluginLoadResult` — Discover, load, and register all plugins.
- pub `watch` function L173-298 — `( &self, skill_registry: Arc<SkillRegistry>, notify: Option<Arc<dyn Fn(bool, Str...` — Spawn a file watcher that hot-reloads plugins when the cache directory changes.
-  `PluginRuntime` type L54-299 — `= PluginRuntime` — to hot-reload when plugins are installed or changed.

#### crates/arawn-engine/src/plugins/settings.rs

- pub `PluginSettings` struct L32-40 — `{ enabled_plugins: HashMap<String, bool>, plugin_configs: HashMap<String, Plugin...` — Plugin settings section from `.arawn/settings.json`.
- pub `PluginConfigEntry` struct L44-48 — `{ options: HashMap<String, serde_json::Value> }` — Per-plugin user configuration entry.
- pub `load_plugin_settings` function L51-69 — `(path: &Path) -> PluginSettings` — Load plugin settings from a JSON settings file.
- pub `apply_enable_disable` function L75-88 — `(plugins: &mut [LoadedPlugin], settings: &PluginSettings)` — Apply enable/disable settings to a list of loaded plugins.
- pub `validate_user_config` function L93-113 — `( plugin_name: &str, declarations: &HashMap<String, UserConfigField>, values: &H...` — Validate user config values against the plugin manifest's `userConfig` declarations.
- pub `resolve_user_config` function L116-131 — `( declarations: &HashMap<String, UserConfigField>, values: &HashMap<String, serd...` — Get resolved user config values for a plugin, applying defaults.
- pub `config_to_env_vars` function L136-151 — `( config: &HashMap<String, serde_json::Value>, ) -> HashMap<String, String>` — Convert resolved user config values to environment variables.
- pub `substitute_user_config` function L154-165 — `(template: &str, config: &HashMap<String, serde_json::Value>) -> String` — Substitute `${user_config.KEY}` placeholders in a string with resolved values.
-  `tests` module L168-403 — `-` — applies them to loaded plugins.
-  `make_plugin` function L174-186 — `(name: &str, marketplace: &str) -> LoadedPlugin` — applies them to loaded plugins.
-  `default_all_enabled` function L189-196 — `()` — applies them to loaded plugins.
-  `disable_by_id` function L199-216 — `()` — applies them to loaded plugins.
-  `disable_by_name_fallback` function L219-230 — `()` — applies them to loaded plugins.
-  `validate_missing_required` function L233-261 — `()` — applies them to loaded plugins.
-  `validate_all_present` function L264-282 — `()` — applies them to loaded plugins.
-  `resolve_with_defaults` function L285-313 — `()` — applies them to loaded plugins.
-  `resolve_value_overrides_default` function L316-334 — `()` — applies them to loaded plugins.
-  `config_to_env` function L337-345 — `()` — applies them to loaded plugins.
-  `substitute_placeholders` function L348-356 — `()` — applies them to loaded plugins.
-  `substitute_no_match_left_alone` function L359-363 — `()` — applies them to loaded plugins.
-  `load_settings_from_json` function L366-395 — `()` — applies them to loaded plugins.
-  `load_missing_settings_returns_defaults` function L398-402 — `()` — applies them to loaded plugins.

### crates/arawn-engine/src/skills

**Role**: Skills are reusable prompt-based workflows stored as markdown files with YAML frontmatter. This module handles parsing, discovery, and registry — they are invoked via the `SkillTool` which injects the skill's prompt into the conversation.

**Key abstractions**:
- `SkillDefinition` — Parsed from a `.md` file: `name`, `description`, `prompt` (body), `argument_hint`, `model` (optional preferred LLM), `user_invocable`, `tools` (optional allowlist). YAML frontmatter is hand-parsed with simple key extraction (no full YAML library dependency).
- `SkillSource` — `Project`, `User`, `Plugin`, or `BuiltIn`. Affects precedence: project overrides user in `load_merged_skills`.
- `SkillRegistry` — Concurrent `RwLock<HashMap<String, SkillDefinition>>` with case-insensitive lookup. `register_builtins()` is called at construction to add the built-in "workflows" skill. `format_skill_listing()` renders a token-budget-aware listing for inclusion in the system prompt.
- `load_skills_dir(dir, source)` — Scans a directory (and one level of subdirectories) for `.md` files and parses each. Malformed files are skipped with a warning.
- `load_merged_skills(project_dir, user_dir)` — Loads project skills first, then user skills, with project taking precedence on name collision.

**Internal flow**: At startup, the main binary builds a `SkillRegistry`, loads project and user skills, then plugin skills are added via `register_plugin_skills` after plugin loading. The `SkillTool` looks up skills by name and returns the prompt text as the tool result, which the LLM then uses as its next instruction.

**Dependencies**: Standard library only (no external parsing crate for YAML).

#### crates/arawn-engine/src/skills/definition.rs

- pub `SkillDefinition` struct L8-30 — `{ name: String, description: String, prompt: String, argument_hint: Option<Strin...` — A skill definition loaded from a markdown file with YAML frontmatter.
- pub `SkillSource` enum L37-47 — `Project | User | Plugin | BuiltIn`
- pub `parse_skill_markdown` function L53-81 — `(content: &str, default_name: &str) -> Result<SkillDefinition, String>` — Parse a skill definition from a markdown file's content.
-  `default_true` function L32-34 — `() -> bool`
-  `split_frontmatter` function L84-96 — `(content: &str) -> Option<(String, String)>` — Split content into frontmatter and body at `---` delimiters.
-  `extract_field` function L99-116 — `(frontmatter: &str, key: &str) -> Option<String>` — Extract a simple `key: value` field from YAML frontmatter.
-  `extract_list_field` function L119-163 — `(frontmatter: &str, key: &str) -> Option<Vec<String>>` — Extract a YAML list field (either inline `[a, b]` or multi-line `- a\n- b`).
-  `tests` module L166-303 — `-`
-  `parse_minimal_skill` function L170-184 — `()`
-  `parse_full_skill` function L187-214 — `()`
-  `parse_inline_array` function L217-230 — `()`
-  `parse_model_inherit` function L233-243 — `()`
-  `parse_user_invocable_false` function L246-256 — `()`
-  `parse_missing_description_errors` function L259-269 — `()`
-  `parse_no_frontmatter_errors` function L272-275 — `()`
-  `name_from_frontmatter_overrides_default` function L278-288 — `()`
-  `split_frontmatter_works` function L291-295 — `()`
-  `extract_list_multiline` function L298-302 — `()`

#### crates/arawn-engine/src/skills/loader.rs

- pub `SkillRegistry` struct L10-12 — `{ skills: RwLock<HashMap<String, SkillDefinition>> }` — Registry of loaded skills, queryable by name.
- pub `new` function L21-27 — `() -> Self`
- pub `register` function L49-52 — `(&self, skill: SkillDefinition)` — Register a skill.
- pub `get` function L55-67 — `(&self, name: &str) -> Option<SkillDefinition>` — Look up a skill by name (case-insensitive).
- pub `all` function L70-72 — `(&self) -> Vec<SkillDefinition>` — Get all registered skills.
- pub `user_invocable` function L75-83 — `(&self) -> Vec<SkillDefinition>` — Get only user-invocable skills.
- pub `len` function L86-88 — `(&self) -> usize` — Number of registered skills.
- pub `is_empty` function L90-92 — `(&self) -> bool`
- pub `load_skills_dir` function L100-136 — `(dir: &Path, source: SkillSource) -> Vec<SkillDefinition>` — Load skill definitions from a directory.
- pub `load_merged_skills` function L163-184 — `( project_dir: Option<&Path>, user_dir: Option<&Path>, ) -> SkillRegistry` — Load and merge skills from project and user directories.
- pub `format_skill_listing` function L190-226 — `(skills: &[SkillDefinition], budget_chars: usize, max_desc_chars: usize) -> Stri...` — Format skill listing for the system prompt, respecting a character budget.
-  `SkillRegistry` type L14-18 — `impl Default for SkillRegistry`
-  `default` function L15-17 — `() -> Self`
-  `SkillRegistry` type L20-93 — `= SkillRegistry`
-  `register_builtins` function L30-46 — `(&self)` — Register built-in skills that ship with the arawn binary.
-  `load_skill_file` function L138-158 — `(path: &Path, default_name: &str, source: SkillSource) -> Option<SkillDefinition...`
-  `tests` module L229-455 — `-`
-  `load_skills_from_files` function L234-264 — `()`
-  `load_skill_from_subdirectory` function L267-285 — `()`
-  `project_overrides_user` function L288-317 — `()`
-  `registry_case_insensitive_lookup` function L320-336 — `()`
-  `empty_dir_returns_no_skills` function L339-343 — `()`
-  `nonexistent_dir_returns_no_skills` function L346-349 — `()`
-  `format_listing_basic` function L352-379 — `()`
-  `format_listing_truncates_description` function L382-398 — `()`
-  `format_listing_respects_budget` function L401-417 — `()`
-  `format_listing_empty` function L420-423 — `()`
-  `user_invocable_filter` function L426-454 — `()`

#### crates/arawn-engine/src/skills/mod.rs

-  `definition` module L7 — `-` — Skills are markdown files with YAML frontmatter that define prompt templates
-  `loader` module L8 — `-` — execute a skill, which injects the skill's prompt into the conversation.

### crates/arawn-engine/src/tools

**Role**: Concrete `Tool` implementations registered into the engine — each wraps engine or system capabilities and exposes them as JSON-schema-documented functions the LLM can call.

**Key abstractions**:
- File tools (`FileReadTool`, `FileWriteTool`, `FileEditTool`) — Operate within the validated working directory. `FileReadTool` marks files as read in `EngineToolContext`; `FileWriteTool` and `FileEditTool` require a prior read of the same file (enforced via `has_read_file`) for existing files to prevent blind overwrites. Both write tools declare `permission_category() = FileWrite`. All three check `sensitive_paths::is_sensitive_path` and `is_secret_file` before access.
- `ShellTool` — Runs commands in an OS-level sandbox (via the `sandbox` crate) that restricts filesystem access to the working directory and `/tmp`, and blocks network access except for binaries in `network_tools`. Passes only a filtered environment (`safe_env()`) to prevent secret leakage. Supports `run_in_background` which hands off to `BackgroundTaskManager`. Declares `permission_category() = Shell`.
- `GrepTool` / `GlobTool` — Search tools that fall back gracefully (grep → system grep if rg unavailable). Both are `ReadOnly`. `GrepTool` supports multiple output modes (content, files_with_matches, count) and a `head_limit` cap.
- `AgentTool` — Spawns a sub-agent `QueryEngine` scoped to a specific agent definition. Resolves LLM preference via the context's `LlmResolverFn`. Depth-limited by `MAX_AGENT_DEPTH`.
- `MemoryStoreTool` / `MemorySearchTool` — Bridge to `MemoryManager`. Store does search-before-create deduplication. Search uses composite FTS5 + optional vector scoring with `ScoredEntity::composite()` ranking.
- `TaskCreateTool`, `TaskUpdateTool`, `TaskListTool`, `TaskGetTool` — Share a `SessionTaskStore` (in-memory `RwLock<HashMap>`) for session-scoped task tracking. Tasks survive tool calls within a session but not across sessions.
- `WebFetchTool` — Fetches URLs, converts HTML to markdown via `htmd`, caches results for 15 minutes (LRU, 100 entries), optionally summarizes with an LLM if a `prompt` parameter is provided.
- `safe_env` / `sensitive_paths` — Supporting modules. `safe_env()` returns a whitelist-filtered copy of the process environment. `sensitive_paths` defines the directory and filename deny list enforced by file tools and the shell sandbox.
- Plan mode tools (`EnterPlanModeTool`, `ExitPlanModeTool`) — Mutate `PlanModeState`. Both declare `is_read_only() = true` so they are permitted in plan mode itself (allowing the agent to exit plan mode it just entered).

**Mixed concerns / gotchas**: `sensitive_paths.rs` and `safe_env.rs` are shared between file tools and the shell tool. The `is_secret_file` deny list includes patterns like `.env`, `*.pem`, `id_rsa`, but explicitly allows `*.env.rs` (Rust env files) to avoid false positives. The `tool_result_limiter` truncates results exceeding `DEFAULT_MAX_RESULT_SIZE_CHARS` and persists the full content to a temp file under `data_dir`.

**Dependencies**: `arawn-tool` (Tool trait, PermissionCategory), `arawn-engine` context, `arawn-memory`, `arawn-embed`, `arawn-workflow`; `globwalk` (glob), `sandbox` (shell), `htmd` (HTML-to-markdown), `lru` (web fetch cache).

#### crates/arawn-engine/src/tools/agent.rs

- pub `AgentTool` struct L28-32 — `{ registry: Arc<ToolRegistry>, definitions: Vec<AgentDefinition>, bg_manager: Op...` — Spawns a sub-agent that runs a full `QueryEngine` loop in an isolated
- pub `new` function L35-41 — `(registry: Arc<ToolRegistry>, definitions: Vec<AgentDefinition>) -> Self`
- pub `with_background_manager` function L44-47 — `(mut self, mgr: Arc<BackgroundTaskManager>) -> Self` — Attach a background task manager for `run_in_background` support.
-  `DEFAULT_MAX_TURNS` variable L20 — `: usize`
-  `AgentTool` type L34-48 — `= AgentTool`
-  `AgentTool` type L51-300 — `impl Tool for AgentTool`
-  `name` function L52-54 — `(&self) -> &str`
-  `description` function L56-75 — `(&self) -> &str`
-  `category` function L77-79 — `(&self) -> ToolCategory`
-  `parameters_schema` function L81-108 — `(&self) -> Value`
-  `execute` function L110-299 — `(&self, ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput, ...`
-  `tests` module L303-563 — `-`
-  `test_ctx_with_mock` function L312-321 — `( responses: Vec<MockResponse>, ) -> (EngineToolContext, Arc<MockLlmClient>, Arc...`
-  `schema_is_valid` function L324-333 — `()`
-  `text_only_sub_agent` function L336-353 — `()`
-  `test_resolver` function L358-382 — `( named_client: Arc<dyn arawn_llm::LlmClient>, named_model: String, named_key: S...` — Build a test resolver closure that returns `named_client` for
-  `sub_agent_uses_resolved_llm_preference` function L385-415 — `()`
-  `sub_agent_falls_back_to_parent_llm_when_resolution_unavailable` function L418-435 — `()`
-  `sub_agent_with_tool_call` function L438-455 — `()`
-  `sub_agent_no_llm_errors` function L458-467 — `()`
-  `sub_agent_max_iterations_returns_last_text` function L470-492 — `()`
-  `depth_limit_prevents_infinite_recursion` function L495-509 — `()`
-  `explore_agent_type_used` function L512-528 — `()`
-  `unknown_type_falls_back_to_general` function L531-545 — `()`
-  `for_sub_agent_increments_depth` function L548-562 — `()`

#### crates/arawn-engine/src/tools/ask_user.rs

- pub `AskUserTool` struct L11 — `-` — Asks the user structured multiple-choice questions to gather requirements
-  `AskUserTool` type L14-137 — `impl Tool for AskUserTool`
-  `name` function L15-17 — `(&self) -> &str`
-  `description` function L19-28 — `(&self) -> &str`
-  `is_read_only` function L30-32 — `(&self) -> bool`
-  `category` function L34-36 — `(&self) -> ToolCategory`
-  `parameters_schema` function L38-83 — `(&self) -> Value`
-  `execute` function L85-136 — `(&self, _ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput,...`
-  `tests` module L140-253 — `-`
-  `test_ctx` function L147-150 — `() -> EngineToolContext`
-  `schema_is_valid` function L153-160 — `()`
-  `is_read_only` function L163-165 — `()`
-  `single_question` function L168-192 — `()`
-  `multi_select_shows_hint` function L195-216 — `()`
-  `multiple_questions` function L219-244 — `()`
-  `empty_questions_errors` function L247-252 — `()`

#### crates/arawn-engine/src/tools/enter_plan_mode.rs

- pub `EnterPlanModeTool` struct L12-14 — `{ plan_state: Arc<PlanModeState> }` — Tool that enters plan mode — restricts the agent to observation-only tools
- pub `new` function L17-19 — `(plan_state: Arc<PlanModeState>) -> Self`
-  `EnterPlanModeTool` type L16-20 — `= EnterPlanModeTool`
-  `EnterPlanModeTool` type L23-92 — `impl Tool for EnterPlanModeTool`
-  `name` function L24-26 — `(&self) -> &str`
-  `description` function L28-38 — `(&self) -> &str`
-  `is_read_only` function L40-42 — `(&self) -> bool`
-  `category` function L44-46 — `(&self) -> ToolCategory`
-  `parameters_schema` function L48-59 — `(&self) -> Value`
-  `execute` function L61-91 — `(&self, ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput, ...`
-  `tests` module L95-150 — `-`
-  `test_ctx` function L102-105 — `(dir: &std::path::Path) -> EngineToolContext`
-  `enter_plan_mode_activates` function L108-123 — `()`
-  `enter_plan_mode_when_already_active` function L126-142 — `()`
-  `enter_plan_mode_is_read_only` function L145-149 — `()`

#### crates/arawn-engine/src/tools/exit_plan_mode.rs

- pub `ExitPlanModeTool` struct L12-14 — `{ plan_state: Arc<PlanModeState> }` — Tool that exits plan mode — writes the plan to disk and deactivates plan mode
- pub `new` function L17-19 — `(plan_state: Arc<PlanModeState>) -> Self`
-  `ExitPlanModeTool` type L16-20 — `= ExitPlanModeTool`
-  `ExitPlanModeTool` type L23-95 — `impl Tool for ExitPlanModeTool`
-  `name` function L24-26 — `(&self) -> &str`
-  `description` function L28-33 — `(&self) -> &str`
-  `is_read_only` function L35-38 — `(&self) -> bool`
-  `category` function L40-42 — `(&self) -> ToolCategory`
-  `parameters_schema` function L44-55 — `(&self) -> Value`
-  `execute` function L57-94 — `(&self, _ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput,...`
-  `tests` module L98-177 — `-`
-  `test_ctx` function L106-109 — `() -> EngineToolContext`
-  `setup` function L111-119 — `() -> (Arc<PlanModeState>, ExitPlanModeTool, std::path::PathBuf)`
-  `exit_not_in_plan_mode` function L122-130 — `()`
-  `exit_with_empty_plan` function L133-140 — `()`
-  `exit_deactivates_plan_mode` function L143-156 — `()`
-  `plan_written_to_disk` function L159-169 — `()`
-  `exit_plan_mode_is_read_only` function L172-176 — `()`

#### crates/arawn-engine/src/tools/feed_search.rs

- pub `FeedSearchTool` struct L38-43 — `{ store: Arc<ProjectionStore>, embedder: Option<Arc<dyn Embedder>> }` — fusion, no API change.
- pub `new` function L46-48 — `(store: Arc<ProjectionStore>, embedder: Option<Arc<dyn Embedder>>) -> Self` — fusion, no API change.
-  `KNOWN_FEED_TYPES` variable L21-31 — `: &[&str]` — fusion, no API change.
-  `RRF_K` variable L36 — `: f32` — RRF constant (Cormack et al.
-  `FeedSearchTool` type L45-49 — `= FeedSearchTool` — fusion, no API change.
-  `FeedSearchTool` type L52-230 — `impl Tool for FeedSearchTool` — fusion, no API change.
-  `name` function L53-55 — `(&self) -> &str` — fusion, no API change.
-  `description` function L57-62 — `(&self) -> &str` — fusion, no API change.
-  `is_read_only` function L64-66 — `(&self) -> bool` — fusion, no API change.
-  `category` function L68-70 — `(&self) -> ToolCategory` — fusion, no API change.
-  `parameters_schema` function L72-100 — `(&self) -> Value` — fusion, no API change.
-  `execute` function L102-229 — `( &self, _ctx: &dyn arawn_tool::ToolContext, params: Value, ) -> Result<ToolOutp...` — fusion, no API change.
-  `Hit` struct L232-235 — `{ score: f32, row: arawn_projections::ProjectionRow }` — fusion, no API change.
-  `FusedHit` struct L238-242 — `{ feed_type: String, projection_id: String, score: f32 }` — Per-(feed_type, projection_id) accumulator for RRF scores.
-  `FusedHit` type L244-252 — `= FusedHit` — fusion, no API change.
-  `new` function L245-251 — `(feed_type: String, projection_id: String) -> Self` — fusion, no API change.
-  `key` function L254-256 — `(feed_type: &str, projection_id: &str) -> String` — fusion, no API change.
-  `rrf_score` function L259-261 — `(rank: usize) -> f32` — Reciprocal rank fusion contribution from a single ranked list.
-  `snippet` function L263-269 — `(text: &str, cap: usize) -> String` — fusion, no API change.

#### crates/arawn-engine/src/tools/file_edit.rs

- pub `FileEditTool` struct L8 — `-` — Edit a file by replacing a string.
-  `FileEditTool` type L11-163 — `impl Tool for FileEditTool`
-  `name` function L12-14 — `(&self) -> &str`
-  `permission_category` function L16-18 — `(&self) -> arawn_tool::PermissionCategory`
-  `description` function L20-30 — `(&self) -> &str`
-  `parameters_schema` function L32-55 — `(&self) -> Value`
-  `execute` function L57-162 — `(&self, ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput, ...`
-  `tests` module L166-342 — `-`
-  `test_ctx` function L174-177 — `(dir: &std::path::Path) -> EngineToolContext`
-  `mark_read` function L180-183 — `(ctx: &EngineToolContext, dir: &std::path::Path, name: &str)` — Mark a file as read in the context (simulates a prior file_read call).
-  `edit_replaces_string` function L186-207 — `()`
-  `edit_fails_on_missing_string` function L210-228 — `()`
-  `edit_fails_on_ambiguous_match` function L231-249 — `()`
-  `edit_replace_all` function L252-273 — `()`
-  `edit_rejects_path_traversal` function L276-290 — `()`
-  `edit_fails_without_prior_read` function L293-311 — `()`
-  `edit_rejects_secret_filename` function L314-332 — `()`
-  `schema_is_valid` function L335-341 — `()`

#### crates/arawn-engine/src/tools/file_read.rs

- pub `FileReadTool` struct L9 — `-` — Read a file within the workstream's working directory.
-  `FileReadTool` type L12-135 — `impl Tool for FileReadTool`
-  `name` function L13-15 — `(&self) -> &str`
-  `description` function L17-26 — `(&self) -> &str`
-  `is_read_only` function L28-30 — `(&self) -> bool`
-  `parameters_schema` function L32-51 — `(&self) -> Value`
-  `execute` function L53-134 — `(&self, ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput, ...`
-  `tests` module L139-292 — `-`
-  `test_ctx_with_dir` function L148-151 — `(dir: &Path) -> EngineToolContext`
-  `read_existing_file` function L154-169 — `()`
-  `read_with_offset_and_limit` function L172-186 — `()`
-  `read_nonexistent_file` function L189-200 — `()`
-  `path_traversal_rejected` function L203-223 — `()`
-  `missing_path_param` function L226-232 — `()`
-  `schema_is_valid` function L235-240 — `()`
-  `refuses_token_dir_path` function L243-262 — `()`
-  `refuses_dotenv_in_workstream` function L265-275 — `()`
-  `allows_legitimate_env_rs` function L278-290 — `()`

#### crates/arawn-engine/src/tools/file_write.rs

- pub `FileWriteTool` struct L9 — `-` — Write content to a file within the workstream's working directory.
-  `FileWriteTool` type L12-149 — `impl Tool for FileWriteTool`
-  `name` function L13-15 — `(&self) -> &str`
-  `permission_category` function L17-19 — `(&self) -> arawn_tool::PermissionCategory`
-  `description` function L21-30 — `(&self) -> &str`
-  `parameters_schema` function L32-47 — `(&self) -> Value`
-  `execute` function L49-148 — `(&self, ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput, ...`
-  `normalize_path` function L151-163 — `(path: &std::path::Path) -> std::path::PathBuf`
-  `tests` module L166-315 — `-`
-  `test_ctx` function L174-177 — `(dir: &std::path::Path) -> EngineToolContext`
-  `mark_read` function L179-182 — `(ctx: &EngineToolContext, path: &std::path::Path)`
-  `write_creates_file` function L185-201 — `()`
-  `write_creates_parent_dirs` function L204-219 — `()`
-  `write_overwrites_existing` function L222-240 — `()`
-  `write_rejects_path_traversal` function L243-258 — `()`
-  `write_new_file_without_read_ok` function L261-272 — `()`
-  `write_existing_file_without_read_fails` function L275-290 — `()`
-  `write_rejects_secret_filename` function L293-305 — `()`
-  `schema_is_valid` function L308-314 — `()`

#### crates/arawn-engine/src/tools/glob.rs

- pub `GlobTool` struct L14 — `-` — Fast file pattern matching using globwalk.
-  `MAX_RESULTS` variable L10 — `: usize` — Maximum number of files to return before truncating.
-  `GlobTool` type L17-145 — `impl Tool for GlobTool`
-  `name` function L18-20 — `(&self) -> &str`
-  `description` function L22-28 — `(&self) -> &str`
-  `is_read_only` function L30-32 — `(&self) -> bool`
-  `parameters_schema` function L34-49 — `(&self) -> Value`
-  `execute` function L51-144 — `(&self, ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput, ...`
-  `tests` module L148-264 — `-`
-  `test_ctx` function L155-158 — `(dir: &std::path::Path) -> EngineToolContext`
-  `schema_is_valid` function L161-168 — `()`
-  `is_read_only` function L171-173 — `()`
-  `glob_in_tempdir` function L176-195 — `()`
-  `glob_no_matches` function L198-210 — `()`
-  `glob_respects_gitignore` function L213-233 — `()`
-  `glob_path_traversal_rejected` function L236-248 — `()`
-  `glob_absolute_path_rejected` function L251-263 — `()`

#### crates/arawn-engine/src/tools/grep.rs

- pub `GrepTool` struct L15 — `-` — Search file contents using ripgrep (rg) or grep as fallback.
-  `DEFAULT_HEAD_LIMIT` variable L9 — `: usize` — Default cap on grep results when head_limit is unspecified.
-  `VCS_EXCLUDES` variable L12 — `: &[&str]` — VCS directories to exclude from searches.
-  `GrepTool` type L18-224 — `impl Tool for GrepTool`
-  `name` function L19-21 — `(&self) -> &str`
-  `description` function L23-33 — `(&self) -> &str`
-  `is_read_only` function L35-37 — `(&self) -> bool`
-  `parameters_schema` function L39-103 — `(&self) -> Value`
-  `execute` function L105-223 — `(&self, ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput, ...`
-  `has_rg` function L226-228 — `() -> bool`
-  `run_rg` function L231-309 — `( cwd: &std::path::Path, pattern: &str, path: &str, glob: Option<&str>, file_typ...`
-  `run_grep_fallback` function L311-347 — `( cwd: &std::path::Path, pattern: &str, path: &str, case_insensitive: bool, outp...`
-  `tests` module L350-570 — `-`
-  `test_ctx` function L357-360 — `(dir: &std::path::Path) -> EngineToolContext`
-  `grep_finds_matches` function L363-381 — `()`
-  `grep_no_matches` function L384-398 — `()`
-  `grep_case_insensitive` function L401-415 — `()`
-  `grep_with_glob` function L418-433 — `()`
-  `grep_content_mode` function L436-454 — `()`
-  `grep_files_with_matches_mode` function L457-476 — `()`
-  `grep_head_limit` function L479-502 — `()`
-  `schema_is_valid` function L505-514 — `()`
-  `grep_path_traversal_rejected` function L517-535 — `()`
-  `grep_absolute_path_rejected` function L538-550 — `()`
-  `grep_relative_path_within_root_allowed` function L553-569 — `()`

#### crates/arawn-engine/src/tools/memory_search.rs

- pub `MemorySearchTool` struct L16-19 — `{ memory: MemoryHandle, embedder: Option<Arc<dyn Embedder>> }` — Tool that searches the knowledge base using composite retrieval:
- pub `new` function L22-27 — `(memory: impl Into<MemoryHandle>, embedder: Option<Arc<dyn Embedder>>) -> Self`
-  `MemorySearchTool` type L21-28 — `= MemorySearchTool`
-  `MemorySearchTool` type L31-272 — `impl Tool for MemorySearchTool`
-  `name` function L32-34 — `(&self) -> &str`
-  `description` function L36-40 — `(&self) -> &str`
-  `is_read_only` function L42-44 — `(&self) -> bool`
-  `category` function L46-48 — `(&self) -> ToolCategory`
-  `parameters_schema` function L50-84 — `(&self) -> Value`
-  `execute` function L86-271 — `(&self, _ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput,...`
-  `ScoredEntity` struct L274-280 — `{ entity: Entity, fts_score: f32, semantic_score: f32, confidence: f32, related:...`
-  `ScoredEntity` type L282-286 — `= ScoredEntity`
-  `composite` function L283-285 — `(&self) -> f32`
-  `tests` module L289-400 — `-`
-  `setup` function L296-303 — `() -> (TempDir, Arc<MemoryManager>, crate::context::EngineToolContext)`
-  `populate` function L305-327 — `(mgr: &MemoryManager)`
-  `search_fts_both_tiers` function L330-343 — `()`
-  `search_with_type_filter` function L346-358 — `()`
-  `search_global_only` function L361-372 — `()`
-  `search_no_results` function L375-385 — `()`
-  `search_with_tags` function L388-399 — `()`

#### crates/arawn-engine/src/tools/memory_store.rs

- pub `MemoryStoreTool` struct L16-19 — `{ memory: MemoryHandle, embedder: Option<Arc<dyn Embedder>> }` — Tool that stores knowledge in the KB with search-before-create deduplication.
- pub `new` function L22-27 — `(memory: impl Into<MemoryHandle>, embedder: Option<Arc<dyn Embedder>>) -> Self`
-  `MemoryStoreTool` type L21-28 — `= MemoryStoreTool`
-  `MemoryStoreTool` type L31-212 — `impl Tool for MemoryStoreTool`
-  `name` function L32-34 — `(&self) -> &str`
-  `description` function L36-47 — `(&self) -> &str`
-  `category` function L49-51 — `(&self) -> ToolCategory`
-  `parameters_schema` function L53-83 — `(&self) -> Value`
-  `execute` function L85-211 — `(&self, ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput, ...`
-  `tests` module L215-324 — `-`
-  `setup` function L222-231 — `() -> (TempDir, Arc<MemoryManager>, crate::context::EngineToolContext)`
-  `store_new_fact` function L234-246 — `()`
-  `store_preference_goes_global` function L249-259 — `()`
-  `store_decision_goes_workstream` function L262-272 — `()`
-  `store_reinforces_duplicate` function L275-290 — `()`
-  `store_with_tags` function L293-306 — `()`
-  `store_with_explicit_scope_override` function L309-323 — `()`

#### crates/arawn-engine/src/tools/mod.rs

- pub `agent` module L1 — `-`
- pub `ask_user` module L2 — `-`
- pub `enter_plan_mode` module L3 — `-`
- pub `exit_plan_mode` module L4 — `-`
- pub `file_edit` module L5 — `-`
- pub `file_read` module L6 — `-`
- pub `feed_search` module L7 — `-`
- pub `file_write` module L8 — `-`
- pub `glob` module L9 — `-`
- pub `grep` module L10 — `-`
- pub `memory_search` module L11 — `-`
- pub `memory_store` module L12 — `-`
- pub `safe_env` module L13 — `-`
- pub `sensitive_paths` module L14 — `-`
- pub `shell` module L15 — `-`
- pub `signal` module L16 — `-`
- pub `steward` module L17 — `-`
- pub `skill` module L18 — `-`
- pub `sleep` module L19 — `-`
- pub `task_list` module L20 — `-`
- pub `task_output` module L21 — `-`
- pub `task_stop` module L22 — `-`
- pub `think` module L23 — `-`
- pub `web_fetch` module L24 — `-`
- pub `web_search` module L25 — `-`
- pub `workstream` module L26 — `-`

#### crates/arawn-engine/src/tools/safe_env.rs

- pub `safe_env` function L45-47 — `() -> HashMap<String, String>` — Returns a filtered copy of the parent process environment, dropping any
- pub `is_safe_env_name` function L50-55 — `(name: &str) -> bool` — Returns true if `name` is on the safe allowlist.
-  `SAFE_EXACT` variable L13-35 — `: &[&str]` — Exact env var names that are always safe to forward to children.
-  `SAFE_PREFIXES` variable L38-41 — `: &[&str]` — Prefixes for env var names that are safe to forward.
-  `tests` module L58-101 — `-` — development tooling (PATH, build caches, locale).
-  `allows_path_and_home` function L62-66 — `()` — development tooling (PATH, build caches, locale).
-  `allows_lc_and_xdg_prefixes` function L69-73 — `()` — development tooling (PATH, build caches, locale).
-  `blocks_secrets` function L76-86 — `()` — development tooling (PATH, build caches, locale).
-  `safe_env_strips_test_secret` function L89-100 — `()` — development tooling (PATH, build caches, locale).

#### crates/arawn-engine/src/tools/sensitive_paths.rs

- pub `sensitive_deny_read_paths` function L15-60 — `() -> Vec<String>` — Build the list of sensitive paths that should be denied for reading.
- pub `is_sensitive_path` function L66-90 — `(path: &Path) -> bool` — Returns true if `path` resolves into any sensitive directory.
- pub `is_token_path` function L96-105 — `(path: &Path, data_dir: &Path) -> bool` — Returns true if `path` resolves into the OAuth token directory under
- pub `is_secret_file` function L111-116 — `(path: &Path) -> bool` — Returns true if the file at `path` matches a known secret-file pattern.
-  `is_secret_filename` function L118-154 — `(name: &str) -> bool` — reject paths that resolve into any of these directories.
-  `EXACT` variable L120-132 — `: &[&str]` — reject paths that resolve into any of these directories.
-  `EXTENSIONS` variable L138 — `: &[&str]` — reject paths that resolve into any of these directories.
-  `ALLOWED_ENV_SUFFIXES` variable L147 — `: &[&str]` — reject paths that resolve into any of these directories.
-  `tests` module L157-263 — `-` — reject paths that resolve into any of these directories.
-  `deny_list_includes_ssh_and_aws` function L161-167 — `()` — reject paths that resolve into any of these directories.
-  `ssh_dir_is_sensitive` function L170-177 — `()` — reject paths that resolve into any of these directories.
-  `aws_dir_is_sensitive` function L180-186 — `()` — reject paths that resolve into any of these directories.
-  `ordinary_path_is_not_sensitive` function L189-192 — `()` — reject paths that resolve into any of these directories.
-  `etc_shadow_is_sensitive` function L195-197 — `()` — reject paths that resolve into any of these directories.
-  `secret_file_basenames_blocked` function L200-215 — `()` — reject paths that resolve into any of these directories.
-  `token_path_detection` function L218-235 — `()` — reject paths that resolve into any of these directories.
-  `token_path_defeats_dotdot_traversal` function L238-250 — `()` — reject paths that resolve into any of these directories.
-  `legitimate_files_not_secret` function L253-262 — `()` — reject paths that resolve into any of these directories.

#### crates/arawn-engine/src/tools/shell.rs

- pub `ShellTool` struct L24-29 — `{ network_tools: Vec<String>, bg_manager: Option<Arc<BackgroundTaskManager>> }` — Execute a shell command within an OS-level sandbox.
- pub `with_network_tools` function L36-41 — `(network_tools: Vec<String>) -> Self` — Create a ShellTool with the given list of network-allowed tool binaries.
- pub `with_background_manager` function L44-47 — `(mut self, mgr: Arc<BackgroundTaskManager>) -> Self` — Attach a background task manager for `run_in_background` support.
-  `DEFAULT_TIMEOUT_MS` variable L31 — `: u64`
-  `ShellTool` type L34-207 — `= ShellTool`
-  `spawn_background` function L55-206 — `( &self, command: &str, working_dir: &std::path::Path, ) -> Result<ToolOutput, T...` — Spawn a shell command as a background task.
-  `init_sandbox_for_background` function L213-248 — `( command: &str, working_dir: &std::path::Path, network_tools: &[String], ) -> R...` — Initialize a sandbox manager for a background command and return it together
-  `command_needs_network` function L252-271 — `(command: &str, network_tools: &[String]) -> bool` — Check if a command invokes any tool that needs network access.
-  `build_sandbox_config` function L274-323 — `( command: &str, working_dir: &std::path::Path, network_tools: &[String], ) -> S...` — Build a sandbox config for executing a command in the given working directory.
-  `ShellTool` type L326-414 — `impl Tool for ShellTool`
-  `name` function L327-329 — `(&self) -> &str`
-  `permission_category` function L331-333 — `(&self) -> arawn_tool::PermissionCategory`
-  `description` function L335-350 — `(&self) -> &str`
-  `parameters_schema` function L352-371 — `(&self) -> Value`
-  `execute` function L373-413 — `(&self, ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput, ...`
-  `SandboxExecError` enum L416-421 — `Unavailable | Tool`
-  `execute_sandboxed` function L423-523 — `( command: &str, working_dir: &std::path::Path, timeout_ms: u64, network_tools: ...`
-  `execute_unsandboxed` function L525-571 — `( command: &str, working_dir: &std::path::Path, timeout_ms: u64, ) -> Result<Too...`
-  `tests` module L574-1003 — `-`
-  `test_ctx` function L582-585 — `() -> EngineToolContext`
-  `test_ctx_in` function L587-590 — `(dir: &std::path::Path) -> EngineToolContext`
-  `shell_echo` function L594-602 — `()`
-  `shell_nonzero_exit` function L606-614 — `()`
-  `shell_timeout` function L618-629 — `()`
-  `shell_missing_command` function L633-637 — `()`
-  `shell_env_does_not_leak_secrets` function L641-666 — `()`
-  `background_command_runs_sandboxed` function L670-704 — `()`
-  `background_command_sandbox_blocks_sensitive_read` function L708-754 — `()`
-  `shell_env_preserves_path` function L758-766 — `()`
-  `shell_schema_is_valid` function L769-774 — `()`
-  `sensitive_paths_includes_ssh` function L777-780 — `()`
-  `sensitive_paths_includes_aws` function L783-786 — `()`
-  `sandbox_config_allows_working_dir_and_tmp` function L789-800 — `()`
-  `network_detection_recognizes_tools` function L803-810 — `()`
-  `network_detection_blocks_unknown` function L813-818 — `()`
-  `network_detection_empty_list_blocks_all` function L821-824 — `()`
-  `sandbox_write_inside_allowed` function L830-849 — `()`
-  `sandbox_mkdir_inside_allowed` function L853-874 — `()`
-  `sandbox_unlink_inside_allowed` function L878-903 — `()`
-  `sandbox_build_tool_workflow` function L907-929 — `()`
-  `sandbox_write_outside_blocked` function L933-970 — `()`
-  `sandbox_read_sensitive_path_blocked` function L974-1002 — `()`

#### crates/arawn-engine/src/tools/signal.rs

- pub `SignalSearchTool` struct L81-85 — `{ memory: MemoryHandle, router: Option<Arc<WorkstreamMemoryRouter>>, embedder: O...` — Person) is reachable via the existing `memory_search` tool.
- pub `new` function L88-102 — `( memory: impl Into<MemoryHandle>, embedder: Option<Arc<dyn Embedder>>, ) -> Sel...` — Person) is reachable via the existing `memory_search` tool.
- pub `SignalQueryTool` struct L241-244 — `{ memory: MemoryHandle, router: Option<Arc<WorkstreamMemoryRouter>> }` — Person) is reachable via the existing `memory_search` tool.
- pub `new` function L247-254 — `(memory: impl Into<MemoryHandle>) -> Self` — Person) is reachable via the existing `memory_search` tool.
- pub `SignalTimelineTool` struct L376-379 — `{ memory: MemoryHandle, router: Option<Arc<WorkstreamMemoryRouter>> }` — Person) is reachable via the existing `memory_search` tool.
- pub `new` function L382-389 — `(memory: impl Into<MemoryHandle>) -> Self` — Person) is reachable via the existing `memory_search` tool.
-  `RRF_K` variable L29 — `: f32` — RRF constant — same value `feed_search` uses.
-  `rrf` function L31-33 — `(rank: usize) -> f32` — Person) is reachable via the existing `memory_search` tool.
-  `resolve_manager` function L38-53 — `( handle: &MemoryHandle, explicit: Option<&str>, router: Option<&Arc<WorkstreamM...` — Resolve the manager for the active workstream, or the explicit
-  `entity_summary` function L55-67 — `(e: &Entity) -> Value` — Person) is reachable via the existing `memory_search` tool.
-  `snippet` function L69-75 — `(s: &str, cap: usize) -> String` — Person) is reachable via the existing `memory_search` tool.
-  `SignalSearchTool` type L87-103 — `= SignalSearchTool` — Person) is reachable via the existing `memory_search` tool.
-  `SignalSearchTool` type L106-221 — `impl Tool for SignalSearchTool` — Person) is reachable via the existing `memory_search` tool.
-  `name` function L107-109 — `(&self) -> &str` — Person) is reachable via the existing `memory_search` tool.
-  `description` function L111-116 — `(&self) -> &str` — Person) is reachable via the existing `memory_search` tool.
-  `is_read_only` function L118-120 — `(&self) -> bool` — Person) is reachable via the existing `memory_search` tool.
-  `category` function L122-124 — `(&self) -> ToolCategory` — Person) is reachable via the existing `memory_search` tool.
-  `parameters_schema` function L126-139 — `(&self) -> Value` — Person) is reachable via the existing `memory_search` tool.
-  `execute` function L141-220 — `( &self, _ctx: &dyn arawn_tool::ToolContext, params: Value, ) -> Result<ToolOutp...` — Person) is reachable via the existing `memory_search` tool.
-  `FusedHit` struct L223-226 — `{ entity: Entity, score: f32 }` — Person) is reachable via the existing `memory_search` tool.
-  `FusedHit` type L228-235 — `= FusedHit` — Person) is reachable via the existing `memory_search` tool.
-  `new` function L229-234 — `(entity: Entity) -> Self` — Person) is reachable via the existing `memory_search` tool.
-  `SignalQueryTool` type L246-255 — `= SignalQueryTool` — Person) is reachable via the existing `memory_search` tool.
-  `SignalQueryTool` type L258-370 — `impl Tool for SignalQueryTool` — Person) is reachable via the existing `memory_search` tool.
-  `name` function L259-261 — `(&self) -> &str` — Person) is reachable via the existing `memory_search` tool.
-  `description` function L263-268 — `(&self) -> &str` — Person) is reachable via the existing `memory_search` tool.
-  `is_read_only` function L270-272 — `(&self) -> bool` — Person) is reachable via the existing `memory_search` tool.
-  `category` function L274-276 — `(&self) -> ToolCategory` — Person) is reachable via the existing `memory_search` tool.
-  `parameters_schema` function L278-297 — `(&self) -> Value` — Person) is reachable via the existing `memory_search` tool.
-  `execute` function L299-369 — `( &self, _ctx: &dyn arawn_tool::ToolContext, params: Value, ) -> Result<ToolOutp...` — Person) is reachable via the existing `memory_search` tool.
-  `SignalTimelineTool` type L381-390 — `= SignalTimelineTool` — Person) is reachable via the existing `memory_search` tool.
-  `SignalTimelineTool` type L393-482 — `impl Tool for SignalTimelineTool` — Person) is reachable via the existing `memory_search` tool.
-  `name` function L394-396 — `(&self) -> &str` — Person) is reachable via the existing `memory_search` tool.
-  `description` function L398-402 — `(&self) -> &str` — Person) is reachable via the existing `memory_search` tool.
-  `is_read_only` function L404-406 — `(&self) -> bool` — Person) is reachable via the existing `memory_search` tool.
-  `category` function L408-410 — `(&self) -> ToolCategory` — Person) is reachable via the existing `memory_search` tool.
-  `parameters_schema` function L412-422 — `(&self) -> Value` — Person) is reachable via the existing `memory_search` tool.
-  `execute` function L424-481 — `( &self, _ctx: &dyn arawn_tool::ToolContext, params: Value, ) -> Result<ToolOutp...` — Person) is reachable via the existing `memory_search` tool.
-  `tests` module L489-681 — `-` — Person) is reachable via the existing `memory_search` tool.
-  `setup` function L495-502 — `() -> (TempDir, Arc<MemoryManager>, crate::context::EngineToolContext)` — Person) is reachable via the existing `memory_search` tool.
-  `seed` function L504-525 — `(mgr: &MemoryManager)` — Person) is reachable via the existing `memory_search` tool.
-  `signal_search_finds_decision_by_title` function L528-546 — `()` — Person) is reachable via the existing `memory_search` tool.
-  `signal_search_empty_kb_returns_zero` function L549-558 — `()` — Person) is reachable via the existing `memory_search` tool.
-  `signal_query_filters_by_entity_type` function L561-578 — `()` — Person) is reachable via the existing `memory_search` tool.
-  `signal_query_filters_by_tag_any_of` function L581-596 — `()` — Person) is reachable via the existing `memory_search` tool.
-  `signal_query_no_filters_returns_all_active` function L599-606 — `()` — Person) is reachable via the existing `memory_search` tool.
-  `signal_query_window_filters` function L609-623 — `()` — Person) is reachable via the existing `memory_search` tool.
-  `signal_timeline_orders_desc_and_caps_to_window` function L626-641 — `()` — Person) is reachable via the existing `memory_search` tool.
-  `explicit_workstream_arg_routes_via_router` function L644-680 — `()` — Person) is reachable via the existing `memory_search` tool.

#### crates/arawn-engine/src/tools/skill.rs

- pub `SkillTool` struct L14-16 — `{ registry: Arc<SkillRegistry> }` — Tool that executes skills (reusable prompt-based workflows).
- pub `new` function L19-21 — `(registry: Arc<SkillRegistry>) -> Self`
-  `SkillTool` type L18-22 — `= SkillTool`
-  `SkillTool` type L25-97 — `impl Tool for SkillTool`
-  `name` function L26-28 — `(&self) -> &str`
-  `description` function L30-35 — `(&self) -> &str`
-  `parameters_schema` function L37-52 — `(&self) -> Value`
-  `execute` function L54-91 — `(&self, _ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput,...`
-  `is_read_only` function L93-96 — `(&self) -> bool`
-  `tests` module L100-205 — `-`
-  `make_registry` function L104-137 — `() -> Arc<SkillRegistry>`
-  `ctx` function L139-142 — `() -> crate::context::EngineToolContext`
-  `execute_existing_skill` function L145-153 — `()`
-  `execute_with_args` function L156-168 — `()`
-  `execute_missing_skill` function L171-181 — `()`
-  `execute_missing_param` function L184-188 — `()`
-  `tool_metadata` function L191-196 — `()`
-  `schema_has_required_skill` function L199-204 — `()`

#### crates/arawn-engine/src/tools/sleep.rs

- pub `SleepTool` struct L13 — `-` — Waits for a specified duration.
-  `MAX_SLEEP_SECS` variable L9 — `: u64` — Maximum sleep duration in seconds.
-  `SleepTool` type L16-72 — `impl Tool for SleepTool`
-  `name` function L17-19 — `(&self) -> &str`
-  `description` function L21-26 — `(&self) -> &str`
-  `is_read_only` function L28-30 — `(&self) -> bool`
-  `category` function L32-34 — `(&self) -> ToolCategory`
-  `parameters_schema` function L36-47 — `(&self) -> Value`
-  `execute` function L49-71 — `(&self, _ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput,...`
-  `tests` module L75-143 — `-`
-  `test_ctx` function L82-85 — `() -> EngineToolContext`
-  `schema_is_valid` function L88-95 — `()`
-  `is_read_only` function L98-100 — `()`
-  `sleep_short_duration` function L103-115 — `()`
-  `sleep_negative_errors` function L118-126 — `()`
-  `sleep_clamped` function L129-142 — `()`

#### crates/arawn-engine/src/tools/steward.rs

- pub `WorkstreamJournalTool` struct L64-67 — `{ data_dir: PathBuf, router: Arc<WorkstreamMemoryRouter> }` — via `arawn_steward::rollback::apply_inverse`.
- pub `new` function L70-75 — `(data_dir: impl Into<PathBuf>, router: Arc<WorkstreamMemoryRouter>) -> Self` — via `arawn_steward::rollback::apply_inverse`.
- pub `WorkstreamRefineTool` struct L139-142 — `{ data_dir: PathBuf, router: Arc<WorkstreamMemoryRouter> }` — via `arawn_steward::rollback::apply_inverse`.
- pub `new` function L145-150 — `(data_dir: impl Into<PathBuf>, router: Arc<WorkstreamMemoryRouter>) -> Self` — via `arawn_steward::rollback::apply_inverse`.
- pub `WorkstreamRollbackTool` struct L214-217 — `{ data_dir: PathBuf, router: Arc<WorkstreamMemoryRouter> }` — via `arawn_steward::rollback::apply_inverse`.
- pub `new` function L220-225 — `(data_dir: impl Into<PathBuf>, router: Arc<WorkstreamMemoryRouter>) -> Self` — via `arawn_steward::rollback::apply_inverse`.
-  `open_journal` function L21-24 — `(data_dir: &PathBuf, workstream: &str) -> Result<Journal, ToolError>` — via `arawn_steward::rollback::apply_inverse`.
-  `resolve_workstream` function L26-43 — `( memory: &MemoryHandle, explicit: Option<&str>, ) -> Result<String, ToolError>` — via `arawn_steward::rollback::apply_inverse`.
-  `row_summary` function L46-58 — `(row: &arawn_steward::JournalRow) -> Value` — Lightweight summary of one journal row for tool output.
-  `WorkstreamJournalTool` type L69-76 — `= WorkstreamJournalTool` — via `arawn_steward::rollback::apply_inverse`.
-  `WorkstreamJournalTool` type L79-133 — `impl Tool for WorkstreamJournalTool` — via `arawn_steward::rollback::apply_inverse`.
-  `name` function L80-82 — `(&self) -> &str` — via `arawn_steward::rollback::apply_inverse`.
-  `description` function L84-88 — `(&self) -> &str` — via `arawn_steward::rollback::apply_inverse`.
-  `is_read_only` function L90-92 — `(&self) -> bool` — via `arawn_steward::rollback::apply_inverse`.
-  `category` function L94-96 — `(&self) -> ToolCategory` — via `arawn_steward::rollback::apply_inverse`.
-  `parameters_schema` function L98-106 — `(&self) -> Value` — via `arawn_steward::rollback::apply_inverse`.
-  `execute` function L108-132 — `( &self, _ctx: &dyn arawn_tool::ToolContext, params: Value, ) -> Result<ToolOutp...` — via `arawn_steward::rollback::apply_inverse`.
-  `WorkstreamRefineTool` type L144-151 — `= WorkstreamRefineTool` — via `arawn_steward::rollback::apply_inverse`.
-  `WorkstreamRefineTool` type L154-208 — `impl Tool for WorkstreamRefineTool` — via `arawn_steward::rollback::apply_inverse`.
-  `name` function L155-157 — `(&self) -> &str` — via `arawn_steward::rollback::apply_inverse`.
-  `description` function L159-163 — `(&self) -> &str` — via `arawn_steward::rollback::apply_inverse`.
-  `is_read_only` function L165-167 — `(&self) -> bool` — via `arawn_steward::rollback::apply_inverse`.
-  `category` function L169-171 — `(&self) -> ToolCategory` — via `arawn_steward::rollback::apply_inverse`.
-  `parameters_schema` function L173-181 — `(&self) -> Value` — via `arawn_steward::rollback::apply_inverse`.
-  `execute` function L183-207 — `( &self, _ctx: &dyn arawn_tool::ToolContext, params: Value, ) -> Result<ToolOutp...` — via `arawn_steward::rollback::apply_inverse`.
-  `WorkstreamRollbackTool` type L219-226 — `= WorkstreamRollbackTool` — via `arawn_steward::rollback::apply_inverse`.
-  `WorkstreamRollbackTool` type L229-297 — `impl Tool for WorkstreamRollbackTool` — via `arawn_steward::rollback::apply_inverse`.
-  `name` function L230-232 — `(&self) -> &str` — via `arawn_steward::rollback::apply_inverse`.
-  `description` function L234-238 — `(&self) -> &str` — via `arawn_steward::rollback::apply_inverse`.
-  `is_read_only` function L240-242 — `(&self) -> bool` — via `arawn_steward::rollback::apply_inverse`.
-  `category` function L244-246 — `(&self) -> ToolCategory` — via `arawn_steward::rollback::apply_inverse`.
-  `parameters_schema` function L248-257 — `(&self) -> Value` — via `arawn_steward::rollback::apply_inverse`.
-  `execute` function L259-296 — `( &self, _ctx: &dyn arawn_tool::ToolContext, params: Value, ) -> Result<ToolOutp...` — via `arawn_steward::rollback::apply_inverse`.
-  `_unused` function L302-304 — `(memory: &MemoryHandle, explicit: Option<&str>) -> Result<String, ToolError>` — via `arawn_steward::rollback::apply_inverse`.
-  `tests` module L307-444 — `-` — via `arawn_steward::rollback::apply_inverse`.
-  `setup` function L315-331 — `() -> ( TempDir, Arc<WorkstreamMemoryRouter>, crate::context::EngineToolContext,...` — via `arawn_steward::rollback::apply_inverse`.
-  `write_proposal_row` function L333-345 — `(j: &Journal) -> i64` — via `arawn_steward::rollback::apply_inverse`.
-  `write_delete_row` function L347-358 — `(j: &Journal, e: &Entity) -> i64` — via `arawn_steward::rollback::apply_inverse`.
-  `journal_lists_recent_rows` function L361-371 — `()` — via `arawn_steward::rollback::apply_inverse`.
-  `refine_returns_pending_proposals_only` function L374-395 — `()` — via `arawn_steward::rollback::apply_inverse`.
-  `rollback_reverts_delete_action_end_to_end` function L398-419 — `()` — via `arawn_steward::rollback::apply_inverse`.
-  `rollback_is_idempotent` function L422-435 — `()` — via `arawn_steward::rollback::apply_inverse`.
-  `rollback_unknown_id_errors` function L438-443 — `()` — via `arawn_steward::rollback::apply_inverse`.

#### crates/arawn-engine/src/tools/task_list.rs

- pub `TaskStatus` enum L14-18 — `Pending | InProgress | Completed` — Session-scoped task status.
- pub `SessionTask` struct L32-40 — `{ id: String, subject: String, description: Option<String>, active_form: Option<...` — A single session-scoped task.
- pub `SessionTaskStore` struct L45-48 — `{ tasks: Arc<RwLock<HashMap<String, SessionTask>>>, order: Arc<RwLock<Vec<String...` — Shared in-memory task store for a session.
- pub `new` function L51-53 — `() -> Self`
- pub `TaskCreateTool` struct L129-131 — `{ store: SessionTaskStore }` — Creates a new session-scoped task for tracking work within the current session.
- pub `new` function L134-136 — `(store: SessionTaskStore) -> Self`
- pub `TaskUpdateTool` struct L212-214 — `{ store: SessionTaskStore }` — Updates a session task's status or details.
- pub `new` function L217-219 — `(store: SessionTaskStore) -> Self`
- pub `TaskListTool` struct L344-346 — `{ store: SessionTaskStore }` — Lists all session tasks with their status.
- pub `new` function L349-351 — `(store: SessionTaskStore) -> Self`
- pub `TaskGetTool` struct L411-413 — `{ store: SessionTaskStore }` — Gets full details of a session task by ID.
- pub `new` function L416-418 — `(store: SessionTaskStore) -> Self`
-  `TaskStatus` type L20-28 — `= TaskStatus`
-  `fmt` function L21-27 — `(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result`
-  `SessionTaskStore` type L50-115 — `= SessionTaskStore`
-  `create` function L55-72 — `( &self, subject: String, description: Option<String>, active_form: Option<Strin...`
-  `update` function L74-93 — `(&self, id: &str, updates: TaskUpdates) -> Option<SessionTask>`
-  `get` function L95-97 — `(&self, id: &str) -> Option<SessionTask>`
-  `delete` function L99-105 — `(&self, id: &str) -> bool`
-  `list` function L107-114 — `(&self) -> Vec<SessionTask>`
-  `TaskUpdates` struct L117-122 — `{ status: Option<TaskStatus>, subject: Option<String>, description: Option<Strin...`
-  `TaskCreateTool` type L133-137 — `= TaskCreateTool`
-  `TaskCreateTool` type L140-205 — `impl Tool for TaskCreateTool`
-  `name` function L141-143 — `(&self) -> &str`
-  `description` function L145-156 — `(&self) -> &str`
-  `category` function L158-160 — `(&self) -> ToolCategory`
-  `parameters_schema` function L162-181 — `(&self) -> Value`
-  `execute` function L183-204 — `(&self, _ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput,...`
-  `TaskUpdateTool` type L216-220 — `= TaskUpdateTool`
-  `TaskUpdateTool` type L223-337 — `impl Tool for TaskUpdateTool`
-  `name` function L224-226 — `(&self) -> &str`
-  `description` function L228-237 — `(&self) -> &str`
-  `category` function L239-241 — `(&self) -> ToolCategory`
-  `parameters_schema` function L243-271 — `(&self) -> Value`
-  `execute` function L273-336 — `(&self, _ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput,...`
-  `TaskListTool` type L348-352 — `= TaskListTool`
-  `TaskListTool` type L355-404 — `impl Tool for TaskListTool`
-  `name` function L356-358 — `(&self) -> &str`
-  `description` function L360-368 — `(&self) -> &str`
-  `is_read_only` function L370-372 — `(&self) -> bool`
-  `category` function L374-376 — `(&self) -> ToolCategory`
-  `parameters_schema` function L378-383 — `(&self) -> Value`
-  `execute` function L385-403 — `(&self, _ctx: &dyn arawn_tool::ToolContext, _params: Value) -> Result<ToolOutput...`
-  `TaskGetTool` type L415-419 — `= TaskGetTool`
-  `TaskGetTool` type L422-469 — `impl Tool for TaskGetTool`
-  `name` function L423-425 — `(&self) -> &str`
-  `description` function L427-433 — `(&self) -> &str`
-  `is_read_only` function L435-437 — `(&self) -> bool`
-  `category` function L439-441 — `(&self) -> ToolCategory`
-  `parameters_schema` function L443-454 — `(&self) -> Value`
-  `execute` function L456-468 — `(&self, _ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput,...`
-  `tests` module L472-815 — `-`
-  `test_ctx` function L478-481 — `() -> crate::context::EngineToolContext`
-  `store_create_and_list` function L484-494 — `()`
-  `store_update_status` function L497-512 — `()`
-  `store_update_subject_and_description` function L515-532 — `()`
-  `store_delete` function L535-540 — `()`
-  `store_delete_nonexistent` function L543-546 — `()`
-  `store_update_nonexistent` function L549-564 — `()`
-  `store_preserves_order` function L567-575 — `()`
-  `task_create_tool` function L578-595 — `()`
-  `task_create_with_active_form` function L598-614 — `()`
-  `task_update_status` function L617-630 — `()`
-  `task_update_delete` function L633-647 — `()`
-  `task_update_invalid_status` function L650-661 — `()`
-  `task_update_no_fields_errors` function L664-673 — `()`
-  `task_update_not_found` function L676-688 — `()`
-  `task_list_empty` function L691-698 — `()`
-  `task_list_with_tasks` function L701-721 — `()`
-  `full_lifecycle` function L724-759 — `()`
-  `schemas_are_valid` function L762-781 — `()`
-  `task_get_found` function L784-799 — `()`
-  `task_get_not_found` function L802-814 — `()`

#### crates/arawn-engine/src/tools/task_output.rs

- pub `TaskOutputTool` struct L11-13 — `{ bg_manager: Arc<BackgroundTaskManager> }` — Read the output and status of a background task.
- pub `new` function L16-18 — `(bg_manager: Arc<BackgroundTaskManager>) -> Self`
-  `TaskOutputTool` type L15-19 — `= TaskOutputTool`
-  `TaskOutputTool` type L22-135 — `impl Tool for TaskOutputTool`
-  `name` function L23-25 — `(&self) -> &str`
-  `description` function L27-31 — `(&self) -> &str`
-  `is_read_only` function L33-35 — `(&self) -> bool`
-  `category` function L37-39 — `(&self) -> ToolCategory`
-  `parameters_schema` function L41-60 — `(&self) -> Value`
-  `execute` function L62-134 — `(&self, _ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput,...`
-  `tests` module L138-213 — `-`
-  `test_ctx` function L145-148 — `() -> crate::context::EngineToolContext`
-  `unknown_task_returns_error` function L151-160 — `()`
-  `completed_task_returns_output` function L163-188 — `()`
-  `running_task_non_blocking` function L191-212 — `()`

#### crates/arawn-engine/src/tools/task_stop.rs

- pub `TaskStopTool` struct L11-13 — `{ bg_manager: Arc<BackgroundTaskManager> }` — Stop a running background task.
- pub `new` function L16-18 — `(bg_manager: Arc<BackgroundTaskManager>) -> Self`
-  `TaskStopTool` type L15-19 — `= TaskStopTool`
-  `TaskStopTool` type L22-78 — `impl Tool for TaskStopTool`
-  `name` function L23-25 — `(&self) -> &str`
-  `description` function L27-30 — `(&self) -> &str`
-  `is_read_only` function L32-34 — `(&self) -> bool`
-  `category` function L36-38 — `(&self) -> ToolCategory`
-  `parameters_schema` function L40-51 — `(&self) -> Value`
-  `execute` function L53-77 — `(&self, _ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput,...`
-  `tests` module L81-156 — `-`
-  `test_ctx` function L88-91 — `() -> crate::context::EngineToolContext`
-  `stop_unknown_task` function L94-103 — `()`
-  `stop_running_task` function L106-131 — `()`
-  `stop_already_completed_task` function L134-155 — `()`

#### crates/arawn-engine/src/tools/think.rs

- pub `ThinkTool` struct L8 — `-` — A no-op reasoning scratchpad tool.
-  `ThinkTool` type L11-50 — `impl Tool for ThinkTool`
-  `name` function L12-14 — `(&self) -> &str`
-  `description` function L16-23 — `(&self) -> &str`
-  `is_read_only` function L25-27 — `(&self) -> bool`
-  `parameters_schema` function L29-40 — `(&self) -> Value`
-  `execute` function L42-49 — `(&self, _ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput,...`
-  `tests` module L53-91 — `-`
-  `test_ctx` function L60-63 — `() -> EngineToolContext`
-  `think_returns_thought` function L66-74 — `()`
-  `think_with_empty_thought` function L77-82 — `()`
-  `think_schema_is_valid` function L85-90 — `()`

#### crates/arawn-engine/src/tools/web_fetch.rs

- pub `WebFetchTool` struct L37-39 — `{ cache: Arc<Mutex<LruCache<String, CacheEntry>>> }` — Fetches content from a URL, converts HTML to markdown, caches results,
- pub `new` function L42-48 — `() -> Self`
-  `CACHE_TTL` variable L14 — `: Duration` — Cache TTL: 15 minutes.
-  `CACHE_MAX_ENTRIES` variable L17 — `: usize` — Maximum cache entries.
-  `MAX_CONTENT_BYTES` variable L20 — `: usize` — Max content size before truncation (100KB).
-  `CacheEntry` struct L23-27 — `{ content: String, content_type: String, fetched_at: Instant }` — Cached fetch result.
-  `CacheEntry` type L29-33 — `= CacheEntry`
-  `is_expired` function L30-32 — `(&self) -> bool`
-  `WebFetchTool` type L41-49 — `= WebFetchTool`
-  `WebFetchTool` type L51-55 — `impl Default for WebFetchTool`
-  `default` function L52-54 — `() -> Self`
-  `WebFetchTool` type L58-169 — `impl Tool for WebFetchTool`
-  `name` function L59-61 — `(&self) -> &str`
-  `description` function L63-69 — `(&self) -> &str`
-  `category` function L71-73 — `(&self) -> ToolCategory`
-  `parameters_schema` function L75-90 — `(&self) -> Value`
-  `execute` function L92-168 — `(&self, ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput, ...`
-  `process_content` function L172-185 — `(body: &str, content_type: &str) -> String` — Convert HTML to markdown, or return non-HTML as-is.
-  `html_to_markdown` function L188-193 — `(html: &str) -> String` — Convert HTML to markdown using htmd (Turndown-equivalent).
-  `strip_html_tags` function L196-227 — `(html: &str) -> String` — Fallback: simple HTML tag stripper (used if htmd fails).
-  `finish` function L230-241 — `( ctx: &dyn arawn_tool::ToolContext, prompt: &str, url: &str, text: String, ) ->...` — If we have an LLM and a prompt, summarize.
-  `summarize_with_llm` function L243-286 — `( llm: &Arc<dyn arawn_llm::LlmClient>, model: &str, prompt: &str, url: &str, con...`
-  `tests` module L289-525 — `-`
-  `test_ctx` function L300-303 — `() -> EngineToolContext`
-  `test_ctx_with_mock` function L305-311 — `(responses: Vec<MockResponse>) -> (EngineToolContext, Arc<MockLlmClient>)`
-  `html_to_markdown_headings` function L316-320 — `()`
-  `html_to_markdown_links` function L323-327 — `()`
-  `html_to_markdown_lists` function L330-334 — `()`
-  `html_to_markdown_code` function L337-340 — `()`
-  `non_html_passthrough` function L343-346 — `()`
-  `strip_tags_basic` function L351-353 — `()`
-  `strip_tags_collapses_whitespace` function L356-361 — `()`
-  `cache_entry_expiry` function L366-380 — `()`
-  `cache_stores_and_retrieves` function L383-402 — `()`
-  `large_content_truncated` function L407-412 — `()`
-  `schema_is_valid` function L417-426 — `()`
-  `http_upgraded_description` function L429-432 — `()`
-  `summarize_with_mock_llm` function L437-455 — `()`
-  `summarize_sends_correct_request_shape` function L458-473 — `()`
-  `execute_without_llm_returns_raw_text` function L476-479 — `()`
-  `summarize_empty_content` function L482-497 — `()`
-  `summarize_multipart_response` function L500-524 — `()`

#### crates/arawn-engine/src/tools/web_search.rs

- pub `WebSearchTool` struct L7 — `-` — Searches the web and returns results to inform responses.
-  `WebSearchTool` type L10-140 — `impl Tool for WebSearchTool`
-  `name` function L11-13 — `(&self) -> &str`
-  `description` function L15-22 — `(&self) -> &str`
-  `is_read_only` function L24-26 — `(&self) -> bool`
-  `category` function L28-30 — `(&self) -> ToolCategory`
-  `parameters_schema` function L32-54 — `(&self) -> Value`
-  `execute` function L56-139 — `(&self, _ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput,...`
-  `SearchResult` struct L142-146 — `{ title: String, url: String, snippet: String }`
-  `parse_ddg_results` function L148-171 — `(html: &str, max: usize) -> Vec<SearchResult>`
-  `extract_tag_content` function L173-181 — `(html: &str, after: &str) -> String`
-  `extract_href` function L183-196 — `(html: &str) -> String`
-  `extract_after_class` function L198-210 — `(html: &str, class: &str) -> String`
-  `strip_tags` function L212-224 — `(html: &str) -> String`
-  `urlencod` function L226-234 — `(s: &str) -> String`
-  `urldecod` function L236-254 — `(s: &str) -> String`
-  `tests` module L257-396 — `-`
-  `urlencod_spaces` function L261-263 — `()`
-  `urlencod_special_chars` function L266-268 — `()`
-  `urldecod_percent` function L271-273 — `()`
-  `urldecod_stops_at_ampersand` function L276-278 — `()`
-  `urldecod_plus_to_space` function L281-283 — `()`
-  `strip_tags_removes_html` function L286-288 — `()`
-  `strip_tags_empty` function L291-293 — `()`
-  `schema_is_valid` function L296-305 — `()`
-  `parse_ddg_results_empty_html` function L308-311 — `()`
-  `parse_ddg_results_no_results` function L314-318 — `()`
-  `parse_ddg_results_respects_max` function L321-332 — `()`
-  `parse_ddg_results_extracts_fields` function L335-345 — `()`
-  `blocked_domains_filter` function L348-373 — `()`
-  `allowed_domains_builds_site_clause` function L376-389 — `()`
-  `is_read_only` function L392-395 — `()`

#### crates/arawn-engine/src/tools/workstream.rs

- pub `SessionWorkstream` struct L26-28 — `{ inner: Arc<Mutex<String>> }` — Holder for the session-active workstream name.
- pub `new` function L31-35 — `(initial: impl Into<String>) -> Self` — the shim is enough to make `switch` / `show` work.
- pub `scratch` function L37-39 — `() -> Self` — the shim is enough to make `switch` / `show` work.
- pub `current` function L41-43 — `(&self) -> String` — the shim is enough to make `switch` / `show` work.
- pub `set` function L45-47 — `(&self, name: impl Into<String>)` — the shim is enough to make `switch` / `show` work.
- pub `WorkstreamCreateTool` struct L60-62 — `{ store: Arc<Mutex<Store>> }` — the shim is enough to make `switch` / `show` work.
- pub `new` function L65-67 — `(store: Arc<Mutex<Store>>) -> Self` — the shim is enough to make `switch` / `show` work.
- pub `WorkstreamListTool` struct L153-156 — `{ store: Arc<Mutex<Store>>, active: SessionWorkstream }` — the shim is enough to make `switch` / `show` work.
- pub `new` function L159-164 — `(store: Arc<Mutex<Store>>) -> Self` — the shim is enough to make `switch` / `show` work.
- pub `with_active` function L166-169 — `(mut self, active: SessionWorkstream) -> Self` — the shim is enough to make `switch` / `show` work.
- pub `WorkstreamSwitchTool` struct L242-245 — `{ store: Arc<Mutex<Store>>, active: SessionWorkstream }` — the shim is enough to make `switch` / `show` work.
- pub `new` function L248-250 — `(store: Arc<Mutex<Store>>, active: SessionWorkstream) -> Self` — the shim is enough to make `switch` / `show` work.
- pub `WorkstreamShowTool` struct L322-325 — `{ store: Arc<Mutex<Store>>, active: SessionWorkstream }` — the shim is enough to make `switch` / `show` work.
- pub `new` function L328-330 — `(store: Arc<Mutex<Store>>, active: SessionWorkstream) -> Self` — the shim is enough to make `switch` / `show` work.
- pub `WorkstreamDescribeTool` struct L401-403 — `{ store: Arc<Mutex<Store>> }` — the shim is enough to make `switch` / `show` work.
- pub `new` function L406-408 — `(store: Arc<Mutex<Store>>) -> Self` — the shim is enough to make `switch` / `show` work.
- pub `BindBackfillHook` interface L468-470 — `{ fn on_bind() }` — Side-channel that fires when `/workstream bind` lands a new
- pub `WorkstreamBindTool` struct L472-475 — `{ store: Arc<Mutex<Store>>, hook: Option<Arc<dyn BindBackfillHook>> }` — the shim is enough to make `switch` / `show` work.
- pub `new` function L478-480 — `(store: Arc<Mutex<Store>>) -> Self` — the shim is enough to make `switch` / `show` work.
- pub `with_backfill_hook` function L482-485 — `(mut self, hook: Arc<dyn BindBackfillHook>) -> Self` — the shim is enough to make `switch` / `show` work.
- pub `WorkstreamUnbindTool` struct L553-555 — `{ store: Arc<Mutex<Store>> }` — the shim is enough to make `switch` / `show` work.
- pub `new` function L558-560 — `(store: Arc<Mutex<Store>>) -> Self` — the shim is enough to make `switch` / `show` work.
- pub `WorkstreamPromoteTool` struct L624-627 — `{ store: Arc<Mutex<Store>>, router: Arc<crate::workstream_router::WorkstreamMemo...` — Move one entity from the `scratch` workstream into a named target.
- pub `new` function L630-635 — `( store: Arc<Mutex<Store>>, router: Arc<crate::workstream_router::WorkstreamMemo...` — the shim is enough to make `switch` / `show` work.
- pub `WorkstreamDeleteTool` struct L778-781 — `{ store: Arc<Mutex<Store>>, active: SessionWorkstream }` — the shim is enough to make `switch` / `show` work.
- pub `new` function L784-786 — `(store: Arc<Mutex<Store>>, active: SessionWorkstream) -> Self` — the shim is enough to make `switch` / `show` work.
-  `SessionWorkstream` type L30-48 — `= SessionWorkstream` — the shim is enough to make `switch` / `show` work.
-  `SessionWorkstream` type L50-54 — `impl Default for SessionWorkstream` — the shim is enough to make `switch` / `show` work.
-  `default` function L51-53 — `() -> Self` — the shim is enough to make `switch` / `show` work.
-  `WorkstreamCreateTool` type L64-68 — `= WorkstreamCreateTool` — the shim is enough to make `switch` / `show` work.
-  `WorkstreamCreateTool` type L71-147 — `impl Tool for WorkstreamCreateTool` — the shim is enough to make `switch` / `show` work.
-  `name` function L72-74 — `(&self) -> &str` — the shim is enough to make `switch` / `show` work.
-  `description` function L76-80 — `(&self) -> &str` — the shim is enough to make `switch` / `show` work.
-  `category` function L82-84 — `(&self) -> ToolCategory` — the shim is enough to make `switch` / `show` work.
-  `parameters_schema` function L86-96 — `(&self) -> Value` — the shim is enough to make `switch` / `show` work.
-  `execute` function L98-146 — `( &self, ctx: &dyn arawn_tool::ToolContext, params: Value, ) -> Result<ToolOutpu...` — the shim is enough to make `switch` / `show` work.
-  `WorkstreamListTool` type L158-170 — `= WorkstreamListTool` — the shim is enough to make `switch` / `show` work.
-  `WorkstreamListTool` type L173-236 — `impl Tool for WorkstreamListTool` — the shim is enough to make `switch` / `show` work.
-  `name` function L174-176 — `(&self) -> &str` — the shim is enough to make `switch` / `show` work.
-  `description` function L178-180 — `(&self) -> &str` — the shim is enough to make `switch` / `show` work.
-  `is_read_only` function L182-184 — `(&self) -> bool` — the shim is enough to make `switch` / `show` work.
-  `category` function L186-188 — `(&self) -> ToolCategory` — the shim is enough to make `switch` / `show` work.
-  `parameters_schema` function L190-198 — `(&self) -> Value` — the shim is enough to make `switch` / `show` work.
-  `execute` function L200-235 — `( &self, _ctx: &dyn arawn_tool::ToolContext, params: Value, ) -> Result<ToolOutp...` — the shim is enough to make `switch` / `show` work.
-  `WorkstreamSwitchTool` type L247-251 — `= WorkstreamSwitchTool` — the shim is enough to make `switch` / `show` work.
-  `WorkstreamSwitchTool` type L254-316 — `impl Tool for WorkstreamSwitchTool` — the shim is enough to make `switch` / `show` work.
-  `name` function L255-257 — `(&self) -> &str` — the shim is enough to make `switch` / `show` work.
-  `description` function L259-263 — `(&self) -> &str` — the shim is enough to make `switch` / `show` work.
-  `category` function L265-267 — `(&self) -> ToolCategory` — the shim is enough to make `switch` / `show` work.
-  `parameters_schema` function L269-275 — `(&self) -> Value` — the shim is enough to make `switch` / `show` work.
-  `execute` function L277-315 — `( &self, _ctx: &dyn arawn_tool::ToolContext, params: Value, ) -> Result<ToolOutp...` — the shim is enough to make `switch` / `show` work.
-  `WorkstreamShowTool` type L327-331 — `= WorkstreamShowTool` — the shim is enough to make `switch` / `show` work.
-  `WorkstreamShowTool` type L334-395 — `impl Tool for WorkstreamShowTool` — the shim is enough to make `switch` / `show` work.
-  `name` function L335-337 — `(&self) -> &str` — the shim is enough to make `switch` / `show` work.
-  `description` function L339-342 — `(&self) -> &str` — the shim is enough to make `switch` / `show` work.
-  `is_read_only` function L344-346 — `(&self) -> bool` — the shim is enough to make `switch` / `show` work.
-  `category` function L348-350 — `(&self) -> ToolCategory` — the shim is enough to make `switch` / `show` work.
-  `parameters_schema` function L352-360 — `(&self) -> Value` — the shim is enough to make `switch` / `show` work.
-  `execute` function L362-394 — `( &self, _ctx: &dyn arawn_tool::ToolContext, params: Value, ) -> Result<ToolOutp...` — the shim is enough to make `switch` / `show` work.
-  `WorkstreamDescribeTool` type L405-409 — `= WorkstreamDescribeTool` — the shim is enough to make `switch` / `show` work.
-  `WorkstreamDescribeTool` type L412-459 — `impl Tool for WorkstreamDescribeTool` — the shim is enough to make `switch` / `show` work.
-  `name` function L413-415 — `(&self) -> &str` — the shim is enough to make `switch` / `show` work.
-  `description` function L417-420 — `(&self) -> &str` — the shim is enough to make `switch` / `show` work.
-  `category` function L422-424 — `(&self) -> ToolCategory` — the shim is enough to make `switch` / `show` work.
-  `parameters_schema` function L426-435 — `(&self) -> Value` — the shim is enough to make `switch` / `show` work.
-  `execute` function L437-458 — `( &self, _ctx: &dyn arawn_tool::ToolContext, params: Value, ) -> Result<ToolOutp...` — the shim is enough to make `switch` / `show` work.
-  `WorkstreamBindTool` type L477-486 — `= WorkstreamBindTool` — the shim is enough to make `switch` / `show` work.
-  `WorkstreamBindTool` type L489-551 — `impl Tool for WorkstreamBindTool` — the shim is enough to make `switch` / `show` work.
-  `name` function L490-492 — `(&self) -> &str` — the shim is enough to make `switch` / `show` work.
-  `description` function L494-497 — `(&self) -> &str` — the shim is enough to make `switch` / `show` work.
-  `category` function L499-501 — `(&self) -> ToolCategory` — the shim is enough to make `switch` / `show` work.
-  `parameters_schema` function L503-512 — `(&self) -> Value` — the shim is enough to make `switch` / `show` work.
-  `execute` function L514-550 — `( &self, _ctx: &dyn arawn_tool::ToolContext, params: Value, ) -> Result<ToolOutp...` — the shim is enough to make `switch` / `show` work.
-  `WorkstreamUnbindTool` type L557-561 — `= WorkstreamUnbindTool` — the shim is enough to make `switch` / `show` work.
-  `WorkstreamUnbindTool` type L564-614 — `impl Tool for WorkstreamUnbindTool` — the shim is enough to make `switch` / `show` work.
-  `name` function L565-567 — `(&self) -> &str` — the shim is enough to make `switch` / `show` work.
-  `description` function L569-571 — `(&self) -> &str` — the shim is enough to make `switch` / `show` work.
-  `category` function L573-575 — `(&self) -> ToolCategory` — the shim is enough to make `switch` / `show` work.
-  `parameters_schema` function L577-586 — `(&self) -> Value` — the shim is enough to make `switch` / `show` work.
-  `execute` function L588-613 — `( &self, _ctx: &dyn arawn_tool::ToolContext, params: Value, ) -> Result<ToolOutp...` — the shim is enough to make `switch` / `show` work.
-  `WorkstreamPromoteTool` type L629-636 — `= WorkstreamPromoteTool` — the shim is enough to make `switch` / `show` work.
-  `WorkstreamPromoteTool` type L639-772 — `impl Tool for WorkstreamPromoteTool` — the shim is enough to make `switch` / `show` work.
-  `name` function L640-642 — `(&self) -> &str` — the shim is enough to make `switch` / `show` work.
-  `description` function L644-649 — `(&self) -> &str` — the shim is enough to make `switch` / `show` work.
-  `category` function L651-653 — `(&self) -> ToolCategory` — the shim is enough to make `switch` / `show` work.
-  `parameters_schema` function L655-664 — `(&self) -> Value` — the shim is enough to make `switch` / `show` work.
-  `execute` function L666-771 — `( &self, _ctx: &dyn arawn_tool::ToolContext, params: Value, ) -> Result<ToolOutp...` — the shim is enough to make `switch` / `show` work.
-  `WorkstreamDeleteTool` type L783-787 — `= WorkstreamDeleteTool` — the shim is enough to make `switch` / `show` work.
-  `WorkstreamDeleteTool` type L790-838 — `impl Tool for WorkstreamDeleteTool` — the shim is enough to make `switch` / `show` work.
-  `name` function L791-793 — `(&self) -> &str` — the shim is enough to make `switch` / `show` work.
-  `description` function L795-798 — `(&self) -> &str` — the shim is enough to make `switch` / `show` work.
-  `category` function L800-802 — `(&self) -> ToolCategory` — the shim is enough to make `switch` / `show` work.
-  `parameters_schema` function L804-810 — `(&self) -> Value` — the shim is enough to make `switch` / `show` work.
-  `execute` function L812-837 — `( &self, _ctx: &dyn arawn_tool::ToolContext, params: Value, ) -> Result<ToolOutp...` — the shim is enough to make `switch` / `show` work.
-  `tests` module L841-1108 — `-` — the shim is enough to make `switch` / `show` work.
-  `setup` function L845-850 — `() -> (tempfile::TempDir, Arc<Mutex<Store>>, SessionWorkstream)` — the shim is enough to make `switch` / `show` work.
-  `test_ctx` function L852-856 — `(tmp: &tempfile::TempDir) -> crate::context::EngineToolContext` — the shim is enough to make `switch` / `show` work.
-  `create_succeeds_with_valid_slug` function L859-868 — `()` — the shim is enough to make `switch` / `show` work.
-  `create_refuses_scratch` function L871-879 — `()` — the shim is enough to make `switch` / `show` work.
-  `switch_updates_active` function L882-896 — `()` — the shim is enough to make `switch` / `show` work.
-  `switch_unknown_errors` function L899-908 — `()` — the shim is enough to make `switch` / `show` work.
-  `show_defaults_to_active` function L911-917 — `()` — the shim is enough to make `switch` / `show` work.
-  `describe_updates_description` function L920-943 — `()` — the shim is enough to make `switch` / `show` work.
-  `bind_and_unbind_round_trip` function L946-976 — `()` — the shim is enough to make `switch` / `show` work.
-  `delete_refuses_scratch` function L979-988 — `()` — the shim is enough to make `switch` / `show` work.
-  `delete_refuses_currently_active` function L991-1006 — `()` — the shim is enough to make `switch` / `show` work.
-  `delete_soft_marks_archived` function L1009-1026 — `()` — the shim is enough to make `switch` / `show` work.
-  `promote_moves_entity_from_scratch_to_target` function L1029-1069 — `()` — the shim is enough to make `switch` / `show` work.
-  `promote_refuses_unknown_target` function L1072-1091 — `()` — the shim is enough to make `switch` / `show` work.
-  `list_marks_active` function L1094-1107 — `()` — the shim is enough to make `switch` / `show` work.

### crates/arawn-extractor/src

> *Semantic summary to be generated by AI agent.*

#### crates/arawn-extractor/src/chain.rs

- pub `ChainOutcome` struct L20-27 — `{ entities_written: Vec<Uuid>, relations_written: usize, skipped: bool }` — Per-row outcome of a single chain run.
- pub `ExtractionChain` interface L30-40 — `{ fn run() }` — real 4-stage chain (classify → extract → link-by-name → write).
- pub `StubChain` struct L45 — `-` — No-op chain.
-  `StubChain` type L48-61 — `impl ExtractionChain for StubChain` — real 4-stage chain (classify → extract → link-by-name → write).
-  `run` function L49-60 — `( &self, _workstream: &Workstream, _row: &ProjectionRow, _kb: &MemoryManager, ) ...` — real 4-stage chain (classify → extract → link-by-name → write).

#### crates/arawn-extractor/src/cot.rs

- pub `CotChain` struct L37-43 — `{ client: Arc<dyn LlmClient>, model: String, link_score_floor: f32 }` — The real CoT chain.
- pub `new` function L46-52 — `(client: Arc<dyn LlmClient>, model: impl Into<String>) -> Self` — steward (Phase 5) refines vocabulary later.
- pub `with_link_score_floor` function L54-57 — `(mut self, floor: f32) -> Self` — steward (Phase 5) refines vocabulary later.
-  `CotChain` type L45-58 — `= CotChain` — steward (Phase 5) refines vocabulary later.
-  `CotChain` type L61-96 — `impl ExtractionChain for CotChain` — steward (Phase 5) refines vocabulary later.
-  `run` function L62-95 — `( &self, workstream: &Workstream, row: &ProjectionRow, kb: &MemoryManager, ) -> ...` — steward (Phase 5) refines vocabulary later.
-  `ClassifyResult` struct L103-107 — `{ in_scope: bool, reason: String }` — steward (Phase 5) refines vocabulary later.
-  `CotChain` type L109-139 — `= CotChain` — steward (Phase 5) refines vocabulary later.
-  `classify` function L110-138 — `( &self, ws: &Workstream, row: &ProjectionRow, ) -> Result<ClassifyResult, Extra...` — steward (Phase 5) refines vocabulary later.
-  `parse_classify` function L141-145 — `(raw: &str) -> Result<ClassifyResult, ExtractionError>` — steward (Phase 5) refines vocabulary later.
-  `ExtractedCandidate` struct L152-159 — `{ entity_type: String, title: String, content: String, tags: Vec<String> }` — steward (Phase 5) refines vocabulary later.
-  `CotChain` type L161-189 — `= CotChain` — steward (Phase 5) refines vocabulary later.
-  `extract` function L162-188 — `( &self, ws: &Workstream, row: &ProjectionRow, ) -> Result<Vec<ExtractedCandidat...` — steward (Phase 5) refines vocabulary later.
-  `parse_candidates` function L191-195 — `(raw: &str) -> Result<Vec<ExtractedCandidate>, ExtractionError>` — steward (Phase 5) refines vocabulary later.
-  `LinkProposal` struct L202-206 — `{ from: String, rel: String, to_name: String }` — steward (Phase 5) refines vocabulary later.
-  `CotChain` type L208-247 — `= CotChain` — steward (Phase 5) refines vocabulary later.
-  `link_by_name` function L209-246 — `( &self, ws: &Workstream, candidates: &[ExtractedCandidate], ) -> Result<Vec<Lin...` — steward (Phase 5) refines vocabulary later.
-  `parse_links` function L249-253 — `(raw: &str) -> Result<Vec<LinkProposal>, ExtractionError>` — steward (Phase 5) refines vocabulary later.
-  `CotChain` type L259-334 — `= CotChain` — steward (Phase 5) refines vocabulary later.
-  `write` function L260-333 — `( &self, row: &ProjectionRow, candidates: &[ExtractedCandidate], links: &[LinkPr...` — steward (Phase 5) refines vocabulary later.
-  `resolve_by_fts` function L338-352 — `( kb: &MemoryManager, name: &str, _floor: f32, ) -> Option<(Uuid, Scope)>` — FTS-resolve a name against both KB tiers.
-  `first_fts_hit` function L354-359 — `(store: &Arc<MemoryStore>, query: &str) -> Option<Uuid>` — steward (Phase 5) refines vocabulary later.
-  `parse_entity_type` function L361-363 — `(s: &str) -> Option<EntityType>` — steward (Phase 5) refines vocabulary later.
-  `parse_relation_type` function L365-367 — `(s: &str) -> Option<RelationType>` — steward (Phase 5) refines vocabulary later.
-  `projection_id_to_uuid` function L371-373 — `(projection_id: &str) -> Uuid` — Derive a deterministic Uuid v5 from the projection row id so the
-  `truncate` function L375-380 — `(s: &str, max_chars: usize) -> String` — steward (Phase 5) refines vocabulary later.
-  `tests` module L383-456 — `-` — steward (Phase 5) refines vocabulary later.
-  `parse_classify_in_scope` function L387-392 — `()` — steward (Phase 5) refines vocabulary later.
-  `parse_classify_out_of_scope` function L395-399 — `()` — steward (Phase 5) refines vocabulary later.
-  `parse_candidates_empty_array` function L402-405 — `()` — steward (Phase 5) refines vocabulary later.
-  `parse_candidates_basic` function L408-415 — `()` — steward (Phase 5) refines vocabulary later.
-  `parse_links_basic` function L418-423 — `()` — steward (Phase 5) refines vocabulary later.
-  `entity_type_lowercased_for_parse` function L426-430 — `()` — steward (Phase 5) refines vocabulary later.
-  `relation_type_lowercased_for_parse` function L433-437 — `()` — steward (Phase 5) refines vocabulary later.
-  `projection_id_to_uuid_is_deterministic` function L440-446 — `()` — steward (Phase 5) refines vocabulary later.
-  `truncate_preserves_short_input` function L449-455 — `()` — steward (Phase 5) refines vocabulary later.
-  `integration` module L464-902 — `-` — steward (Phase 5) refines vocabulary later.
-  `KeyedMockLlm` struct L490-497 — `{ classify: Mutex<VecDeque<Value>>, extract: Mutex<VecDeque<Value>>, link: Mutex...` — Inspects the system prompt to detect which CoT stage is calling
-  `KeyedMockLlm` type L499-527 — `= KeyedMockLlm` — steward (Phase 5) refines vocabulary later.
-  `new` function L500-509 — `() -> Self` — steward (Phase 5) refines vocabulary later.
-  `default_classify` function L511-514 — `(self, v: Value) -> Self` — steward (Phase 5) refines vocabulary later.
-  `default_extract` function L515-518 — `(self, v: Value) -> Self` — steward (Phase 5) refines vocabulary later.
-  `default_link` function L519-522 — `(self, v: Value) -> Self` — steward (Phase 5) refines vocabulary later.
-  `push_classify` function L524-526 — `(&self, v: Value)` — steward (Phase 5) refines vocabulary later.
-  `classify_stage` function L529-531 — `(sys: &str) -> bool` — steward (Phase 5) refines vocabulary later.
-  `extract_stage` function L532-534 — `(sys: &str) -> bool` — steward (Phase 5) refines vocabulary later.
-  `link_stage` function L535-537 — `(sys: &str) -> bool` — steward (Phase 5) refines vocabulary later.
-  `KeyedMockLlm` type L540-580 — `= KeyedMockLlm` — steward (Phase 5) refines vocabulary later.
-  `stream` function L541-579 — `( &self, request: ChatRequest, ) -> Result< Pin<Box<dyn futures::Stream<Item = R...` — steward (Phase 5) refines vocabulary later.
-  `ws` function L584-588 — `(name: &str, desc: &str) -> Workstream` — steward (Phase 5) refines vocabulary later.
-  `fixture_proj` function L590-603 — `(id: &str, body: &str, ts_offset: i64) -> GmailMessageProjection` — steward (Phase 5) refines vocabulary later.
-  `Fixture` struct L605-611 — `{ _tmp: tempfile::TempDir, store: Arc<std::sync::Mutex<Store>>, proj: Arc<Projec...` — steward (Phase 5) refines vocabulary later.
-  `setup` function L613-648 — `() -> Fixture` — steward (Phase 5) refines vocabulary later.
-  `Fixture` type L650-663 — `= Fixture` — steward (Phase 5) refines vocabulary later.
-  `kb` function L651-656 — `(&self, name: &str) -> Arc<MemoryManager>` — steward (Phase 5) refines vocabulary later.
-  `cursor` function L658-662 — `(&self, ws_name: &str, feed_type: &str) -> Option<chrono::DateTime<chrono::Utc>>` — steward (Phase 5) refines vocabulary later.
-  `runner_with` function L665-679 — `( fx: &Fixture, mock: Arc<KeyedMockLlm>, batch_size: usize, ) -> ExtractorRunner` — steward (Phase 5) refines vocabulary later.
-  `happy_path_extracts_into_workstream` function L684-716 — `()` — steward (Phase 5) refines vocabulary later.
-  `out_of_scope_skips_but_advances_cursor` function L719-740 — `()` — steward (Phase 5) refines vocabulary later.
-  `link_by_name_resolves_to_existing_kb_entity` function L743-778 — `()` — steward (Phase 5) refines vocabulary later.
-  `link_to_missing_target_is_dropped_without_panic` function L781-804 — `()` — steward (Phase 5) refines vocabulary later.
-  `backfill_walks_existing_rows` function L807-833 — `()` — steward (Phase 5) refines vocabulary later.
-  `rerun_is_idempotent_via_cursor` function L836-861 — `()` — steward (Phase 5) refines vocabulary later.
-  `two_workstreams_each_get_the_entity` function L864-901 — `()` — steward (Phase 5) refines vocabulary later.

#### crates/arawn-extractor/src/error.rs

- pub `ExtractionError` enum L4-19 — `Storage | Memory | Llm | Parse | NotFound`
-  `ExtractionError` type L21-25 — `= ExtractionError`
-  `from` function L22-24 — `(e: arawn_storage::StorageError) -> Self`
-  `ExtractionError` type L27-31 — `= ExtractionError`
-  `from` function L28-30 — `(e: arawn_memory::MemoryError) -> Self`
-  `ExtractionError` type L33-37 — `= ExtractionError`
-  `from` function L34-36 — `(e: arawn_projections::ProjectionError) -> Self`
-  `ExtractionError` type L39-43 — `= ExtractionError`
-  `from` function L40-42 — `(e: serde_json::Error) -> Self`

#### crates/arawn-extractor/src/lib.rs

- pub `chain` module L10 — `-` — Sits between feed-driven projections and per-workstream memory KBs.
- pub `cot` module L11 — `-` — pick up only new rows.
- pub `error` module L12 — `-` — pick up only new rows.
- pub `llm_text` module L13 — `-` — pick up only new rows.
- pub `runner` module L14 — `-` — pick up only new rows.

#### crates/arawn-extractor/src/llm_text.rs

- pub `complete_text` function L19-54 — `( client: &Arc<dyn LlmClient>, model: &str, system: &str, user: &str, ) -> Resul...` — Send a single-turn (system + user) chat request and collect every
- pub `extract_json_block` function L59-83 — `(raw: &str) -> Option<&str>` — Many LLMs wrap JSON output in ```json fences or prose.
-  `tests` module L86-111 — `-` — before parsing JSON, so streaming buys us nothing — just collect.
-  `extracts_object_from_fenced_block` function L90-93 — `()` — before parsing JSON, so streaming buys us nothing — just collect.
-  `extracts_array_from_prose` function L96-99 — `()` — before parsing JSON, so streaming buys us nothing — just collect.
-  `handles_nested_braces` function L102-105 — `()` — before parsing JSON, so streaming buys us nothing — just collect.
-  `returns_none_when_absent` function L108-110 — `()` — before parsing JSON, so streaming buys us nothing — just collect.

#### crates/arawn-extractor/src/runner.rs

- pub `RunStats` struct L26-33 — `{ processed: usize, kept: usize, skipped: usize, errors: usize, entities_written...` — Stats for one `run_for_workstream` invocation.
- pub `DEFAULT_BATCH_SIZE` variable L37 — `: usize` — Default cap on rows per `run_for_workstream` invocation.
- pub `MemoryResolver` type L42-46 — `= Arc< dyn Fn(&str) -> Result<Arc<arawn_memory::MemoryManager>, ExtractionError>...` — Function that materializes the `MemoryManager` for a workstream
- pub `ExtractorRunner` struct L51-60 — `{ store: Arc<std::sync::Mutex<Store>>, projections: Arc<ProjectionStore>, memory...` — The runner owns the bits that survive across calls — store handles,
- pub `new` function L63-77 — `( store: Arc<std::sync::Mutex<Store>>, projections: Arc<ProjectionStore>, memory...` — hook after a projection write.
- pub `with_batch_size` function L79-82 — `(mut self, n: usize) -> Self` — hook after a projection write.
- pub `run_for_workstream` function L88-163 — `( &self, workstream: &Workstream, feed_type: &str, ) -> Result<RunStats, Extract...` — Process one batch of new projection rows for `workstream`.
- pub `run_for_workstream_until_exhausted` function L171-205 — `( &self, workstream: &Workstream, feed_type: &str, max_duration: std::time::Dura...` — Run `run_for_workstream` in a loop until either the projection
- pub `spawn_backfill` function L215-274 — `(self: Arc<Self>, workstream_name: String, feed_types: Vec<String>)` — Spawn a backfill task for `(workstream_name, feed_types)`.
- pub `run_for_all_workstreams` function L280-314 — `( &self, feed_type: &str, ) -> Result<Vec<(String, RunStats)>, ExtractionError>` — Iterate every active (non-archived) workstream and run extraction
-  `ExtractorRunner` type L62-315 — `= ExtractorRunner` — hook after a projection write.
-  `MAX` variable L216 — `: std::time::Duration` — hook after a projection write.
-  `fetch_projection_rows` function L319-379 — `( store: &ProjectionStore, feed_type: &str, cursor_ts: Option<DateTime<Utc>>, li...` — Page projection rows of a given feed_type whose `source_ts` is
-  `tests` module L382-567 — `-` — hook after a projection write.
-  `ws` function L388-392 — `(name: &str) -> Workstream` — hook after a projection write.
-  `fixture_proj` function L394-407 — `(id: &str, body: &str, ts_offset: i64) -> GmailMessageProjection` — hook after a projection write.
-  `setup` function L409-431 — `() -> ( tempfile::TempDir, Arc<std::sync::Mutex<Store>>, Arc<ProjectionStore>, M...` — hook after a projection write.
-  `empty_projection_table_is_a_noop` function L434-442 — `()` — hook after a projection write.
-  `stub_chain_advances_cursor_and_marks_skipped` function L445-472 — `()` — hook after a projection write.
-  `rerun_with_no_new_rows_is_a_noop` function L475-488 — `()` — hook after a projection write.
-  `run_until_exhausted_walks_all_pages` function L491-510 — `()` — hook after a projection write.
-  `spawn_backfill_is_idempotent_for_in_flight_key` function L513-545 — `()` — hook after a projection write.
-  `run_for_all_workstreams_iterates_active_only` function L548-566 — `()` — hook after a projection write.

### crates/arawn-feeds/src

> *Semantic summary to be generated by AI agent.*

#### crates/arawn-feeds/src/cadence.rs

- pub `MIN_CADENCE` variable L15 — `: Duration` — Minimum allowed cadence.
- pub `validate_cadence` function L20-52 — `(cron_expr: &str) -> Result<(), FeedError>` — Validate a cron expression in UTC and reject anything whose minimum
-  `tests` module L55-86 — `-` — interval that's also polite to providers' rate limits.
-  `fifteen_minute_cadence_is_accepted` function L59-66 — `()` — interval that's also polite to providers' rate limits.
-  `sub_fifteen_minute_cadence_is_rejected` function L69-79 — `()` — interval that's also polite to providers' rate limits.
-  `malformed_cron_is_rejected` function L82-85 — `()` — interval that's also polite to providers' rate limits.

#### crates/arawn-feeds/src/dispatch.rs

- pub `FeedRuntimeContext` struct L42-57 — `{ conn: Arc<Mutex<Connection>>, layout: Arc<DataLayout>, registry: Arc<FeedTempl...` — Shared handles the dispatch task needs to actually run.
- pub `FeedDispatchTask` struct L62-68 — `{ feed_id: String, runtime: FeedRuntimeContext, deps: Vec<TaskNamespace> }` — One cloacina-compatible task per feed.
- pub `new` function L71-77 — `(feed_id: impl Into<String>, runtime: FeedRuntimeContext) -> Self` — retry/audit machinery handles the rest.
- pub `run_feed` function L113-118 — `( feed_id: &str, runtime: &FeedRuntimeContext, ) -> Result<crate::template::RunO...` — The actual fetch+write cycle.
- pub `run_feed_force` function L123-128 — `( feed_id: &str, runtime: &FeedRuntimeContext, ) -> Result<crate::template::RunO...` — Variant that ignores the `enabled` flag — used by the backfill
- pub `projection_feed_types_for` function L280-295 — `(template_name: &str) -> Vec<String>` — Map a feed template name to the projection feed_types it produces.
-  `FeedDispatchTask` type L70-78 — `= FeedDispatchTask` — retry/audit machinery handles the rest.
-  `FeedDispatchTask` type L81-103 — `impl Task for FeedDispatchTask` — retry/audit machinery handles the rest.
-  `id` function L82-84 — `(&self) -> &str` — retry/audit machinery handles the rest.
-  `dependencies` function L86-88 — `(&self) -> &[TaskNamespace]` — retry/audit machinery handles the rest.
-  `execute` function L90-102 — `( &self, context: Context<Value>, ) -> Result<Context<Value>, TaskError>` — retry/audit machinery handles the rest.
-  `run_feed_inner` function L130-274 — `( feed_id: &str, runtime: &FeedRuntimeContext, force: bool, ) -> Result<crate::t...` — retry/audit machinery handles the rest.
-  `persist_meta_failure` function L297-312 — `( feed_dir: &std::path::Path, template: &str, params: &crate::types::TemplatePar...` — retry/audit machinery handles the rest.
-  `tests` module L315-452 — `-` — retry/audit machinery handles the rest.
-  `open_test_db` function L324-339 — `() -> Connection` — retry/audit machinery handles the rest.
-  `build_runtime` function L341-350 — `(tmp_root: &std::path::Path, conn: Connection) -> FeedRuntimeContext` — retry/audit machinery handles the rest.
-  `run_feed_executes_stub_template_and_persists_meta` function L353-382 — `()` — retry/audit machinery handles the rest.
-  `run_feed_increments_cursor_across_invocations` function L385-416 — `()` — retry/audit machinery handles the rest.
-  `run_feed_skips_disabled_feed` function L419-439 — `()` — retry/audit machinery handles the rest.
-  `run_feed_returns_storage_error_for_missing_id` function L442-451 — `()` — retry/audit machinery handles the rest.

#### crates/arawn-feeds/src/error.rs

- pub `FeedError` enum L8-40 — `Auth | RateLimited | Storage | Schema | Provider | InvalidParams` — Error type used by templates and the runtime.

#### crates/arawn-feeds/src/layout.rs

- pub `DataLayout` struct L19-22 — `{ root: PathBuf }` — is the template's territory.
- pub `new` function L28-32 — `(data_root: impl Into<PathBuf>) -> Self` — `data_root` is the arawn data dir (e.g.
- pub `root` function L34-36 — `(&self) -> &Path` — is the template's territory.
- pub `feed_dir` function L42-49 — `(&self, template_name: &str, feed_id: &str) -> Result<PathBuf, FeedError>` — `{root}/<provider>/<template_name>/<feed_id>/`.
- pub `ensure_feed_dir` function L52-61 — `( &self, template_name: &str, feed_id: &str, ) -> Result<PathBuf, FeedError>` — Create the feed dir if it doesn't exist; return its path.
-  `DataLayout` type L24-62 — `= DataLayout` — is the template's territory.
-  `tests` module L65-91 — `-` — is the template's territory.
-  `feed_dir_splits_on_slash` function L69-73 — `()` — is the template's territory.
-  `feed_dir_rejects_template_without_provider` function L76-80 — `()` — is the template's territory.
-  `ensure_feed_dir_creates_path` function L83-90 — `()` — is the template's territory.

#### crates/arawn-feeds/src/lib.rs

- pub `cadence` module L24 — `-` — ingestion across personal + watched spaces.
- pub `clients` module L25 — `-` — retry, audit, single-instance enforcement.
- pub `dispatch` module L26 — `-` — retry, audit, single-instance enforcement.
- pub `error` module L27 — `-` — retry, audit, single-instance enforcement.
- pub `layout` module L28 — `-` — retry, audit, single-instance enforcement.
- pub `meta` module L29 — `-` — retry, audit, single-instance enforcement.
- pub `registry` module L30 — `-` — retry, audit, single-instance enforcement.
- pub `runtime` module L31 — `-` — retry, audit, single-instance enforcement.
- pub `store` module L32 — `-` — retry, audit, single-instance enforcement.
- pub `template` module L33 — `-` — retry, audit, single-instance enforcement.
- pub `templates` module L34 — `-` — retry, audit, single-instance enforcement.
- pub `types` module L35 — `-` — retry, audit, single-instance enforcement.

#### crates/arawn-feeds/src/meta.rs

- pub `MetaStore` struct L15 — `-` — filesystem.
- pub `read` function L21-33 — `(feed_dir: &Path) -> Result<Option<FeedMeta>, FeedError>` — Read `feed_dir/meta.json`.
- pub `write` function L38-61 — `(feed_dir: &Path, meta: &FeedMeta) -> Result<(), FeedError>` — Atomically write `meta.json` to `feed_dir`.
-  `META_FILENAME` variable L13 — `: &str` — filesystem.
-  `MetaStore` type L17-62 — `= MetaStore` — filesystem.
-  `tests` module L65-121 — `-` — filesystem.
-  `sample_meta` function L71-77 — `() -> FeedMeta` — filesystem.
-  `read_returns_none_when_missing` function L80-84 — `()` — filesystem.
-  `write_then_read_round_trips` function L87-94 — `()` — filesystem.
-  `write_creates_feed_dir_if_missing` function L97-103 — `()` — filesystem.
-  `atomic_write_does_not_corrupt_on_replace` function L106-120 — `()` — filesystem.

#### crates/arawn-feeds/src/registry.rs

- pub `FeedTemplateRegistry` struct L16-18 — `{ inner: HashMap<&'static str, Arc<dyn FeedTemplate>> }` — Maps template name (`<provider>/<name>`) → impl.
- pub `new` function L21-23 — `() -> Self` — name when firing.
- pub `register` function L25-27 — `(&mut self, template: Arc<dyn FeedTemplate>)` — name when firing.
- pub `get` function L29-31 — `(&self, name: &str) -> Option<Arc<dyn FeedTemplate>>` — name when firing.
- pub `require` function L35-39 — `(&self, name: &str) -> Result<Arc<dyn FeedTemplate>, FeedError>` — Look up or return a structured error so callers don't have to
- pub `names` function L41-43 — `(&self) -> impl Iterator<Item = &'static str> + '_` — name when firing.
-  `FeedTemplateRegistry` type L20-44 — `= FeedTemplateRegistry` — name when firing.
-  `tests` module L47-99 — `-` — name when firing.
-  `DummyTemplate` struct L54 — `-` — name when firing.
-  `DummyTemplate` type L57-79 — `impl FeedTemplate for DummyTemplate` — name when firing.
-  `name` function L58-60 — `(&self) -> &'static str` — name when firing.
-  `validate` function L61-63 — `(&self, _params: &TemplateParams) -> Result<(), FeedError>` — name when firing.
-  `defaults` function L64-69 — `(&self, _params: &TemplateParams) -> FeedDefaults` — name when firing.
-  `run` function L70-78 — `( &self, _ctx: &crate::template::TemplateCtx, _params: &TemplateParams, _feed_di...` — name when firing.
-  `register_and_lookup_round_trips` function L82-88 — `()` — name when firing.
-  `require_returns_invalid_params_for_unknown_name` function L91-98 — `()` — name when firing.

#### crates/arawn-feeds/src/runtime.rs

- pub `CloacinaRunner` type L34 — `= DefaultRunner` — arawn-feeds doesn't depend on arawn-workflow directly to avoid a
- pub `feed_workflow_name` function L43-45 — `(feed_id: &str) -> String` — Format the cloacina workflow name for a feed.
- pub `start` function L51-109 — `( runner: Arc<CloacinaRunner>, conn: Arc<Mutex<Connection>>, layout: Arc<DataLay...` — One-stop entry the server boot calls after the workflow runner is
- pub `FeedRuntime` struct L112-115 — `{ runner: Arc<CloacinaRunner>, runtime_ctx: FeedRuntimeContext }` — Live handle for dynamic feed registration (Phase 6: `/watch`).
- pub `register_feed_runtime` function L120-125 — `( &self, record: &FeedRecord, ) -> Result<(), FeedError>` — Register an additional feed without a server restart.
- pub `runtime_ctx` function L127-129 — `(&self) -> &FeedRuntimeContext` — audit are all inherited from cloacina.
- pub `register_feed_dynamic` function L143-223 — `( &self, template: &str, feed_id: &str, params: TemplateParams, cadence_override...` — Full dynamic-registration flow used by the `/watch` command.
- pub `run_feed_once` function L234-239 — `( &self, feed_id: &str, ) -> Result<crate::template::RunOutcome, FeedError>` — Trigger a one-off run of an enabled feed, outside the cron
- pub `pause_feed` function L247-264 — `(&self, feed_id: &str) -> Result<FeedRecord, FeedError>` — Pause a feed: drop its cloacina cron schedule and flip the row
- pub `resume_feed` function L269-287 — `(&self, feed_id: &str) -> Result<FeedRecord, FeedError>` — Resume a previously-paused feed: re-register the cloacina
- pub `remove_feed` function L296-325 — `( &self, feed_id: &str, ) -> Result<RemoveOutcome, FeedError>` — Decommission: drop the cloacina cron schedule, delete the DB
- pub `discover_template` function L333-340 — `( &self, template_name: &str, ) -> Result<Option<Vec<DiscoveryRow>>, FeedError>` — Run the template's discovery hook.
- pub `list_summaries` function L344-375 — `(&self) -> Result<Vec<FeedSummary>, FeedError>` — List every feed in the DB (enabled or paused) with on-disk
- pub `resume_pending_backfills` function L637-666 — `( runner: Arc<CloacinaRunner>, runtime_ctx: FeedRuntimeContext, records: &[FeedR...` — On boot, find feeds whose `meta.json.last_status == "backfilling"`
- pub `RemoveOutcome` struct L672-675 — `{ record: FeedRecord, bytes_wiped: u64 }` — Outcome of a successful `remove_feed` — the row that was deleted
-  `FeedRuntime` type L117-376 — `= FeedRuntime` — audit are all inherited from cloacina.
-  `BACKFILL_PAGE_CAP` variable L382 — `: u32` — Hard cap on backfill loop iterations.
-  `BASE_BACKOFF` variable L386 — `: std::time::Duration` — Base delay used when a provider rate-limits us without a Retry-After
-  `MAX_RATE_LIMIT_WAIT` variable L391 — `: std::time::Duration` — Wall-clock cap on cumulative rate-limit waits inside a single
-  `TRANSIENT_MAX_ATTEMPTS` variable L395 — `: u32` — How many consecutive transient errors (Provider/Storage) we'll
-  `transient_backoff` function L401-404 — `(attempt: u32) -> std::time::Duration` — Pure helper: backoff for the Nth consecutive transient retry
-  `BackfillExit` enum L409-415 — `Complete | RateLimitDeferred` — How a backfill ended.
-  `spawn_backfill_task` function L429-481 — `( runner: Arc<CloacinaRunner>, runtime_ctx: FeedRuntimeContext, feed_id: String,...` — Spawn the backfill loop as a detached tokio task.
-  `BackfillStats` struct L484-487 — `{ pages: u32, items: u64 }` — audit are all inherited from cloacina.
-  `run_backfill_loop` function L489-568 — `( _runner: &Arc<CloacinaRunner>, runtime_ctx: &FeedRuntimeContext, feed_id: &str...` — audit are all inherited from cloacina.
-  `finalize_backfill_success` function L570-609 — `( runner: &Arc<CloacinaRunner>, runtime_ctx: &FeedRuntimeContext, feed_id: &str,...` — audit are all inherited from cloacina.
-  `mark_backfill_failed` function L611-632 — `( runtime_ctx: &FeedRuntimeContext, feed_id: &str, err: &str, ) -> Result<(), Fe...` — audit are all inherited from cloacina.
-  `delete_schedule_for` function L679-699 — `( runner: &CloacinaRunner, workflow_name: &str, ) -> Result<(), FeedError>` — Look up cloacina's cron schedule by workflow name and delete it
-  `dir_size_bytes` function L701-721 — `(path: &std::path::Path) -> u64` — audit are all inherited from cloacina.
-  `walk` function L702-717 — `(p: &std::path::Path, acc: &mut u64)` — audit are all inherited from cloacina.
-  `register_one` function L723-809 — `( runner: &CloacinaRunner, ctx: &FeedRuntimeContext, record: &FeedRecord, ) -> R...` — audit are all inherited from cloacina.
-  `tests` module L812-830 — `-` — audit are all inherited from cloacina.
-  `transient_backoff_doubles_per_attempt` function L817-821 — `()` — audit are all inherited from cloacina.
-  `transient_backoff_clamps` function L824-829 — `()` — audit are all inherited from cloacina.

#### crates/arawn-feeds/src/store.rs

- pub `FeedRecord` struct L17-25 — `{ id: String, template: String, params: TemplateParams, cadence: String, enabled...` — One row from the `feeds` table.
- pub `FeedStore` struct L29-31 — `{ conn: &'a Connection }` — CRUD over the `feeds` table.
- pub `new` function L34-36 — `(conn: &'a Connection) -> Self` — source of truth for *what we've fetched* (cursor + last_run).
- pub `insert` function L38-57 — `(&self, rec: &FeedRecord) -> Result<(), FeedError>` — source of truth for *what we've fetched* (cursor + last_run).
- pub `get` function L59-71 — `(&self, id: &str) -> Result<Option<FeedRecord>, FeedError>` — source of truth for *what we've fetched* (cursor + last_run).
- pub `list_enabled` function L73-86 — `(&self) -> Result<Vec<FeedRecord>, FeedError>` — source of truth for *what we've fetched* (cursor + last_run).
- pub `list_all` function L88-101 — `(&self) -> Result<Vec<FeedRecord>, FeedError>` — source of truth for *what we've fetched* (cursor + last_run).
- pub `set_enabled` function L103-116 — `(&self, id: &str, enabled: bool) -> Result<(), FeedError>` — source of truth for *what we've fetched* (cursor + last_run).
- pub `delete` function L118-123 — `(&self, id: &str) -> Result<(), FeedError>` — source of truth for *what we've fetched* (cursor + last_run).
- pub `new_record` function L157-173 — `( id: impl Into<String>, template: impl Into<String>, params: TemplateParams, ca...` — Convenience builder for tests / `/watch` registration.
-  `row_to_record` function L126-154 — `(row: &rusqlite::Row) -> rusqlite::Result<Result<FeedRecord, FeedError>>` — source of truth for *what we've fetched* (cursor + last_run).
-  `_value_marker` function L179 — `(_: Value)` — source of truth for *what we've fetched* (cursor + last_run).
-  `tests` module L182-284 — `-` — source of truth for *what we've fetched* (cursor + last_run).
-  `open_test_db` function L186-203 — `() -> Connection` — source of truth for *what we've fetched* (cursor + last_run).
-  `insert_get_round_trip` function L206-221 — `()` — source of truth for *what we've fetched* (cursor + last_run).
-  `list_enabled_omits_disabled` function L224-251 — `()` — source of truth for *what we've fetched* (cursor + last_run).
-  `set_enabled_round_trips` function L254-265 — `()` — source of truth for *what we've fetched* (cursor + last_run).
-  `set_enabled_errors_for_unknown_id` function L268-273 — `()` — source of truth for *what we've fetched* (cursor + last_run).
-  `delete_removes_row` function L276-283 — `()` — source of truth for *what we've fetched* (cursor + last_run).

#### crates/arawn-feeds/src/template.rs

- pub `RunOutcome` struct L20-29 — `{ cursor: Value, summary: RunSummary, status: String }` — Result returned from a single feed run.
- pub `TemplateCtx` struct L36-38 — `{ clients: Arc<dyn FeedClients> }` — Per-run handle a template uses to reach providers and emit metadata.
- pub `new` function L41-43 — `(clients: Arc<dyn FeedClients>) -> Self` — use to reach providers and emit logs).
- pub `noop` function L48-52 — `() -> Self` — Test-only convenience: a ctx where every provider client returns
- pub `clients` function L54-56 — `(&self) -> &Arc<dyn FeedClients>` — use to reach providers and emit logs).
- pub `FeedTemplate` interface L65-117 — `{ fn name(), fn validate(), fn defaults(), fn run(), fn discover() }` — One named, parameterized fetch+write recipe owned by an integration.
- pub `DiscoveryRow` struct L127-132 — `{ label: String, hint: Option<String>, params: Value }` — One pickable choice surfaced by `FeedTemplate::discover`.
-  `TemplateCtx` type L40-57 — `= TemplateCtx` — use to reach providers and emit logs).
-  `discover` function L111-116 — `( &self, _ctx: &TemplateCtx, ) -> Result<Option<Vec<DiscoveryRow>>, FeedError>` — Optional discovery hook for the `/watch` picker.

#### crates/arawn-feeds/src/types.rs

- pub `TemplateParams` struct L12 — `-` — Template-specific parameters from the feed config row.
- pub `new` function L15-17 — `(v: Value) -> Self` — Shared types passed between the runtime and template impls.
- pub `as_value` function L19-21 — `(&self) -> &Value` — Shared types passed between the runtime and template impls.
- pub `get_str` function L24-26 — `(&'a self, key: &str) -> Option<&'a str>` — Convenience getter for a string field on the params object.
- pub `FeedDefaults` struct L33-40 — `{ cadence: String, initial_cursor: Value }` — Sensible default cadence + initial cursor a template suggests for a
- pub `RunSummary` struct L45-50 — `{ items_written: u64, bytes_written: u64, duration: Duration }` — Summary metrics from one fetch+write cycle, persisted to cloacina's
- pub `FeedMeta` struct L57-73 — `{ template: String, params: TemplateParams, cursor: Value, last_run_at: Option<S...` — What the runtime persists to `meta.json` at the feed dir root.
- pub `new` function L76-85 — `(template: impl Into<String>, params: TemplateParams, initial_cursor: Value) -> ...` — Shared types passed between the runtime and template impls.
- pub `FeedSummary` struct L95-113 — `{ id: String, template: String, cadence: String, enabled: bool, created_at: Stri...` — User-facing snapshot of one feed: the row state, last-run health
-  `TemplateParams` type L14-27 — `= TemplateParams` — Shared types passed between the runtime and template impls.
-  `FeedMeta` type L75-86 — `= FeedMeta` — Shared types passed between the runtime and template impls.

### crates/arawn-feeds/src/clients

> *Semantic summary to be generated by AI agent.*

#### crates/arawn-feeds/src/clients/atlassian.rs

- pub `ConfluencePageMeta` struct L31-42 — `{ id: String, title: String, space_key: String, version: Option<i64>, modified_t...` — Page metadata as feeds care about it.
- pub `ConfluencePageBody` struct L46-52 — `{ id: String, storage_xml: Option<String>, version: Option<i64> }` — Body of a Confluence page in storage format (raw XML).
- pub `JiraIssueMeta` struct L57-64 — `{ key: String, id: String, updated: Option<String>, summary: Option<String> }` — Lightweight Jira issue summary returned by [`AtlassianFeedClient::jql_search`].
- pub `JiraIssueDetail` struct L72-82 — `{ meta: JiraIssueMeta, fields: Value, comments: Option<Vec<Value>>, changelog: O...` — Full issue snapshot — meta + raw fields blob + optional changelog
- pub `AtlassianFeedClient` interface L89-140 — `{ fn space_pages_modified_since(), fn page_body_storage(), fn jql_search(), fn i...` — What feeds need from Atlassian.
- pub `JiraProjectMeta` struct L144-148 — `{ id: String, key: String, name: String }` — Project summary as the picker cares about it.
- pub `ConfluenceSpaceMeta` struct L152-155 — `{ key: String, name: String }` — Space summary as the picker cares about it.
- pub `RealAtlassianClient` struct L159-161 — `{ integration: Arc<AtlassianIntegration> }` — Confluence/Jira tools use.
- pub `new` function L164-166 — `(integration: Arc<AtlassianIntegration>) -> Self` — Confluence/Jira tools use.
-  `RealAtlassianClient` type L163-167 — `= RealAtlassianClient` — Confluence/Jira tools use.
-  `integ_err` function L169-179 — `(e: arawn_integrations::IntegrationError) -> FeedError` — Confluence/Jira tools use.
-  `classify_provider_error` function L184-201 — `(msg: &str) -> FeedError` — Provider errors arrive as opaque strings from the Atlassian client.
-  `V1SearchResp` struct L206-211 — `{ results: Vec<V1SearchResult>, links: serde_json::Map<String, serde_json::Value...` — Confluence/Jira tools use.
-  `V1SearchResult` struct L214-222 — `{ title: Option<String>, content: Option<V1Content>, last_modified: Option<Strin...` — Confluence/Jira tools use.
-  `V1Content` struct L225-229 — `{ id: String, space: Option<V1Space>, version: Option<V1Version> }` — Confluence/Jira tools use.
-  `V1Space` struct L232-234 — `{ key: Option<String> }` — Confluence/Jira tools use.
-  `V1Version` struct L237-240 — `{ number: Option<i64>, when: Option<String> }` — Confluence/Jira tools use.
-  `V2PageDetail` struct L245-249 — `{ id: String, body: Option<V2Body>, version: Option<V2Version> }` — Confluence/Jira tools use.
-  `V2Body` struct L252-254 — `{ storage: Option<V2BodyStorage> }` — Confluence/Jira tools use.
-  `V2BodyStorage` struct L257-259 — `{ value: Option<String> }` — Confluence/Jira tools use.
-  `V2Version` struct L262-264 — `{ number: Option<i64> }` — Confluence/Jira tools use.
-  `RealAtlassianClient` type L267-561 — `impl AtlassianFeedClient for RealAtlassianClient` — Confluence/Jira tools use.
-  `space_pages_modified_since` function L268-346 — `( &self, space_key: &str, since: Option<DateTime<Utc>>, ) -> Result<Vec<Confluen...` — Confluence/Jira tools use.
-  `page_body_storage` function L348-366 — `( &self, page_id: &str, ) -> Result<ConfluencePageBody, FeedError>` — Confluence/Jira tools use.
-  `jql_search` function L368-405 — `( &self, jql: &str, max_results: u32, ) -> Result<Vec<JiraIssueMeta>, FeedError>` — Confluence/Jira tools use.
-  `issue_full` function L407-502 — `( &self, key: &str, want_changelog: bool, want_comments: bool, ) -> Result<JiraI...` — Confluence/Jira tools use.
-  `resolve_project` function L504-520 — `(&self, key_or_id: &str) -> Result<String, FeedError>` — Confluence/Jira tools use.
-  `list_jira_projects` function L522-542 — `(&self) -> Result<Vec<JiraProjectMeta>, FeedError>` — Confluence/Jira tools use.
-  `list_confluence_spaces` function L544-560 — `( &self, ) -> Result<Vec<ConfluenceSpaceMeta>, FeedError>` — Confluence/Jira tools use.
-  `V2SpacesResp` struct L564-567 — `{ results: Vec<V2Space> }` — Confluence/Jira tools use.
-  `V2Space` struct L570-574 — `{ key: String, name: Option<String> }` — Confluence/Jira tools use.
-  `jira_err` function L576-588 — `(e: jira_v3_openapi::apis::Error<E>) -> FeedError` — Confluence/Jira tools use.

#### crates/arawn-feeds/src/clients/calendar.rs

- pub `CalendarFeedClient` interface L22-33 — `{ fn list_events() }` — What feeds need from Google Calendar.
- pub `RealCalendarClient` struct L37-39 — `{ integration: Arc<GoogleCalendarIntegration> }` — existing calendar tools use.
- pub `new` function L42-44 — `(integration: Arc<GoogleCalendarIntegration>) -> Self` — existing calendar tools use.
-  `RealCalendarClient` type L41-45 — `= RealCalendarClient` — existing calendar tools use.
-  `integ_err` function L47-54 — `(e: arawn_integrations::IntegrationError) -> FeedError` — existing calendar tools use.
-  `google_err` function L56-67 — `(op: &str, msg: String) -> FeedError` — existing calendar tools use.
-  `RealCalendarClient` type L70-97 — `impl CalendarFeedClient for RealCalendarClient` — existing calendar tools use.
-  `list_events` function L71-96 — `( &self, calendar_id: &str, time_min: DateTime<Utc>, time_max: DateTime<Utc>, ) ...` — existing calendar tools use.

#### crates/arawn-feeds/src/clients/drive.rs

- pub `DriveFile` struct L22-41 — `{ id: String, name: String, mime_type: String, modified_time: Option<String>, md...` — One file as feeds care about it.
- pub `folder_mime` function L46-48 — `() -> &'static str` — Drive tools use.
- pub `DriveFeedClient` interface L53-81 — `{ fn resolve_folder(), fn list_folder_children(), fn list_modified_since(), fn d...` — What feeds need from Drive.
- pub `export_for` function L87-95 — `(mime: &str) -> Option<(&'static str, &'static str)>` — Pick the export mime + filename suffix for Google native types.
- pub `is_unsupported_google_native` function L99-103 — `(mime: &str) -> bool` — True if `mime` is a Google native type with no export mapping
- pub `RealDriveClient` struct L112-114 — `{ integration: Arc<GoogleDriveIntegration> }` — Drive tools use.
- pub `new` function L117-119 — `(integration: Arc<GoogleDriveIntegration>) -> Self` — Drive tools use.
-  `MIME_FOLDER` variable L43 — `: &str` — Drive tools use.
-  `DriveFile` type L45-49 — `= DriveFile` — Drive tools use.
-  `FIELDS_LIST` variable L107-108 — `: &str` — Drive tools use.
-  `FIELDS_ONE` variable L109-110 — `: &str` — Drive tools use.
-  `RealDriveClient` type L116-120 — `= RealDriveClient` — Drive tools use.
-  `integ_err` function L122-129 — `(e: arawn_integrations::IntegrationError) -> FeedError` — Drive tools use.
-  `google_err` function L131-142 — `(op: &str, msg: String) -> FeedError` — Drive tools use.
-  `from_api` function L144-156 — `(f: google_drive3::api::File) -> DriveFile` — Drive tools use.
-  `RealDriveClient` type L159-310 — `impl DriveFeedClient for RealDriveClient` — Drive tools use.
-  `resolve_folder` function L160-198 — `(&self, path_or_id: &str) -> Result<String, FeedError>` — Drive tools use.
-  `list_folder_children` function L200-228 — `(&self, folder_id: &str) -> Result<Vec<DriveFile>, FeedError>` — Drive tools use.
-  `list_modified_since` function L230-271 — `( &self, since: DateTime<Utc>, max_results: u32, ) -> Result<Vec<DriveFile>, Fee...` — Drive tools use.
-  `DRIVE_MAX_PAGE_SIZE` variable L239 — `: u32` — Drive tools use.
-  `download` function L273-309 — `( &self, file_id: &str, export_mime: Option<&str>, ) -> Result<Vec<u8>, FeedErro...` — Drive tools use.
-  `try_id_lookup` function L317-335 — `( integration: &arawn_integrations::drive::GoogleDriveIntegration, id: &str, ) -...` — Try a Drive `files.get` against `path_or_id` as a literal id.
-  `walk_path` function L341-371 — `( integration: &arawn_integrations::drive::GoogleDriveIntegration, path: &str, )...` — Walk a slash-delimited folder path under My Drive root one
-  `is_not_found` function L377-380 — `(provider_msg: &str) -> bool` — Detect Drive's 404 error body in a `FeedError::Provider` message.
-  `tests` module L383-424 — `-` — Drive tools use.
-  `export_for_covers_known_natives` function L387-397 — `()` — Drive tools use.
-  `is_not_found_recognizes_drive_404_shapes` function L400-411 — `()` — Drive tools use.
-  `unsupported_native_excludes_folders_and_known_exports` function L414-423 — `()` — Drive tools use.

#### crates/arawn-feeds/src/clients/gmail.rs

- pub `GmailFeedClient` interface L24-37 — `{ fn list_message_ids(), fn get_message() }` — What feeds need from Gmail.
- pub `RealGmailClient` struct L41-43 — `{ integration: Arc<GmailIntegration> }` — provider-agnostic and makes mocking trivial.
- pub `new` function L46-48 — `(integration: Arc<GmailIntegration>) -> Self` — provider-agnostic and makes mocking trivial.
-  `RealGmailClient` type L45-49 — `= RealGmailClient` — provider-agnostic and makes mocking trivial.
-  `integ_err` function L51-58 — `(e: arawn_integrations::IntegrationError) -> FeedError` — provider-agnostic and makes mocking trivial.
-  `google_err` function L60-71 — `(op: &str, msg: String) -> FeedError` — provider-agnostic and makes mocking trivial.
-  `RealGmailClient` type L74-131 — `impl GmailFeedClient for RealGmailClient` — provider-agnostic and makes mocking trivial.
-  `list_message_ids` function L75-117 — `( &self, query: &str, max_results: u32, ) -> Result<Vec<String>, FeedError>` — provider-agnostic and makes mocking trivial.
-  `GMAIL_MAX_PAGE_SIZE` variable L86 — `: u32` — provider-agnostic and makes mocking trivial.
-  `get_message` function L119-130 — `(&self, id: &str) -> Result<Value, FeedError>` — provider-agnostic and makes mocking trivial.

#### crates/arawn-feeds/src/clients/mod.rs

- pub `atlassian` module L20 — `-` — `slack-morphism` directly — keeps templates mock-testable.
- pub `calendar` module L21 — `-` — `slack-morphism` directly — keeps templates mock-testable.
- pub `drive` module L22 — `-` — `slack-morphism` directly — keeps templates mock-testable.
- pub `gmail` module L23 — `-` — `slack-morphism` directly — keeps templates mock-testable.
- pub `slack` module L24 — `-` — `slack-morphism` directly — keeps templates mock-testable.
- pub `FeedClients` interface L41-47 — `{ fn slack(), fn calendar(), fn gmail(), fn drive(), fn atlassian() }` — Bundle of every provider client a template might want to use.
- pub `NoopClients` struct L52 — `-` — No-op `FeedClients`: every provider returns `None`.
- pub `RealClients` struct L76-82 — `{ slack: Option<Arc<dyn SlackFeedClient>>, calendar: Option<Arc<dyn CalendarFeed...` — Production bundle.
- pub `new` function L85-87 — `() -> Self` — `slack-morphism` directly — keeps templates mock-testable.
- pub `with_slack` function L89-95 — `( mut self, integration: Arc<arawn_integrations::slack::SlackIntegration>, ) -> ...` — `slack-morphism` directly — keeps templates mock-testable.
- pub `with_calendar` function L97-103 — `( mut self, integration: Arc<arawn_integrations::calendar::GoogleCalendarIntegra...` — `slack-morphism` directly — keeps templates mock-testable.
- pub `with_gmail` function L105-111 — `( mut self, integration: Arc<arawn_integrations::gmail::GmailIntegration>, ) -> ...` — `slack-morphism` directly — keeps templates mock-testable.
- pub `with_drive` function L113-119 — `( mut self, integration: Arc<arawn_integrations::drive::GoogleDriveIntegration>,...` — `slack-morphism` directly — keeps templates mock-testable.
- pub `with_atlassian` function L121-127 — `( mut self, integration: Arc<arawn_integrations::atlassian::AtlassianIntegration...` — `slack-morphism` directly — keeps templates mock-testable.
-  `NoopClients` type L54-70 — `impl FeedClients for NoopClients` — `slack-morphism` directly — keeps templates mock-testable.
-  `slack` function L55-57 — `(&self) -> Option<Arc<dyn SlackFeedClient>>` — `slack-morphism` directly — keeps templates mock-testable.
-  `calendar` function L58-60 — `(&self) -> Option<Arc<dyn CalendarFeedClient>>` — `slack-morphism` directly — keeps templates mock-testable.
-  `gmail` function L61-63 — `(&self) -> Option<Arc<dyn GmailFeedClient>>` — `slack-morphism` directly — keeps templates mock-testable.
-  `drive` function L64-66 — `(&self) -> Option<Arc<dyn DriveFeedClient>>` — `slack-morphism` directly — keeps templates mock-testable.
-  `atlassian` function L67-69 — `(&self) -> Option<Arc<dyn AtlassianFeedClient>>` — `slack-morphism` directly — keeps templates mock-testable.
-  `RealClients` type L84-128 — `= RealClients` — `slack-morphism` directly — keeps templates mock-testable.
-  `RealClients` type L130-146 — `impl FeedClients for RealClients` — `slack-morphism` directly — keeps templates mock-testable.
-  `slack` function L131-133 — `(&self) -> Option<Arc<dyn SlackFeedClient>>` — `slack-morphism` directly — keeps templates mock-testable.
-  `calendar` function L134-136 — `(&self) -> Option<Arc<dyn CalendarFeedClient>>` — `slack-morphism` directly — keeps templates mock-testable.
-  `gmail` function L137-139 — `(&self) -> Option<Arc<dyn GmailFeedClient>>` — `slack-morphism` directly — keeps templates mock-testable.
-  `drive` function L140-142 — `(&self) -> Option<Arc<dyn DriveFeedClient>>` — `slack-morphism` directly — keeps templates mock-testable.
-  `atlassian` function L143-145 — `(&self) -> Option<Arc<dyn AtlassianFeedClient>>` — `slack-morphism` directly — keeps templates mock-testable.

#### crates/arawn-feeds/src/clients/slack.rs

- pub `SlackFeedClient` interface L29-97 — `{ fn resolve_channel(), fn channel_history(), fn thread_replies(), fn open_dm(),...` — What feeds need from Slack.
- pub `SlackChannel` struct L101-107 — `{ id: String, name: String, is_private: bool, is_dm: bool }` — Channel summary as the picker cares about it.
- pub `SlackAuthInfo` struct L111-114 — `{ user_id: String, team_id: String }` — Subset of Slack `auth.test` response that feeds care about.
- pub `SlackHistoryPage` struct L120-129 — `{ messages: Vec<serde_json::Value>, next_cursor_ts: Option<String> }` — One page of Slack channel history.
- pub `RealSlackClient` struct L133-135 — `{ integration: Arc<SlackIntegration> }` — Slack tools use.
- pub `new` function L138-140 — `(integration: Arc<SlackIntegration>) -> Self` — Slack tools use.
- pub `ChannelKind` enum L554-567 — `Public | Private | DirectMessage | GroupDm` — Slack conversation kind, classified by id prefix.
- pub `history_scope` function L573-580 — `(self) -> &'static str` — Required Slack OAuth scope to call `conversations.history` on
- pub `recommended_template` function L583-590 — `(self) -> &'static str` — Recommended template to archive this kind.
- pub `classify_channel_id` function L595-607 — `(s: &str) -> Option<ChannelKind>` — Classify a Slack id by its prefix.
-  `RealSlackClient` type L137-141 — `= RealSlackClient` — Slack tools use.
-  `integ_err` function L143-149 — `(e: arawn_integrations::IntegrationError) -> FeedError` — Slack tools use.
-  `slack_morphism_err` function L151-170 — `(op: &str, e: E) -> FeedError` — Slack tools use.
-  `find_slack_retry_after` function L174-190 — `( e: &(dyn std::error::Error + 'static), ) -> Option<Option<std::time::Duration>...` — Walk the source chain of a slack-morphism error looking for a typed
-  `RealSlackClient` type L193-484 — `impl SlackFeedClient for RealSlackClient` — Slack tools use.
-  `resolve_channel` function L194-230 — `(&self, name_or_id: &str) -> Result<String, FeedError>` — Slack tools use.
-  `channel_history` function L232-275 — `( &self, channel_id: &str, oldest_ts: Option<&str>, ) -> Result<SlackHistoryPage...` — Slack tools use.
-  `thread_replies` function L277-322 — `( &self, channel_id: &str, parent_ts: &str, oldest_ts: Option<&str>, ) -> Result...` — Slack tools use.
-  `open_dm` function L324-346 — `(&self, user_id_or_name: &str) -> Result<String, FeedError>` — Slack tools use.
-  `auth_test` function L348-365 — `(&self) -> Result<SlackAuthInfo, FeedError>` — Slack tools use.
-  `search_messages` function L367-445 — `( &self, query: &str, oldest_ts: Option<&str>, ) -> Result<SlackHistoryPage, Fee...` — Slack tools use.
-  `list_channels` function L447-483 — `(&self) -> Result<Vec<SlackChannel>, FeedError>` — Slack tools use.
-  `ts_to_yyyy_mm_dd` function L489-494 — `(ts: &str) -> Option<String>` — Lossy conversion from Slack's float-string `ts` to a `YYYY-MM-DD`
-  `RealSlackClient` type L496-526 — `= RealSlackClient` — Slack tools use.
-  `resolve_user_name_to_id` function L497-525 — `(&self, name: &str) -> Result<String, FeedError>` — Slack tools use.
-  `looks_like_user_id` function L528-533 — `(s: &str) -> bool` — Slack tools use.
-  `looks_like_channel_id` function L535-537 — `(s: &str) -> bool` — Slack tools use.
-  `ChannelKind` type L569-591 — `= ChannelKind` — Slack tools use.
-  `tests` module L610-678 — `-` — Slack tools use.
-  `channel_id_recognized_by_prefix` function L614-619 — `()` — Slack tools use.
-  `names_not_recognized_as_ids` function L622-627 — `()` — Slack tools use.
-  `classify_returns_kind_for_each_prefix` function L630-638 — `()` — Slack tools use.
-  `channel_kind_exposes_required_scope` function L641-646 — `()` — Slack tools use.
-  `channel_kind_recommends_correct_template` function L649-667 — `()` — Slack tools use.
-  `user_id_recognized_by_prefix` function L670-677 — `()` — Slack tools use.

### crates/arawn-feeds/src/templates/calendar

> *Semantic summary to be generated by AI agent.*

#### crates/arawn-feeds/src/templates/calendar/mod.rs

- pub `upcoming_archive` module L3 — `-` — Calendar feed templates.

#### crates/arawn-feeds/src/templates/calendar/upcoming_archive.rs

- pub `UpcomingArchiveTemplate` struct L50 — `-` — - `window_days` (optional, default `7`)
-  `NAME` variable L52 — `: &str` — - `window_days` (optional, default `7`)
-  `DEFAULT_CALENDAR_ID` variable L53 — `: &str` — - `window_days` (optional, default `7`)
-  `DEFAULT_WINDOW_DAYS` variable L54 — `: i64` — - `window_days` (optional, default `7`)
-  `UpcomingArchiveTemplate` type L57-163 — `impl FeedTemplate for UpcomingArchiveTemplate` — - `window_days` (optional, default `7`)
-  `name` function L58-60 — `(&self) -> &'static str` — - `window_days` (optional, default `7`)
-  `validate` function L62-83 — `(&self, params: &TemplateParams) -> Result<(), FeedError>` — - `window_days` (optional, default `7`)
-  `defaults` function L85-93 — `(&self, _params: &TemplateParams) -> FeedDefaults` — - `window_days` (optional, default `7`)
-  `run` function L95-162 — `( &self, ctx: &TemplateCtx, params: &TemplateParams, feed_dir: &Path, _cursor: &...` — - `window_days` (optional, default `7`)
-  `sanitize_event_id` function L165-172 — `(id: &str) -> String` — - `window_days` (optional, default `7`)
-  `write_event_file` function L174-186 — `(path: &Path, event: &Value) -> Result<u64, FeedError>` — - `window_days` (optional, default `7`)
-  `tests` module L189-224 — `-` — - `window_days` (optional, default `7`)
-  `validate_accepts_default_params` function L193-197 — `()` — - `window_days` (optional, default `7`)
-  `validate_rejects_bad_window_days` function L200-207 — `()` — - `window_days` (optional, default `7`)
-  `defaults_use_30min_cadence` function L210-213 — `()` — - `window_days` (optional, default `7`)
-  `sanitize_keeps_safe_chars` function L216-223 — `()` — - `window_days` (optional, default `7`)

### crates/arawn-feeds/src/templates/confluence

> *Semantic summary to be generated by AI agent.*

#### crates/arawn-feeds/src/templates/confluence/mod.rs

- pub `space_archive` module L3 — `-` — Confluence feed templates.

#### crates/arawn-feeds/src/templates/confluence/space_archive.rs

- pub `SpaceArchiveTemplate` struct L51 — `-` — - Attachments.
-  `NAME` variable L53 — `: &str` — - Attachments.
-  `SpaceArchiveTemplate` type L56-198 — `impl FeedTemplate for SpaceArchiveTemplate` — - Attachments.
-  `name` function L57-59 — `(&self) -> &'static str` — - Attachments.
-  `validate` function L61-75 — `(&self, params: &TemplateParams) -> Result<(), FeedError>` — - Attachments.
-  `defaults` function L77-82 — `(&self, _params: &TemplateParams) -> FeedDefaults` — - Attachments.
-  `run` function L84-172 — `( &self, ctx: &TemplateCtx, params: &TemplateParams, feed_dir: &Path, cursor: &V...` — - Attachments.
-  `discover` function L174-197 — `( &self, ctx: &TemplateCtx, ) -> Result<Option<Vec<DiscoveryRow>>, FeedError>` — - Attachments.
-  `write_meta` function L200-211 — `(page_dir: &Path, page: &ConfluencePageMeta) -> Result<u64, FeedError>` — - Attachments.
-  `write_body` function L213-223 — `(page_dir: &Path, storage_xml: Option<&str>) -> Result<u64, FeedError>` — - Attachments.
-  `tests` module L226-245 — `-` — - Attachments.
-  `validate_requires_space_key` function L230-238 — `()` — - Attachments.
-  `defaults_use_30min_cadence` function L241-244 — `()` — - Attachments.

### crates/arawn-feeds/src/templates/drive

> *Semantic summary to be generated by AI agent.*

#### crates/arawn-feeds/src/templates/drive/common.rs

- pub `sanitize_path_component` function L12-25 — `(name: &str) -> String` — Sanitize one path component from a Drive file or folder name into
- pub `is_under` function L32-54 — `(root: &std::path::Path, candidate: &std::path::Path) -> bool` — Confirm `candidate` lives strictly under `root`.
- pub `change_token` function L60-66 — `(md5: Option<&str>, modified_time: Option<&str>) -> String` — Map an `md5_checksum` (binary) or `modified_time` (Google natives)
- pub `modified_to_yyyy_mm_dd` function L70-80 — `(modified_time: Option<&str>) -> Result<String, FeedError>` — Read a `modifiedTime` ISO string into an `i64` ms-since-epoch for
-  `tests` module L83-114 — `-` — Shared helpers for Drive feed templates.
-  `sanitize_strips_separators_and_traversal` function L87-95 — `()` — Shared helpers for Drive feed templates.
-  `change_token_prefers_md5` function L98-103 — `()` — Shared helpers for Drive feed templates.
-  `modified_to_day_basic` function L106-113 — `()` — Shared helpers for Drive feed templates.

#### crates/arawn-feeds/src/templates/drive/folder_sync.rs

- pub `FolderSyncTemplate` struct L60 — `-` — the API ever surprises us.
-  `NAME` variable L62 — `: &str` — the API ever surprises us.
-  `MAX_DEPTH` variable L65 — `: usize` — Cap recursion to keep a misbehaving folder graph from spinning
-  `Cursor` struct L68-73 — `{ files: BTreeMap<String, FileEntry> }` — the API ever surprises us.
-  `FileEntry` struct L76-82 — `{ token: String, path: String }` — the API ever surprises us.
-  `FolderSyncTemplate` type L85-273 — `impl FeedTemplate for FolderSyncTemplate` — the API ever surprises us.
-  `name` function L86-88 — `(&self) -> &'static str` — the API ever surprises us.
-  `validate` function L90-100 — `(&self, params: &TemplateParams) -> Result<(), FeedError>` — the API ever surprises us.
-  `defaults` function L102-107 — `(&self, _params: &TemplateParams) -> FeedDefaults` — the API ever surprises us.
-  `run` function L109-272 — `( &self, ctx: &TemplateCtx, params: &TemplateParams, feed_dir: &Path, cursor: &V...` — the API ever surprises us.
-  `RemoteFile` struct L276-280 — `{ file: DriveFile, relative_path: String }` — the API ever surprises us.
-  `walk` function L284-336 — `( drive: Arc<dyn DriveFeedClient>, folder_id: &'a str, rel_prefix: PathBuf, dept...` — Recursively walk a Drive folder, collecting every file (not
-  `atomic_write` function L338-345 — `(path: &Path, body: &[u8]) -> Result<(), FeedError>` — the API ever surprises us.
-  `safe_remove_file` function L347-360 — `(feed_dir: &Path, path: &Path) -> Result<(), FeedError>` — the API ever surprises us.
-  `prune_empty_dirs` function L362-379 — `(root: &Path)` — the API ever surprises us.
-  `tests` module L382-401 — `-` — the API ever surprises us.
-  `validate_requires_folder` function L386-394 — `()` — the API ever surprises us.
-  `defaults_use_hourly_cadence` function L397-400 — `()` — the API ever surprises us.

#### crates/arawn-feeds/src/templates/drive/mod.rs

- pub `common` module L3 — `-` — Drive feed templates.
- pub `folder_sync` module L4 — `-` — Drive feed templates.
- pub `recent` module L5 — `-` — Drive feed templates.

#### crates/arawn-feeds/src/templates/drive/recent.rs

- pub `RecentTemplate` struct L43 — `-` — the first run, when the cursor is null.
-  `NAME` variable L45 — `: &str` — the first run, when the cursor is null.
-  `DEFAULT_DAYS_BACK` variable L46 — `: i64` — the first run, when the cursor is null.
-  `MAX_RESULTS_PER_RUN` variable L47 — `: u32` — the first run, when the cursor is null.
-  `BACKFILL_MAX_RESULTS` variable L51 — `: u32` — Cap used when in backfill mode (cursor null + `since` present).
-  `RecentTemplate` type L54-189 — `impl FeedTemplate for RecentTemplate` — the first run, when the cursor is null.
-  `name` function L55-57 — `(&self) -> &'static str` — the first run, when the cursor is null.
-  `validate` function L59-71 — `(&self, params: &TemplateParams) -> Result<(), FeedError>` — the first run, when the cursor is null.
-  `defaults` function L73-78 — `(&self, _params: &TemplateParams) -> FeedDefaults` — the first run, when the cursor is null.
-  `run` function L80-188 — `( &self, ctx: &TemplateCtx, params: &TemplateParams, feed_dir: &Path, cursor: &V...` — the first run, when the cursor is null.
-  `write_file_metadata` function L191-201 — `(path: &Path, file: &DriveFile) -> Result<u64, FeedError>` — the first run, when the cursor is null.
-  `tests` module L204-225 — `-` — the first run, when the cursor is null.
-  `validate_default_params` function L208-210 — `()` — the first run, when the cursor is null.
-  `validate_rejects_bad_days_back` function L213-218 — `()` — the first run, when the cursor is null.
-  `defaults_use_30min_cadence` function L221-224 — `()` — the first run, when the cursor is null.

### crates/arawn-feeds/src/templates/gmail

> *Semantic summary to be generated by AI agent.*

#### crates/arawn-feeds/src/templates/gmail/common.rs

- pub `DEFAULT_MAX_RESULTS` variable L49 — `: u32` — Steady-state per-call cap.
- pub `BACKFILL_MAX_RESULTS` variable L55 — `: u32` — Cap used by the backfill spawn loop (T-0234).
- pub `compose_time_bound` function L66-83 — `( cursor: &Value, params_since: Option<&str>, days_back: u64, ) -> (String, u32)` — Compose the time-bound clause + per-call cap for one Gmail run.
- pub `archive_query` function L92-185 — `( gmail: Arc<dyn GmailFeedClient>, feed_dir: &Path, query: &str, cursor: &Value,...` — Run a Gmail archive over `query`, writing every new message under
-  `existing_message_path` function L192-205 — `(feed_dir: &Path, id: &str) -> Option<std::path::PathBuf>` — Probe every day partition under `feed_dir` for an existing
-  `parse_internal_date` function L207-215 — `(msg: &Value) -> Option<i64>` — list ordering, so it's the right key.
-  `ms_to_yyyy_mm_dd` function L217-225 — `(ms: i64) -> Result<String, FeedError>` — list ordering, so it's the right key.
-  `write_message_file` function L227-240 — `(path: &Path, msg: &Value) -> Result<u64, FeedError>` — list ordering, so it's the right key.
-  `tests` module L243-300 — `-` — list ordering, so it's the right key.
-  `ms_to_yyyy_mm_dd_basic` function L247-253 — `()` — list ordering, so it's the right key.
-  `compose_time_bound_steady_state_uses_newer_than` function L256-262 — `()` — list ordering, so it's the right key.
-  `compose_time_bound_first_run_with_since_uses_after` function L265-272 — `()` — list ordering, so it's the right key.
-  `compose_time_bound_first_run_without_since_falls_back_to_days_back` function L275-280 — `()` — list ordering, so it's the right key.
-  `compose_time_bound_garbage_since_falls_back` function L283-289 — `()` — list ordering, so it's the right key.
-  `parse_internal_date_string_or_number` function L292-299 — `()` — list ordering, so it's the right key.

#### crates/arawn-feeds/src/templates/gmail/inbox_archive.rs

- pub `InboxArchiveTemplate` struct L25 — `-` — pause.
-  `NAME` variable L27 — `: &str` — pause.
-  `DEFAULT_DAYS_BACK` variable L28 — `: u32` — pause.
-  `InboxArchiveTemplate` type L31-80 — `impl FeedTemplate for InboxArchiveTemplate` — pause.
-  `name` function L32-34 — `(&self) -> &'static str` — pause.
-  `validate` function L36-48 — `(&self, params: &TemplateParams) -> Result<(), FeedError>` — pause.
-  `defaults` function L50-55 — `(&self, _params: &TemplateParams) -> FeedDefaults` — pause.
-  `run` function L57-79 — `( &self, ctx: &TemplateCtx, params: &TemplateParams, feed_dir: &Path, cursor: &V...` — pause.
-  `tests` module L83-108 — `-` — pause.
-  `validate_default_params` function L87-91 — `()` — pause.
-  `validate_rejects_bad_days_back` function L94-101 — `()` — pause.
-  `defaults_use_15min_cadence` function L104-107 — `()` — pause.

#### crates/arawn-feeds/src/templates/gmail/label_archive.rs

- pub `LabelArchiveTemplate` struct L33 — `-` — the feed run as a no-op than to bind validity at registration time.
-  `NAME` variable L35 — `: &str` — the feed run as a no-op than to bind validity at registration time.
-  `DEFAULT_DAYS_BACK` variable L36 — `: u32` — the feed run as a no-op than to bind validity at registration time.
-  `LabelArchiveTemplate` type L39-101 — `impl FeedTemplate for LabelArchiveTemplate` — the feed run as a no-op than to bind validity at registration time.
-  `name` function L40-42 — `(&self) -> &'static str` — the feed run as a no-op than to bind validity at registration time.
-  `validate` function L44-64 — `(&self, params: &TemplateParams) -> Result<(), FeedError>` — the feed run as a no-op than to bind validity at registration time.
-  `defaults` function L66-71 — `(&self, _params: &TemplateParams) -> FeedDefaults` — the feed run as a no-op than to bind validity at registration time.
-  `run` function L73-100 — `( &self, ctx: &TemplateCtx, params: &TemplateParams, feed_dir: &Path, cursor: &V...` — the feed run as a no-op than to bind validity at registration time.
-  `tests` module L104-117 — `-` — the feed run as a no-op than to bind validity at registration time.
-  `validate_requires_label` function L108-116 — `()` — the feed run as a no-op than to bind validity at registration time.

#### crates/arawn-feeds/src/templates/gmail/mod.rs

- pub `common` module L3 — `-` — Gmail feed templates.
- pub `inbox_archive` module L4 — `-` — Gmail feed templates.
- pub `label_archive` module L5 — `-` — Gmail feed templates.
- pub `sender_filter` module L6 — `-` — Gmail feed templates.

#### crates/arawn-feeds/src/templates/gmail/sender_filter.rs

- pub `SenderFilterTemplate` struct L28 — `-` — [`super::common`].
-  `NAME` variable L30 — `: &str` — [`super::common`].
-  `DEFAULT_DAYS_BACK` variable L31 — `: u32` — [`super::common`].
-  `SenderFilterTemplate` type L34-102 — `impl FeedTemplate for SenderFilterTemplate` — [`super::common`].
-  `name` function L35-37 — `(&self) -> &'static str` — [`super::common`].
-  `validate` function L39-63 — `(&self, params: &TemplateParams) -> Result<(), FeedError>` — [`super::common`].
-  `defaults` function L65-70 — `(&self, _params: &TemplateParams) -> FeedDefaults` — [`super::common`].
-  `run` function L72-101 — `( &self, ctx: &TemplateCtx, params: &TemplateParams, feed_dir: &Path, cursor: &V...` — [`super::common`].
-  `tests` module L105-127 — `-` — [`super::common`].
-  `validate_requires_sender_pattern` function L109-117 — `()` — [`super::common`].
-  `validate_rejects_bad_days_back` function L120-126 — `()` — [`super::common`].

### crates/arawn-feeds/src/templates/jira

> *Semantic summary to be generated by AI agent.*

#### crates/arawn-feeds/src/templates/jira/assignee_tracker.rs

- pub `AssigneeTrackerTemplate` struct L24 — `-` — are no append-only logs to advance independently of the snapshot.
-  `NAME` variable L26 — `: &str` — are no append-only logs to advance independently of the snapshot.
-  `MAX_RESULTS_PER_RUN` variable L27 — `: u32` — are no append-only logs to advance independently of the snapshot.
-  `AssigneeTrackerTemplate` type L30-120 — `impl FeedTemplate for AssigneeTrackerTemplate` — are no append-only logs to advance independently of the snapshot.
-  `name` function L31-33 — `(&self) -> &'static str` — are no append-only logs to advance independently of the snapshot.
-  `validate` function L35-37 — `(&self, _params: &TemplateParams) -> Result<(), FeedError>` — are no append-only logs to advance independently of the snapshot.
-  `defaults` function L39-47 — `(&self, _params: &TemplateParams) -> FeedDefaults` — are no append-only logs to advance independently of the snapshot.
-  `run` function L49-119 — `( &self, ctx: &TemplateCtx, params: &TemplateParams, feed_dir: &Path, cursor: &V...` — are no append-only logs to advance independently of the snapshot.
-  `build_jql` function L122-129 — `(since: Option<&str>) -> String` — are no append-only logs to advance independently of the snapshot.
-  `tests` module L132-153 — `-` — are no append-only logs to advance independently of the snapshot.
-  `validate_takes_no_params` function L136-140 — `()` — are no append-only logs to advance independently of the snapshot.
-  `jql_uses_currentUser` function L143-152 — `()` — are no append-only logs to advance independently of the snapshot.

#### crates/arawn-feeds/src/templates/jira/common.rs

- pub `PerIssueCursor` struct L50-58 — `{ last_comment_id: Option<String>, last_history_id: Option<String> }` — Per-issue cursor state.
- pub `CursorState` struct L61-69 — `{ latest_updated_iso: Option<String>, issues: BTreeMap<String, PerIssueCursor> }` — `assignee-tracker` feed only carries `latest_updated_iso`.
- pub `from_value` function L72-74 — `(v: &Value) -> Self` — `assignee-tracker` feed only carries `latest_updated_iso`.
- pub `into_value` function L75-77 — `(self) -> Value` — `assignee-tracker` feed only carries `latest_updated_iso`.
- pub `write_json_atomic` function L81-88 — `(path: &Path, body: &[u8]) -> Result<(), FeedError>` — Atomic-rename write of a JSON snapshot to `path`.
- pub `append_jsonl` function L92-112 — `(path: &Path, line: &Value) -> Result<u64, FeedError>` — Append a single JSON-serializable item as one line to `path`.
- pub `IssueWriteOutcome` struct L115-120 — `{ bytes_written: u64, cursor: PerIssueCursor }` — Result of writing one issue's snapshot + (optional) logs.
- pub `write_issue_snapshot` function L123-143 — `( issue_dir: &Path, detail: &JiraIssueDetail, ) -> Result<u64, FeedError>` — Write `<issue_dir>/issue.json` (overwrite).
- pub `append_logs` function L151-202 — `( issue_dir: &Path, detail: &JiraIssueDetail, prior: PerIssueCursor, ) -> Result...` — Write any new comments + changelog entries to per-issue jsonl
-  `CursorState` type L71-78 — `= CursorState` — `assignee-tracker` feed only carries `latest_updated_iso`.
-  `parse_id` function L204-206 — `(s: Option<&str>) -> Option<u64>` — `assignee-tracker` feed only carries `latest_updated_iso`.
-  `tests` module L209-243 — `-` — `assignee-tracker` feed only carries `latest_updated_iso`.
-  `cursor_round_trips_through_value` function L213-235 — `()` — `assignee-tracker` feed only carries `latest_updated_iso`.
-  `parse_id_handles_missing_and_numeric` function L238-242 — `()` — `assignee-tracker` feed only carries `latest_updated_iso`.

#### crates/arawn-feeds/src/templates/jira/mod.rs

- pub `assignee_tracker` module L3 — `-` — Jira feed templates.
- pub `common` module L4 — `-` — Jira feed templates.
- pub `project_tracker` module L5 — `-` — Jira feed templates.

#### crates/arawn-feeds/src/templates/jira/project_tracker.rs

- pub `ProjectTrackerTemplate` struct L27 — `-` — plus a per-issue `{ last_comment_id, last_history_id }` map.
-  `NAME` variable L29 — `: &str` — plus a per-issue `{ last_comment_id, last_history_id }` map.
-  `MAX_RESULTS_PER_RUN` variable L30 — `: u32` — plus a per-issue `{ last_comment_id, last_history_id }` map.
-  `ProjectTrackerTemplate` type L33-177 — `impl FeedTemplate for ProjectTrackerTemplate` — plus a per-issue `{ last_comment_id, last_history_id }` map.
-  `name` function L34-36 — `(&self) -> &'static str` — plus a per-issue `{ last_comment_id, last_history_id }` map.
-  `validate` function L38-52 — `(&self, params: &TemplateParams) -> Result<(), FeedError>` — plus a per-issue `{ last_comment_id, last_history_id }` map.
-  `defaults` function L54-62 — `(&self, _params: &TemplateParams) -> FeedDefaults` — plus a per-issue `{ last_comment_id, last_history_id }` map.
-  `run` function L64-155 — `( &self, ctx: &TemplateCtx, params: &TemplateParams, feed_dir: &Path, cursor: &V...` — plus a per-issue `{ last_comment_id, last_history_id }` map.
-  `discover` function L157-176 — `( &self, ctx: &TemplateCtx, ) -> Result<Option<Vec<DiscoveryRow>>, FeedError>` — plus a per-issue `{ last_comment_id, last_history_id }` map.
-  `effective_since` function L189-199 — `(cursor_iso: Option<&str>, params_since: Option<&str>) -> Option<String>` — Resolve the JQL time-floor for this run.
-  `build_jql` function L201-211 — `(project: &str, since: Option<&str>) -> String` — plus a per-issue `{ last_comment_id, last_history_id }` map.
-  `tests` module L214-264 — `-` — plus a per-issue `{ last_comment_id, last_history_id }` map.
-  `validate_requires_project` function L218-226 — `()` — plus a per-issue `{ last_comment_id, last_history_id }` map.
-  `effective_since_prefers_cursor_then_falls_back_to_params` function L229-251 — `()` — plus a per-issue `{ last_comment_id, last_history_id }` map.
-  `jql_includes_since_when_present` function L254-263 — `()` — plus a per-issue `{ last_comment_id, last_history_id }` map.

### crates/arawn-feeds/src/templates

> *Semantic summary to be generated by AI agent.*

#### crates/arawn-feeds/src/templates/mod.rs

- pub `calendar` module L3 — `-` — Concrete `FeedTemplate` impls organized per provider.
- pub `confluence` module L4 — `-` — Concrete `FeedTemplate` impls organized per provider.
- pub `drive` module L5 — `-` — Concrete `FeedTemplate` impls organized per provider.
- pub `gmail` module L6 — `-` — Concrete `FeedTemplate` impls organized per provider.
- pub `jira` module L7 — `-` — Concrete `FeedTemplate` impls organized per provider.
- pub `slack` module L8 — `-` — Concrete `FeedTemplate` impls organized per provider.
- pub `stub` module L9 — `-` — Concrete `FeedTemplate` impls organized per provider.
- pub `default_registry` function L18-34 — `() -> FeedTemplateRegistry` — Build the registry of every template the binary supports.

#### crates/arawn-feeds/src/templates/stub.rs

- pub `EchoTemplate` struct L21 — `-` — integration without involving any real provider client.
-  `NAME` variable L23 — `: &str` — integration without involving any real provider client.
-  `EchoTemplate` type L26-87 — `impl FeedTemplate for EchoTemplate` — integration without involving any real provider client.
-  `name` function L27-29 — `(&self) -> &'static str` — integration without involving any real provider client.
-  `validate` function L31-35 — `(&self, _params: &TemplateParams) -> Result<(), FeedError>` — integration without involving any real provider client.
-  `defaults` function L37-42 — `(&self, _params: &TemplateParams) -> FeedDefaults` — integration without involving any real provider client.
-  `run` function L44-86 — `( &self, _ctx: &TemplateCtx, params: &TemplateParams, feed_dir: &Path, cursor: &...` — integration without involving any real provider client.

### crates/arawn-feeds/src/templates/slack

> *Semantic summary to be generated by AI agent.*

#### crates/arawn-feeds/src/templates/slack/channel_archive.rs

- pub `ChannelArchiveTemplate` struct L43 — `-` — on one thread doesn't drop the channel cursor or block other threads.
-  `NAME` variable L45 — `: &str` — on one thread doesn't drop the channel cursor or block other threads.
-  `ChannelArchiveTemplate` type L48-143 — `impl FeedTemplate for ChannelArchiveTemplate` — on one thread doesn't drop the channel cursor or block other threads.
-  `name` function L49-51 — `(&self) -> &'static str` — on one thread doesn't drop the channel cursor or block other threads.
-  `validate` function L53-66 — `(&self, params: &TemplateParams) -> Result<(), FeedError>` — on one thread doesn't drop the channel cursor or block other threads.
-  `defaults` function L68-73 — `(&self, _params: &TemplateParams) -> FeedDefaults` — on one thread doesn't drop the channel cursor or block other threads.
-  `run` function L75-106 — `( &self, ctx: &TemplateCtx, params: &TemplateParams, feed_dir: &Path, cursor: &V...` — on one thread doesn't drop the channel cursor or block other threads.
-  `discover` function L108-142 — `( &self, ctx: &TemplateCtx, ) -> Result<Option<Vec<DiscoveryRow>>, FeedError>` — on one thread doesn't drop the channel cursor or block other threads.
-  `tests` module L146-174 — `-` — on one thread doesn't drop the channel cursor or block other threads.
-  `validate_rejects_missing_channel` function L151-155 — `()` — on one thread doesn't drop the channel cursor or block other threads.
-  `validate_rejects_empty_channel` function L158-164 — `()` — on one thread doesn't drop the channel cursor or block other threads.
-  `validate_accepts_named_or_id_channel` function L167-173 — `()` — on one thread doesn't drop the channel cursor or block other threads.

#### crates/arawn-feeds/src/templates/slack/common.rs

- pub `archive_channel_with_threads` function L34-183 — `( slack: &dyn SlackFeedClient, channel_id: &str, feed_dir: &Path, cursor: &Value...` — Two-pass dual-layer archive of a single Slack conversation.
- pub `synth_since_cursor` function L194-218 — `( cursor: &Value, params: &crate::types::TemplateParams, ) -> Result<Value, Feed...` — First-run `since=` seeding for slack archive templates.
-  `append_message_to_day` function L222-226 — `(feed_dir: &Path, msg: &Value, ts: &str) -> Result<u64, FeedError>` — per-thread reply fetch + thread-file writes, cursor management.
-  `append_message_to_thread` function L228-238 — `( feed_dir: &Path, parent_ts: &str, msg: &Value, ) -> Result<u64, FeedError>` — per-thread reply fetch + thread-file writes, cursor management.
-  `append_line` function L240-253 — `(path: &Path, msg: &Value) -> Result<u64, FeedError>` — per-thread reply fetch + thread-file writes, cursor management.
-  `has_replies` function L255-260 — `(msg: &Value) -> bool` — per-thread reply fetch + thread-file writes, cursor management.
-  `ts_to_yyyy_mm_dd` function L264-274 — `(ts: &str) -> Result<String, FeedError>` — Parse Slack's float-string `ts` (`"1715000000.001234"`) and format
-  `tests` module L277-302 — `-` — per-thread reply fetch + thread-file writes, cursor management.
-  `ts_to_yyyy_mm_dd_parses_slack_format` function L282-286 — `()` — per-thread reply fetch + thread-file writes, cursor management.
-  `ts_to_yyyy_mm_dd_rejects_garbage` function L289-294 — `()` — per-thread reply fetch + thread-file writes, cursor management.
-  `has_replies_detects_reply_count` function L297-301 — `()` — per-thread reply fetch + thread-file writes, cursor management.

#### crates/arawn-feeds/src/templates/slack/dm_archive.rs

- pub `DmArchiveTemplate` struct L30 — `-` — ```
-  `NAME` variable L32 — `: &str` — ```
-  `DmArchiveTemplate` type L35-90 — `impl FeedTemplate for DmArchiveTemplate` — ```
-  `name` function L36-38 — `(&self) -> &'static str` — ```
-  `validate` function L40-53 — `(&self, params: &TemplateParams) -> Result<(), FeedError>` — ```
-  `defaults` function L55-63 — `(&self, _params: &TemplateParams) -> FeedDefaults` — ```
-  `run` function L65-89 — `( &self, ctx: &TemplateCtx, params: &TemplateParams, feed_dir: &Path, cursor: &V...` — ```
-  `tests` module L93-123 — `-` — ```
-  `validate_rejects_missing_user` function L98-102 — `()` — ```
-  `validate_rejects_empty_user` function L105-111 — `()` — ```
-  `validate_accepts_user_id_or_name` function L114-122 — `()` — ```

#### crates/arawn-feeds/src/templates/slack/mod.rs

-  `channel_archive` module L3 — `-` — Slack feed templates.
-  `common` module L4 — `-` — Slack feed templates.
-  `dm_archive` module L5 — `-` — Slack feed templates.
-  `my_mentions` module L6 — `-` — Slack feed templates.

#### crates/arawn-feeds/src/templates/slack/my_mentions.rs

- pub `MyMentionsTemplate` struct L48 — `-` — - Custom alert keywords.
-  `NAME` variable L50 — `: &str` — - Custom alert keywords.
-  `MyMentionsTemplate` type L53-144 — `impl FeedTemplate for MyMentionsTemplate` — - Custom alert keywords.
-  `name` function L54-56 — `(&self) -> &'static str` — - Custom alert keywords.
-  `validate` function L58-61 — `(&self, _params: &TemplateParams) -> Result<(), FeedError>` — - Custom alert keywords.
-  `defaults` function L63-68 — `(&self, _params: &TemplateParams) -> FeedDefaults` — - Custom alert keywords.
-  `run` function L70-143 — `( &self, ctx: &TemplateCtx, _params: &TemplateParams, feed_dir: &Path, cursor: &...` — - Custom alert keywords.
-  `append_message_to_day` function L148-163 — `(feed_dir: &Path, msg: &Value, ts: &str) -> Result<u64, FeedError>` — - Custom alert keywords.
-  `ts_to_yyyy_mm_dd` function L165-175 — `(ts: &str) -> Result<String, FeedError>` — - Custom alert keywords.
-  `tests` module L178-195 — `-` — - Custom alert keywords.
-  `validate_accepts_no_params` function L182-185 — `()` — - Custom alert keywords.
-  `defaults_provide_cursor_with_null_user_id` function L188-194 — `()` — - Custom alert keywords.

### crates/arawn-feeds/tests

> *Semantic summary to be generated by AI agent.*

#### crates/arawn-feeds/tests/calendar_upcoming_archive.rs

-  `MockCalendarClient` struct L24-29 — `{ responses: Mutex<Vec<Vec<Value>>>, calls: Mutex<Vec<(String, DateTime<Utc>, Da...` — - Auth error when calendar integration not connected.
-  `MockCalendarClient` type L31-38 — `= MockCalendarClient` — - Auth error when calendar integration not connected.
-  `queue` function L32-34 — `(&self, events: Vec<Value>)` — - Auth error when calendar integration not connected.
-  `calls` function L35-37 — `(&self) -> Vec<(String, DateTime<Utc>, DateTime<Utc>)>` — - Auth error when calendar integration not connected.
-  `MockCalendarClient` type L41-55 — `impl CalendarFeedClient for MockCalendarClient` — - Auth error when calendar integration not connected.
-  `list_events` function L42-54 — `( &self, calendar_id: &str, time_min: DateTime<Utc>, time_max: DateTime<Utc>, ) ...` — - Auth error when calendar integration not connected.
-  `MockClients` struct L57-59 — `{ calendar: Arc<MockCalendarClient> }` — - Auth error when calendar integration not connected.
-  `MockClients` type L61-77 — `impl FeedClients for MockClients` — - Auth error when calendar integration not connected.
-  `slack` function L62-64 — `(&self) -> Option<Arc<dyn SlackFeedClient>>` — - Auth error when calendar integration not connected.
-  `calendar` function L65-67 — `(&self) -> Option<Arc<dyn CalendarFeedClient>>` — - Auth error when calendar integration not connected.
-  `gmail` function L68-70 — `(&self) -> Option<Arc<dyn GmailFeedClient>>` — - Auth error when calendar integration not connected.
-  `drive` function L71-73 — `(&self) -> Option<Arc<dyn DriveFeedClient>>` — - Auth error when calendar integration not connected.
-  `atlassian` function L74-76 — `(&self) -> Option<Arc<dyn AtlassianFeedClient>>` — - Auth error when calendar integration not connected.
-  `event` function L79-87 — `(id: &str, summary: &str, start: &str) -> Value` — - Auth error when calendar integration not connected.
-  `read_event_file` function L89-96 — `(feed_dir: &PathBuf, safe_id: &str) -> Option<Value>` — - Auth error when calendar integration not connected.
-  `run_once` function L98-123 — `( template: &dyn FeedTemplate, ctx: &TemplateCtx, params: &TemplateParams, feed_...` — - Auth error when calendar integration not connected.
-  `first_run_writes_one_file_per_event` function L126-169 — `()` — - Auth error when calendar integration not connected.
-  `second_run_overwrites_changed_events` function L172-215 — `()` — - Auth error when calendar integration not connected.
-  `cancelled_events_are_preserved` function L218-246 — `()` — - Auth error when calendar integration not connected.
-  `params_reach_the_client` function L249-271 — `()` — - Auth error when calendar integration not connected.
-  `returns_auth_when_calendar_not_connected` function L274-306 — `()` — - Auth error when calendar integration not connected.
-  `NoCal` struct L275 — `-` — - Auth error when calendar integration not connected.
-  `NoCal` type L276-292 — `impl FeedClients for NoCal` — - Auth error when calendar integration not connected.
-  `slack` function L277-279 — `(&self) -> Option<Arc<dyn SlackFeedClient>>` — - Auth error when calendar integration not connected.
-  `calendar` function L280-282 — `(&self) -> Option<Arc<dyn CalendarFeedClient>>` — - Auth error when calendar integration not connected.
-  `gmail` function L283-285 — `(&self) -> Option<Arc<dyn GmailFeedClient>>` — - Auth error when calendar integration not connected.
-  `drive` function L286-288 — `(&self) -> Option<Arc<dyn DriveFeedClient>>` — - Auth error when calendar integration not connected.
-  `atlassian` function L289-291 — `(&self) -> Option<Arc<dyn AtlassianFeedClient>>` — - Auth error when calendar integration not connected.
-  `empty_window_writes_nothing_and_status_no_new_items` function L309-328 — `()` — - Auth error when calendar integration not connected.
-  `malformed_event_without_id_is_skipped` function L331-366 — `()` — - Auth error when calendar integration not connected.

#### crates/arawn-feeds/tests/cloacina_fire.rs

-  `create_feeds_schema` function L26-39 — `(conn: &Connection)` — workflow registration + execution machinery.
-  `build_runner` function L41-54 — `(workflows_db: &std::path::Path) -> Arc<DefaultRunner>` — workflow registration + execution machinery.
-  `cloacina_fires_feed_workflow_end_to_end` function L57-129 — `()` — workflow registration + execution machinery.
-  `cloacina_fires_advance_cursor_across_two_executions` function L132-186 — `()` — workflow registration + execution machinery.
-  `registering_a_feed_with_unknown_template_is_skipped_at_boot` function L189-248 — `()` — workflow registration + execution machinery.

#### crates/arawn-feeds/tests/confluence_space_archive.rs

-  `MockAtlassianClient` struct L18-29 — `{ page_lists: Mutex<Vec<Vec<ConfluencePageMeta>>>, bodies: Mutex<std::collection...` — Integration tests for `confluence/space-archive`.
-  `MockAtlassianClient` type L31-47 — `= MockAtlassianClient` — Integration tests for `confluence/space-archive`.
-  `queue_pages` function L32-34 — `(&self, pages: Vec<ConfluencePageMeta>)` — Integration tests for `confluence/space-archive`.
-  `set_body` function L35-37 — `(&self, page_id: &str, xml: Option<String>)` — Integration tests for `confluence/space-archive`.
-  `fail_body_for` function L38-40 — `(&self, page_id: &str)` — Integration tests for `confluence/space-archive`.
-  `list_calls` function L41-43 — `(&self) -> Vec<(String, Option<DateTime<Utc>>)>` — Integration tests for `confluence/space-archive`.
-  `body_calls` function L44-46 — `(&self) -> Vec<String>` — Integration tests for `confluence/space-archive`.
-  `MockAtlassianClient` type L50-112 — `impl AtlassianFeedClient for MockAtlassianClient` — Integration tests for `confluence/space-archive`.
-  `space_pages_modified_since` function L51-62 — `( &self, space_key: &str, since: Option<DateTime<Utc>>, ) -> Result<Vec<Confluen...` — Integration tests for `confluence/space-archive`.
-  `jql_search` function L64-70 — `( &self, _: &str, _: u32, ) -> Result<Vec<JiraIssueMeta>, FeedError>` — Integration tests for `confluence/space-archive`.
-  `issue_full` function L72-79 — `( &self, _: &str, _: bool, _: bool, ) -> Result<JiraIssueDetail, FeedError>` — Integration tests for `confluence/space-archive`.
-  `resolve_project` function L81-83 — `(&self, _: &str) -> Result<String, FeedError>` — Integration tests for `confluence/space-archive`.
-  `list_jira_projects` function L85-89 — `( &self, ) -> Result<Vec<arawn_feeds::JiraProjectMeta>, FeedError>` — Integration tests for `confluence/space-archive`.
-  `list_confluence_spaces` function L91-95 — `( &self, ) -> Result<Vec<arawn_feeds::ConfluenceSpaceMeta>, FeedError>` — Integration tests for `confluence/space-archive`.
-  `page_body_storage` function L97-111 — `( &self, page_id: &str, ) -> Result<ConfluencePageBody, FeedError>` — Integration tests for `confluence/space-archive`.
-  `MockClients` struct L114-116 — `{ atlassian: Arc<MockAtlassianClient> }` — Integration tests for `confluence/space-archive`.
-  `MockClients` type L118-134 — `impl FeedClients for MockClients` — Integration tests for `confluence/space-archive`.
-  `slack` function L119-121 — `(&self) -> Option<Arc<dyn SlackFeedClient>>` — Integration tests for `confluence/space-archive`.
-  `calendar` function L122-124 — `(&self) -> Option<Arc<dyn CalendarFeedClient>>` — Integration tests for `confluence/space-archive`.
-  `gmail` function L125-127 — `(&self) -> Option<Arc<dyn GmailFeedClient>>` — Integration tests for `confluence/space-archive`.
-  `drive` function L128-130 — `(&self) -> Option<Arc<dyn DriveFeedClient>>` — Integration tests for `confluence/space-archive`.
-  `atlassian` function L131-133 — `(&self) -> Option<Arc<dyn AtlassianFeedClient>>` — Integration tests for `confluence/space-archive`.
-  `page` function L136-145 — `(id: &str, title: &str, modified: &str, version: i64) -> ConfluencePageMeta` — Integration tests for `confluence/space-archive`.
-  `run_once` function L147-170 — `( template: &dyn FeedTemplate, ctx: &TemplateCtx, params: &TemplateParams, feed_...` — Integration tests for `confluence/space-archive`.
-  `writes_per_page_metadata_and_body` function L173-209 — `()` — Integration tests for `confluence/space-archive`.
-  `second_run_passes_cursor_as_since` function L212-238 — `()` — Integration tests for `confluence/space-archive`.
-  `body_fetch_failure_skips_page_without_aborting_run` function L241-267 — `()` — Integration tests for `confluence/space-archive`.
-  `body_overwritten_on_re_fetch` function L270-296 — `()` — Integration tests for `confluence/space-archive`.
-  `page_with_no_body_writes_empty_xml` function L299-317 — `()` — Integration tests for `confluence/space-archive`.
-  `empty_run_is_no_op_with_status` function L320-333 — `()` — Integration tests for `confluence/space-archive`.
-  `returns_auth_when_atlassian_not_connected` function L336-368 — `()` — Integration tests for `confluence/space-archive`.
-  `NoAtlassian` struct L337 — `-` — Integration tests for `confluence/space-archive`.
-  `NoAtlassian` type L338-354 — `impl FeedClients for NoAtlassian` — Integration tests for `confluence/space-archive`.
-  `slack` function L339-341 — `(&self) -> Option<Arc<dyn SlackFeedClient>>` — Integration tests for `confluence/space-archive`.
-  `calendar` function L342-344 — `(&self) -> Option<Arc<dyn CalendarFeedClient>>` — Integration tests for `confluence/space-archive`.
-  `gmail` function L345-347 — `(&self) -> Option<Arc<dyn GmailFeedClient>>` — Integration tests for `confluence/space-archive`.
-  `drive` function L348-350 — `(&self) -> Option<Arc<dyn DriveFeedClient>>` — Integration tests for `confluence/space-archive`.
-  `atlassian` function L351-353 — `(&self) -> Option<Arc<dyn AtlassianFeedClient>>` — Integration tests for `confluence/space-archive`.
-  `validate_rejects_missing_space_key` function L371-379 — `()` — Integration tests for `confluence/space-archive`.

#### crates/arawn-feeds/tests/discovery.rs

-  `StubClients` struct L22-26 — `{ slack_channels: Vec<SlackChannel>, jira_projects: Vec<JiraProjectMeta>, conflu...` — return `None`.
-  `StubSlack` struct L28 — `-` — return `None`.
-  `StubSlack` type L31-66 — `impl SlackFeedClient for StubSlack` — return `None`.
-  `resolve_channel` function L32-34 — `(&self, _: &str) -> Result<String, FeedError>` — return `None`.
-  `channel_history` function L35-41 — `( &self, _: &str, _: Option<&str>, ) -> Result<SlackHistoryPage, FeedError>` — return `None`.
-  `thread_replies` function L42-49 — `( &self, _: &str, _: &str, _: Option<&str>, ) -> Result<SlackHistoryPage, FeedEr...` — return `None`.
-  `open_dm` function L50-52 — `(&self, _: &str) -> Result<String, FeedError>` — return `None`.
-  `auth_test` function L53-55 — `(&self) -> Result<SlackAuthInfo, FeedError>` — return `None`.
-  `search_messages` function L56-62 — `( &self, _: &str, _: Option<&str>, ) -> Result<SlackHistoryPage, FeedError>` — return `None`.
-  `list_channels` function L63-65 — `(&self) -> Result<Vec<SlackChannel>, FeedError>` — return `None`.
-  `StubAtlassian` struct L68-71 — `{ projects: Vec<JiraProjectMeta>, spaces: Vec<ConfluenceSpaceMeta> }` — return `None`.
-  `StubAtlassian` type L74-107 — `impl AtlassianFeedClient for StubAtlassian` — return `None`.
-  `space_pages_modified_since` function L75-81 — `( &self, _: &str, _: Option<DateTime<Utc>>, ) -> Result<Vec<ConfluencePageMeta>,...` — return `None`.
-  `page_body_storage` function L82-84 — `(&self, _: &str) -> Result<ConfluencePageBody, FeedError>` — return `None`.
-  `jql_search` function L85-87 — `(&self, _: &str, _: u32) -> Result<Vec<JiraIssueMeta>, FeedError>` — return `None`.
-  `issue_full` function L88-95 — `( &self, _: &str, _: bool, _: bool, ) -> Result<JiraIssueDetail, FeedError>` — return `None`.
-  `resolve_project` function L96-98 — `(&self, _: &str) -> Result<String, FeedError>` — return `None`.
-  `list_jira_projects` function L99-101 — `(&self) -> Result<Vec<JiraProjectMeta>, FeedError>` — return `None`.
-  `list_confluence_spaces` function L102-106 — `( &self, ) -> Result<Vec<ConfluenceSpaceMeta>, FeedError>` — return `None`.
-  `StubClients` type L109-136 — `impl FeedClients for StubClients` — return `None`.
-  `slack` function L110-116 — `(&self) -> Option<Arc<dyn SlackFeedClient>>` — return `None`.
-  `calendar` function L117-119 — `(&self) -> Option<Arc<dyn CalendarFeedClient>>` — return `None`.
-  `gmail` function L120-122 — `(&self) -> Option<Arc<dyn GmailFeedClient>>` — return `None`.
-  `drive` function L123-125 — `(&self) -> Option<Arc<dyn DriveFeedClient>>` — return `None`.
-  `atlassian` function L126-135 — `(&self) -> Option<Arc<dyn AtlassianFeedClient>>` — return `None`.
-  `slack_channel_archive_discovers_channels` function L139-176 — `()` — return `None`.
-  `jira_project_tracker_discovers_projects` function L179-205 — `()` — return `None`.
-  `confluence_space_archive_discovers_spaces` function L208-233 — `()` — return `None`.
-  `discover_returns_none_when_provider_missing` function L236-246 — `()` — return `None`.
-  `non_pickable_template_returns_none` function L249-260 — `()` — return `None`.

#### crates/arawn-feeds/tests/drive_folder_sync.rs

-  `MockDriveClient` struct L20-29 — `{ children: Mutex<HashMap<String, Vec<DriveFile>>>, raw_bodies: Mutex<HashMap<St...` — In-memory Drive emulator.
-  `MockDriveClient` type L31-50 — `= MockDriveClient` — Integration tests for `drive/folder-sync`.
-  `add_folder` function L32-34 — `(&self, id: &str, children: Vec<DriveFile>)` — Integration tests for `drive/folder-sync`.
-  `add_raw` function L35-40 — `(&self, file_id: &str, body: &[u8])` — Integration tests for `drive/folder-sync`.
-  `add_export` function L41-46 — `(&self, file_id: &str, export_mime: &str, body: &[u8])` — Integration tests for `drive/folder-sync`.
-  `download_calls` function L47-49 — `(&self) -> Vec<(String, Option<String>)>` — Integration tests for `drive/folder-sync`.
-  `MockDriveClient` type L53-98 — `impl DriveFeedClient for MockDriveClient` — Integration tests for `drive/folder-sync`.
-  `resolve_folder` function L54-56 — `(&self, path_or_id: &str) -> Result<String, FeedError>` — Integration tests for `drive/folder-sync`.
-  `list_folder_children` function L57-65 — `(&self, folder_id: &str) -> Result<Vec<DriveFile>, FeedError>` — Integration tests for `drive/folder-sync`.
-  `list_modified_since` function L66-72 — `( &self, _since: DateTime<Utc>, _max_results: u32, ) -> Result<Vec<DriveFile>, F...` — Integration tests for `drive/folder-sync`.
-  `download` function L73-97 — `( &self, file_id: &str, export_mime: Option<&str>, ) -> Result<Vec<u8>, FeedErro...` — Integration tests for `drive/folder-sync`.
-  `MockClients` struct L100-102 — `{ drive: Arc<MockDriveClient> }` — Integration tests for `drive/folder-sync`.
-  `MockClients` type L104-120 — `impl FeedClients for MockClients` — Integration tests for `drive/folder-sync`.
-  `slack` function L105-107 — `(&self) -> Option<Arc<dyn SlackFeedClient>>` — Integration tests for `drive/folder-sync`.
-  `calendar` function L108-110 — `(&self) -> Option<Arc<dyn CalendarFeedClient>>` — Integration tests for `drive/folder-sync`.
-  `gmail` function L111-113 — `(&self) -> Option<Arc<dyn GmailFeedClient>>` — Integration tests for `drive/folder-sync`.
-  `drive` function L114-116 — `(&self) -> Option<Arc<dyn DriveFeedClient>>` — Integration tests for `drive/folder-sync`.
-  `atlassian` function L117-119 — `(&self) -> Option<Arc<dyn AtlassianFeedClient>>` — Integration tests for `drive/folder-sync`.
-  `raw_file` function L122-133 — `(id: &str, name: &str, mime: &str, md5: &str) -> DriveFile` — Integration tests for `drive/folder-sync`.
-  `folder` function L135-146 — `(id: &str, name: &str) -> DriveFile` — Integration tests for `drive/folder-sync`.
-  `google_doc` function L148-159 — `(id: &str, name: &str, modified: &str) -> DriveFile` — Integration tests for `drive/folder-sync`.
-  `run_once` function L161-184 — `( template: &dyn FeedTemplate, ctx: &TemplateCtx, params: &TemplateParams, feed_...` — Integration tests for `drive/folder-sync`.
-  `mirrors_native_files_and_exports_google_natives` function L187-234 — `()` — Integration tests for `drive/folder-sync`.
-  `skips_unchanged_via_change_token_cursor` function L237-261 — `()` — Integration tests for `drive/folder-sync`.
-  `deletes_local_when_remote_deleted` function L264-293 — `()` — Integration tests for `drive/folder-sync`.
-  `moved_file_cleans_up_old_path` function L296-331 — `()` — Integration tests for `drive/folder-sync`.
-  `unsupported_google_native_is_skipped` function L334-363 — `()` — Integration tests for `drive/folder-sync`.
-  `returns_auth_when_drive_not_connected` function L366-398 — `()` — Integration tests for `drive/folder-sync`.
-  `NoDrive` struct L367 — `-` — Integration tests for `drive/folder-sync`.
-  `NoDrive` type L368-384 — `impl FeedClients for NoDrive` — Integration tests for `drive/folder-sync`.
-  `slack` function L369-371 — `(&self) -> Option<Arc<dyn SlackFeedClient>>` — Integration tests for `drive/folder-sync`.
-  `calendar` function L372-374 — `(&self) -> Option<Arc<dyn CalendarFeedClient>>` — Integration tests for `drive/folder-sync`.
-  `gmail` function L375-377 — `(&self) -> Option<Arc<dyn GmailFeedClient>>` — Integration tests for `drive/folder-sync`.
-  `drive` function L378-380 — `(&self) -> Option<Arc<dyn DriveFeedClient>>` — Integration tests for `drive/folder-sync`.
-  `atlassian` function L381-383 — `(&self) -> Option<Arc<dyn AtlassianFeedClient>>` — Integration tests for `drive/folder-sync`.
-  `validate_rejects_missing_folder` function L401-409 — `()` — Integration tests for `drive/folder-sync`.
-  `skips_file_with_provider_error_and_continues_batch` function L412-443 — `()` — Integration tests for `drive/folder-sync`.

#### crates/arawn-feeds/tests/drive_recent.rs

-  `MockDriveClient` struct L17-21 — `{ pages: Mutex<Vec<Vec<DriveFile>>>, calls: Mutex<Vec<DateTime<Utc>>> }` — Integration tests for `drive/recent`.
-  `MockDriveClient` type L23-30 — `= MockDriveClient` — Integration tests for `drive/recent`.
-  `queue` function L24-26 — `(&self, files: Vec<DriveFile>)` — Integration tests for `drive/recent`.
-  `last_since` function L27-29 — `(&self) -> Option<DateTime<Utc>>` — Integration tests for `drive/recent`.
-  `MockDriveClient` type L33-52 — `impl DriveFeedClient for MockDriveClient` — Integration tests for `drive/recent`.
-  `resolve_folder` function L34-36 — `(&self, _: &str) -> Result<String, FeedError>` — Integration tests for `drive/recent`.
-  `list_folder_children` function L37-39 — `(&self, _: &str) -> Result<Vec<DriveFile>, FeedError>` — Integration tests for `drive/recent`.
-  `list_modified_since` function L40-48 — `( &self, since: DateTime<Utc>, _max_results: u32, ) -> Result<Vec<DriveFile>, Fe...` — Integration tests for `drive/recent`.
-  `download` function L49-51 — `(&self, _: &str, _: Option<&str>) -> Result<Vec<u8>, FeedError>` — Integration tests for `drive/recent`.
-  `MockClients` struct L54-56 — `{ drive: Arc<MockDriveClient> }` — Integration tests for `drive/recent`.
-  `MockClients` type L58-74 — `impl FeedClients for MockClients` — Integration tests for `drive/recent`.
-  `slack` function L59-61 — `(&self) -> Option<Arc<dyn SlackFeedClient>>` — Integration tests for `drive/recent`.
-  `calendar` function L62-64 — `(&self) -> Option<Arc<dyn CalendarFeedClient>>` — Integration tests for `drive/recent`.
-  `gmail` function L65-67 — `(&self) -> Option<Arc<dyn GmailFeedClient>>` — Integration tests for `drive/recent`.
-  `drive` function L68-70 — `(&self) -> Option<Arc<dyn DriveFeedClient>>` — Integration tests for `drive/recent`.
-  `atlassian` function L71-73 — `(&self) -> Option<Arc<dyn AtlassianFeedClient>>` — Integration tests for `drive/recent`.
-  `file` function L76-87 — `(id: &str, name: &str, mime: &str, modified: &str) -> DriveFile` — Integration tests for `drive/recent`.
-  `run_once` function L89-112 — `( template: &dyn FeedTemplate, ctx: &TemplateCtx, params: &TemplateParams, feed_...` — Integration tests for `drive/recent`.
-  `writes_per_file_metadata_partitioned_by_modified_date` function L115-145 — `()` — Integration tests for `drive/recent`.
-  `second_run_uses_cursor_as_since` function L148-164 — `()` — Integration tests for `drive/recent`.
-  `empty_run_is_no_op_with_status` function L167-182 — `()` — Integration tests for `drive/recent`.
-  `returns_auth_when_drive_not_connected` function L185-214 — `()` — Integration tests for `drive/recent`.
-  `NoDrive` struct L186 — `-` — Integration tests for `drive/recent`.
-  `NoDrive` type L187-203 — `impl FeedClients for NoDrive` — Integration tests for `drive/recent`.
-  `slack` function L188-190 — `(&self) -> Option<Arc<dyn SlackFeedClient>>` — Integration tests for `drive/recent`.
-  `calendar` function L191-193 — `(&self) -> Option<Arc<dyn CalendarFeedClient>>` — Integration tests for `drive/recent`.
-  `gmail` function L194-196 — `(&self) -> Option<Arc<dyn GmailFeedClient>>` — Integration tests for `drive/recent`.
-  `drive` function L197-199 — `(&self) -> Option<Arc<dyn DriveFeedClient>>` — Integration tests for `drive/recent`.
-  `atlassian` function L200-202 — `(&self) -> Option<Arc<dyn AtlassianFeedClient>>` — Integration tests for `drive/recent`.
-  `second_run_skips_already_archived_boundary_file` function L217-242 — `()` — Integration tests for `drive/recent`.
-  `validate_rejects_bad_days_back` function L245-250 — `()` — Integration tests for `drive/recent`.

#### crates/arawn-feeds/tests/dynamic_register.rs

-  `migrate` function L17-32 — `(conn: &Connection)` — firings happen (so the run_count is 0 and last_run_at is None).
-  `dynamic_register_full_flow` function L35-112 — `()` — firings happen (so the run_count is 0 and last_run_at is None).
-  `pause_resume_round_trip_through_cloacina` function L115-186 — `()` — firings happen (so the run_count is 0 and last_run_at is None).
-  `remove_wipes_cron_row_and_data_dir` function L189-259 — `()` — firings happen (so the run_count is 0 and last_run_at is None).
-  `pause_unknown_feed_returns_invalid_params` function L262-290 — `()` — firings happen (so the run_count is 0 and last_run_at is None).
-  `dynamic_register_is_idempotent_via_unique_constraint` function L293-347 — `()` — firings happen (so the run_count is 0 and last_run_at is None).
-  `since_param_triggers_backfill_loop_then_registers_cron` function L350-434 — `()` — firings happen (so the run_count is 0 and last_run_at is None).
-  `no_since_uses_existing_immediate_cron_path` function L437-489 — `()` — firings happen (so the run_count is 0 and last_run_at is None).
-  `dynamic_register_rolls_back_on_unknown_template` function L492-535 — `()` — firings happen (so the run_count is 0 and last_run_at is None).

#### crates/arawn-feeds/tests/gmail_archive.rs

-  `message` function L21-34 — `(id: &str, internal_date_ms: i64, subject: &str) -> Value` — Minimal Gmail message JSON for tests.
-  `MockGmailClient` struct L37-45 — `{ pages: Mutex<Vec<(Vec<String>, std::collections::HashMap<String, Value>)>>, li...` — per-template query construction.
-  `MockGmailClient` type L47-65 — `= MockGmailClient` — per-template query construction.
-  `queue_messages` function L48-58 — `(&self, msgs: Vec<Value>)` — per-template query construction.
-  `list_calls` function L59-61 — `(&self) -> Vec<(String, u32)>` — per-template query construction.
-  `get_call_count` function L62-64 — `(&self) -> usize` — per-template query construction.
-  `MockGmailClient` type L68-97 — `impl GmailFeedClient for MockGmailClient` — per-template query construction.
-  `list_message_ids` function L69-84 — `( &self, query: &str, max_results: u32, ) -> Result<Vec<String>, FeedError>` — per-template query construction.
-  `get_message` function L86-96 — `(&self, id: &str) -> Result<Value, FeedError>` — per-template query construction.
-  `MockClients` struct L99-101 — `{ gmail: Arc<MockGmailClient> }` — per-template query construction.
-  `MockClients` type L103-119 — `impl FeedClients for MockClients` — per-template query construction.
-  `slack` function L104-106 — `(&self) -> Option<Arc<dyn SlackFeedClient>>` — per-template query construction.
-  `calendar` function L107-109 — `(&self) -> Option<Arc<dyn CalendarFeedClient>>` — per-template query construction.
-  `gmail` function L110-112 — `(&self) -> Option<Arc<dyn GmailFeedClient>>` — per-template query construction.
-  `drive` function L113-115 — `(&self) -> Option<Arc<dyn DriveFeedClient>>` — per-template query construction.
-  `atlassian` function L116-118 — `(&self) -> Option<Arc<dyn AtlassianFeedClient>>` — per-template query construction.
-  `run_once` function L121-144 — `( template: &dyn FeedTemplate, ctx: &TemplateCtx, params: &TemplateParams, feed_...` — per-template query construction.
-  `ymd_ms` function L146-152 — `(y: i32, m: u32, d: u32) -> i64` — per-template query construction.
-  `read_msg` function L154-160 — `(feed_dir: &PathBuf, day: &str, id: &str) -> Option<Value>` — per-template query construction.
-  `inbox_archive_writes_per_message_partitioned_by_internal_date` function L163-203 — `()` — per-template query construction.
-  `second_run_skips_already_archived_ids` function L206-244 — `()` — per-template query construction.
-  `sender_filter_query_uses_from_operator` function L247-270 — `()` — per-template query construction.
-  `label_archive_query_uses_label_operator` function L273-293 — `()` — per-template query construction.
-  `returns_auth_when_gmail_not_connected` function L296-328 — `()` — per-template query construction.
-  `NoGmail` struct L297 — `-` — per-template query construction.
-  `NoGmail` type L298-314 — `impl FeedClients for NoGmail` — per-template query construction.
-  `slack` function L299-301 — `(&self) -> Option<Arc<dyn SlackFeedClient>>` — per-template query construction.
-  `calendar` function L302-304 — `(&self) -> Option<Arc<dyn CalendarFeedClient>>` — per-template query construction.
-  `gmail` function L305-307 — `(&self) -> Option<Arc<dyn GmailFeedClient>>` — per-template query construction.
-  `drive` function L308-310 — `(&self) -> Option<Arc<dyn DriveFeedClient>>` — per-template query construction.
-  `atlassian` function L311-313 — `(&self) -> Option<Arc<dyn AtlassianFeedClient>>` — per-template query construction.
-  `malformed_message_skipped_without_aborting_batch` function L331-371 — `()` — per-template query construction.

#### crates/arawn-feeds/tests/jira_trackers.rs

-  `MockAtlassian` struct L20-31 — `{ jql_pages: Mutex<Vec<Vec<JiraIssueMeta>>>, issue_details: Mutex<HashMap<String...` — In-memory atlassian emulator.
-  `MockAtlassian` type L33-54 — `= MockAtlassian` — Integration tests for the two Jira templates.
-  `queue_search` function L34-36 — `(&self, list: Vec<JiraIssueMeta>)` — Integration tests for the two Jira templates.
-  `queue_detail` function L37-44 — `(&self, key: &str, detail: JiraIssueDetail)` — Integration tests for the two Jira templates.
-  `fail_full` function L45-47 — `(&self, key: &str)` — Integration tests for the two Jira templates.
-  `jql_calls` function L48-50 — `(&self) -> Vec<(String, u32)>` — Integration tests for the two Jira templates.
-  `full_calls` function L51-53 — `(&self) -> Vec<(String, bool, bool)>` — Integration tests for the two Jira templates.
-  `MockAtlassian` type L57-121 — `impl AtlassianFeedClient for MockAtlassian` — Integration tests for the two Jira templates.
-  `space_pages_modified_since` function L58-64 — `( &self, _: &str, _: Option<DateTime<Utc>>, ) -> Result<Vec<ConfluencePageMeta>,...` — Integration tests for the two Jira templates.
-  `page_body_storage` function L65-67 — `(&self, _: &str) -> Result<ConfluencePageBody, FeedError>` — Integration tests for the two Jira templates.
-  `jql_search` function L69-80 — `( &self, jql: &str, max_results: u32, ) -> Result<Vec<JiraIssueMeta>, FeedError>` — Integration tests for the two Jira templates.
-  `issue_full` function L82-103 — `( &self, key: &str, want_changelog: bool, want_comments: bool, ) -> Result<JiraI...` — Integration tests for the two Jira templates.
-  `resolve_project` function L105-108 — `(&self, key_or_id: &str) -> Result<String, FeedError>` — Integration tests for the two Jira templates.
-  `list_jira_projects` function L110-114 — `( &self, ) -> Result<Vec<arawn_feeds::JiraProjectMeta>, FeedError>` — Integration tests for the two Jira templates.
-  `list_confluence_spaces` function L116-120 — `( &self, ) -> Result<Vec<arawn_feeds::ConfluenceSpaceMeta>, FeedError>` — Integration tests for the two Jira templates.
-  `MockClients` struct L123-125 — `{ atlassian: Arc<MockAtlassian> }` — Integration tests for the two Jira templates.
-  `MockClients` type L127-143 — `impl FeedClients for MockClients` — Integration tests for the two Jira templates.
-  `slack` function L128-130 — `(&self) -> Option<Arc<dyn SlackFeedClient>>` — Integration tests for the two Jira templates.
-  `calendar` function L131-133 — `(&self) -> Option<Arc<dyn CalendarFeedClient>>` — Integration tests for the two Jira templates.
-  `gmail` function L134-136 — `(&self) -> Option<Arc<dyn GmailFeedClient>>` — Integration tests for the two Jira templates.
-  `drive` function L137-139 — `(&self) -> Option<Arc<dyn DriveFeedClient>>` — Integration tests for the two Jira templates.
-  `atlassian` function L140-142 — `(&self) -> Option<Arc<dyn AtlassianFeedClient>>` — Integration tests for the two Jira templates.
-  `issue_meta` function L145-152 — `(key: &str, updated: &str) -> JiraIssueMeta` — Integration tests for the two Jira templates.
-  `issue_detail` function L154-170 — `( key: &str, updated: &str, comments: Option<Vec<Value>>, changelog: Option<Vec<...` — Integration tests for the two Jira templates.
-  `comment` function L172-179 — `(id: &str, body: &str) -> Value` — Integration tests for the two Jira templates.
-  `history` function L181-187 — `(id: &str, field: &str, to: &str) -> Value` — Integration tests for the two Jira templates.
-  `run_once` function L189-212 — `( template: &dyn FeedTemplate, ctx: &TemplateCtx, params: &TemplateParams, feed_...` — Integration tests for the two Jira templates.
-  `read_jsonl` function L214-224 — `(path: &PathBuf) -> Vec<Value>` — Integration tests for the two Jira templates.
-  `project_tracker_appends_new_comments_overwrites_issue_snapshot` function L229-286 — `()` — Integration tests for the two Jira templates.
-  `project_tracker_history_advances_independently_of_comments` function L289-331 — `()` — Integration tests for the two Jira templates.
-  `project_tracker_partial_failure_doesnt_block_other_issues` function L334-364 — `()` — Integration tests for the two Jira templates.
-  `project_tracker_validates_project` function L367-375 — `()` — Integration tests for the two Jira templates.
-  `assignee_tracker_writes_only_issue_json_no_logs` function L380-419 — `()` — Integration tests for the two Jira templates.
-  `assignee_tracker_uses_currentUser_jql_and_advances_cursor` function L422-457 — `()` — Integration tests for the two Jira templates.
-  `returns_auth_when_atlassian_not_connected` function L460-492 — `()` — Integration tests for the two Jira templates.
-  `NoAtlassian` struct L461 — `-` — Integration tests for the two Jira templates.
-  `NoAtlassian` type L462-478 — `impl FeedClients for NoAtlassian` — Integration tests for the two Jira templates.
-  `slack` function L463-465 — `(&self) -> Option<Arc<dyn SlackFeedClient>>` — Integration tests for the two Jira templates.
-  `calendar` function L466-468 — `(&self) -> Option<Arc<dyn CalendarFeedClient>>` — Integration tests for the two Jira templates.
-  `gmail` function L469-471 — `(&self) -> Option<Arc<dyn GmailFeedClient>>` — Integration tests for the two Jira templates.
-  `drive` function L472-474 — `(&self) -> Option<Arc<dyn DriveFeedClient>>` — Integration tests for the two Jira templates.
-  `atlassian` function L475-477 — `(&self) -> Option<Arc<dyn AtlassianFeedClient>>` — Integration tests for the two Jira templates.
-  `assignee_tracker_partial_failure_doesnt_block_other_issues` function L495-526 — `()` — Integration tests for the two Jira templates.

#### crates/arawn-feeds/tests/slack_channel_archive.rs

-  `MockSlackClient` struct L26-42 — `{ history_responses: Mutex<Vec<SlackHistoryPage>>, resolved_id: Mutex<String>, h...` — every Slack-touching template test will reuse.
-  `MockSlackClient` type L44-76 — `= MockSlackClient` — every Slack-touching template test will reuse.
-  `new` function L45-50 — `() -> Self` — every Slack-touching template test will reuse.
-  `queue` function L51-53 — `(&self, page: SlackHistoryPage)` — every Slack-touching template test will reuse.
-  `queue_thread` function L54-61 — `(&self, parent_ts: &str, page: SlackHistoryPage)` — every Slack-touching template test will reuse.
-  `queue_thread_error` function L62-69 — `(&self, parent_ts: &str, err: FeedError)` — every Slack-touching template test will reuse.
-  `calls` function L70-72 — `(&self) -> Vec<(String, Option<String>)>` — every Slack-touching template test will reuse.
-  `thread_calls` function L73-75 — `(&self) -> Vec<(String, String, Option<String>)>` — every Slack-touching template test will reuse.
-  `MockSlackClient` type L79-152 — `impl SlackFeedClient for MockSlackClient` — every Slack-touching template test will reuse.
-  `resolve_channel` function L80-82 — `(&self, _name_or_id: &str) -> Result<String, FeedError>` — every Slack-touching template test will reuse.
-  `channel_history` function L84-102 — `( &self, channel_id: &str, oldest_ts: Option<&str>, ) -> Result<SlackHistoryPage...` — every Slack-touching template test will reuse.
-  `open_dm` function L104-106 — `(&self, _user_id_or_name: &str) -> Result<String, FeedError>` — every Slack-touching template test will reuse.
-  `auth_test` function L108-110 — `(&self) -> Result<SlackAuthInfo, FeedError>` — every Slack-touching template test will reuse.
-  `search_messages` function L112-118 — `( &self, _query: &str, _oldest_ts: Option<&str>, ) -> Result<SlackHistoryPage, F...` — every Slack-touching template test will reuse.
-  `list_channels` function L120-122 — `(&self) -> Result<Vec<arawn_feeds::SlackChannel>, FeedError>` — every Slack-touching template test will reuse.
-  `thread_replies` function L124-151 — `( &self, channel_id: &str, parent_ts: &str, oldest_ts: Option<&str>, ) -> Result...` — every Slack-touching template test will reuse.
-  `MockClients` struct L154-156 — `{ slack: Arc<MockSlackClient> }` — every Slack-touching template test will reuse.
-  `MockClients` type L158-174 — `impl FeedClients for MockClients` — every Slack-touching template test will reuse.
-  `slack` function L159-161 — `(&self) -> Option<Arc<dyn SlackFeedClient>>` — every Slack-touching template test will reuse.
-  `calendar` function L162-164 — `(&self) -> Option<Arc<dyn CalendarFeedClient>>` — every Slack-touching template test will reuse.
-  `gmail` function L165-167 — `(&self) -> Option<Arc<dyn GmailFeedClient>>` — every Slack-touching template test will reuse.
-  `drive` function L168-170 — `(&self) -> Option<Arc<dyn DriveFeedClient>>` — every Slack-touching template test will reuse.
-  `atlassian` function L171-173 — `(&self) -> Option<Arc<dyn AtlassianFeedClient>>` — every Slack-touching template test will reuse.
-  `slack_msg` function L176-183 — `(ts: &str, text: &str) -> Value` — every Slack-touching template test will reuse.
-  `read_jsonl` function L187-197 — `(feed_dir: &PathBuf, day: &str) -> Vec<Value>` — Walk a YYYY-MM-DD.jsonl file in `feed_dir` and return all parsed
-  `run_once` function L199-225 — `( template: &dyn FeedTemplate, ctx: &TemplateCtx, params: &TemplateParams, feed_...` — every Slack-touching template test will reuse.
-  `first_run_writes_messages_and_advances_cursor` function L228-276 — `()` — every Slack-touching template test will reuse.
-  `second_run_passes_cursor_and_only_writes_new` function L279-325 — `()` — every Slack-touching template test will reuse.
-  `empty_run_is_a_no_op_with_status` function L328-361 — `()` — every Slack-touching template test will reuse.
-  `messages_partition_across_days` function L364-402 — `()` — every Slack-touching template test will reuse.
-  `run_returns_auth_when_slack_not_connected` function L405-440 — `()` — every Slack-touching template test will reuse.
-  `NoSlack` struct L406 — `-` — every Slack-touching template test will reuse.
-  `NoSlack` type L407-423 — `impl FeedClients for NoSlack` — every Slack-touching template test will reuse.
-  `slack` function L408-410 — `(&self) -> Option<Arc<dyn SlackFeedClient>>` — every Slack-touching template test will reuse.
-  `calendar` function L411-413 — `(&self) -> Option<Arc<dyn CalendarFeedClient>>` — every Slack-touching template test will reuse.
-  `gmail` function L414-416 — `(&self) -> Option<Arc<dyn GmailFeedClient>>` — every Slack-touching template test will reuse.
-  `drive` function L417-419 — `(&self) -> Option<Arc<dyn DriveFeedClient>>` — every Slack-touching template test will reuse.
-  `atlassian` function L420-422 — `(&self) -> Option<Arc<dyn AtlassianFeedClient>>` — every Slack-touching template test will reuse.
-  `slack_msg_with_replies` function L444-452 — `(ts: &str, text: &str, reply_count: u64) -> Value` — every Slack-touching template test will reuse.
-  `parent_with_replies_seeds_thread_file_and_advances_thread_cursor` function L455-527 — `()` — every Slack-touching template test will reuse.
-  `second_run_advances_thread_cursor_independently` function L530-595 — `()` — every Slack-touching template test will reuse.
-  `channel_archive_works_for_dm_id_passthrough` function L598-630 — `()` — every Slack-touching template test will reuse.
-  `channel_archive_works_for_mpim_id_passthrough` function L633-660 — `()` — every Slack-touching template test will reuse.
-  `classify_helper_resolves_kinds_for_picker_use` function L663-677 — `()` — every Slack-touching template test will reuse.
-  `thread_failure_does_not_block_channel_or_other_threads` function L680-744 — `()` — every Slack-touching template test will reuse.

#### crates/arawn-feeds/tests/slack_dm_archive.rs

-  `MockSlackClient` struct L22-29 — `{ history_responses: Mutex<Vec<SlackHistoryPage>>, dm_channel_id: Mutex<String>,...` — channel-archive already exercises.
-  `MockSlackClient` type L31-47 — `= MockSlackClient` — channel-archive already exercises.
-  `new` function L32-37 — `() -> Self` — channel-archive already exercises.
-  `queue` function L38-40 — `(&self, page: SlackHistoryPage)` — channel-archive already exercises.
-  `open_dm_calls` function L41-43 — `(&self) -> Vec<String>` — channel-archive already exercises.
-  `history_calls` function L44-46 — `(&self) -> Vec<(String, Option<String>)>` — channel-archive already exercises.
-  `MockSlackClient` type L50-110 — `impl SlackFeedClient for MockSlackClient` — channel-archive already exercises.
-  `resolve_channel` function L51-53 — `(&self, _name_or_id: &str) -> Result<String, FeedError>` — channel-archive already exercises.
-  `channel_history` function L55-73 — `( &self, channel_id: &str, oldest_ts: Option<&str>, ) -> Result<SlackHistoryPage...` — channel-archive already exercises.
-  `thread_replies` function L75-85 — `( &self, _channel_id: &str, _parent_ts: &str, oldest_ts: Option<&str>, ) -> Resu...` — channel-archive already exercises.
-  `open_dm` function L87-93 — `(&self, user_id_or_name: &str) -> Result<String, FeedError>` — channel-archive already exercises.
-  `auth_test` function L95-97 — `(&self) -> Result<SlackAuthInfo, FeedError>` — channel-archive already exercises.
-  `search_messages` function L99-105 — `( &self, _query: &str, _oldest_ts: Option<&str>, ) -> Result<SlackHistoryPage, F...` — channel-archive already exercises.
-  `list_channels` function L107-109 — `(&self) -> Result<Vec<arawn_feeds::SlackChannel>, FeedError>` — channel-archive already exercises.
-  `MockClients` struct L112-114 — `{ slack: Arc<MockSlackClient> }` — channel-archive already exercises.
-  `MockClients` type L116-132 — `impl FeedClients for MockClients` — channel-archive already exercises.
-  `slack` function L117-119 — `(&self) -> Option<Arc<dyn SlackFeedClient>>` — channel-archive already exercises.
-  `calendar` function L120-122 — `(&self) -> Option<Arc<dyn CalendarFeedClient>>` — channel-archive already exercises.
-  `gmail` function L123-125 — `(&self) -> Option<Arc<dyn GmailFeedClient>>` — channel-archive already exercises.
-  `drive` function L126-128 — `(&self) -> Option<Arc<dyn DriveFeedClient>>` — channel-archive already exercises.
-  `atlassian` function L129-131 — `(&self) -> Option<Arc<dyn AtlassianFeedClient>>` — channel-archive already exercises.
-  `dm_msg` function L134-141 — `(ts: &str, text: &str) -> Value` — channel-archive already exercises.
-  `read_jsonl` function L143-153 — `(feed_dir: &PathBuf, day: &str) -> Vec<Value>` — channel-archive already exercises.
-  `run_once` function L155-180 — `( template: &dyn FeedTemplate, ctx: &TemplateCtx, params: &TemplateParams, feed_...` — channel-archive already exercises.
-  `dm_archive_opens_dm_then_writes_messages` function L183-229 — `()` — channel-archive already exercises.
-  `dm_archive_returns_auth_when_slack_not_connected` function L232-267 — `()` — channel-archive already exercises.
-  `NoSlack` struct L233 — `-` — channel-archive already exercises.
-  `NoSlack` type L234-250 — `impl FeedClients for NoSlack` — channel-archive already exercises.
-  `slack` function L235-237 — `(&self) -> Option<Arc<dyn SlackFeedClient>>` — channel-archive already exercises.
-  `calendar` function L238-240 — `(&self) -> Option<Arc<dyn CalendarFeedClient>>` — channel-archive already exercises.
-  `gmail` function L241-243 — `(&self) -> Option<Arc<dyn GmailFeedClient>>` — channel-archive already exercises.
-  `drive` function L244-246 — `(&self) -> Option<Arc<dyn DriveFeedClient>>` — channel-archive already exercises.
-  `atlassian` function L247-249 — `(&self) -> Option<Arc<dyn AtlassianFeedClient>>` — channel-archive already exercises.

#### crates/arawn-feeds/tests/slack_my_mentions.rs

-  `MockSlackClient` struct L24-29 — `{ auth_info: Mutex<SlackAuthInfo>, auth_test_calls: Mutex<u32>, search_responses...` — - Empty result writes nothing and reports `no-new-items`.
-  `MockSlackClient` type L31-50 — `= MockSlackClient` — - Empty result writes nothing and reports `no-new-items`.
-  `new` function L32-40 — `() -> Self` — - Empty result writes nothing and reports `no-new-items`.
-  `queue_search` function L41-43 — `(&self, page: SlackHistoryPage)` — - Empty result writes nothing and reports `no-new-items`.
-  `auth_test_count` function L44-46 — `(&self) -> u32` — - Empty result writes nothing and reports `no-new-items`.
-  `search_calls` function L47-49 — `(&self) -> Vec<(String, Option<String>)>` — - Empty result writes nothing and reports `no-new-items`.
-  `MockSlackClient` type L53-104 — `impl SlackFeedClient for MockSlackClient` — - Empty result writes nothing and reports `no-new-items`.
-  `resolve_channel` function L54-56 — `(&self, _: &str) -> Result<String, FeedError>` — - Empty result writes nothing and reports `no-new-items`.
-  `channel_history` function L57-63 — `( &self, _: &str, _: Option<&str>, ) -> Result<SlackHistoryPage, FeedError>` — - Empty result writes nothing and reports `no-new-items`.
-  `thread_replies` function L64-71 — `( &self, _: &str, _: &str, _: Option<&str>, ) -> Result<SlackHistoryPage, FeedEr...` — - Empty result writes nothing and reports `no-new-items`.
-  `open_dm` function L72-74 — `(&self, _: &str) -> Result<String, FeedError>` — - Empty result writes nothing and reports `no-new-items`.
-  `auth_test` function L76-79 — `(&self) -> Result<SlackAuthInfo, FeedError>` — - Empty result writes nothing and reports `no-new-items`.
-  `search_messages` function L81-99 — `( &self, query: &str, oldest_ts: Option<&str>, ) -> Result<SlackHistoryPage, Fee...` — - Empty result writes nothing and reports `no-new-items`.
-  `list_channels` function L101-103 — `(&self) -> Result<Vec<arawn_feeds::SlackChannel>, FeedError>` — - Empty result writes nothing and reports `no-new-items`.
-  `MockClients` struct L106-108 — `{ slack: Arc<MockSlackClient> }` — - Empty result writes nothing and reports `no-new-items`.
-  `MockClients` type L110-126 — `impl FeedClients for MockClients` — - Empty result writes nothing and reports `no-new-items`.
-  `slack` function L111-113 — `(&self) -> Option<Arc<dyn SlackFeedClient>>` — - Empty result writes nothing and reports `no-new-items`.
-  `calendar` function L114-116 — `(&self) -> Option<Arc<dyn CalendarFeedClient>>` — - Empty result writes nothing and reports `no-new-items`.
-  `gmail` function L117-119 — `(&self) -> Option<Arc<dyn GmailFeedClient>>` — - Empty result writes nothing and reports `no-new-items`.
-  `drive` function L120-122 — `(&self) -> Option<Arc<dyn DriveFeedClient>>` — - Empty result writes nothing and reports `no-new-items`.
-  `atlassian` function L123-125 — `(&self) -> Option<Arc<dyn AtlassianFeedClient>>` — - Empty result writes nothing and reports `no-new-items`.
-  `mention_msg` function L128-137 — `(ts: &str, channel: &str, text: &str) -> Value` — - Empty result writes nothing and reports `no-new-items`.
-  `read_jsonl` function L139-150 — `(feed_dir: &PathBuf, day: &str) -> Vec<Value>` — - Empty result writes nothing and reports `no-new-items`.
-  `run_once` function L152-177 — `( template: &dyn FeedTemplate, ctx: &TemplateCtx, params: &TemplateParams, feed_...` — - Empty result writes nothing and reports `no-new-items`.
-  `first_run_resolves_user_id_and_writes_mentions` function L180-225 — `()` — - Empty result writes nothing and reports `no-new-items`.
-  `second_run_uses_cached_user_id_and_dedupes_overlap` function L228-285 — `()` — - Empty result writes nothing and reports `no-new-items`.
-  `empty_run_is_a_no_op` function L288-318 — `()` — - Empty result writes nothing and reports `no-new-items`.
-  `returns_auth_when_slack_not_connected` function L321-354 — `()` — - Empty result writes nothing and reports `no-new-items`.
-  `NoSlack` struct L322 — `-` — - Empty result writes nothing and reports `no-new-items`.
-  `NoSlack` type L323-339 — `impl FeedClients for NoSlack` — - Empty result writes nothing and reports `no-new-items`.
-  `slack` function L324-326 — `(&self) -> Option<Arc<dyn SlackFeedClient>>` — - Empty result writes nothing and reports `no-new-items`.
-  `calendar` function L327-329 — `(&self) -> Option<Arc<dyn CalendarFeedClient>>` — - Empty result writes nothing and reports `no-new-items`.
-  `gmail` function L330-332 — `(&self) -> Option<Arc<dyn GmailFeedClient>>` — - Empty result writes nothing and reports `no-new-items`.
-  `drive` function L333-335 — `(&self) -> Option<Arc<dyn DriveFeedClient>>` — - Empty result writes nothing and reports `no-new-items`.
-  `atlassian` function L336-338 — `(&self) -> Option<Arc<dyn AtlassianFeedClient>>` — - Empty result writes nothing and reports `no-new-items`.

### crates/arawn-integrations/src/atlassian

> *Semantic summary to be generated by AI agent.*

#### crates/arawn-integrations/src/atlassian/adf.rs

- pub `md_to_adf` function L29-52 — `(md: &str) -> Value` — Convert markdown to an ADF document.
-  `AdfBuilder` struct L55-66 — `{ marks: Vec<Value>, inline: Vec<Value>, current_block: BlockKind, list_items: V...` — empty paragraph — which Jira accepts).
-  `BlockKind` enum L69-84 — `None | Paragraph | Heading | BulletList | OrderedList | ListItem | BlockQuote | ...` — empty paragraph — which Jira accepts).
-  `AdfBuilder` type L86-305 — `= AdfBuilder` — empty paragraph — which Jira accepts).
-  `process` function L87-139 — `(&mut self, event: Event<'_>, blocks: &mut Vec<Value>)` — empty paragraph — which Jira accepts).
-  `start_paragraph` function L143-150 — `(&mut self)` — empty paragraph — which Jira accepts).
-  `end_paragraph` function L152-163 — `(&mut self, blocks: &mut Vec<Value>)` — empty paragraph — which Jira accepts).
-  `start_heading` function L165-176 — `(&mut self, level: HeadingLevel)` — empty paragraph — which Jira accepts).
-  `end_heading` function L178-190 — `(&mut self, blocks: &mut Vec<Value>)` — empty paragraph — which Jira accepts).
-  `start_list` function L192-199 — `(&mut self, start_num: Option<u64>)` — empty paragraph — which Jira accepts).
-  `end_list` function L201-209 — `(&mut self, blocks: &mut Vec<Value>)` — empty paragraph — which Jira accepts).
-  `start_item` function L211-215 — `(&mut self)` — empty paragraph — which Jira accepts).
-  `end_item` function L217-226 — `(&mut self)` — empty paragraph — which Jira accepts).
-  `start_block_quote` function L228-231 — `(&mut self)` — empty paragraph — which Jira accepts).
-  `end_block_quote` function L233-238 — `(&mut self, blocks: &mut Vec<Value>)` — empty paragraph — which Jira accepts).
-  `start_code_block` function L240-245 — `(&mut self, language: Option<String>)` — empty paragraph — which Jira accepts).
-  `end_code_block` function L247-259 — `(&mut self, blocks: &mut Vec<Value>)` — empty paragraph — which Jira accepts).
-  `flush_pending` function L261-266 — `(&mut self, blocks: &mut Vec<Value>)` — empty paragraph — which Jira accepts).
-  `push_text` function L270-276 — `(&mut self, text: &str)` — empty paragraph — which Jira accepts).
-  `push_inline_code` function L278-284 — `(&mut self, text: &str)` — empty paragraph — which Jira accepts).
-  `push_hard_break` function L286-288 — `(&mut self)` — empty paragraph — which Jira accepts).
-  `text_node` function L290-296 — `(&self, text: &str) -> Value` — empty paragraph — which Jira accepts).
-  `push_mark` function L298-300 — `(&mut self, mark: Value)` — empty paragraph — which Jira accepts).
-  `pop_mark` function L302-304 — `(&mut self)` — empty paragraph — which Jira accepts).
-  `tests` module L308-387 — `-` — empty paragraph — which Jira accepts).
-  `empty_input_produces_doc_with_empty_paragraph` function L312-317 — `()` — empty paragraph — which Jira accepts).
-  `plain_paragraph` function L320-324 — `()` — empty paragraph — which Jira accepts).
-  `bold_and_italic` function L327-335 — `()` — empty paragraph — which Jira accepts).
-  `inline_code` function L338-343 — `()` — empty paragraph — which Jira accepts).
-  `heading_levels` function L346-352 — `()` — empty paragraph — which Jira accepts).
-  `bullet_list` function L355-362 — `()` — empty paragraph — which Jira accepts).
-  `ordered_list` function L365-368 — `()` — empty paragraph — which Jira accepts).
-  `fenced_code_block_with_language` function L371-377 — `()` — empty paragraph — which Jira accepts).
-  `link_marks` function L380-386 — `()` — empty paragraph — which Jira accepts).

#### crates/arawn-integrations/src/atlassian/client.rs

- pub `AtlassianClient` struct L28-31 — `{ integration: Arc<AtlassianIntegration>, http: Client }` — Refresh-aware Atlassian HTTP client.
- pub `new` function L34-39 — `(integration: Arc<AtlassianIntegration>) -> Self` — when needed, persisting the new token through the integration.
- pub `jira_config` function L92-101 — `(&self, site: Option<&str>) -> Result<JiraConfig, IntegrationError>` — Build a `jira_v3_openapi::Configuration` for the selected site,
- pub `confluence_get` function L104-113 — `( &self, path: &str, site: Option<&str>, query: &[(&str, String)], ) -> Result<T...` — GET a JSON-bodied resource from Confluence.
- pub `confluence_post` function L116-125 — `( &self, path: &str, site: Option<&str>, body: &B, ) -> Result<T, IntegrationErr...` — POST a JSON body to Confluence.
- pub `confluence_put` function L128-137 — `( &self, path: &str, site: Option<&str>, body: &B, ) -> Result<T, IntegrationErr...` — PUT a JSON body to Confluence (used by page update).
- pub `confluence_v1_get` function L141-150 — `( &self, path: &str, site: Option<&str>, query: &[(&str, String)], ) -> Result<T...` — GET against the legacy Confluence v1 API.
-  `AtlassianClient` type L33-204 — `= AtlassianClient` — when needed, persisting the new token through the integration.
-  `product_base` function L47-64 — `( &self, product: Product, site: Option<&str>, ) -> Result<(AtlassianSite, Strin...` — Resolve the target site (defaulting to the first one) and return
-  `fresh_access_token` function L67-86 — `(&self) -> Result<String, IntegrationError>` — Get a fresh access token.
-  `send_json` function L152-183 — `( &self, method: Method, url: &str, query: &[(&str, String)], body: Option<&B>, ...` — when needed, persisting the new token through the integration.
-  `send` function L185-203 — `( &self, method: Method, url: &str, query: &[(&str, String)], body: Option<&B>, ...` — when needed, persisting the new token through the integration.
-  `Product` enum L207-213 — `Confluence | ConfluenceV1` — when needed, persisting the new token through the integration.
-  `is_expired` function L215-221 — `(token: &Token) -> bool` — when needed, persisting the new token through the integration.
-  `merge_prior_extras` function L230-240 — `( new_token: &mut Token, prior_extras: &serde_json::Map<String, serde_json::Valu...` — Carry the prior token's extras into the refreshed token.
-  `tests` module L243-309 — `-` — when needed, persisting the new token through the integration.
-  `token_with_extras` function L247-256 — `(extras: serde_json::Map<String, serde_json::Value>) -> Token` — when needed, persisting the new token through the integration.
-  `refresh_preserves_sites_when_new_token_extras_empty` function L259-274 — `()` — when needed, persisting the new token through the integration.
-  `refresh_doesnt_overwrite_extras_the_provider_set` function L277-296 — `()` — when needed, persisting the new token through the integration.
-  `refresh_with_empty_prior_extras_is_no_op` function L299-308 — `()` — when needed, persisting the new token through the integration.

#### crates/arawn-integrations/src/atlassian/confluence.rs

- pub `ConfluenceSearchTool` struct L373-376 — `{ integration: Arc<AtlassianIntegration>, description: String }` — Confluence tools — search, get page, create, update, list spaces.
- pub `new` function L379-387 — `(integration: Arc<AtlassianIntegration>) -> Self` — Confluence tools — search, get page, create, update, list spaces.
- pub `ConfluenceGetPageTool` struct L473-476 — `{ integration: Arc<AtlassianIntegration>, description: String }` — Confluence tools — search, get page, create, update, list spaces.
- pub `new` function L479-487 — `(integration: Arc<AtlassianIntegration>) -> Self` — Confluence tools — search, get page, create, update, list spaces.
- pub `ConfluenceCreatePageTool` struct L583-586 — `{ integration: Arc<AtlassianIntegration>, description: String }` — Confluence tools — search, get page, create, update, list spaces.
- pub `new` function L589-597 — `(integration: Arc<AtlassianIntegration>) -> Self` — Confluence tools — search, get page, create, update, list spaces.
- pub `ConfluenceUpdatePageTool` struct L694-697 — `{ integration: Arc<AtlassianIntegration>, description: String }` — Confluence tools — search, get page, create, update, list spaces.
- pub `new` function L700-708 — `(integration: Arc<AtlassianIntegration>) -> Self` — Confluence tools — search, get page, create, update, list spaces.
- pub `ConfluenceListSpacesTool` struct L789-792 — `{ integration: Arc<AtlassianIntegration>, description: String }` — Confluence tools — search, get page, create, update, list spaces.
- pub `new` function L795-803 — `(integration: Arc<AtlassianIntegration>) -> Self` — Confluence tools — search, get page, create, update, list spaces.
-  `integ_err` function L14-16 — `(e: crate::IntegrationError) -> ToolError` — Confluence tools — search, get page, create, update, list spaces.
-  `check_scopes` function L18-37 — `( integration: &AtlassianIntegration, required: &[&str], ) -> Result<(), ToolErr...` — Confluence tools — search, get page, create, update, list spaces.
-  `site_param` function L39-41 — `(params: &Value) -> Option<&str>` — Confluence tools — search, get page, create, update, list spaces.
-  `markdown_to_storage` function L54-121 — `(md: &str) -> String` — Wrap a markdown body into a Confluence storage-format string.
-  `inline_md_to_storage` function L125-128 — `(s: &str) -> String` — Apply inline markdown (bold/italic/code) to a text fragment, escaping
-  `apply_inline` function L130-173 — `(s: &str) -> String` — Confluence tools — search, get page, create, update, list spaces.
-  `take_until` function L175-196 — `( chars: &mut std::iter::Peekable<std::str::Chars>, delim: &str, ) -> (String, b...` — Confluence tools — search, get page, create, update, list spaces.
-  `xml_escape` function L198-211 — `(s: &str) -> String` — Confluence tools — search, get page, create, update, list spaces.
-  `storage_to_markdown` function L215-262 — `(storage: &str) -> String` — Strip Confluence storage-format tags into rough markdown.
-  `SearchResp` struct L267-270 — `{ results: Vec<RawSearchResult> }` — Confluence tools — search, get page, create, update, list spaces.
-  `RawSearchResult` struct L273-279 — `{ title: Option<String>, links: serde_json::Map<String, Value>, content: Option<...` — Confluence tools — search, get page, create, update, list spaces.
-  `RawContentRef` struct L282-287 — `{ id: String, kind: Option<String>, space: Option<RawSpaceRef> }` — Confluence tools — search, get page, create, update, list spaces.
-  `RawSpaceRef` struct L290-292 — `{ key: Option<String> }` — Confluence tools — search, get page, create, update, list spaces.
-  `SearchHit` struct L295-301 — `{ id: Option<String>, title: Option<String>, kind: Option<String>, space_key: Op...` — Confluence tools — search, get page, create, update, list spaces.
-  `PageDetailRaw` struct L308-317 — `{ id: String, title: Option<String>, space_id: Option<String>, body: Option<RawB...` — Confluence tools — search, get page, create, update, list spaces.
-  `RawBody` struct L320-322 — `{ storage: Option<RawBodyContent> }` — Confluence tools — search, get page, create, update, list spaces.
-  `RawBodyContent` struct L325-327 — `{ value: Option<String> }` — Confluence tools — search, get page, create, update, list spaces.
-  `RawVersion` struct L330-332 — `{ number: Option<u64> }` — Confluence tools — search, get page, create, update, list spaces.
-  `PageSummary` struct L335-345 — `{ id: String, title: Option<String>, kind: Option<String>, space_key: Option<Str...` — Confluence tools — search, get page, create, update, list spaces.
-  `SpacesResp` struct L351-354 — `{ results: Vec<RawSpace> }` — Confluence tools — search, get page, create, update, list spaces.
-  `RawSpace` struct L357-363 — `{ id: String, key: String, name: Option<String>, kind: Option<String> }` — Confluence tools — search, get page, create, update, list spaces.
-  `CQL_SEARCH_BASE` variable L367-370 — `: &str` — Confluence tools — search, get page, create, update, list spaces.
-  `CQL_SEARCH_SCOPES` variable L371 — `: &[&str]` — Confluence tools — search, get page, create, update, list spaces.
-  `ConfluenceSearchTool` type L378-388 — `= ConfluenceSearchTool` — Confluence tools — search, get page, create, update, list spaces.
-  `ConfluenceSearchTool` type L391-463 — `impl Tool for ConfluenceSearchTool` — Confluence tools — search, get page, create, update, list spaces.
-  `name` function L392-394 — `(&self) -> &str` — Confluence tools — search, get page, create, update, list spaces.
-  `description` function L395-397 — `(&self) -> &str` — Confluence tools — search, get page, create, update, list spaces.
-  `category` function L398-400 — `(&self) -> ToolCategory` — Confluence tools — search, get page, create, update, list spaces.
-  `permission_category` function L401-403 — `(&self) -> PermissionCategory` — Confluence tools — search, get page, create, update, list spaces.
-  `parameters_schema` function L404-414 — `(&self) -> Value` — Confluence tools — search, get page, create, update, list spaces.
-  `execute` function L415-462 — `(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError>` — Confluence tools — search, get page, create, update, list spaces.
-  `CONFLUENCE_GET_PAGE_BASE` variable L467-470 — `: &str` — Confluence tools — search, get page, create, update, list spaces.
-  `CONFLUENCE_GET_PAGE_SCOPES` variable L471 — `: &[&str]` — Confluence tools — search, get page, create, update, list spaces.
-  `ConfluenceGetPageTool` type L478-488 — `= ConfluenceGetPageTool` — Confluence tools — search, get page, create, update, list spaces.
-  `ConfluenceGetPageTool` type L491-573 — `impl Tool for ConfluenceGetPageTool` — Confluence tools — search, get page, create, update, list spaces.
-  `name` function L492-494 — `(&self) -> &str` — Confluence tools — search, get page, create, update, list spaces.
-  `description` function L495-497 — `(&self) -> &str` — Confluence tools — search, get page, create, update, list spaces.
-  `category` function L498-500 — `(&self) -> ToolCategory` — Confluence tools — search, get page, create, update, list spaces.
-  `permission_category` function L501-503 — `(&self) -> PermissionCategory` — Confluence tools — search, get page, create, update, list spaces.
-  `parameters_schema` function L504-514 — `(&self) -> Value` — Confluence tools — search, get page, create, update, list spaces.
-  `execute` function L515-572 — `(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError>` — Confluence tools — search, get page, create, update, list spaces.
-  `CONFLUENCE_CREATE_PAGE_BASE` variable L577-580 — `: &str` — Confluence tools — search, get page, create, update, list spaces.
-  `CONFLUENCE_CREATE_PAGE_SCOPES` variable L581 — `: &[&str]` — Confluence tools — search, get page, create, update, list spaces.
-  `ConfluenceCreatePageTool` type L588-598 — `= ConfluenceCreatePageTool` — Confluence tools — search, get page, create, update, list spaces.
-  `ConfluenceCreatePageTool` type L601-683 — `impl Tool for ConfluenceCreatePageTool` — Confluence tools — search, get page, create, update, list spaces.
-  `name` function L602-604 — `(&self) -> &str` — Confluence tools — search, get page, create, update, list spaces.
-  `description` function L605-607 — `(&self) -> &str` — Confluence tools — search, get page, create, update, list spaces.
-  `category` function L608-610 — `(&self) -> ToolCategory` — Confluence tools — search, get page, create, update, list spaces.
-  `permission_category` function L611-613 — `(&self) -> PermissionCategory` — Confluence tools — search, get page, create, update, list spaces.
-  `parameters_schema` function L614-626 — `(&self) -> Value` — Confluence tools — search, get page, create, update, list spaces.
-  `execute` function L627-682 — `(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError>` — Confluence tools — search, get page, create, update, list spaces.
-  `CONFLUENCE_UPDATE_PAGE_BASE` variable L687-691 — `: &str` — Confluence tools — search, get page, create, update, list spaces.
-  `CONFLUENCE_UPDATE_PAGE_SCOPES` variable L692 — `: &[&str]` — Confluence tools — search, get page, create, update, list spaces.
-  `ConfluenceUpdatePageTool` type L699-709 — `= ConfluenceUpdatePageTool` — Confluence tools — search, get page, create, update, list spaces.
-  `ConfluenceUpdatePageTool` type L712-780 — `impl Tool for ConfluenceUpdatePageTool` — Confluence tools — search, get page, create, update, list spaces.
-  `name` function L713-715 — `(&self) -> &str` — Confluence tools — search, get page, create, update, list spaces.
-  `description` function L716-718 — `(&self) -> &str` — Confluence tools — search, get page, create, update, list spaces.
-  `category` function L719-721 — `(&self) -> ToolCategory` — Confluence tools — search, get page, create, update, list spaces.
-  `permission_category` function L722-724 — `(&self) -> PermissionCategory` — Confluence tools — search, get page, create, update, list spaces.
-  `parameters_schema` function L725-736 — `(&self) -> Value` — Confluence tools — search, get page, create, update, list spaces.
-  `execute` function L737-779 — `(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError>` — Confluence tools — search, get page, create, update, list spaces.
-  `CONFLUENCE_LIST_SPACES_BASE` variable L784-786 — `: &str` — Confluence tools — search, get page, create, update, list spaces.
-  `CONFLUENCE_LIST_SPACES_SCOPES` variable L787 — `: &[&str]` — Confluence tools — search, get page, create, update, list spaces.
-  `ConfluenceListSpacesTool` type L794-804 — `= ConfluenceListSpacesTool` — Confluence tools — search, get page, create, update, list spaces.
-  `SpaceSummary` struct L807-812 — `{ id: String, key: String, name: Option<String>, kind: Option<String> }` — Confluence tools — search, get page, create, update, list spaces.
-  `ConfluenceListSpacesTool` type L815-858 — `impl Tool for ConfluenceListSpacesTool` — Confluence tools — search, get page, create, update, list spaces.
-  `name` function L816-818 — `(&self) -> &str` — Confluence tools — search, get page, create, update, list spaces.
-  `description` function L819-821 — `(&self) -> &str` — Confluence tools — search, get page, create, update, list spaces.
-  `category` function L822-824 — `(&self) -> ToolCategory` — Confluence tools — search, get page, create, update, list spaces.
-  `permission_category` function L825-827 — `(&self) -> PermissionCategory` — Confluence tools — search, get page, create, update, list spaces.
-  `parameters_schema` function L828-835 — `(&self) -> Value` — Confluence tools — search, get page, create, update, list spaces.
-  `execute` function L836-857 — `(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError>` — Confluence tools — search, get page, create, update, list spaces.
-  `tests` module L861-916 — `-` — Confluence tools — search, get page, create, update, list spaces.
-  `markdown_paragraphs_become_p_tags` function L865-870 — `()` — Confluence tools — search, get page, create, update, list spaces.
-  `markdown_headers_become_hN_tags` function L873-878 — `()` — Confluence tools — search, get page, create, update, list spaces.
-  `markdown_lists_round_through_ul` function L881-884 — `()` — Confluence tools — search, get page, create, update, list spaces.
-  `markdown_inline_emphasis` function L887-892 — `()` — Confluence tools — search, get page, create, update, list spaces.
-  `markdown_code_block_uses_confluence_macro` function L895-899 — `()` — Confluence tools — search, get page, create, update, list spaces.
-  `xml_escape_handles_lt_gt_amp` function L902-907 — `()` — Confluence tools — search, get page, create, update, list spaces.
-  `storage_to_markdown_strips_basic_tags` function L910-915 — `()` — Confluence tools — search, get page, create, update, list spaces.

#### crates/arawn-integrations/src/atlassian/integration.rs

- pub `SERVICE_NAME` variable L15 — `: &str` — Stable service name.
- pub `DEFAULT_ATLASSIAN_REDIRECT_PORT` variable L19 — `: u16` — Default fixed port for the OAuth callback.
- pub `ATLASSIAN_OAUTH_SCOPES` variable L29-47 — `: &[&str]` — Bot scopes requested at OAuth time.
- pub `AtlassianSite` struct L54-60 — `{ id: String, url: String, name: String, scopes: Vec<String> }` — One Atlassian site (workspace) the user authorized arawn to access.
- pub `AtlassianProviderConfig` struct L63-68 — `{ auth_url: Url, token_url: Url, scopes: Vec<String>, redirect_port: u16 }` — Default Atlassian OAuth provider config.
- pub `into_oauth_provider` function L82-98 — `( self, client_id: String, client_secret: String, ) -> OAuthProviderConfig`
- pub `AtlassianIntegration` struct L102-107 — `{ data_dir: PathBuf, client_id: String, client_secret: String, provider_config: ...` — Atlassian integration.
- pub `new` function L110-117 — `(data_dir: PathBuf, client_id: String, client_secret: String) -> Self`
- pub `with_provider_config` function L119-122 — `(mut self, config: AtlassianProviderConfig) -> Self`
- pub `load_token` function L125-130 — `(&self) -> Result<Token, IntegrationError>` — Load the persisted token.
- pub `save_token` function L133-137 — `(&self, token: &Token) -> Result<(), IntegrationError>` — Persist the (potentially-refreshed) token back to disk.
- pub `sites` function L142-151 — `(&self) -> Result<Vec<AtlassianSite>, IntegrationError>` — Read the persisted set of accessible Atlassian sites (cloud_ids
- pub `select_site` function L155-186 — `( &self, which: Option<&str>, ) -> Result<AtlassianSite, IntegrationError>` — Resolve a site by URL or name (e.g.
- pub `granted_scopes` function L189-199 — `( &self, ) -> Result<std::collections::HashSet<String>, IntegrationError>` — Read the granted scope set from the persisted token.
- pub `missing_scopes` function L210-224 — `(&self) -> Option<Vec<String>>` — Compare the persisted token's scopes against what the current
- pub `oauth_config` function L226-231 — `(&self) -> OAuthProviderConfig`
-  `AtlassianProviderConfig` type L70-79 — `impl Default for AtlassianProviderConfig`
-  `default` function L71-78 — `() -> Self`
-  `AtlassianProviderConfig` type L81-99 — `= AtlassianProviderConfig`
-  `AtlassianIntegration` type L109-248 — `= AtlassianIntegration`
-  `provider` function L233-243 — `(&self) -> AtlassianProviderConfig`
-  `token_store` function L245-247 — `(&self) -> Result<TokenStore, IntegrationError>`
-  `AtlassianIntegration` type L251-366 — `impl Integration for AtlassianIntegration`
-  `name` function L252-254 — `(&self) -> &str`
-  `is_connected` function L256-261 — `(&self) -> bool`
-  `connect` function L263-329 — `(&self, ctx: &dyn ConnectContext) -> Result<(), IntegrationError>`
-  `disconnect` function L331-335 — `(&self) -> Result<(), IntegrationError>`
-  `capabilities_summary` function L337-365 — `(&self) -> Option<String>`
-  `RawAccessibleResource` struct L371-377 — `{ id: String, url: String, name: String, scopes: Vec<String> }` — Atlassian's accessible-resources response shape (snake-case-d to
-  `retry_accessible_resources` function L387-415 — `( access: &str, attempts: u32, ) -> Result<Vec<AtlassianSite>, IntegrationError>` — Hit `https://api.atlassian.com/oauth/token/accessible-resources` to
-  `fetch_accessible_resources` function L417-447 — `( access_token: &str, ) -> Result<Vec<AtlassianSite>, IntegrationError>`
-  `tests` module L450-492 — `-`
-  `default_provider_carries_jira_classic_and_confluence_v2_scopes` function L454-478 — `()`
-  `provider_lifts_into_oauth_config_with_audience` function L481-491 — `()`

#### crates/arawn-integrations/src/atlassian/jira.rs

- pub `JiraSearchTool` struct L196-199 — `{ integration: Arc<AtlassianIntegration>, description: String }` — follows API moves with each `cargo update`.
- pub `new` function L202-210 — `(integration: Arc<AtlassianIntegration>) -> Self` — follows API moves with each `cargo update`.
- pub `JiraGetIssueTool` struct L303-306 — `{ integration: Arc<AtlassianIntegration>, description: String }` — follows API moves with each `cargo update`.
- pub `new` function L309-317 — `(integration: Arc<AtlassianIntegration>) -> Self` — follows API moves with each `cargo update`.
- pub `JiraCreateIssueTool` struct L474-477 — `{ integration: Arc<AtlassianIntegration>, description: String }` — follows API moves with each `cargo update`.
- pub `new` function L480-488 — `(integration: Arc<AtlassianIntegration>) -> Self` — follows API moves with each `cargo update`.
- pub `JiraUpdateIssueTool` struct L575-578 — `{ integration: Arc<AtlassianIntegration>, description: String }` — follows API moves with each `cargo update`.
- pub `new` function L581-589 — `(integration: Arc<AtlassianIntegration>) -> Self` — follows API moves with each `cargo update`.
- pub `JiraAddCommentTool` struct L680-683 — `{ integration: Arc<AtlassianIntegration>, description: String }` — follows API moves with each `cargo update`.
- pub `new` function L686-694 — `(integration: Arc<AtlassianIntegration>) -> Self` — follows API moves with each `cargo update`.
- pub `JiraTransitionIssueTool` struct L759-762 — `{ integration: Arc<AtlassianIntegration>, description: String }` — follows API moves with each `cargo update`.
- pub `new` function L765-773 — `(integration: Arc<AtlassianIntegration>) -> Self` — follows API moves with each `cargo update`.
-  `integ_err` function L26-28 — `(e: crate::IntegrationError) -> ToolError` — follows API moves with each `cargo update`.
-  `check_scopes` function L30-52 — `( integration: &AtlassianIntegration, required: &[&str], ) -> Result<(), ToolErr...` — follows API moves with each `cargo update`.
-  `site_param` function L54-56 — `(params: &Value) -> Option<&str>` — follows API moves with each `cargo update`.
-  `openapi_err` function L61-68 — `(e: jira_v3_openapi::apis::Error<E>) -> ToolError` — Map an `openapi::Error<E>` (from the generated client) into our common
-  `tolerate_empty_body` function L74-84 — `( e: jira_v3_openapi::apis::Error<E>, ) -> Result<(), ToolError>` — Some Jira write endpoints (transitions, edit-without-return) respond
-  `fields_map` function L89-95 — `(issue: &IssueBean) -> Map<String, Value>` — follows API moves with each `cargo update`.
-  `IssueSummary` struct L100-109 — `{ key: String, summary: Option<String>, status: Option<String>, issue_type: Opti...` — follows API moves with each `cargo update`.
-  `summarize_issue` function L111-142 — `(key: &str, fields: &Map<String, Value>) -> IssueSummary` — follows API moves with each `cargo update`.
-  `IssueDetail` struct L145-158 — `{ key: String, summary: Option<String>, status: Option<String>, issue_type: Opti...` — follows API moves with each `cargo update`.
-  `CommentSummary` struct L161-166 — `{ id: String, author: Option<String>, body: Option<String>, created: Option<Stri...` — follows API moves with each `cargo update`.
-  `TransitionSummary` struct L169-174 — `{ id: String, name: String, to: Option<String> }` — follows API moves with each `cargo update`.
-  `adf_from_markdown` function L179-181 — `(text: &str) -> Value` — follows API moves with each `cargo update`.
-  `JIRA_SEARCH_BASE` variable L185-193 — `: &str` — follows API moves with each `cargo update`.
-  `JIRA_SEARCH_SCOPES` variable L194 — `: &[&str]` — follows API moves with each `cargo update`.
-  `JiraSearchTool` type L201-211 — `= JiraSearchTool` — follows API moves with each `cargo update`.
-  `JiraSearchTool` type L214-294 — `impl Tool for JiraSearchTool` — follows API moves with each `cargo update`.
-  `name` function L215-217 — `(&self) -> &str` — follows API moves with each `cargo update`.
-  `description` function L218-220 — `(&self) -> &str` — follows API moves with each `cargo update`.
-  `category` function L221-223 — `(&self) -> ToolCategory` — follows API moves with each `cargo update`.
-  `permission_category` function L224-226 — `(&self) -> PermissionCategory` — follows API moves with each `cargo update`.
-  `parameters_schema` function L227-242 — `(&self) -> Value` — follows API moves with each `cargo update`.
-  `execute` function L243-293 — `(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError>` — follows API moves with each `cargo update`.
-  `JIRA_GET_ISSUE_BASE` variable L298-300 — `: &str` — follows API moves with each `cargo update`.
-  `JIRA_GET_ISSUE_SCOPES` variable L301 — `: &[&str]` — follows API moves with each `cargo update`.
-  `JiraGetIssueTool` type L308-318 — `= JiraGetIssueTool` — follows API moves with each `cargo update`.
-  `JiraGetIssueTool` type L321-463 — `impl Tool for JiraGetIssueTool` — follows API moves with each `cargo update`.
-  `name` function L322-324 — `(&self) -> &str` — follows API moves with each `cargo update`.
-  `description` function L325-327 — `(&self) -> &str` — follows API moves with each `cargo update`.
-  `category` function L328-330 — `(&self) -> ToolCategory` — follows API moves with each `cargo update`.
-  `permission_category` function L331-333 — `(&self) -> PermissionCategory` — follows API moves with each `cargo update`.
-  `parameters_schema` function L334-343 — `(&self) -> Value` — follows API moves with each `cargo update`.
-  `execute` function L344-462 — `(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError>` — follows API moves with each `cargo update`.
-  `JIRA_CREATE_ISSUE_BASE` variable L467-471 — `: &str` — follows API moves with each `cargo update`.
-  `JIRA_CREATE_ISSUE_SCOPES` variable L472 — `: &[&str]` — follows API moves with each `cargo update`.
-  `JiraCreateIssueTool` type L479-489 — `= JiraCreateIssueTool` — follows API moves with each `cargo update`.
-  `JiraCreateIssueTool` type L492-562 — `impl Tool for JiraCreateIssueTool` — follows API moves with each `cargo update`.
-  `name` function L493-495 — `(&self) -> &str` — follows API moves with each `cargo update`.
-  `description` function L496-498 — `(&self) -> &str` — follows API moves with each `cargo update`.
-  `category` function L499-501 — `(&self) -> ToolCategory` — follows API moves with each `cargo update`.
-  `permission_category` function L502-504 — `(&self) -> PermissionCategory` — follows API moves with each `cargo update`.
-  `parameters_schema` function L505-517 — `(&self) -> Value` — follows API moves with each `cargo update`.
-  `execute` function L518-561 — `(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError>` — follows API moves with each `cargo update`.
-  `JIRA_UPDATE_ISSUE_BASE` variable L566-572 — `: &str` — follows API moves with each `cargo update`.
-  `JIRA_UPDATE_ISSUE_SCOPES` variable L573 — `: &[&str]` — follows API moves with each `cargo update`.
-  `JiraUpdateIssueTool` type L580-590 — `= JiraUpdateIssueTool` — follows API moves with each `cargo update`.
-  `JiraUpdateIssueTool` type L593-671 — `impl Tool for JiraUpdateIssueTool` — follows API moves with each `cargo update`.
-  `name` function L594-596 — `(&self) -> &str` — follows API moves with each `cargo update`.
-  `description` function L597-599 — `(&self) -> &str` — follows API moves with each `cargo update`.
-  `category` function L600-602 — `(&self) -> ToolCategory` — follows API moves with each `cargo update`.
-  `permission_category` function L603-605 — `(&self) -> PermissionCategory` — follows API moves with each `cargo update`.
-  `parameters_schema` function L606-616 — `(&self) -> Value` — follows API moves with each `cargo update`.
-  `execute` function L617-670 — `(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError>` — follows API moves with each `cargo update`.
-  `JIRA_ADD_COMMENT_BASE` variable L675-677 — `: &str` — follows API moves with each `cargo update`.
-  `JIRA_ADD_COMMENT_SCOPES` variable L678 — `: &[&str]` — follows API moves with each `cargo update`.
-  `JiraAddCommentTool` type L685-695 — `= JiraAddCommentTool` — follows API moves with each `cargo update`.
-  `JiraAddCommentTool` type L698-749 — `impl Tool for JiraAddCommentTool` — follows API moves with each `cargo update`.
-  `name` function L699-701 — `(&self) -> &str` — follows API moves with each `cargo update`.
-  `description` function L702-704 — `(&self) -> &str` — follows API moves with each `cargo update`.
-  `category` function L705-707 — `(&self) -> ToolCategory` — follows API moves with each `cargo update`.
-  `permission_category` function L708-710 — `(&self) -> PermissionCategory` — follows API moves with each `cargo update`.
-  `parameters_schema` function L711-721 — `(&self) -> Value` — follows API moves with each `cargo update`.
-  `execute` function L722-748 — `(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError>` — follows API moves with each `cargo update`.
-  `JIRA_TRANSITION_ISSUE_BASE` variable L753-756 — `: &str` — follows API moves with each `cargo update`.
-  `JIRA_TRANSITION_ISSUE_SCOPES` variable L757 — `: &[&str]` — follows API moves with each `cargo update`.
-  `JiraTransitionIssueTool` type L764-774 — `= JiraTransitionIssueTool` — follows API moves with each `cargo update`.
-  `JiraTransitionIssueTool` type L777-860 — `impl Tool for JiraTransitionIssueTool` — follows API moves with each `cargo update`.
-  `name` function L778-780 — `(&self) -> &str` — follows API moves with each `cargo update`.
-  `description` function L781-783 — `(&self) -> &str` — follows API moves with each `cargo update`.
-  `category` function L784-786 — `(&self) -> ToolCategory` — follows API moves with each `cargo update`.
-  `permission_category` function L787-789 — `(&self) -> PermissionCategory` — follows API moves with each `cargo update`.
-  `parameters_schema` function L790-803 — `(&self) -> Value` — follows API moves with each `cargo update`.
-  `execute` function L804-859 — `(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError>` — follows API moves with each `cargo update`.
-  `tests` module L863-895 — `-` — follows API moves with each `cargo update`.
-  `summarize_issue_extracts_nested_fields` function L867-886 — `()` — follows API moves with each `cargo update`.
-  `summarize_issue_handles_missing_fields` function L889-894 — `()` — follows API moves with each `cargo update`.

#### crates/arawn-integrations/src/atlassian/mod.rs

-  `adf` module L20 — `-` — One OAuth dance, one client_id/secret, one persisted token; both tool
-  `client` module L21 — `-` — See `docs/src/integrations/atlassian.md` for setup.
-  `confluence` module L22 — `-` — See `docs/src/integrations/atlassian.md` for setup.
-  `integration` module L23 — `-` — See `docs/src/integrations/atlassian.md` for setup.
-  `jira` module L24 — `-` — See `docs/src/integrations/atlassian.md` for setup.

### crates/arawn-integrations/src/calendar

> *Semantic summary to be generated by AI agent.*

#### crates/arawn-integrations/src/calendar/client.rs

- pub `CalendarHub` type L13 — `= GoogleCalendarHub<HttpsConnector>` — Concrete CalendarHub the integration exposes.
- pub `client_from_token_store` function L18-28 — `( data_dir: std::path::PathBuf, oauth_config: OAuthProviderConfig, ) -> Result<C...` — Open the persisted Calendar token, build the hyper-util client + auth

#### crates/arawn-integrations/src/calendar/integration.rs

- pub `SERVICE_NAME` variable L16 — `: &str` — Stable service name.
- pub `CALENDAR_OAUTH_SCOPE` variable L19 — `: &str` — The OAuth scope Google Calendar reads/writes need.
- pub `GoogleCalendarProviderConfig` struct L22-26 — `{ auth_url: Url, token_url: Url, scopes: Vec<String> }` — Default Google Calendar OAuth provider config.
- pub `into_oauth_provider` function L39-48 — `(self, client_id: String, client_secret: String) -> OAuthProviderConfig`
- pub `GoogleCalendarIntegration` struct L52-57 — `{ data_dir: PathBuf, client_id: String, client_secret: String, provider_config: ...` — Google Calendar integration.
- pub `new` function L60-67 — `(data_dir: PathBuf, client_id: String, client_secret: String) -> Self`
- pub `with_provider_config` function L69-72 — `(mut self, config: GoogleCalendarProviderConfig) -> Self`
- pub `hub` function L76-78 — `(&self) -> Result<CalendarHub, IntegrationError>` — Build a fully-wired `CalendarHub` for tools.
-  `GoogleCalendarProviderConfig` type L28-36 — `impl Default for GoogleCalendarProviderConfig`
-  `default` function L29-35 — `() -> Self`
-  `GoogleCalendarProviderConfig` type L38-49 — `= GoogleCalendarProviderConfig`
-  `GoogleCalendarIntegration` type L59-96 — `= GoogleCalendarIntegration`
-  `oauth_config` function L80-91 — `(&self) -> OAuthProviderConfig`
-  `token_store` function L93-95 — `(&self) -> Result<TokenStore, IntegrationError>`
-  `GoogleCalendarIntegration` type L99-123 — `impl Integration for GoogleCalendarIntegration`
-  `name` function L100-102 — `(&self) -> &str`
-  `is_connected` function L104-109 — `(&self) -> bool`
-  `connect` function L111-116 — `(&self, ctx: &dyn ConnectContext) -> Result<(), IntegrationError>`
-  `disconnect` function L118-122 — `(&self) -> Result<(), IntegrationError>`
-  `tests` module L126-143 — `-`
-  `default_provider_has_calendar_events_scope` function L130-133 — `()`
-  `provider_lifts_into_oauth_config` function L136-142 — `()`

#### crates/arawn-integrations/src/calendar/mod.rs

-  `client` module L11 — `-` — - [`GoogleCalendarIntegration`] implements [`crate::Integration`].
-  `integration` module L12 — `-` — See `docs/src/integrations/calendar.md` for setup.
-  `tools` module L13 — `-` — See `docs/src/integrations/calendar.md` for setup.

#### crates/arawn-integrations/src/calendar/tools.rs

- pub `CalendarUpcomingTool` struct L75-77 — `{ integration: Arc<GoogleCalendarIntegration> }` — timezone math here, the model handles those concerns.
- pub `new` function L80-82 — `(integration: Arc<GoogleCalendarIntegration>) -> Self` — timezone math here, the model handles those concerns.
- pub `CalendarCreateEventTool` struct L157-159 — `{ integration: Arc<GoogleCalendarIntegration> }` — timezone math here, the model handles those concerns.
- pub `new` function L162-164 — `(integration: Arc<GoogleCalendarIntegration>) -> Self` — timezone math here, the model handles those concerns.
- pub `CalendarFindConflictsTool` struct L284-286 — `{ integration: Arc<GoogleCalendarIntegration> }` — timezone math here, the model handles those concerns.
- pub `new` function L289-291 — `(integration: Arc<GoogleCalendarIntegration>) -> Self` — timezone math here, the model handles those concerns.
-  `integ_err` function L20-22 — `(e: crate::IntegrationError) -> ToolError` — timezone math here, the model handles those concerns.
-  `google_err` function L24-26 — `(stage: &str, e: google_calendar3::Error) -> ToolError` — timezone math here, the model handles those concerns.
-  `EventSummary` struct L30-39 — `{ id: Option<String>, summary: Option<String>, description: Option<String>, loca...` — One row of the `calendar_upcoming` / `calendar_find_conflicts` response.
-  `summary_from_event` function L41-56 — `(e: &Event) -> EventSummary` — timezone math here, the model handles those concerns.
-  `format_event_datetime` function L60-65 — `(dt: &EventDateTime) -> Option<String>` — Render an `EventDateTime` as the most informative RFC3339-ish string we
-  `parse_rfc3339` function L67-71 — `(s: &str, field: &str) -> Result<DateTime<Utc>, ToolError>` — timezone math here, the model handles those concerns.
-  `CalendarUpcomingTool` type L79-83 — `= CalendarUpcomingTool` — timezone math here, the model handles those concerns.
-  `CalendarUpcomingTool` type L86-153 — `impl Tool for CalendarUpcomingTool` — timezone math here, the model handles those concerns.
-  `name` function L87-89 — `(&self) -> &str` — timezone math here, the model handles those concerns.
-  `description` function L90-94 — `(&self) -> &str` — timezone math here, the model handles those concerns.
-  `category` function L95-97 — `(&self) -> ToolCategory` — timezone math here, the model handles those concerns.
-  `permission_category` function L98-100 — `(&self) -> PermissionCategory` — timezone math here, the model handles those concerns.
-  `parameters_schema` function L101-117 — `(&self) -> Value` — timezone math here, the model handles those concerns.
-  `execute` function L118-152 — `(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError>` — timezone math here, the model handles those concerns.
-  `CalendarCreateEventTool` type L161-165 — `= CalendarCreateEventTool` — timezone math here, the model handles those concerns.
-  `CalendarCreateEventTool` type L168-280 — `impl Tool for CalendarCreateEventTool` — timezone math here, the model handles those concerns.
-  `name` function L169-171 — `(&self) -> &str` — timezone math here, the model handles those concerns.
-  `description` function L172-175 — `(&self) -> &str` — timezone math here, the model handles those concerns.
-  `category` function L176-178 — `(&self) -> ToolCategory` — timezone math here, the model handles those concerns.
-  `permission_category` function L179-183 — `(&self) -> PermissionCategory` — timezone math here, the model handles those concerns.
-  `parameters_schema` function L184-205 — `(&self) -> Value` — timezone math here, the model handles those concerns.
-  `execute` function L206-279 — `(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError>` — timezone math here, the model handles those concerns.
-  `CalendarFindConflictsTool` type L288-292 — `= CalendarFindConflictsTool` — timezone math here, the model handles those concerns.
-  `CalendarFindConflictsTool` type L295-384 — `impl Tool for CalendarFindConflictsTool` — timezone math here, the model handles those concerns.
-  `name` function L296-298 — `(&self) -> &str` — timezone math here, the model handles those concerns.
-  `description` function L299-302 — `(&self) -> &str` — timezone math here, the model handles those concerns.
-  `category` function L303-305 — `(&self) -> ToolCategory` — timezone math here, the model handles those concerns.
-  `permission_category` function L306-308 — `(&self) -> PermissionCategory` — timezone math here, the model handles those concerns.
-  `parameters_schema` function L309-322 — `(&self) -> Value` — timezone math here, the model handles those concerns.
-  `execute` function L323-383 — `(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError>` — timezone math here, the model handles those concerns.
-  `tests` module L387-436 — `-` — timezone math here, the model handles those concerns.
-  `format_event_datetime_prefers_datetime_over_date` function L392-400 — `()` — timezone math here, the model handles those concerns.
-  `format_event_datetime_falls_back_to_date_for_all_day` function L403-410 — `()` — timezone math here, the model handles those concerns.
-  `summary_from_event_extracts_attendee_emails` function L413-428 — `()` — timezone math here, the model handles those concerns.
-  `parse_rfc3339_accepts_offset_and_z` function L431-435 — `()` — timezone math here, the model handles those concerns.

### crates/arawn-integrations/src

> *Semantic summary to be generated by AI agent.*

#### crates/arawn-integrations/src/credential_store.rs

- pub `CredentialStore` struct L34-39 — `{ integrations_dir: PathBuf, service: String, cipher: ChaCha20Poly1305, _phantom...` — Encrypted blob store, keyed by `<data_dir>/integrations/<service>/<entry>.bin`.
- pub `open` function L45-76 — `(data_dir: &Path, service: &str) -> Result<Self, IntegrationError>` — Open or initialize the store rooted at `<data_dir>/integrations/<service>/`.
- pub `save` function L79-102 — `(&self, entry: &str, value: &T) -> Result<(), IntegrationError>` — Persist a serializable value under `entry`.
- pub `load` function L106-132 — `(&self, entry: &str) -> Result<Option<T>, IntegrationError>` — Load `entry`.
- pub `delete` function L135-142 — `(&self, entry: &str) -> Result<(), IntegrationError>` — Remove `entry` if present.
- pub `exists` function L145-147 — `(&self, entry: &str) -> bool` — True if this store has anything stored under `entry`.
- pub `service` function L150-152 — `(&self) -> &str` — Service name this store is bound to.
- pub `integrations_dir` function L155-157 — `(&self) -> &Path` — Path to the per-service directory.
-  `KEY_LEN` variable L24 — `: usize` — install bootstraps the same way regardless of which gets opened first.
-  `NONCE_LEN` variable L25 — `: usize` — install bootstraps the same way regardless of which gets opened first.
-  `KEY_FILENAME` variable L28 — `: &str` — Same filename TokenStore uses, same parent dir.
-  `KEY_PARENT` variable L29 — `: &str` — install bootstraps the same way regardless of which gets opened first.
-  `path_for` function L159-161 — `(&self, entry: &str) -> PathBuf` — install bootstraps the same way regardless of which gets opened first.
-  `safe_segment` function L165-175 — `(s: &str) -> String` — Refuse path-separator characters in user-supplied service / entry names.
-  `set_dir_mode` function L178-184 — `(path: &Path) -> Result<(), IntegrationError>` — install bootstraps the same way regardless of which gets opened first.
-  `set_dir_mode` function L187-189 — `(_path: &Path) -> Result<(), IntegrationError>` — install bootstraps the same way regardless of which gets opened first.
-  `set_file_mode` function L192-198 — `(path: &Path, mode: u32) -> Result<(), IntegrationError>` — install bootstraps the same way regardless of which gets opened first.
-  `set_file_mode` function L201-203 — `(_path: &Path, _mode: u32) -> Result<(), IntegrationError>` — install bootstraps the same way regardless of which gets opened first.
-  `write_key` function L205-211 — `(path: &Path, bytes: &[u8]) -> Result<(), IntegrationError>` — install bootstraps the same way regardless of which gets opened first.
-  `tests` module L214-312 — `-` — install bootstraps the same way regardless of which gets opened first.
-  `WebhookCred` struct L220-223 — `{ url: String, signing_secret: Option<String> }` — install bootstraps the same way regardless of which gets opened first.
-  `round_trip_returns_what_was_saved` function L226-237 — `()` — install bootstraps the same way regardless of which gets opened first.
-  `load_returns_none_when_absent` function L240-245 — `()` — install bootstraps the same way regardless of which gets opened first.
-  `delete_is_idempotent` function L248-264 — `()` — install bootstraps the same way regardless of which gets opened first.
-  `second_store_on_same_data_dir_uses_same_key` function L267-285 — `()` — install bootstraps the same way regardless of which gets opened first.
-  `path_segments_with_slashes_get_sanitized` function L288-297 — `()` — install bootstraps the same way regardless of which gets opened first.
-  `corrupted_blob_yields_format_error_not_panic` function L300-311 — `()` — install bootstraps the same way regardless of which gets opened first.

#### crates/arawn-integrations/src/error.rs

- pub `IntegrationError` enum L9-37 — `UnknownService | NotConnected | Auth | Io | Format | Provider | RateLimited | Ca...` — Errors surfaced by the integration layer.
- pub `user_message` function L41-59 — `(&self) -> String` — User-facing one-liner suitable for the engine error chain (T-0191).
-  `IntegrationError` type L39-60 — `= IntegrationError`

#### crates/arawn-integrations/src/google_common.rs

- pub `HttpsConnector` type L29 — `= hyper_rustls::HttpsConnector<HttpConnector>` — HTTPS connector flavour we wire all Google integrations against.
- pub `build_https_client` function L33-44 — `() -> Client<HttpsConnector>` — Build the shared hyper-util client every Google integration uses.
- pub `TokenStoreHandle` struct L50-53 — `{ data_dir: PathBuf, service_name: String }` — Per-service `arawn-auth::TokenStore` handle.
- pub `new` function L56-61 — `(data_dir: PathBuf, service_name: impl Into<String>) -> Self` — we hand it.
- pub `save_token` function L63-67 — `(&self, token: &Token) -> Result<(), IntegrationError>` — we hand it.
- pub `load_token` function L69-72 — `(&self) -> Result<Option<Token>, IntegrationError>` — we hand it.
- pub `ArawnGetToken` struct L81-83 — `{ inner: Arc<ArawnGetTokenInner> }` — `google_apis_common::GetToken` impl backed by `arawn-auth`.
- pub `new` function L92-100 — `(token: Token, oauth_config: OAuthProviderConfig, token_store: TokenStoreHandle)...` — we hand it.
-  `TokenStoreHandle` type L55-73 — `= TokenStoreHandle` — we hand it.
-  `ArawnGetTokenInner` struct L85-89 — `{ token: AsyncMutex<Token>, oauth: OAuthClient, token_store: TokenStoreHandle }` — we hand it.
-  `ArawnGetToken` type L91-101 — `= ArawnGetToken` — we hand it.
-  `ArawnGetToken` type L103-146 — `impl GetToken for ArawnGetToken` — we hand it.
-  `get_token` function L104-145 — `( &'a self, _scopes: &'a [&str], ) -> std::pin::Pin< Box< dyn std::future::Futur...` — we hand it.
-  `tests` module L149-178 — `-` — we hand it.
-  `unexpired_token_returned_directly_no_refresh` function L153-177 — `()` — we hand it.

#### crates/arawn-integrations/src/integration.rs

- pub `Integration` interface L20-62 — `{ fn name(), fn is_connected(), fn connect(), fn disconnect(), fn capabilities_s...` — Lifecycle contract every external integration implements.
- pub `ConnectContext` interface L71-82 — `{ fn service(), fn publish_auth_url(), fn publish_progress() }` — Hooks an `Integration::connect` impl needs from its caller (the server).
- pub `IntegrationStatus` struct L86-89 — `{ name: String, connected: bool }` — Snapshot of one integration's state, returned by `list_integrations` RPC.
-  `capabilities_summary` function L59-61 — `(&self) -> Option<String>` — One-line capability summary for the LLM system prompt.

#### crates/arawn-integrations/src/lib.rs

- pub `atlassian` module L23 — `-` — Provides three things to the rest of arawn:
- pub `calendar` module L24 — `-` — ChaCha20Poly1305 + per-data-dir master key that `TokenStore` uses.
- pub `credential_store` module L25 — `-` — ChaCha20Poly1305 + per-data-dir master key that `TokenStore` uses.
- pub `drive` module L26 — `-` — ChaCha20Poly1305 + per-data-dir master key that `TokenStore` uses.
- pub `error` module L27 — `-` — ChaCha20Poly1305 + per-data-dir master key that `TokenStore` uses.
- pub `gmail` module L28 — `-` — ChaCha20Poly1305 + per-data-dir master key that `TokenStore` uses.
- pub `google_common` module L29 — `-` — ChaCha20Poly1305 + per-data-dir master key that `TokenStore` uses.
- pub `integration` module L30 — `-` — ChaCha20Poly1305 + per-data-dir master key that `TokenStore` uses.
- pub `oauth_flow` module L31 — `-` — ChaCha20Poly1305 + per-data-dir master key that `TokenStore` uses.
- pub `retry_after` module L32 — `-` — ChaCha20Poly1305 + per-data-dir master key that `TokenStore` uses.
- pub `slack` module L33 — `-` — ChaCha20Poly1305 + per-data-dir master key that `TokenStore` uses.
- pub `install_default_crypto_provider` function L48-50 — `()` — Install rustls' `ring` crypto provider as the process default.

#### crates/arawn-integrations/src/oauth_flow.rs

- pub `OAuthOutcome` struct L23-25 — `{ token: Token }` — Result of a successful OAuth flow.
- pub `run_oauth_flow` function L30-74 — `( provider_config: OAuthProviderConfig, token_store: &TokenStore, service_name: ...` — Drive the OAuth dance end-to-end.
-  `tests` module L77-125 — `-` — 6.
-  `CaptureCtx` struct L84-88 — `{ service: String, auth_url: Mutex<Option<Url>>, progress: Mutex<Vec<String>> }` — Captures everything published; lets tests assert without a real TUI.
-  `CaptureCtx` type L91-101 — `impl ConnectContext for CaptureCtx` — 6.
-  `service` function L92-94 — `(&self) -> &str` — 6.
-  `publish_auth_url` function L95-97 — `(&self, url: &Url)` — 6.
-  `publish_progress` function L98-100 — `(&self, message: &str)` — 6.
-  `ctx_capture_smoke` function L104-124 — `()` — 6.

#### crates/arawn-integrations/src/retry_after.rs

- pub `parse_retry_after` function L18-20 — `(raw: Option<&str>) -> Option<Duration>` — Parse a `Retry-After` header value.
-  `parse_retry_after_at` function L22-37 — `(raw: Option<&str>, now: DateTime<Utc>) -> Option<Duration>` — re-exports it.
-  `tests` module L40-74 — `-` — re-exports it.
-  `at` function L43-45 — `(s: &str) -> DateTime<Utc>` — re-exports it.
-  `delta_seconds` function L48-52 — `()` — re-exports it.
-  `http_date_future` function L55-59 — `()` — re-exports it.
-  `http_date_past_clamps_to_zero` function L62-66 — `()` — re-exports it.
-  `missing_or_garbage` function L69-73 — `()` — re-exports it.

### crates/arawn-integrations/src/drive

> *Semantic summary to be generated by AI agent.*

#### crates/arawn-integrations/src/drive/client.rs

- pub `DriveHub` type L13 — `= GoogleDriveHub<HttpsConnector>` — Concrete DriveHub the integration exposes.
- pub `client_from_token_store` function L18-28 — `( data_dir: std::path::PathBuf, oauth_config: OAuthProviderConfig, ) -> Result<D...` — Open the persisted Drive token, build the hyper-util client + auth

#### crates/arawn-integrations/src/drive/integration.rs

- pub `SERVICE_NAME` variable L16 — `: &str` — Stable service name.
- pub `DRIVE_OAUTH_SCOPE` variable L23 — `: &str` — Full read+write scope.
- pub `GoogleDriveProviderConfig` struct L26-30 — `{ auth_url: Url, token_url: Url, scopes: Vec<String> }` — Default Google Drive OAuth provider config.
- pub `into_oauth_provider` function L43-52 — `(self, client_id: String, client_secret: String) -> OAuthProviderConfig`
- pub `GoogleDriveIntegration` struct L56-61 — `{ data_dir: PathBuf, client_id: String, client_secret: String, provider_config: ...` — Google Drive integration.
- pub `new` function L64-71 — `(data_dir: PathBuf, client_id: String, client_secret: String) -> Self`
- pub `with_provider_config` function L73-76 — `(mut self, config: GoogleDriveProviderConfig) -> Self`
- pub `hub` function L80-82 — `(&self) -> Result<DriveHub, IntegrationError>` — Build a fully-wired `DriveHub` for tools.
-  `GoogleDriveProviderConfig` type L32-40 — `impl Default for GoogleDriveProviderConfig`
-  `default` function L33-39 — `() -> Self`
-  `GoogleDriveProviderConfig` type L42-53 — `= GoogleDriveProviderConfig`
-  `GoogleDriveIntegration` type L63-100 — `= GoogleDriveIntegration`
-  `oauth_config` function L84-95 — `(&self) -> OAuthProviderConfig`
-  `token_store` function L97-99 — `(&self) -> Result<TokenStore, IntegrationError>`
-  `GoogleDriveIntegration` type L103-137 — `impl Integration for GoogleDriveIntegration`
-  `name` function L104-106 — `(&self) -> &str`
-  `is_connected` function L108-113 — `(&self) -> bool`
-  `connect` function L115-120 — `(&self, ctx: &dyn ConnectContext) -> Result<(), IntegrationError>`
-  `disconnect` function L122-126 — `(&self) -> Result<(), IntegrationError>`
-  `capabilities_summary` function L128-136 — `(&self) -> Option<String>`
-  `tests` module L140-157 — `-`
-  `default_provider_has_drive_scope` function L144-147 — `()`
-  `provider_lifts_into_oauth_config` function L150-156 — `()`

#### crates/arawn-integrations/src/drive/mod.rs

-  `client` module L12 — `-` — - [`GoogleDriveIntegration`] implements [`crate::Integration`].
-  `integration` module L13 — `-` — See `docs/src/integrations/drive.md` for setup.
-  `tools` module L14 — `-` — See `docs/src/integrations/drive.md` for setup.

#### crates/arawn-integrations/src/drive/tools.rs

- pub `DriveSearchTool` struct L87-89 — `{ integration: Arc<GoogleDriveIntegration> }` — - `drive_delete` — trash (recoverable) — does not permadelete
- pub `new` function L92-94 — `(integration: Arc<GoogleDriveIntegration>) -> Self` — - `drive_delete` — trash (recoverable) — does not permadelete
- pub `DriveListTool` struct L193-195 — `{ integration: Arc<GoogleDriveIntegration> }` — - `drive_delete` — trash (recoverable) — does not permadelete
- pub `new` function L198-200 — `(integration: Arc<GoogleDriveIntegration>) -> Self` — - `drive_delete` — trash (recoverable) — does not permadelete
- pub `DriveGetMetadataTool` struct L281-283 — `{ integration: Arc<GoogleDriveIntegration> }` — - `drive_delete` — trash (recoverable) — does not permadelete
- pub `new` function L286-288 — `(integration: Arc<GoogleDriveIntegration>) -> Self` — - `drive_delete` — trash (recoverable) — does not permadelete
- pub `DriveReadTool` struct L337-339 — `{ integration: Arc<GoogleDriveIntegration> }` — - `drive_delete` — trash (recoverable) — does not permadelete
- pub `new` function L342-344 — `(integration: Arc<GoogleDriveIntegration>) -> Self` — - `drive_delete` — trash (recoverable) — does not permadelete
- pub `DriveUploadTool` struct L497-499 — `{ integration: Arc<GoogleDriveIntegration> }` — - `drive_delete` — trash (recoverable) — does not permadelete
- pub `new` function L502-504 — `(integration: Arc<GoogleDriveIntegration>) -> Self` — - `drive_delete` — trash (recoverable) — does not permadelete
- pub `DriveUpdateTool` struct L603-605 — `{ integration: Arc<GoogleDriveIntegration> }` — - `drive_delete` — trash (recoverable) — does not permadelete
- pub `new` function L608-610 — `(integration: Arc<GoogleDriveIntegration>) -> Self` — - `drive_delete` — trash (recoverable) — does not permadelete
- pub `DriveDeleteTool` struct L695-697 — `{ integration: Arc<GoogleDriveIntegration> }` — - `drive_delete` — trash (recoverable) — does not permadelete
- pub `new` function L700-702 — `(integration: Arc<GoogleDriveIntegration>) -> Self` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `integ_err` function L24-26 — `(e: crate::IntegrationError) -> ToolError` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `google_err` function L28-30 — `(stage: &str, e: google_drive3::Error) -> ToolError` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `FileSummary` struct L35-51 — `{ id: Option<String>, name: Option<String>, mime_type: Option<String>, size: Opt...` — Compact file row used by list / search / get-metadata.
-  `summarize_file` function L53-73 — `(f: &DriveFile, include_parents: bool) -> FileSummary` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `FILE_FIELDS_LIST` variable L77 — `: &str` — Standard projection passed to `fields` so we get the same shape across
-  `FILE_FIELDS_ONE` variable L78 — `: &str` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `DRIVE_READ_DEFAULT_MAX_BYTES` variable L82 — `: usize` — Cap returned content for `drive_read` so a 50MB binary doesn't fill the
-  `DRIVE_READ_HARD_MAX_BYTES` variable L83 — `: usize` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `DriveSearchTool` type L91-95 — `= DriveSearchTool` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `DriveSearchTool` type L98-189 — `impl Tool for DriveSearchTool` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `name` function L99-101 — `(&self) -> &str` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `description` function L102-110 — `(&self) -> &str` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `category` function L111-113 — `(&self) -> ToolCategory` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `permission_category` function L114-116 — `(&self) -> PermissionCategory` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `parameters_schema` function L117-142 — `(&self) -> Value` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `execute` function L143-188 — `(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError>` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `DriveListTool` type L197-201 — `= DriveListTool` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `DriveListTool` type L204-277 — `impl Tool for DriveListTool` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `name` function L205-207 — `(&self) -> &str` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `description` function L208-212 — `(&self) -> &str` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `category` function L213-215 — `(&self) -> ToolCategory` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `permission_category` function L216-218 — `(&self) -> PermissionCategory` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `parameters_schema` function L219-239 — `(&self) -> Value` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `execute` function L240-276 — `(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError>` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `DriveGetMetadataTool` type L285-289 — `= DriveGetMetadataTool` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `DriveGetMetadataTool` type L292-333 — `impl Tool for DriveGetMetadataTool` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `name` function L293-295 — `(&self) -> &str` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `description` function L296-300 — `(&self) -> &str` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `category` function L301-303 — `(&self) -> ToolCategory` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `permission_category` function L304-306 — `(&self) -> PermissionCategory` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `parameters_schema` function L307-315 — `(&self) -> Value` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `execute` function L316-332 — `(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError>` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `DriveReadTool` type L341-345 — `= DriveReadTool` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `export_mime_for` function L349-359 — `(google_mime: &str) -> Option<&'static str>` — Pick the export format for Google's native types.
-  `DriveReadTool` type L362-493 — `impl Tool for DriveReadTool` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `name` function L363-365 — `(&self) -> &str` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `description` function L366-372 — `(&self) -> &str` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `category` function L373-375 — `(&self) -> ToolCategory` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `permission_category` function L376-378 — `(&self) -> PermissionCategory` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `parameters_schema` function L379-393 — `(&self) -> Value` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `execute` function L394-492 — `(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError>` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `DriveUploadTool` type L501-505 — `= DriveUploadTool` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `DriveUploadTool` type L508-599 — `impl Tool for DriveUploadTool` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `name` function L509-511 — `(&self) -> &str` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `description` function L512-517 — `(&self) -> &str` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `category` function L518-520 — `(&self) -> ToolCategory` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `permission_category` function L521-523 — `(&self) -> PermissionCategory` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `parameters_schema` function L524-546 — `(&self) -> Value` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `execute` function L547-598 — `(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError>` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `DriveUpdateTool` type L607-611 — `= DriveUpdateTool` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `DriveUpdateTool` type L614-691 — `impl Tool for DriveUpdateTool` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `name` function L615-617 — `(&self) -> &str` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `description` function L618-623 — `(&self) -> &str` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `category` function L624-626 — `(&self) -> ToolCategory` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `permission_category` function L627-629 — `(&self) -> PermissionCategory` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `parameters_schema` function L630-648 — `(&self) -> Value` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `execute` function L649-690 — `(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError>` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `DriveDeleteTool` type L699-703 — `= DriveDeleteTool` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `DriveDeleteTool` type L706-759 — `impl Tool for DriveDeleteTool` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `name` function L707-709 — `(&self) -> &str` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `description` function L710-716 — `(&self) -> &str` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `category` function L717-719 — `(&self) -> ToolCategory` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `permission_category` function L720-722 — `(&self) -> PermissionCategory` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `parameters_schema` function L723-731 — `(&self) -> Value` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `execute` function L732-758 — `(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError>` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `tests` module L762-808 — `-` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `export_mime_dispatch_covers_known_google_types` function L766-782 — `()` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `summarize_file_extracts_owner_emails` function L785-799 — `()` — - `drive_delete` — trash (recoverable) — does not permadelete
-  `summarize_file_includes_parents_when_requested` function L802-807 — `()` — - `drive_delete` — trash (recoverable) — does not permadelete

### crates/arawn-integrations/src/gmail

> *Semantic summary to be generated by AI agent.*

#### crates/arawn-integrations/src/gmail/client.rs

- pub `GmailHub` type L13 — `= Gmail<HttpsConnector>` — Concrete Gmail Hub the integration exposes.
- pub `client_from_token_store` function L18-28 — `( data_dir: std::path::PathBuf, oauth_config: OAuthProviderConfig, ) -> Result<G...` — Open the persisted Gmail token, build the hyper-util client + auth

#### crates/arawn-integrations/src/gmail/integration.rs

- pub `SERVICE_NAME` variable L16 — `: &str` — Stable service name.
- pub `GmailProviderConfig` struct L20-24 — `{ auth_url: Url, token_url: Url, scopes: Vec<String> }` — Standard Gmail OAuth provider configuration.
- pub `into_oauth_provider` function L44-53 — `(self, client_id: String, client_secret: String) -> OAuthProviderConfig` — Build the underlying [`OAuthProviderConfig`] given a client_id /
- pub `GmailIntegration` struct L58-68 — `{ data_dir: PathBuf, client_id: String, client_secret: String, provider_config: ...` — Gmail integration.
- pub `new` function L72-79 — `(data_dir: PathBuf, client_id: String, client_secret: String) -> Self` — Standard constructor.
- pub `with_provider_config` function L82-85 — `(mut self, config: GmailProviderConfig) -> Self` — Override the OAuth provider config — used by tests.
- pub `hub` function L89-92 — `(&self) -> Result<GmailHub, IntegrationError>` — Build a fully-wired `Gmail` Hub for tools.
-  `GmailProviderConfig` type L26-38 — `impl Default for GmailProviderConfig`
-  `default` function L27-37 — `() -> Self`
-  `GmailProviderConfig` type L40-54 — `= GmailProviderConfig`
-  `GmailIntegration` type L70-110 — `= GmailIntegration`
-  `oauth_config` function L94-105 — `(&self) -> OAuthProviderConfig`
-  `token_store` function L107-109 — `(&self) -> Result<TokenStore, IntegrationError>`
-  `GmailIntegration` type L113-140 — `impl Integration for GmailIntegration`
-  `name` function L114-116 — `(&self) -> &str`
-  `is_connected` function L118-126 — `(&self) -> bool`
-  `connect` function L128-133 — `(&self, ctx: &dyn ConnectContext) -> Result<(), IntegrationError>`
-  `disconnect` function L135-139 — `(&self) -> Result<(), IntegrationError>`
-  `tests` module L143-164 — `-`
-  `default_provider_has_three_gmail_scopes` function L147-153 — `()`
-  `provider_lifts_into_oauth_config` function L156-163 — `()`

#### crates/arawn-integrations/src/gmail/mod.rs

-  `client` module L12 — `-` — Provides:
-  `integration` module L13 — `-` — setup steps users need to complete before connecting.
-  `tools` module L14 — `-` — setup steps users need to complete before connecting.

#### crates/arawn-integrations/src/gmail/tools.rs

- pub `GmailInboxReadTool` struct L92-94 — `{ integration: Arc<GmailIntegration> }` — picked up by the next call automatically.
- pub `new` function L97-99 — `(integration: Arc<GmailIntegration>) -> Self` — picked up by the next call automatically.
- pub `GmailSearchTool` struct L165-167 — `{ integration: Arc<GmailIntegration> }` — picked up by the next call automatically.
- pub `new` function L170-172 — `(integration: Arc<GmailIntegration>) -> Self` — picked up by the next call automatically.
- pub `GmailGetMessageTool` struct L238-240 — `{ integration: Arc<GmailIntegration> }` — picked up by the next call automatically.
- pub `new` function L243-245 — `(integration: Arc<GmailIntegration>) -> Self` — picked up by the next call automatically.
- pub `GmailSendTool` struct L331-333 — `{ integration: Arc<GmailIntegration> }` — picked up by the next call automatically.
- pub `new` function L336-338 — `(integration: Arc<GmailIntegration>) -> Self` — picked up by the next call automatically.
- pub `GmailMarkReadTool` struct L435-437 — `{ integration: Arc<GmailIntegration> }` — picked up by the next call automatically.
- pub `new` function L440-442 — `(integration: Arc<GmailIntegration>) -> Self` — picked up by the next call automatically.
-  `MessageSummary` struct L22-30 — `{ id: String, thread_id: Option<String>, from: Option<String>, subject: Option<S...` — One-line summary of a Gmail message — what `inbox_read` and `search` return per row.
-  `integ_err` function L32-34 — `(e: crate::IntegrationError) -> ToolError` — picked up by the next call automatically.
-  `google_err` function L36-38 — `(stage: &str, e: google_gmail1::Error) -> ToolError` — picked up by the next call automatically.
-  `fetch_summaries` function L42-61 — `( hub: &super::client::GmailHub, ids: &[String], ) -> Result<Vec<MessageSummary>...` — Pull metadata + snippet for a list of message ids.
-  `summary_from_message` function L63-88 — `(m: &Message) -> MessageSummary` — picked up by the next call automatically.
-  `GmailInboxReadTool` type L96-100 — `= GmailInboxReadTool` — picked up by the next call automatically.
-  `GmailInboxReadTool` type L103-161 — `impl Tool for GmailInboxReadTool` — picked up by the next call automatically.
-  `name` function L104-106 — `(&self) -> &str` — picked up by the next call automatically.
-  `description` function L107-111 — `(&self) -> &str` — picked up by the next call automatically.
-  `category` function L112-114 — `(&self) -> ToolCategory` — picked up by the next call automatically.
-  `permission_category` function L115-117 — `(&self) -> PermissionCategory` — picked up by the next call automatically.
-  `parameters_schema` function L118-134 — `(&self) -> Value` — picked up by the next call automatically.
-  `execute` function L135-160 — `(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError>` — picked up by the next call automatically.
-  `GmailSearchTool` type L169-173 — `= GmailSearchTool` — picked up by the next call automatically.
-  `GmailSearchTool` type L176-234 — `impl Tool for GmailSearchTool` — picked up by the next call automatically.
-  `name` function L177-179 — `(&self) -> &str` — picked up by the next call automatically.
-  `description` function L180-183 — `(&self) -> &str` — picked up by the next call automatically.
-  `category` function L184-186 — `(&self) -> ToolCategory` — picked up by the next call automatically.
-  `permission_category` function L187-189 — `(&self) -> PermissionCategory` — picked up by the next call automatically.
-  `parameters_schema` function L190-207 — `(&self) -> Value` — picked up by the next call automatically.
-  `execute` function L208-233 — `(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError>` — picked up by the next call automatically.
-  `GmailGetMessageTool` type L242-246 — `= GmailGetMessageTool` — picked up by the next call automatically.
-  `GmailGetMessageTool` type L249-303 — `impl Tool for GmailGetMessageTool` — picked up by the next call automatically.
-  `name` function L250-252 — `(&self) -> &str` — picked up by the next call automatically.
-  `description` function L253-256 — `(&self) -> &str` — picked up by the next call automatically.
-  `category` function L257-259 — `(&self) -> ToolCategory` — picked up by the next call automatically.
-  `permission_category` function L260-262 — `(&self) -> PermissionCategory` — picked up by the next call automatically.
-  `parameters_schema` function L263-271 — `(&self) -> Value` — picked up by the next call automatically.
-  `execute` function L272-302 — `(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError>` — picked up by the next call automatically.
-  `extract_plain_text_body` function L307-310 — `(m: &Message) -> Option<String>` — Walk a `Message`'s payload tree looking for the first `text/plain` part.
-  `walk_for_plain_text` function L312-327 — `(part: &google_gmail1::api::MessagePart) -> Option<String>` — picked up by the next call automatically.
-  `GmailSendTool` type L335-339 — `= GmailSendTool` — picked up by the next call automatically.
-  `GmailSendTool` type L342-410 — `impl Tool for GmailSendTool` — picked up by the next call automatically.
-  `name` function L343-345 — `(&self) -> &str` — picked up by the next call automatically.
-  `description` function L346-349 — `(&self) -> &str` — picked up by the next call automatically.
-  `category` function L350-352 — `(&self) -> ToolCategory` — picked up by the next call automatically.
-  `permission_category` function L353-357 — `(&self) -> PermissionCategory` — picked up by the next call automatically.
-  `parameters_schema` function L358-372 — `(&self) -> Value` — picked up by the next call automatically.
-  `execute` function L373-409 — `(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError>` — picked up by the next call automatically.
-  `build_rfc2822` function L413-431 — `( to: &str, subject: &str, body: &str, in_reply_to: Option<&str>, ) -> String` — Tiny RFC 2822 builder.
-  `GmailMarkReadTool` type L439-443 — `= GmailMarkReadTool` — picked up by the next call automatically.
-  `GmailMarkReadTool` type L446-488 — `impl Tool for GmailMarkReadTool` — picked up by the next call automatically.
-  `name` function L447-449 — `(&self) -> &str` — picked up by the next call automatically.
-  `description` function L450-452 — `(&self) -> &str` — picked up by the next call automatically.
-  `category` function L453-455 — `(&self) -> ToolCategory` — picked up by the next call automatically.
-  `permission_category` function L456-460 — `(&self) -> PermissionCategory` — picked up by the next call automatically.
-  `parameters_schema` function L461-469 — `(&self) -> Value` — picked up by the next call automatically.
-  `execute` function L470-487 — `(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError>` — picked up by the next call automatically.
-  `tests` module L491-617 — `-` — picked up by the next call automatically.
-  `header` function L495-500 — `(name: &str, value: &str) -> MessagePartHeader` — picked up by the next call automatically.
-  `summary_from_message_extracts_known_headers` function L503-526 — `()` — picked up by the next call automatically.
-  `summary_handles_empty_payload` function L529-538 — `()` — picked up by the next call automatically.
-  `extract_plain_text_finds_top_level_text_plain` function L541-554 — `()` — picked up by the next call automatically.
-  `extract_plain_text_descends_into_multipart_alternative` function L557-584 — `()` — picked up by the next call automatically.
-  `extract_plain_text_returns_none_when_html_only` function L587-600 — `()` — picked up by the next call automatically.
-  `rfc2822_includes_required_headers_and_body` function L603-609 — `()` — picked up by the next call automatically.
-  `rfc2822_threads_via_in_reply_to` function L612-616 — `()` — picked up by the next call automatically.

### crates/arawn-integrations/src/slack

> *Semantic summary to be generated by AI agent.*

#### crates/arawn-integrations/src/slack/client.rs

- pub `SlackContext` struct L16-19 — `{ client: Arc<SlackHyperClient>, token: SlackApiToken }` — Bundle the slack-morphism client + token a tool needs to make API calls.
- pub `session` function L24-26 — `(&self) -> SlackClientSession<'_, SlackClientHyperHttpsConnector>` — Convenience: open a slack-morphism session against the bundled token.
- pub `build_slack_client` function L33-40 — `(token: &Token) -> SlackContext` — Build a [`SlackContext`] from a persisted `arawn_auth::Token`.
-  `SlackContext` type L21-27 — `= SlackContext` — time.
-  `tests` module L43-69 — `-` — time.
-  `build_constructs_bot_token_from_access` function L49-68 — `()` — time.

#### crates/arawn-integrations/src/slack/integration.rs

- pub `SERVICE_NAME` variable L15 — `: &str` — Stable service name.
- pub `SLACK_OAUTH_SCOPES` variable L24-51 — `: &[&str]` — Bot scopes requested at OAuth time.
- pub `SLACK_OAUTH_USER_SCOPES` variable L77-88 — `: &[&str]` — User-token scopes — the second leg of Slack's dual-token OAuth model.
- pub `SlackProviderConfig` struct L92-101 — `{ auth_url: Url, token_url: Url, scopes: Vec<String>, redirect_port: u16 }` — Slack OAuth v2 provider config.
- pub `DEFAULT_SLACK_REDIRECT_PORT` variable L107 — `: u16` — Default callback port for Slack.
- pub `into_oauth_provider` function L121-135 — `(self, client_id: String, client_secret: String) -> OAuthProviderConfig`
- pub `SlackIntegration` struct L139-144 — `{ data_dir: PathBuf, client_id: String, client_secret: String, provider_config: ...` — Slack integration.
- pub `new` function L147-154 — `(data_dir: PathBuf, client_id: String, client_secret: String) -> Self`
- pub `with_provider_config` function L156-159 — `(mut self, config: SlackProviderConfig) -> Self`
- pub `context` function L164-166 — `(&self) -> Result<SlackContext, IntegrationError>` — Build a `SlackContext` backed by the **bot** token.
- pub `bot_context` function L170-173 — `(&self) -> Result<SlackContext, IntegrationError>` — Same as [`Self::context`] — kept as the canonical name for the
- pub `user_context` function L185-213 — `(&self) -> Result<SlackContext, IntegrationError>` — Build a `SlackContext` backed by the **user** token (the half of
- pub `granted_scopes` function L226-229 — `(&self) -> Result<std::collections::HashSet<String>, IntegrationError>` — Bot-token scope set from the persisted token's `scope` field.
- pub `granted_user_scopes` function L234-245 — `( &self, ) -> Result<std::collections::HashSet<String>, IntegrationError>` — User-token scope set from `extras.authed_user.scope`.
-  `parse_scope_string` function L55-60 — `(s: &str) -> std::collections::HashSet<String>` — Split a Slack-style scope string (comma- or whitespace-delimited)
-  `SlackProviderConfig` type L109-118 — `impl Default for SlackProviderConfig`
-  `default` function L110-117 — `() -> Self`
-  `SlackProviderConfig` type L120-136 — `= SlackProviderConfig`
-  `SlackIntegration` type L146-266 — `= SlackIntegration`
-  `load_token` function L215-220 — `(&self) -> Result<arawn_auth::Token, IntegrationError>`
-  `oauth_config` function L247-249 — `(&self) -> OAuthProviderConfig`
-  `provider` function L251-261 — `(&self) -> SlackProviderConfig`
-  `token_store` function L263-265 — `(&self) -> Result<TokenStore, IntegrationError>`
-  `SlackIntegration` type L269-338 — `impl Integration for SlackIntegration`
-  `name` function L270-272 — `(&self) -> &str`
-  `is_connected` function L274-279 — `(&self) -> bool`
-  `connect` function L281-295 — `(&self, ctx: &dyn ConnectContext) -> Result<(), IntegrationError>`
-  `disconnect` function L297-301 — `(&self) -> Result<(), IntegrationError>`
-  `capabilities_summary` function L303-337 — `(&self) -> Option<String>`
-  `tests` module L341-369 — `-`
-  `default_provider_carries_sixteen_bot_scopes` function L345-358 — `()`
-  `provider_lifts_into_oauth_config` function L361-368 — `()`

#### crates/arawn-integrations/src/slack/mod.rs

-  `client` module L16 — `-` — post messages, and react.
-  `integration` module L17 — `-` — for the design call (full OAuth, not webhook).
-  `tools` module L18 — `-` — for the design call (full OAuth, not webhook).

#### crates/arawn-integrations/src/slack/tools.rs

- pub `SlackListChannelsTool` struct L218-221 — `{ integration: Arc<SlackIntegration>, description: String }` — questions in the meantime.
- pub `new` function L224-229 — `(integration: Arc<SlackIntegration>) -> Self` — questions in the meantime.
- pub `SlackHistoryTool` struct L312-315 — `{ integration: Arc<SlackIntegration>, description: String }` — questions in the meantime.
- pub `new` function L318-323 — `(integration: Arc<SlackIntegration>) -> Self` — questions in the meantime.
- pub `SlackPostTool` struct L414-417 — `{ integration: Arc<SlackIntegration>, description: String }` — questions in the meantime.
- pub `new` function L426-431 — `(integration: Arc<SlackIntegration>) -> Self` — questions in the meantime.
- pub `SlackReactTool` struct L512-515 — `{ integration: Arc<SlackIntegration>, description: String }` — questions in the meantime.
- pub `new` function L518-523 — `(integration: Arc<SlackIntegration>) -> Self` — questions in the meantime.
- pub `SlackUsersListTool` struct L626-629 — `{ integration: Arc<SlackIntegration>, description: String }` — questions in the meantime.
- pub `new` function L632-637 — `(integration: Arc<SlackIntegration>) -> Self` — questions in the meantime.
- pub `SlackOpenDmTool` struct L711-714 — `{ integration: Arc<SlackIntegration>, description: String }` — questions in the meantime.
- pub `new` function L717-726 — `(integration: Arc<SlackIntegration>) -> Self` — questions in the meantime.
-  `scope_footer` function L31-37 — `(scopes: &[&str]) -> String` — Format a scope footer for tool descriptions.
-  `granted_scopes` function L40-42 — `(integration: &SlackIntegration) -> Result<HashSet<String>, ToolError>` — Read the granted bot-token scope set from the persisted token.
-  `granted_user_scopes` function L46-48 — `(integration: &SlackIntegration) -> Result<HashSet<String>, ToolError>` — Read the granted user-token scope set from the persisted token.
-  `check_scopes` function L52-54 — `(integration: &SlackIntegration, required: &[&str]) -> Result<(), ToolError>` — Verify the persisted **bot** token covers `required`.
-  `check_user_scopes` function L57-66 — `( integration: &SlackIntegration, required: &[&str], ) -> Result<(), ToolError>` — Verify the persisted **user** token covers `required`.
-  `check_in_set` function L68-87 — `( granted: &HashSet<String>, required: &[&str], section_label: &str, ) -> Result...` — questions in the meantime.
-  `read_ctx_for_listing` function L92-118 — `( integration: &SlackIntegration, include_private: bool, include_dms: bool, ) ->...` — Pick the read context for `slack_list_channels`.
-  `integ_err` function L120-122 — `(e: crate::IntegrationError) -> ToolError` — questions in the meantime.
-  `slack_err` function L126-128 — `(stage: &str, e: slack_morphism::errors::SlackClientError) -> ToolError` — `slack-morphism::ClientError` → `ToolError`.
-  `ChannelSummary` struct L134-142 — `{ id: String, name: Option<String>, kind: String, member_count: Option<u64>, is_...` — Compact, agent-friendly channel summary.
-  `summarize_channel` function L144-163 — `(c: &slack_morphism::prelude::SlackChannelInfo) -> ChannelSummary` — questions in the meantime.
-  `MessageSummary` struct L167-178 — `{ ts: String, user: Option<String>, text: Option<String>, thread_ts: Option<Stri...` — Compact message record — what the agent sees from `slack_history`.
-  `ReactionSummary` struct L181-184 — `{ name: String, count: usize }` — questions in the meantime.
-  `summarize_message` function L186-209 — `(m: &slack_morphism::prelude::SlackHistoryMessage) -> MessageSummary` — questions in the meantime.
-  `SLACK_LIST_CHANNELS_BASE` variable L213-215 — `: &str` — questions in the meantime.
-  `SLACK_LIST_CHANNELS_SCOPES` variable L216 — `: &[&str]` — questions in the meantime.
-  `SlackListChannelsTool` type L223-230 — `= SlackListChannelsTool` — questions in the meantime.
-  `SlackListChannelsTool` type L233-300 — `impl Tool for SlackListChannelsTool` — questions in the meantime.
-  `name` function L234-236 — `(&self) -> &str` — questions in the meantime.
-  `description` function L237-239 — `(&self) -> &str` — questions in the meantime.
-  `category` function L240-242 — `(&self) -> ToolCategory` — questions in the meantime.
-  `permission_category` function L243-245 — `(&self) -> PermissionCategory` — questions in the meantime.
-  `parameters_schema` function L246-266 — `(&self) -> Value` — questions in the meantime.
-  `execute` function L267-299 — `(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError>` — questions in the meantime.
-  `SLACK_HISTORY_BASE` variable L304-306 — `: &str` — questions in the meantime.
-  `SLACK_HISTORY_SCOPES` variable L310 — `: &[&str]` — `channels:history` covers public channels (C-prefixed).
-  `SlackHistoryTool` type L317-324 — `= SlackHistoryTool` — questions in the meantime.
-  `SlackHistoryTool` type L327-410 — `impl Tool for SlackHistoryTool` — questions in the meantime.
-  `name` function L328-330 — `(&self) -> &str` — questions in the meantime.
-  `description` function L331-333 — `(&self) -> &str` — questions in the meantime.
-  `category` function L334-336 — `(&self) -> ToolCategory` — questions in the meantime.
-  `permission_category` function L337-339 — `(&self) -> PermissionCategory` — questions in the meantime.
-  `parameters_schema` function L340-365 — `(&self) -> Value` — questions in the meantime.
-  `execute` function L366-409 — `(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError>` — questions in the meantime.
-  `SLACK_POST_BASE` variable L419-422 — `: &str` — questions in the meantime.
-  `SLACK_POST_SCOPES` variable L423 — `: &[&str]` — questions in the meantime.
-  `SlackPostTool` type L425-432 — `= SlackPostTool` — questions in the meantime.
-  `SlackPostTool` type L435-504 — `impl Tool for SlackPostTool` — questions in the meantime.
-  `name` function L436-438 — `(&self) -> &str` — questions in the meantime.
-  `description` function L439-441 — `(&self) -> &str` — questions in the meantime.
-  `category` function L442-444 — `(&self) -> ToolCategory` — questions in the meantime.
-  `permission_category` function L445-447 — `(&self) -> PermissionCategory` — questions in the meantime.
-  `parameters_schema` function L448-467 — `(&self) -> Value` — questions in the meantime.
-  `execute` function L468-503 — `(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError>` — questions in the meantime.
-  `SLACK_REACT_BASE` variable L508-509 — `: &str` — questions in the meantime.
-  `SLACK_REACT_SCOPES` variable L510 — `: &[&str]` — questions in the meantime.
-  `SlackReactTool` type L517-524 — `= SlackReactTool` — questions in the meantime.
-  `SlackReactTool` type L527-584 — `impl Tool for SlackReactTool` — questions in the meantime.
-  `name` function L528-530 — `(&self) -> &str` — questions in the meantime.
-  `description` function L531-533 — `(&self) -> &str` — questions in the meantime.
-  `category` function L534-536 — `(&self) -> ToolCategory` — questions in the meantime.
-  `permission_category` function L537-539 — `(&self) -> PermissionCategory` — questions in the meantime.
-  `parameters_schema` function L540-550 — `(&self) -> Value` — questions in the meantime.
-  `execute` function L551-583 — `(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError>` — questions in the meantime.
-  `UserSummary` struct L591-603 — `{ id: String, name: Option<String>, real_name: Option<String>, display_name: Opt...` — Compact user record.
-  `summarize_user` function L605-617 — `(u: &slack_morphism::prelude::SlackUser) -> UserSummary` — questions in the meantime.
-  `SLACK_USERS_LIST_BASE` variable L619-623 — `: &str` — questions in the meantime.
-  `SLACK_USERS_LIST_SCOPES` variable L624 — `: &[&str]` — questions in the meantime.
-  `SlackUsersListTool` type L631-638 — `= SlackUsersListTool` — questions in the meantime.
-  `SlackUsersListTool` type L641-697 — `impl Tool for SlackUsersListTool` — questions in the meantime.
-  `name` function L642-644 — `(&self) -> &str` — questions in the meantime.
-  `description` function L645-647 — `(&self) -> &str` — questions in the meantime.
-  `category` function L648-650 — `(&self) -> ToolCategory` — questions in the meantime.
-  `permission_category` function L651-653 — `(&self) -> PermissionCategory` — questions in the meantime.
-  `parameters_schema` function L654-674 — `(&self) -> Value` — questions in the meantime.
-  `execute` function L675-696 — `(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError>` — questions in the meantime.
-  `SLACK_OPEN_DM_BASE` variable L701-704 — `: &str` — questions in the meantime.
-  `SLACK_OPEN_DM_SCOPE_HINT` variable L709 — `: &[&str]` — `conversations.open` requires `im:write` for 1:1 DMs and `mpim:write`
-  `SlackOpenDmTool` type L716-727 — `= SlackOpenDmTool` — questions in the meantime.
-  `SlackOpenDmTool` type L730-799 — `impl Tool for SlackOpenDmTool` — questions in the meantime.
-  `name` function L731-733 — `(&self) -> &str` — questions in the meantime.
-  `description` function L734-736 — `(&self) -> &str` — questions in the meantime.
-  `category` function L737-739 — `(&self) -> ToolCategory` — questions in the meantime.
-  `permission_category` function L740-745 — `(&self) -> PermissionCategory` — questions in the meantime.
-  `parameters_schema` function L746-758 — `(&self) -> Value` — questions in the meantime.
-  `execute` function L759-798 — `(&self, _ctx: &dyn ToolContext, params: Value) -> Result<ToolOutput, ToolError>` — questions in the meantime.
-  `tests` module L802-921 — `-` — questions in the meantime.
-  `channel` function L810-824 — `(id: &str, kind: &str) -> SlackChannelInfo` — questions in the meantime.
-  `summarize_channel_classifies_kind_correctly` function L827-836 — `()` — questions in the meantime.
-  `summarize_channel_carries_topic_and_purpose` function L839-848 — `()` — questions in the meantime.
-  `summarize_message_extracts_user_text_and_reactions` function L851-878 — `()` — questions in the meantime.
-  `summarize_user_extracts_handle_and_profile_fields` function L881-908 — `()` — questions in the meantime.
-  `summarize_user_handles_minimal_record` function L911-920 — `()` — questions in the meantime.

### crates/arawn-llm/src

**Role**: Provider-neutral LLM client abstraction with concrete implementations for Anthropic, Groq, and any OpenAI-compatible API, plus retry wrapping and a mock client for testing.

**Key abstractions**:
- `LlmClient` trait — Single async method: `stream(ChatRequest) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk>>>>, LlmError>`. All callers consume the stream of `ChatChunk`s and assemble them into an `AssembledResponse`.
- `ChatRequest` / `ChatMessage` / `ChatChunk` — Provider-neutral types. `ChatChunk` variants (`TextDelta`, `ToolUseStart`, `ToolUseInputDelta`, `Done`) mirror the SSE event structure. The engine assembles deltas into full tool call objects before dispatching.
- `AnthropicClient` — Calls the Anthropic Messages API. `build_messages` merges consecutive same-role messages (required by Anthropic's API contract). `build_request_body` includes tool definitions in Anthropic's format.
- `OpenAICompatibleClient` — Generic client for Groq, Ollama, OpenAI, vLLM, and any compatible endpoint. Configured with a `base_url`, optional `api_key`, and `provider_name` (used in error messages). Factory methods: `groq()`, `ollama()`, `openai()`, `from_config()`.
- `GroqClient` — A dedicated (slightly older) Groq client with its own SSE parser. Superseded by `OpenAICompatibleClient::groq()` in most new code, but retained for compatibility.
- `RetryClient` — Wraps any `LlmClient` and retries `LlmError::is_retryable()` errors (ServerError, RateLimited) with exponential backoff up to `DEFAULT_MAX_RETRIES`. Non-retryable errors (Auth, ModelNotFound, Api) fail immediately. This handles connection-time failures; `stream_response_with_retry` in the engine handles mid-stream failures separately.
- `MockLlmClient` — Returns scripted `MockResponse` variants in order. `MockResponse::StreamError` yields some chunks then an error mid-stream. Panics if exhausted. Used in all engine unit tests and integration tests.
- `LlmError::from_status(status, body)` — Maps HTTP status codes to typed errors: 401/403 → Auth, 404 → ModelNotFound, 429 → RateLimited, 5xx → ServerError, otherwise Api. `is_retryable()` is true for ServerError and RateLimited.

**Dependencies**: `reqwest` (HTTP + streaming), `futures` (Stream), `serde`/`serde_json` (request/response types), `async-trait`.

#### crates/arawn-llm/src/anthropic.rs

- pub `AnthropicClient` struct L17-20 — `{ http: Client, api_key: String }` — Client for Anthropic's Claude API (Messages API).
- pub `new` function L23-28 — `(api_key: impl Into<String>) -> Self`
- pub `from_env` function L30-34 — `() -> Result<Self, LlmError>`
-  `API_URL` variable L13 — `: &str`
-  `API_VERSION` variable L14 — `: &str`
-  `AnthropicClient` type L22-57 — `= AnthropicClient`
-  `build_request_body` function L36-56 — `(&self, request: &ChatRequest) -> Value`
-  `AnthropicClient` type L60-196 — `impl LlmClient for AnthropicClient`
-  `stream` function L61-195 — `( &self, request: ChatRequest, ) -> Result<Pin<Box<dyn Stream<Item = Result<Chat...`
-  `build_messages` function L202-261 — `(messages: &[ChatMessage]) -> Vec<Value>` — Convert arawn messages to Anthropic format.
-  `merge_consecutive_roles` function L265-301 — `(messages: &mut Vec<Value>)` — Merge consecutive messages with the same role into a single message
-  `normalize_content` function L304-310 — `(content: &Value) -> Vec<Value>` — Normalize content to a Vec<Value> of content blocks.
-  `build_tools` function L313-324 — `(tools: &[ToolDefinition]) -> Vec<Value>` — Convert tool definitions to Anthropic format.
-  `tests` module L327-458 — `-`
-  `user_msg` function L331-338 — `(text: &str) -> ChatMessage`
-  `assistant_text` function L340-347 — `(text: &str) -> ChatMessage`
-  `assistant_with_tool` function L349-360 — `(text: &str, tool_id: &str, tool_name: &str, args: Value) -> ChatMessage`
-  `tool_result` function L362-370 — `(tool_use_id: &str, content: &str) -> ChatMessage`
-  `simple_conversation` function L373-382 — `()`
-  `tool_call_with_result` function L385-408 — `()`
-  `multi_turn_with_tools` function L411-434 — `()`
-  `consecutive_tool_results_merged` function L437-457 — `()`

#### crates/arawn-llm/src/client.rs

- pub `LlmClient` interface L12-48 — `{ fn stream(), fn warmup() }` — Provider-agnostic LLM client trait.
-  `warmup` function L24-47 — `(&self, model: &str) -> Result<(), LlmError>` — Probe a model with a minimal request to confirm it is reachable and

#### crates/arawn-llm/src/error.rs

- pub `LlmError` enum L4-31 — `Api | Auth | ModelNotFound | RateLimited | ServerError | Stream | Config | Reque...`
- pub `is_retryable` function L35-52 — `(&self) -> bool` — Returns true if this error is transient and the request should be retried.
- pub `from_status` function L55-67 — `(status: u16, body: String) -> Self` — Create from an HTTP status code + body.
- pub `user_message` function L70-122 — `(&self) -> String` — Return a user-facing error message with actionable guidance.
-  `LlmError` type L33-123 — `= LlmError`
-  `extract_api_message` function L127-134 — `(body: &str) -> Option<String>` — Try to extract a clean message from a JSON error body.
-  `tests` module L137-211 — `-`
-  `from_status_401_is_auth` function L141-146 — `()`
-  `from_status_403_is_auth` function L149-152 — `()`
-  `from_status_404_is_model_not_found` function L155-163 — `()`
-  `from_status_429_is_rate_limited` function L166-171 — `()`
-  `from_status_500_is_server_error` function L174-179 — `()`
-  `from_status_400_is_api_error` function L182-186 — `()`
-  `extract_message_from_json_body` function L189-193 — `()`
-  `extract_message_from_plain_text_returns_none` function L196-198 — `()`
-  `config_error_user_message` function L201-204 — `()`
-  `stream_error_user_message` function L207-210 — `()`

#### crates/arawn-llm/src/groq.rs

- pub `GroqClient` struct L17-20 — `{ http: Client, api_key: String }` — Groq LLM client using the OpenAI-compatible API.
- pub `new` function L23-28 — `(api_key: impl Into<String>) -> Self`
- pub `from_env` function L30-34 — `() -> Result<Self, LlmError>`
-  `GROQ_API_URL` variable L14 — `: &str`
-  `GroqClient` type L22-56 — `= GroqClient`
-  `build_request_body` function L36-55 — `(&self, request: &ChatRequest) -> Value`
-  `GroqClient` type L59-90 — `impl LlmClient for GroqClient`
-  `stream` function L60-89 — `( &self, request: ChatRequest, ) -> Result<Pin<Box<dyn Stream<Item = Result<Chat...`
-  `SseParser` struct L95-99 — `{ inner: S, buffer: String, pending_chunks: Vec<ChatChunk> }` — Parses Server-Sent Events from a byte stream into ChatChunks.
-  `new` function L102-108 — `(inner: S) -> Self`
-  `Item` type L115 — `= Result<ChatChunk, LlmError>`
-  `poll_next` function L117-158 — `( mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>, ) -> std::task::Pol...`
-  `try_parse_buffer` function L162-214 — `(&mut self) -> Option<Result<ChatChunk, LlmError>>`
-  `parse_groq_chunk` function L217-265 — `(chunk: &GroqStreamChunk) -> Vec<ChatChunk>`
-  `build_messages` function L269-334 — `(system_prompt: &Option<String>, messages: &[ChatMessage]) -> Vec<Value>`
-  `build_tools` function L336-350 — `(tools: &[ToolDefinition]) -> Vec<Value>`
-  `GroqErrorResponse` struct L355-357 — `{ error: Option<GroqError> }`
-  `GroqError` struct L360-364 — `{ message: String, code: Option<String> }`
-  `GroqStreamChunk` struct L369-374 — `{ choices: Vec<GroqChoice>, usage: Option<GroqUsage> }`
-  `GroqChoice` struct L377-379 — `{ delta: GroqDelta }`
-  `GroqDelta` struct L382-385 — `{ content: Option<String>, tool_calls: Option<Vec<GroqToolCall>> }`
-  `GroqToolCall` struct L388-391 — `{ id: Option<String>, function: Option<GroqFunction> }`
-  `GroqFunction` struct L394-397 — `{ name: Option<String>, arguments: Option<String> }`
-  `GroqUsage` struct L400-403 — `{ prompt_tokens: u32, completion_tokens: u32 }`
-  `tests` module L406-619 — `-`
-  `build_messages_with_system_prompt` function L411-425 — `()`
-  `build_messages_with_tool_calls` function L428-445 — `()`
-  `build_tools_format` function L448-463 — `()`
-  `parse_text_delta_chunk` function L466-482 — `()`
-  `parse_tool_use_start_chunk` function L485-510 — `()`
-  `parse_tool_call_with_name_and_args_in_same_chunk` function L513-545 — `()`
-  `parse_tool_use_input_delta_chunk` function L548-572 — `()`
-  `parse_usage_chunk` function L575-592 — `()`
-  `build_request_body_includes_tools` function L595-618 — `()`

#### crates/arawn-llm/src/lib.rs

- pub `anthropic` module L1 — `-`
- pub `client` module L2 — `-`
- pub `error` module L3 — `-`
- pub `groq` module L4 — `-`
- pub `mock` module L5 — `-`
- pub `openai_compat` module L6 — `-`
- pub `retry` module L7 — `-`
- pub `types` module L8 — `-`
- pub `warming` module L9 — `-`

#### crates/arawn-llm/src/mock.rs

- pub `MockResponse` enum L12-30 — `Text | ToolCall | Raw | Error | StreamError` — A scripted response for one LLM turn.
- pub `text` function L33-35 — `(text: impl Into<String>) -> Self`
- pub `tool_call` function L37-47 — `( id: impl Into<String>, name: impl Into<String>, arguments: impl Into<String>, ...`
- pub `raw` function L49-51 — `(chunks: Vec<ChatChunk>) -> Self`
- pub `error` function L53-55 — `(error: LlmError) -> Self`
- pub `stream_error` function L57-62 — `(chunks_before_error: Vec<ChatChunk>, error: LlmError) -> Self`
- pub `MockLlmClient` struct L90-94 — `{ responses: Mutex<Vec<MockResponse>>, call_count: Mutex<usize>, captured_reques...` — Mock LLM client that returns pre-scripted responses.
- pub `new` function L97-103 — `(responses: Vec<MockResponse>) -> Self`
- pub `call_count` function L106-108 — `(&self) -> usize` — How many times `stream()` has been called.
- pub `captured_requests` function L111-113 — `(&self) -> Vec<ChatRequest>` — Returns a clone of all captured requests for test assertions.
-  `MockResponse` type L32-85 — `= MockResponse`
-  `into_chunks` function L64-84 — `(self) -> Vec<ChatChunk>`
-  `MockLlmClient` type L96-114 — `= MockLlmClient`
-  `MockLlmClient` type L117-155 — `impl LlmClient for MockLlmClient`
-  `stream` function L118-154 — `( &self, request: ChatRequest, ) -> Result<Pin<Box<dyn futures::Stream<Item = Re...`
-  `tests` module L158-354 — `-`
-  `mock_text_response` function L163-182 — `()`
-  `mock_tool_call_response` function L185-217 — `()`
-  `mock_multiple_responses_consumed_in_order` function L220-249 — `()`
-  `mock_error_returns_err_immediately` function L252-272 — `()`
-  `mock_stream_error_yields_chunks_then_err` function L275-311 — `()`
-  `mock_error_then_success_simulates_retry` function L314-339 — `()`
-  `mock_panics_when_exhausted` function L343-353 — `()`

#### crates/arawn-llm/src/openai_compat.rs

- pub `OpenAICompatibleClient` struct L18-23 — `{ http: Client, base_url: String, api_key: Option<String>, provider_name: String...` — Generic client for any OpenAI-compatible API (Groq, Ollama, OpenAI, vLLM,
- pub `new` function L26-40 — `( base_url: impl Into<String>, api_key: Option<String>, provider_name: impl Into...`
- pub `groq` function L43-49 — `(api_key: impl Into<String>) -> Self` — Create a client for Groq.
- pub `groq_from_env` function L52-56 — `() -> Result<Self, LlmError>` — Create a client for Groq from the GROQ_API_KEY env var.
- pub `ollama` function L59-61 — `() -> Self` — Create a client for Ollama (local, no API key needed).
- pub `ollama_at` function L64-66 — `(base_url: impl Into<String>) -> Self` — Create a client for Ollama with a custom host/port.
- pub `openai` function L69-75 — `(api_key: impl Into<String>) -> Self` — Create a client for OpenAI.
- pub `openai_from_env` function L78-82 — `() -> Result<Self, LlmError>` — Create a client for OpenAI from the OPENAI_API_KEY env var.
- pub `from_config` function L85-105 — `( provider: &str, base_url: Option<&str>, api_key: Option<String>, ) -> Result<S...` — Create from explicit config values.
-  `OpenAICompatibleClient` type L25-131 — `= OpenAICompatibleClient`
-  `build_request_body` function L107-126 — `(&self, request: &ChatRequest) -> Value`
-  `completions_url` function L128-130 — `(&self) -> String`
-  `OpenAICompatibleClient` type L134-169 — `impl LlmClient for OpenAICompatibleClient`
-  `stream` function L135-168 — `( &self, request: ChatRequest, ) -> Result<Pin<Box<dyn Stream<Item = Result<Chat...`
-  `SseParser` struct L173-178 — `{ inner: S, buffer: String, pending_chunks: Vec<ChatChunk>, provider: String }`
-  `new` function L181-188 — `(inner: S, provider: String) -> Self`
-  `Item` type L195 — `= Result<ChatChunk, LlmError>`
-  `poll_next` function L197-233 — `( mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>, ) -> std::task::Pol...`
-  `try_parse_buffer` function L237-283 — `(&mut self) -> Option<Result<ChatChunk, LlmError>>`
-  `parse_stream_chunk` function L286-331 — `(chunk: &StreamChunk) -> Vec<ChatChunk>`
-  `build_messages` function L335-398 — `(system_prompt: &Option<String>, messages: &[ChatMessage]) -> Vec<Value>`
-  `build_tools` function L400-414 — `(tools: &[ToolDefinition]) -> Vec<Value>`
-  `ApiErrorResponse` struct L419-421 — `{ error: Option<ApiError> }`
-  `ApiError` struct L424-428 — `{ message: String, code: Option<String> }`
-  `StreamChunk` struct L431-436 — `{ choices: Vec<StreamChoice>, usage: Option<StreamUsage> }`
-  `StreamChoice` struct L439-441 — `{ delta: StreamDelta }`
-  `StreamDelta` struct L444-447 — `{ content: Option<String>, tool_calls: Option<Vec<StreamToolCall>> }`
-  `StreamToolCall` struct L450-453 — `{ id: Option<String>, function: Option<StreamFunction> }`
-  `StreamFunction` struct L456-459 — `{ name: Option<String>, arguments: Option<String> }`
-  `StreamUsage` struct L462-465 — `{ prompt_tokens: u32, completion_tokens: u32 }`
-  `tests` module L468-610 — `-`
-  `groq_convenience_constructor` function L473-478 — `()`
-  `ollama_convenience_constructor` function L481-486 — `()`
-  `openai_convenience_constructor` function L489-493 — `()`
-  `custom_base_url` function L496-503 — `()`
-  `from_config_known_providers` function L506-510 — `()`
-  `from_config_custom_url_override` function L513-520 — `()`
-  `build_messages_with_system_prompt` function L523-536 — `()`
-  `parse_text_delta` function L539-552 — `()`
-  `parse_tool_use_start` function L555-574 — `()`
-  `parse_usage` function L577-588 — `()`
-  `no_auth_header_when_no_api_key` function L591-609 — `()`

#### crates/arawn-llm/src/retry.rs

- pub `RetryClient` struct L17-21 — `{ inner: Arc<dyn LlmClient>, max_retries: u32, base_delay_ms: u64 }` — Wraps any LlmClient and adds retry with exponential backoff for transient errors.
- pub `new` function L24-30 — `(inner: Arc<dyn LlmClient>) -> Self`
- pub `with_config` function L32-38 — `(inner: Arc<dyn LlmClient>, max_retries: u32, base_delay_ms: u64) -> Self`
-  `DEFAULT_MAX_RETRIES` variable L13 — `: u32`
-  `DEFAULT_BASE_DELAY_MS` variable L14 — `: u64`
-  `RetryClient` type L23-43 — `= RetryClient`
-  `delay_for_attempt` function L40-42 — `(&self, attempt: u32) -> Duration`
-  `RetryClient` type L46-84 — `impl LlmClient for RetryClient`
-  `stream` function L47-83 — `( &self, request: ChatRequest, ) -> Result<Pin<Box<dyn Stream<Item = Result<Chat...`
-  `tests` module L87-271 — `-`
-  `FailThenSucceed` struct L96-100 — `{ failures_remaining: Mutex<u32>, error_type: LlmError, success_response: Vec<Ch...` — A mock that fails N times then succeeds.
-  `FailThenSucceed` type L103-118 — `impl LlmClient for FailThenSucceed`
-  `stream` function L104-117 — `( &self, _request: ChatRequest, ) -> Result<Pin<Box<dyn Stream<Item = Result<Cha...`
-  `dummy_request` function L120-128 — `() -> ChatRequest`
-  `succeeds_on_first_try` function L131-141 — `()`
-  `retries_on_server_error_then_succeeds` function L144-164 — `()`
-  `gives_up_after_max_retries` function L167-182 — `()`
-  `does_not_retry_terminal_errors` function L185-209 — `()`
-  `AlwaysBadRequest` struct L187 — `-`
-  `AlwaysBadRequest` type L190-198 — `impl LlmClient for AlwaysBadRequest`
-  `stream` function L191-197 — `( &self, _request: ChatRequest, ) -> Result<Pin<Box<dyn Stream<Item = Result<Cha...`
-  `retries_rate_limit_errors` function L212-270 — `()`
-  `RateLimitThenSucceed` struct L225-227 — `{ inner: FailThenSucceed }`
-  `RateLimitThenSucceed` type L230-245 — `impl LlmClient for RateLimitThenSucceed`
-  `stream` function L231-244 — `( &self, request: ChatRequest, ) -> Result<Pin<Box<dyn Stream<Item = Result<Chat...`

#### crates/arawn-llm/src/types.rs

- pub `ChatRequest` struct L6-13 — `{ model: String, system_prompt: Option<String>, messages: Vec<ChatMessage>, tool...` — Provider-neutral chat request.
- pub `ChatMessage` struct L17-24 — `{ role: String, content: ChatContent, tool_calls: Vec<ToolCall>, tool_call_id: O...` — Provider-neutral message for chat requests.
- pub `ChatContent` enum L29-31 — `Text` — Message content — text or structured.
- pub `ToolCall` struct L35-39 — `{ id: String, name: String, arguments: Value }` — A tool call within an assistant message.
- pub `ToolDefinition` struct L43-47 — `{ name: String, description: String, parameters: Value }` — Tool definition sent with the request.
- pub `ChatChunk` enum L51-56 — `TextDelta | ToolUseStart | ToolUseInputDelta | Done` — Streaming chunk from the LLM.
- pub `Usage` struct L60-63 — `{ input_tokens: u32, output_tokens: u32 }` — Token usage statistics.

#### crates/arawn-llm/src/warming.rs

- pub `DEFAULT_WARMUP_TTL` variable L27 — `: Duration` — Default TTL chosen for Ollama Cloud, which unloads idle models aggressively.
- pub `WarmingClient` struct L31-40 — `{ inner: Arc<dyn LlmClient>, provider: String, ttl: Duration, last_warmup: Mutex...` — Wraps any [`LlmClient`] with TTL-based warmup caching and a one-shot
- pub `new` function L43-45 — `(inner: Arc<dyn LlmClient>, provider: impl Into<String>) -> Self` — Pool layering: raw provider → `RetryClient` → `WarmingClient`.
- pub `with_ttl` function L47-58 — `( inner: Arc<dyn LlmClient>, provider: impl Into<String>, ttl: Duration, ) -> Se...` — Pool layering: raw provider → `RetryClient` → `WarmingClient`.
- pub `last_warmup_for_test` function L86-88 — `(&self) -> Option<Instant>` — Returns the cached `last_warmup` timestamp.
-  `WarmingClient` type L42-89 — `= WarmingClient` — Pool layering: raw provider → `RetryClient` → `WarmingClient`.
-  `ensure_warm` function L62-77 — `(&self, model: &str) -> Result<(), LlmError>` — Ensure the cached warmup is fresh.
-  `invalidate` function L79-82 — `(&self)` — Pool layering: raw provider → `RetryClient` → `WarmingClient`.
-  `looks_like_cold_restart` function L94-96 — `(err: &LlmError) -> bool` — Errors that look like the provider unloaded the model and the next request
-  `WarmingClient` type L99-142 — `impl LlmClient for WarmingClient` — Pool layering: raw provider → `RetryClient` → `WarmingClient`.
-  `stream` function L100-131 — `( &self, request: ChatRequest, ) -> Result<Pin<Box<dyn Stream<Item = Result<Chat...` — Pool layering: raw provider → `RetryClient` → `WarmingClient`.
-  `warmup` function L133-141 — `(&self, model: &str) -> Result<(), LlmError>` — Pool layering: raw provider → `RetryClient` → `WarmingClient`.
-  `tests` module L145-339 — `-` — Pool layering: raw provider → `RetryClient` → `WarmingClient`.
-  `ok_response` function L151-158 — `() -> MockResponse` — Pool layering: raw provider → `RetryClient` → `WarmingClient`.
-  `user_request` function L160-173 — `(model: &str) -> ChatRequest` — Pool layering: raw provider → `RetryClient` → `WarmingClient`.
-  `CountingClient` struct L178-181 — `{ inner: MockLlmClient, calls: AtomicUsize }` — Counts how many times `stream` was invoked on the inner client.
-  `CountingClient` type L183-194 — `= CountingClient` — Pool layering: raw provider → `RetryClient` → `WarmingClient`.
-  `new` function L184-189 — `(responses: Vec<MockResponse>) -> Self` — Pool layering: raw provider → `RetryClient` → `WarmingClient`.
-  `calls` function L191-193 — `(&self) -> usize` — Pool layering: raw provider → `RetryClient` → `WarmingClient`.
-  `CountingClient` type L197-208 — `impl LlmClient for CountingClient` — Pool layering: raw provider → `RetryClient` → `WarmingClient`.
-  `stream` function L198-207 — `( &self, request: ChatRequest, ) -> Result< Pin<Box<dyn Stream<Item = Result<Cha...` — Pool layering: raw provider → `RetryClient` → `WarmingClient`.
-  `warmup_probes_inner_and_caches` function L211-219 — `()` — Pool layering: raw provider → `RetryClient` → `WarmingClient`.
-  `stream_skips_warmup_when_cache_fresh` function L222-232 — `()` — Pool layering: raw provider → `RetryClient` → `WarmingClient`.
-  `stream_warms_lazily_when_cache_empty` function L235-245 — `()` — Pool layering: raw provider → `RetryClient` → `WarmingClient`.
-  `stream_re_warms_after_ttl_expiry` function L248-273 — `()` — Pool layering: raw provider → `RetryClient` → `WarmingClient`.
-  `stream_retries_once_on_cold_restart_signature` function L276-291 — `()` — Pool layering: raw provider → `RetryClient` → `WarmingClient`.
-  `stream_does_not_retry_on_non_cold_restart_errors` function L294-307 — `()` — Pool layering: raw provider → `RetryClient` → `WarmingClient`.
-  `warmup_failure_does_not_update_cache` function L310-322 — `()` — Pool layering: raw provider → `RetryClient` → `WarmingClient`.
-  `cold_restart_classifier` function L325-338 — `()` — Pool layering: raw provider → `RetryClient` → `WarmingClient`.

### crates/arawn-mcp/src

**Role**: Model Context Protocol integration — connects to external MCP servers as subprocesses, discovers their tools, and exposes each as an arawn `Tool` via an adapter.

**Key abstractions**:
- `McpManager` — Manages the lifecycle of all MCP server connections. `connect_all` iterates enabled `McpServerConfig` entries; `connect_server` spawns the process via stdio, runs the MCP handshake, lists tools, and wraps each in an `McpToolAdapter` registered in the `ToolRegistry`. `disconnect_server` removes the tools by `mcp_{server_name}_` prefix. `sync_servers` diffs the current connection set against a new config list, connecting/disconnecting as needed (used for hot-reload). `reconnect` does exponential backoff with up to `MAX_ATTEMPTS`.
- `McpToolAdapter` — Implements `arawn_tool::Tool` for a single MCP tool. Names are normalized to `mcp_{server}_{tool}` (non-alphanumeric chars replaced with `_`). `is_read_only()` uses the MCP tool's `readOnlyHint` annotation if present. `execute` calls the MCP peer's `call_tool` method and converts the result.
- `McpServerConfig` — One entry in `[[mcp.servers]]` in `arawn.toml`: name, command, args, env, and `enabled` (defaults true). `load_mcp_config` reads these from the TOML file.

**Internal flow**: At startup, `connect_mcp_servers()` in `main.rs` creates a `McpManager`, calls `connect_all`, and the registered adapters appear in the `ToolRegistry` alongside built-in tools. The MCP peer connection is held in `ConnectedServer` and kept alive for the process lifetime. Plugin-contributed MCP servers (from `PluginLoadResult`) are also connected through this same manager.

**Dependencies**: `rmcp` (the MCP client library — provides `Peer`, `RoleClient`, `RunningService`), `arawn-tool` (Tool trait, ToolRegistry).

#### crates/arawn-mcp/src/adapter.rs

- pub `McpToolAdapter` struct L14-23 — `{ arawn_name: String, mcp_name: String, mcp_tool: McpTool, peer: Arc<Peer<RoleCl...` — An arawn Tool backed by an MCP server tool.
- pub `new` function L26-38 — `(server_name: &str, mcp_tool: McpTool, peer: Arc<Peer<RoleClient>>) -> Self` — McpToolAdapter — wraps an MCP tool as an arawn Tool impl.
- pub `tool_name` function L41-43 — `(&self) -> &str` — Get the arawn tool name (for logging before registration).
-  `McpToolAdapter` type L25-44 — `= McpToolAdapter` — McpToolAdapter — wraps an MCP tool as an arawn Tool impl.
-  `McpToolAdapter` type L47-119 — `impl Tool for McpToolAdapter` — McpToolAdapter — wraps an MCP tool as an arawn Tool impl.
-  `name` function L48-50 — `(&self) -> &str` — McpToolAdapter — wraps an MCP tool as an arawn Tool impl.
-  `description` function L52-57 — `(&self) -> &str` — McpToolAdapter — wraps an MCP tool as an arawn Tool impl.
-  `parameters_schema` function L59-66 — `(&self) -> Value` — McpToolAdapter — wraps an MCP tool as an arawn Tool impl.
-  `is_read_only` function L68-74 — `(&self) -> bool` — McpToolAdapter — wraps an MCP tool as an arawn Tool impl.
-  `execute` function L76-118 — `(&self, _ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput,...` — McpToolAdapter — wraps an MCP tool as an arawn Tool impl.
-  `normalize_name` function L122-132 — `(name: &str) -> String` — Normalize a name for use in tool naming — replace non-alphanumeric chars with _
-  `tests` module L135-150 — `-` — McpToolAdapter — wraps an MCP tool as an arawn Tool impl.
-  `normalize_simple` function L139-142 — `()` — McpToolAdapter — wraps an MCP tool as an arawn Tool impl.
-  `normalize_special_chars` function L145-149 — `()` — McpToolAdapter — wraps an MCP tool as an arawn Tool impl.

#### crates/arawn-mcp/src/config.rs

- pub `McpConfig` struct L9-12 — `{ servers: Vec<McpServerConfig> }` — Top-level MCP configuration section from arawn.toml.
- pub `McpServerConfig` struct L16-30 — `{ name: String, command: String, args: Vec<String>, env: HashMap<String, String>...` — Configuration for a single MCP server.
- pub `load_mcp_config` function L37-61 — `(path: &std::path::Path) -> McpConfig` — Load MCP config from an arawn.toml file.
-  `default_true` function L32-34 — `() -> bool` — MCP server configuration — parsed from arawn.toml [[mcp.servers]] entries.
-  `TomlWrapper` struct L43-46 — `{ mcp: McpConfig }` — MCP server configuration — parsed from arawn.toml [[mcp.servers]] entries.
-  `tests` module L64-131 — `-` — MCP server configuration — parsed from arawn.toml [[mcp.servers]] entries.
-  `parse_mcp_config` function L68-94 — `()` — MCP server configuration — parsed from arawn.toml [[mcp.servers]] entries.
-  `W` struct L83-86 — `{ mcp: McpConfig }` — MCP server configuration — parsed from arawn.toml [[mcp.servers]] entries.
-  `empty_config` function L97-109 — `()` — MCP server configuration — parsed from arawn.toml [[mcp.servers]] entries.
-  `W` struct L103-106 — `{ mcp: McpConfig }` — MCP server configuration — parsed from arawn.toml [[mcp.servers]] entries.
-  `config_with_env` function L112-130 — `()` — MCP server configuration — parsed from arawn.toml [[mcp.servers]] entries.
-  `W` struct L121-124 — `{ mcp: McpConfig }` — MCP server configuration — parsed from arawn.toml [[mcp.servers]] entries.

#### crates/arawn-mcp/src/lib.rs

- pub `adapter` module L1 — `-`
- pub `config` module L2 — `-`
- pub `manager` module L3 — `-`

#### crates/arawn-mcp/src/manager.rs

- pub `McpManager` struct L40-42 — `{ servers: HashMap<String, ConnectedServer> }` — Manages all MCP server connections.
- pub `new` function L51-55 — `() -> Self` — registers them in the ToolRegistry, and handles reconnection.
- pub `connect_all` function L58-70 — `( &mut self, configs: &[McpServerConfig], registry: &Arc<ToolRegistry>, )` — Connect to all enabled servers and discover their tools.
- pub `connect_server` function L73-111 — `( &mut self, config: &McpServerConfig, registry: &Arc<ToolRegistry>, )` — Connect to a single MCP server.
- pub `disconnect_server` function L114-125 — `(&mut self, name: &str, registry: &Arc<ToolRegistry>)` — Disconnect a server and unregister its tools.
- pub `sync_servers` function L128-152 — `( &mut self, configs: &[McpServerConfig], registry: &Arc<ToolRegistry>, )` — Diff current servers against a new config and connect/disconnect as needed.
- pub `reconnect` function L155-202 — `( &mut self, server_name: &str, registry: &Arc<ToolRegistry>, ) -> bool` — Attempt to reconnect a failed server with exponential backoff.
- pub `connected_servers` function L205-207 — `(&self) -> Vec<&str>` — Get the names of all connected servers.
- pub `tool_count` function L210-212 — `(&self) -> usize` — Get tool count across all servers.
- pub `system_prompt` function L215-254 — `(&self) -> String` — Generate a system prompt section describing connected MCP servers and their tools.
-  `ArawnClientHandler` struct L19 — `-` — Handler for MCP client notifications.
-  `ArawnClientHandler` type L21-28 — `impl ClientHandler for ArawnClientHandler` — registers them in the ToolRegistry, and handles reconnection.
-  `get_info` function L22-27 — `(&self) -> ClientInfo` — registers them in the ToolRegistry, and handles reconnection.
-  `ConnectedServer` struct L31-37 — `{ config: McpServerConfig, _service: RunningService<RoleClient, ArawnClientHandl...` — State of a connected MCP server.
-  `McpManager` type L44-48 — `impl Default for McpManager` — registers them in the ToolRegistry, and handles reconnection.
-  `default` function L45-47 — `() -> Self` — registers them in the ToolRegistry, and handles reconnection.
-  `McpManager` type L50-255 — `= McpManager` — registers them in the ToolRegistry, and handles reconnection.
-  `MAX_ATTEMPTS` variable L167 — `: u32` — registers them in the ToolRegistry, and handles reconnection.
-  `normalize_name` function L257-261 — `(name: &str) -> String` — registers them in the ToolRegistry, and handles reconnection.
-  `spawn_and_connect` function L264-292 — `( config: &McpServerConfig, ) -> Result< ( RunningService<RoleClient, ArawnClien...` — Spawn an MCP server process, connect via stdio, initialize, and discover tools.

### crates/arawn-memory/src

**Role**: Two-tier persistent knowledge base (global + workstream-scoped) with SQLite/FTS5 full-text search, optional vector similarity search via sqlite-vec, confidence scoring, relation graph, and session prompt injection.

**Key abstractions**:
- `MemoryStore` — A single SQLite database with FTS5 virtual table for full-text search, a relations table, and (optionally) a sqlite-vec `vec0` virtual table for embeddings. `store_fact` does search-before-create deduplication: if an entity with the same title already exists it reinforces it (increments count, updates timestamp) rather than inserting. `supersede_entity` links old to new via a `Supersedes` relation and marks the old entity so it is excluded from search and ranking. `list_all_ranked` sorts by `ConfidenceSource` base score × reinforcement log × staleness decay.
- `MemoryManager` — Holds two `Arc<MemoryStore>` instances (global and workstream). Routes entities to the right store by their `EntityType::default_scope()` or an explicit `Scope` override. `retrieve_topical` searches both tiers and merges results. `store_fact_embedded` stores the entity then calls the `Embedder` to generate and persist an embedding.
- `MemoryStack` — Three-layer context renderer for system prompt injection: L0 (identity layer: workstream name + Person/Convention entities), L1 (essential story: top-ranked entities by type, within token budget), L2 (on-demand topic-triggered retrieval via `topical_context`). `wake_up()` returns L0 + L1. L1 also applies `shortcodes` compression to repeated entity names.
- `Entity` — The stored unit: `id`, `entity_type`, `title`, `content`, `confidence_source`, `reinforcement_count`, `updated_at`, `superseded`, `tags`, and optionally `session_id`. `confidence_score()` applies the decay formula `compute_confidence(source, reinforcement, days_since_update, superseded)`.
- `EntityType` — `Fact | Decision | Convention | Preference | Person | Note`. Each has a `default_scope()` (Preference → Global, Convention → Workstream, etc.).
- `RelationType` — Directed graph edges: `RelatesTo | Contradicts | Supports | Supersedes | ExtractedFrom | Mentions | BelongsTo`.
- `vector.rs` — Low-level sqlite-vec bindings: `init_vector_extension()` (called once at process start), `create_vector_table(dims)`, `store_embedding`, `search_similar`, `search_similar_filtered` (pre-filters to a candidate set before kNN).
- `inject.rs` — `load_memories_for_injection()` retrieves top-N entities from each tier and formats them as one-line strings for inclusion in the system prompt. Called by `LocalService::build_session_context`.

**Internal flow**: Tool `MemoryStoreTool` calls `MemoryManager::store_fact_embedded` → stores in appropriate tier → generates embedding if embedder available. Tool `MemorySearchTool` calls FTS5 search + optional vector search, merges as `ScoredEntity` with composite score, returns top results. At session start `load_memories_for_injection` pulls context into the system prompt.

**Mixed concerns / gotchas**: `shortcodes.rs` operates only on rendered output, never on stored data. The `MemoryStack` L2 deduplicates against L1 titles to avoid repeating context. `try_open_memory` returns `None` rather than panicking if the database cannot be opened, enabling graceful degradation.

**Dependencies**: `rusqlite` (SQLite + FTS5), `sqlite-vec` extension (vector search), `arawn-embed` (Embedder trait), `uuid`, `chrono`.

#### crates/arawn-memory/src/cypher_schema.rs

- pub `entity_label` function L24-33 — `(t: EntityType) -> &'static str` — Cypher node label for an `EntityType`.
- pub `entity_type_from_label` function L36-46 — `(s: &str) -> Option<EntityType>` — Inverse of `entity_label`.
- pub `relation_type_str` function L49-59 — `(t: RelationType) -> &'static str` — Cypher relationship type for a `RelationType`.
- pub `relation_type_from_str` function L62-73 — `(s: &str) -> Option<RelationType>` — Inverse of `relation_type_str`.
- pub `entity_to_props` function L79-94 — `(e: &Entity) -> JsonValue` — Project an `Entity` into a Cypher parameter map (`$props`).
- pub `node_to_entity` function L100-178 — `(node: &Value) -> Result<Entity, MemoryError>` — Parse a node `Value` (as returned by `MATCH (n) RETURN n`) into an `Entity`.
-  `tests` module L181-222 — `-` — user input.
-  `label_roundtrip` function L185-196 — `()` — user input.
-  `relation_roundtrip` function L199-211 — `()` — user input.
-  `entity_to_props_serializes_tags_as_json_string` function L214-221 — `()` — user input.

#### crates/arawn-memory/src/error.rs

- pub `MemoryError` enum L4-13 — `Storage | NotFound | Validation`

#### crates/arawn-memory/src/inject.rs

- pub `load_memories_for_injection` function L15-91 — `( memory: &MemoryManager, global_limit: Option<usize>, workstream_limit: Option<...` — Load relevant entities from both KB tiers and format as strings
-  `DEFAULT_GLOBAL_LIMIT` variable L7 — `: usize` — Default limits for entities injected per tier.
-  `DEFAULT_WORKSTREAM_LIMIT` variable L8 — `: usize` — Session injection — format KB entities for system prompt context.
-  `format_entity_line` function L93-114 — `(entity: &crate::types::Entity) -> String` — Session injection — format KB entities for system prompt context.
-  `tests` module L117-196 — `-` — Session injection — format KB entities for system prompt context.
-  `setup` function L122-127 — `() -> (TempDir, MemoryManager)` — Session injection — format KB entities for system prompt context.
-  `empty_kb_returns_empty` function L130-134 — `()` — Session injection — format KB entities for system prompt context.
-  `injects_global_preferences` function L137-151 — `()` — Session injection — format KB entities for system prompt context.
-  `injects_workstream_conventions` function L154-169 — `()` — Session injection — format KB entities for system prompt context.
-  `both_tiers_injected` function L172-183 — `()` — Session injection — format KB entities for system prompt context.
-  `reinforcement_shown` function L186-195 — `()` — Session injection — format KB entities for system prompt context.

#### crates/arawn-memory/src/lib.rs

- pub `cypher_schema` module L6 — `-` — Provides graph-backed entity storage with FTS5 search, typed relations,
- pub `error` module L7 — `-` — confidence scoring, tag support, and search-before-create deduplication.
- pub `inject` module L8 — `-` — confidence scoring, tag support, and search-before-create deduplication.
- pub `manager` module L9 — `-` — confidence scoring, tag support, and search-before-create deduplication.
- pub `shortcodes` module L10 — `-` — confidence scoring, tag support, and search-before-create deduplication.
- pub `stack` module L11 — `-` — confidence scoring, tag support, and search-before-create deduplication.
- pub `store` module L12 — `-` — confidence scoring, tag support, and search-before-create deduplication.
- pub `types` module L13 — `-` — confidence scoring, tag support, and search-before-create deduplication.
- pub `vector` module L14 — `-` — confidence scoring, tag support, and search-before-create deduplication.
-  `graphqlite_smoke` module L27-51 — `-` — confidence scoring, tag support, and search-before-create deduplication.
-  `graphqlite_node_and_edge_roundtrip` function L31-50 — `()` — confidence scoring, tag support, and search-before-create deduplication.

#### crates/arawn-memory/src/manager.rs

- pub `MemoryManager` struct L19-28 — `{ global: Arc<MemoryStore>, workstream: Arc<MemoryStore>, vectors_enabled: bool,...` — Two-tier memory manager holding global and workstream knowledge bases.
- pub `open` function L34-71 — `(data_dir: &Path, ws_dir: &str, embedding_dims: Option<usize>) -> Result<Self, M...` — Open both KB tiers.
- pub `for_workstream` function L77-83 — `( data_dir: &Path, workstream_name: &str, embedding_dims: Option<usize>, ) -> Re...` — Convenience wrapper: open a memory manager scoped to a named
- pub `open_with_stores` function L86-93 — `(global: Arc<MemoryStore>, workstream: Arc<MemoryStore>) -> Self` — Create a MemoryManager from pre-built stores (for testing).
- pub `with_embedder` function L96-99 — `(mut self, embedder: Arc<dyn Embedder>) -> Self` — Attach an embedder for automatic embedding on ingest and vector-enhanced retrieval.
- pub `embedder` function L102-104 — `(&self) -> Option<&Arc<dyn Embedder>>` — Get the embedder if available.
- pub `store_fact_embedded` function L109-143 — `( &self, entity: &Entity, scope: Option<Scope>, ) -> Result<StoreFactResult, Mem...` — Store a fact with automatic embedding.
- pub `store_for` function L146-151 — `(&self, scope: Scope) -> &Arc<MemoryStore>` — Get the store for a given scope.
- pub `store_for_type` function L154-156 — `(&self, entity_type: EntityType) -> &Arc<MemoryStore>` — Get the store for a given entity type (uses default scope).
- pub `vectors_enabled` function L159-161 — `(&self) -> bool` — Whether vector storage is available.
- pub `retrieve_topical` function L166-256 — `( &self, keywords: &[String], budget_tokens: usize, ) -> Vec<crate::types::Entit...` — Retrieve entities matching keywords from both tiers.
- pub `try_open_memory` function L260-272 — `( data_dir: &Path, ws_dir: &str, embedding_dims: Option<usize>, ) -> Option<Arc<...` — Try to open a MemoryManager, returning None on failure (graceful degradation).
-  `MemoryManager` type L30-257 — `= MemoryManager` — It abstracts the two-tier scoping and routes entities to the appropriate store.
-  `tests` module L275-382 — `-` — It abstracts the two-tier scoping and routes entities to the appropriate store.
-  `setup` function L280-285 — `() -> (TempDir, MemoryManager)` — It abstracts the two-tier scoping and routes entities to the appropriate store.
-  `setup_with_vectors` function L287-292 — `() -> (TempDir, MemoryManager)` — It abstracts the two-tier scoping and routes entities to the appropriate store.
-  `opens_both_stores` function L295-304 — `()` — It abstracts the two-tier scoping and routes entities to the appropriate store.
-  `scope_routing` function L307-337 — `()` — It abstracts the two-tier scoping and routes entities to the appropriate store.
-  `vectors_disabled_by_default` function L340-343 — `()` — It abstracts the two-tier scoping and routes entities to the appropriate store.
-  `vectors_enabled_with_dims` function L346-357 — `()` — It abstracts the two-tier scoping and routes entities to the appropriate store.
-  `graceful_degradation` function L360-364 — `()` — It abstracts the two-tier scoping and routes entities to the appropriate store.
-  `stores_are_independent` function L367-381 — `()` — It abstracts the two-tier scoping and routes entities to the appropriate store.

#### crates/arawn-memory/src/shortcodes.rs

- pub `apply_shortcodes` function L15-79 — `(text: &str, entity_names: &[String], min_occurrences: usize) -> String` — Scan text for repeated entity-like names and replace with shortcodes.
-  `count_occurrences` function L82-87 — `(haystack: &str, needle: &str) -> usize` — Count non-overlapping occurrences of needle in haystack.
-  `generate_code` function L91-104 — `(name: &str) -> String` — Generate a shortcode from a name: first letter of each word, uppercased.
-  `tests` module L107-158 — `-` — Applied only to rendered output, never to storage.
-  `compresses_repeated_names` function L111-119 — `()` — Applied only to rendered output, never to storage.
-  `skips_single_occurrence` function L122-129 — `()` — Applied only to rendered output, never to storage.
-  `handles_collision` function L132-140 — `()` — Applied only to rendered output, never to storage.
-  `empty_names_returns_unchanged` function L143-147 — `()` — Applied only to rendered output, never to storage.
-  `multi_word_name` function L150-157 — `()` — Applied only to rendered output, never to storage.

#### crates/arawn-memory/src/stack.rs

- pub `MemoryStack` struct L16-19 — `{ manager: &'a MemoryManager, workstream_name: String }` — Layered memory stack.
- pub `new` function L22-27 — `(manager: &'a MemoryManager, workstream_name: &str) -> Self` — L2: On-demand — topic-triggered retrieval (separate method)
- pub `wake_up` function L31-52 — `(&self, budget_tokens: usize) -> String` — Generate L0 + L1 memory context within the given token budget.
- pub `l1_entity_titles` function L128-140 — `(&self) -> Vec<String>` — Get the entity titles included in L1 (for L2 deduplication).
- pub `topical_context` function L144-170 — `( &self, keywords: &[String], l1_titles: &[String], budget_tokens: usize, ) -> O...` — L2: Topic-triggered context.
-  `estimate_tokens` function L11-13 — `(text: &str) -> usize` — Estimate token count from text length (matches arawn-engine's TokenEstimator).
-  `render_l0` function L55-73 — `(&self) -> String` — L0: Identity layer — workstream name + Person/Convention entities.
-  `render_l1_with_names` function L77-125 — `(&self, budget_tokens: usize) -> (String, Vec<String>)` — L1: Essential story — top-ranked entities grouped by type, within budget.
-  `format_entity_brief` function L173-183 — `(entity: &Entity) -> String` — L2: On-demand — topic-triggered retrieval (separate method)
-  `tests` module L186-256 — `-` — L2: On-demand — topic-triggered retrieval (separate method)
-  `setup` function L191-196 — `() -> (TempDir, MemoryManager)` — L2: On-demand — topic-triggered retrieval (separate method)
-  `wake_up_respects_budget` function L199-212 — `()` — L2: On-demand — topic-triggered retrieval (separate method)
-  `wake_up_empty_kb` function L215-222 — `()` — L2: On-demand — topic-triggered retrieval (separate method)
-  `l1_ranks_stated_before_inferred` function L225-243 — `()` — L2: On-demand — topic-triggered retrieval (separate method)
-  `tiny_budget_does_not_panic` function L246-255 — `()` — L2: On-demand — topic-triggered retrieval (separate method)

#### crates/arawn-memory/src/store.rs

- pub `MemoryStore` struct L30-32 — `{ conn: Mutex<GraphConnection> }` — Knowledge base store.
- pub `open` function L36-55 — `(path: &Path) -> Result<Self, MemoryError>` — Open or create a memory database at the given path.
- pub `in_memory` function L58-66 — `() -> Result<Self, MemoryError>` — Create an in-memory store (for testing).
- pub `insert_entity` function L108-117 — `(&self, entity: &Entity) -> Result<(), MemoryError>` — sync via explicit Rust dual-writes inside a single sqlite transaction.
- pub `get_entity` function L119-122 — `(&self, id: Uuid) -> Result<Option<Entity>, MemoryError>` — sync via explicit Rust dual-writes inside a single sqlite transaction.
- pub `update_entity` function L124-132 — `(&self, entity: &Entity) -> Result<(), MemoryError>` — sync via explicit Rust dual-writes inside a single sqlite transaction.
- pub `delete_entity` function L134-165 — `(&self, id: Uuid) -> Result<bool, MemoryError>` — sync via explicit Rust dual-writes inside a single sqlite transaction.
- pub `list_by_type` function L167-183 — `( &self, entity_type: EntityType, limit: usize, ) -> Result<Vec<Entity>, MemoryE...` — sync via explicit Rust dual-writes inside a single sqlite transaction.
- pub `list_all_ranked` function L192-211 — `(&self, limit: usize) -> Result<Vec<Entity>, MemoryError>` — List all non-superseded entities ranked by confidence: stated > observed > inferred,
- pub `count_by_type` function L213-228 — `(&self, entity_type: EntityType) -> Result<usize, MemoryError>` — sync via explicit Rust dual-writes inside a single sqlite transaction.
- pub `count_all` function L230-241 — `(&self) -> Result<usize, MemoryError>` — sync via explicit Rust dual-writes inside a single sqlite transaction.
- pub `search` function L249-263 — `(&self, query: &str, limit: usize) -> Result<Vec<Entity>, MemoryError>` — sync via explicit Rust dual-writes inside a single sqlite transaction.
- pub `search_by_type` function L265-284 — `( &self, query: &str, entity_type: EntityType, limit: usize, ) -> Result<Vec<Ent...` — sync via explicit Rust dual-writes inside a single sqlite transaction.
- pub `add_relation` function L288-297 — `( &self, source_id: Uuid, relation_type: RelationType, target_id: Uuid, ) -> Res...` — sync via explicit Rust dual-writes inside a single sqlite transaction.
- pub `get_relations` function L299-338 — `(&self, entity_id: Uuid) -> Result<Vec<Relation>, MemoryError>` — sync via explicit Rust dual-writes inside a single sqlite transaction.
- pub `get_neighbors` function L340-356 — `(&self, entity_id: Uuid) -> Result<Vec<(Uuid, RelationType)>, MemoryError>` — sync via explicit Rust dual-writes inside a single sqlite transaction.
- pub `delete_relation` function L358-393 — `( &self, source_id: Uuid, relation_type: RelationType, target_id: Uuid, ) -> Res...` — sync via explicit Rust dual-writes inside a single sqlite transaction.
- pub `store_fact` function L400-416 — `(&self, entity: &Entity) -> Result<StoreFactResult, MemoryError>` — Store a fact with search-before-create deduplication.
- pub `supersede_entity` function L459-481 — `( &self, old_id: Uuid, new_entity: &Entity, ) -> Result<StoreFactResult, MemoryE...` — Supersede an existing entity with a new one.
- pub `init_vectors` function L487-491 — `(&self, dims: usize) -> Result<(), MemoryError>` — Initialize vector storage with the given dimensions.
- pub `store_embedding` function L494-497 — `(&self, entity_id: Uuid, embedding: &[f32]) -> Result<(), MemoryError>` — Store an embedding for an entity.
- pub `search_similar` function L500-507 — `( &self, query_embedding: &[f32], limit: usize, ) -> Result<Vec<vector::Similari...` — Search for entities similar to a query embedding.
- pub `search_similar_filtered` function L510-518 — `( &self, query_embedding: &[f32], entity_ids: &[Uuid], limit: usize, ) -> Result...` — Search for entities similar to a query, filtered to a subset.
- pub `has_embedding` function L521-524 — `(&self, entity_id: Uuid) -> Result<bool, MemoryError>` — Check if an entity has a stored embedding.
- pub `count_embeddings` function L527-530 — `(&self) -> Result<usize, MemoryError>` — Count total stored embeddings.
- pub `search_by_tags` function L538-555 — `( &self, tags: &[String], limit: usize, ) -> Result<Vec<Entity>, MemoryError>` — Tag search loads all non-superseded entities and filters in Rust.
-  `MemoryStore` type L34-556 — `= MemoryStore` — sync via explicit Rust dual-writes inside a single sqlite transaction.
-  `migrate` function L68-99 — `(&self) -> Result<(), MemoryError>` — sync via explicit Rust dual-writes inside a single sqlite transaction.
-  `reinforce_entity` function L419-456 — `(&self, entity_id: Uuid) -> Result<StoreFactResult, MemoryError>` — Reinforce an existing entity (increment count, refresh timestamps).
-  `with_tx` function L563-579 — `(conn: &GraphConnection, body: F) -> Result<(), MemoryError>` — Run `body` inside a sqlite transaction on the shared connection.
-  `cypher_entity_exists` function L581-593 — `(conn: &GraphConnection, id: &str) -> Result<bool, MemoryError>` — sync via explicit Rust dual-writes inside a single sqlite transaction.
-  `fetch_entity_by_id` function L595-608 — `(conn: &GraphConnection, id: Uuid) -> Result<Option<Entity>, MemoryError>` — sync via explicit Rust dual-writes inside a single sqlite transaction.
-  `cypher_upsert_entity` function L613-651 — `( conn: &GraphConnection, entity: &Entity, ) -> Result<(), MemoryError>` — MERGE-style upsert: create node-with-label if absent, otherwise SET every
-  `cypher_upsert_relation` function L655-691 — `( conn: &GraphConnection, source_id: Uuid, relation_type: RelationType, target_i...` — MERGE-style edge upsert.
-  `rows_to_entities` function L694-702 — `(result: &graphqlite::CypherResult) -> Result<Vec<Entity>, MemoryError>` — Map a `MATCH … RETURN n` result set into `Vec<Entity>`.
-  `fts_upsert` function L708-721 — `(sql: &rusqlite::Connection, entity: &Entity) -> Result<(), MemoryError>` — Upsert the FTS row for an entity.
-  `fts_search` function L728-753 — `( sql: &rusqlite::Connection, query: &str, _scope: Option<()>, limit: usize, ) -...` — FTS5 text search returning ranked entity_ids.
-  `tests` module L756-1020 — `-` — sync via explicit Rust dual-writes inside a single sqlite transaction.
-  `test_store` function L759-761 — `() -> MemoryStore` — sync via explicit Rust dual-writes inside a single sqlite transaction.
-  `insert_and_get` function L764-772 — `()` — sync via explicit Rust dual-writes inside a single sqlite transaction.
-  `get_nonexistent` function L775-778 — `()` — sync via explicit Rust dual-writes inside a single sqlite transaction.
-  `update_entity` function L781-796 — `()` — sync via explicit Rust dual-writes inside a single sqlite transaction.
-  `delete_entity` function L799-810 — `()` — sync via explicit Rust dual-writes inside a single sqlite transaction.
-  `list_by_type` function L813-824 — `()` — sync via explicit Rust dual-writes inside a single sqlite transaction.
-  `count_by_type` function L827-836 — `()` — sync via explicit Rust dual-writes inside a single sqlite transaction.
-  `fts5_search` function L839-852 — `()` — sync via explicit Rust dual-writes inside a single sqlite transaction.
-  `fts5_search_by_type` function L855-865 — `()` — sync via explicit Rust dual-writes inside a single sqlite transaction.
-  `relations_crud` function L868-887 — `()` — sync via explicit Rust dual-writes inside a single sqlite transaction.
-  `store_fact_insert` function L890-898 — `()` — sync via explicit Rust dual-writes inside a single sqlite transaction.
-  `store_fact_reinforce` function L901-914 — `()` — sync via explicit Rust dual-writes inside a single sqlite transaction.
-  `store_fact_reinforce_case_insensitive` function L917-929 — `()` — sync via explicit Rust dual-writes inside a single sqlite transaction.
-  `supersede_entity` function L932-953 — `()` — sync via explicit Rust dual-writes inside a single sqlite transaction.
-  `tags_on_entity` function L956-964 — `()` — sync via explicit Rust dual-writes inside a single sqlite transaction.
-  `search_by_tags` function L967-988 — `()` — sync via explicit Rust dual-writes inside a single sqlite transaction.
-  `superseded_excluded_from_search` function L991-1002 — `()` — sync via explicit Rust dual-writes inside a single sqlite transaction.
-  `fts_row_present_after_insert_and_gone_after_delete` function L1005-1019 — `()` — sync via explicit Rust dual-writes inside a single sqlite transaction.

#### crates/arawn-memory/src/types.rs

- pub `EntityType` enum L10-17 — `Fact | Decision | Convention | Preference | Person | Note` — Type of entity stored in the knowledge base.
- pub `as_str` function L20-29 — `(&self) -> &'static str` — Core types for the knowledge base memory system.
- pub `from_str` function L32-42 — `(s: &str) -> Option<Self>` — Core types for the knowledge base memory system.
- pub `default_scope` function L45-50 — `(&self) -> Scope` — Default scope for this entity type.
- pub `Scope` enum L56-59 — `Global | Workstream` — Which KB tier an entity belongs to.
- pub `RelationType` enum L64-72 — `RelatesTo | Contradicts | Supports | Supersedes | ExtractedFrom | Mentions | Bel...` — Type of relationship between entities.
- pub `as_str` function L75-85 — `(&self) -> &'static str` — Core types for the knowledge base memory system.
- pub `from_str` function L88-99 — `(s: &str) -> Option<Self>` — Core types for the knowledge base memory system.
- pub `ConfidenceSource` enum L105-112 — `Stated | Observed | Inferred` — How confident we are in this entity's accuracy.
- pub `base_score` function L115-121 — `(&self) -> f32` — Core types for the knowledge base memory system.
- pub `as_str` function L123-129 — `(&self) -> &'static str` — Core types for the knowledge base memory system.
- pub `from_str` function L132-139 — `(s: &str) -> Option<Self>` — Core types for the knowledge base memory system.
- pub `compute_confidence` function L143-168 — `( source: ConfidenceSource, reinforcement_count: u32, days_since_update: f64, su...` — Compute confidence score with reinforcement and staleness.
- pub `Entity` struct L172-185 — `{ id: Uuid, entity_type: EntityType, title: String, content: Option<String>, con...` — A knowledge entity stored in the KB.
- pub `new` function L188-204 — `(entity_type: EntityType, title: impl Into<String>) -> Self` — Core types for the knowledge base memory system.
- pub `with_content` function L206-209 — `(mut self, content: impl Into<String>) -> Self` — Core types for the knowledge base memory system.
- pub `with_confidence` function L211-214 — `(mut self, source: ConfidenceSource) -> Self` — Core types for the knowledge base memory system.
- pub `with_tags` function L216-219 — `(mut self, tags: Vec<String>) -> Self` — Core types for the knowledge base memory system.
- pub `with_session` function L221-224 — `(mut self, session_id: Uuid) -> Self` — Core types for the knowledge base memory system.
- pub `confidence_score` function L227-235 — `(&self) -> f32` — Compute the current confidence score.
- pub `Relation` struct L240-245 — `{ source_id: Uuid, relation_type: RelationType, target_id: Uuid, created_at: Dat...` — A directed relation between two entities.
- pub `StoreFactResult` enum L249-262 — `Inserted | Reinforced | Superseded` — Result of a store_fact operation (search-before-create).
-  `EntityType` type L19-51 — `= EntityType` — Core types for the knowledge base memory system.
-  `RelationType` type L74-100 — `= RelationType` — Core types for the knowledge base memory system.
-  `ConfidenceSource` type L114-140 — `= ConfidenceSource` — Core types for the knowledge base memory system.
-  `Entity` type L187-236 — `= Entity` — Core types for the knowledge base memory system.
-  `tests` module L265-345 — `-` — Core types for the knowledge base memory system.
-  `entity_type_roundtrip` function L269-280 — `()` — Core types for the knowledge base memory system.
-  `relation_type_roundtrip` function L283-295 — `()` — Core types for the knowledge base memory system.
-  `confidence_stated_fresh` function L298-301 — `()` — Core types for the knowledge base memory system.
-  `confidence_reinforced` function L304-308 — `()` — Core types for the knowledge base memory system.
-  `confidence_stale` function L311-315 — `()` — Core types for the knowledge base memory system.
-  `confidence_superseded_is_zero` function L318-321 — `()` — Core types for the knowledge base memory system.
-  `entity_builder` function L324-334 — `()` — Core types for the knowledge base memory system.
-  `default_scopes` function L337-344 — `()` — Core types for the knowledge base memory system.

#### crates/arawn-memory/src/vector.rs

- pub `init_vector_extension` function L15-23 — `()` — Initialize sqlite-vec extension globally for all connections.
- pub `check_vector_extension` function L26-29 — `(conn: &Connection) -> Result<String, MemoryError>` — Check if sqlite-vec extension is loaded.
- pub `create_vector_table` function L32-43 — `(conn: &Connection, dims: usize) -> Result<(), MemoryError>` — Create the vector embeddings table with the given dimensions.
- pub `drop_vector_table` function L46-50 — `(conn: &Connection) -> Result<(), MemoryError>` — Drop the vector embeddings table (for reindex).
- pub `store_embedding` function L53-72 — `( conn: &Connection, entity_id: Uuid, embedding: &[f32], ) -> Result<(), MemoryE...` — Store an embedding for an entity.
- pub `delete_embedding` function L75-83 — `(conn: &Connection, entity_id: Uuid) -> Result<bool, MemoryError>` — Delete an embedding for an entity.
- pub `has_embedding` function L86-95 — `(conn: &Connection, entity_id: Uuid) -> Result<bool, MemoryError>` — Check if an embedding exists for an entity.
- pub `count_embeddings` function L98-105 — `(conn: &Connection) -> Result<usize, MemoryError>` — Count total stored embeddings.
- pub `SimilarityResult` struct L109-113 — `{ entity_id: Uuid, distance: f32 }` — Result of a similarity search.
- pub `search_similar` function L117-151 — `( conn: &Connection, query_embedding: &[f32], limit: usize, ) -> Result<Vec<Simi...` — Search for entities similar to a query embedding.
- pub `search_similar_filtered` function L154-209 — `( conn: &Connection, query_embedding: &[f32], entity_ids: &[Uuid], limit: usize,...` — Search for entities similar to a query, filtered to a subset of entity IDs.
-  `tests` module L212-333 — `-` — SQLite extension (vec0 virtual tables).
-  `test_conn` function L215-220 — `() -> Connection` — SQLite extension (vec0 virtual tables).
-  `extension_loads` function L223-228 — `()` — SQLite extension (vec0 virtual tables).
-  `store_and_check` function L231-237 — `()` — SQLite extension (vec0 virtual tables).
-  `delete_embedding_works` function L240-246 — `()` — SQLite extension (vec0 virtual tables).
-  `similarity_search` function L249-263 — `()` — SQLite extension (vec0 virtual tables).
-  `similarity_search_with_limit` function L266-273 — `()` — SQLite extension (vec0 virtual tables).
-  `update_embedding` function L276-286 — `()` — SQLite extension (vec0 virtual tables).
-  `filtered_search` function L289-303 — `()` — SQLite extension (vec0 virtual tables).
-  `filtered_search_empty` function L306-310 — `()` — SQLite extension (vec0 virtual tables).
-  `search_empty_table` function L313-317 — `()` — SQLite extension (vec0 virtual tables).
-  `delete_nonexistent` function L320-323 — `()` — SQLite extension (vec0 virtual tables).
-  `drop_and_recreate` function L326-332 — `()` — SQLite extension (vec0 virtual tables).

### crates/arawn-memory/tests

**Role**: Offline evaluation harnesses for the memory retrieval system — not part of the regular test suite (both are `#[ignore]` or require external datasets), used to measure retrieval quality against benchmarks.

**Key abstractions**:
- `recall_eval.rs` — Builds a fixture `MemoryStore` with realistic entities and runs Recall@K, Precision@K, and MRR metrics across five query categories (ExactTitle, KeywordOverlap, ContentSearch, Paraphrase, Negative). Covers FTS5 search, `MemoryStack` L1/L2 behavior, superseded entity exclusion, reinforcement ranking, and (optionally) real vector search. Not `#[ignore]` — these run as integration tests.
- `longmemeval_bench.rs` — Adapts the LongMemEval benchmark (a multi-session memory recall dataset requiring a model download). Uses Reciprocal Rank Fusion to merge FTS5 and temporal proximity signals. Marked `#[ignore]` by default because it requires the dataset download (~5 min) and model inference.

**Mixed concerns / gotchas**: `recall_eval.rs` contains a `vector_search_recall_real_embeddings` test that only runs if the `LOCAL_EMBEDDER` env var is set, because it requires a real ONNX model on disk.

#### crates/arawn-memory/tests/longmemeval_bench.rs

-  `reciprocal_rank_fusion` function L25-38 — `( ranked_lists: &[Vec<&str>], k: f64, ) -> Vec<(String, f64)>` — Reciprocal Rank Fusion: merge multiple ranked lists into one.
-  `parse_date_to_days` function L41-52 — `(date_str: &str) -> Option<f64>` — Parse a LongMemEval date string like "2023/01/15 (Sun) 10:20" into days-since-epoch.
-  `temporal_score` function L56-69 — `(question_days: f64, session_days: f64) -> f64` — Temporal proximity score: higher for sessions closer in time to the question.
-  `LongMemEvalEntry` struct L76-93 — `{ question_id: Option<String>, question: String, question_date: Option<String>, ...` — (ignored by default since it requires model download and takes ~5 minutes)
-  `LongMemEvalEntry` type L95-103 — `= LongMemEvalEntry` — (ignored by default since it requires model download and takes ~5 minutes)
-  `ground_truth_ids` function L96-102 — `(&self) -> &[String]` — (ignored by default since it requires model download and takes ~5 minutes)
-  `Turn` struct L106-109 — `{ role: String, content: String }` — (ignored by default since it requires model download and takes ~5 minutes)
-  `recall_any_at_k` function L116-122 — `(retrieved_ids: &[&str], ground_truth_ids: &[String], k: usize) -> f64` — Recall@K (any): at least one ground-truth session appears in top-K.
-  `recall_all_at_k` function L125-131 — `(retrieved_ids: &[&str], ground_truth_ids: &[String], k: usize) -> f64` — Recall@K (all): all ground-truth sessions appear in top-K.
-  `ndcg_at_k` function L134-158 — `(retrieved_ids: &[&str], ground_truth_ids: &[String], k: usize) -> f64` — NDCG@K: Normalized Discounted Cumulative Gain.
-  `DATASET_URL` variable L164 — `: &str` — (ignored by default since it requires model download and takes ~5 minutes)
-  `dataset_path` function L166-170 — `() -> PathBuf` — (ignored by default since it requires model download and takes ~5 minutes)
-  `download_dataset` function L172-194 — `() -> Result<PathBuf, String>` — (ignored by default since it requires model download and takes ~5 minutes)
-  `load_dataset` function L196-199 — `(path: &PathBuf) -> Vec<LongMemEvalEntry>` — (ignored by default since it requires model download and takes ~5 minutes)
-  `longmemeval_benchmark` function L207-427 — `()` — (ignored by default since it requires model download and takes ~5 minutes)

#### crates/arawn-memory/tests/recall_eval.rs

-  `recall_at_k` function L16-26 — `(results: &[Entity], expected_titles: &[&str], k: usize) -> f64` — Recall@K: fraction of expected entities found in the top-K results.
-  `precision_at_k` function L29-37 — `(results: &[Entity], expected_titles: &[&str], k: usize) -> f64` — Precision@K: fraction of top-K results that are in the expected set.
-  `mrr` function L40-48 — `(results: &[Entity], expected_titles: &[&str]) -> f64` — Mean Reciprocal Rank: 1/rank of the first relevant result.
-  `build_fixture_store` function L55-209 — `() -> Arc<MemoryStore>` — Build a populated MemoryStore with realistic entities for evaluation.
-  `build_fixture_manager` function L212-219 — `() -> (Arc<MemoryStore>, MemoryManager)` — Build a MemoryManager for stack tests using the fixture store.
-  `QueryCase` struct L225-230 — `{ description: &'static str, query: &'static str, expected: Vec<&'static str>, c...` — topical retrieval.
-  `QueryCategory` enum L233-239 — `ExactTitle | KeywordOverlap | ContentSearch | Paraphrase | Negative` — topical retrieval.
-  `build_query_corpus` function L241-399 — `() -> Vec<QueryCase>` — topical retrieval.
-  `fts_recall_evaluation` function L406-512 — `()` — topical retrieval.
-  `memory_stack_l1_coverage` function L515-547 — `()` — topical retrieval.
-  `memory_stack_l2_topical_retrieval` function L550-595 — `()` — topical retrieval.
-  `superseded_entities_excluded_from_all_searches` function L598-616 — `()` — topical retrieval.
-  `reinforcement_boosts_ranking` function L619-644 — `()` — topical retrieval.
-  `edge_case_very_short_query` function L647-659 — `()` — topical retrieval.
-  `edge_case_no_matches` function L662-670 — `()` — topical retrieval.
-  `vector_search_recall_real_embeddings` function L677-855 — `()` — topical retrieval.

### crates/arawn-projections/src

> *Semantic summary to be generated by AI agent.*

#### crates/arawn-projections/src/atlassian.rs

- pub `JIRA_ISSUES` variable L33 — `: &str` — ```
- pub `JIRA_COMMENTS` variable L34 — `: &str` — ```
- pub `JIRA_HISTORY` variable L35 — `: &str` — ```
- pub `CONFLUENCE_PAGES` variable L36 — `: &str` — ```
- pub `JiraIssueProjection` struct L39-52 — `{ id: String, feed_id: String, source_id: String, source_ts: DateTime<Utc>, proj...` — ```
- pub `JiraCommentProjection` struct L92-100 — `{ id: String, feed_id: String, source_id: String, source_ts: DateTime<Utc>, issu...` — ```
- pub `JiraHistoryProjection` struct L130-140 — `{ id: String, feed_id: String, source_id: String, source_ts: DateTime<Utc>, issu...` — ```
- pub `ConfluencePageProjection` struct L180-191 — `{ id: String, feed_id: String, source_id: String, source_ts: DateTime<Utc>, spac...` — ```
- pub `walk_jira_feed_dir` function L235-242 — `( feed_id: &str, feed_dir: &Path, ) -> Result<JiraWalkResult, ProjectionError>` — Walk a Jira feed dir.
- pub `JiraWalkResult` struct L245-249 — `{ issues: Vec<JiraIssueProjection>, comments: Vec<JiraCommentProjection>, histor...` — ```
- pub `walk_confluence_feed_dir` function L522-602 — `( feed_id: &str, feed_dir: &Path, ) -> Result<Vec<ConfluencePageProjection>, Pro...` — Walk a Confluence space-archive dir.
-  `JiraIssueProjection` type L54-89 — `impl Projection for JiraIssueProjection` — ```
-  `feed_type` function L55-57 — `(&self) -> &'static str` — ```
-  `row` function L58-88 — `(&self) -> ProjectionRow` — ```
-  `JiraCommentProjection` type L102-127 — `impl Projection for JiraCommentProjection` — ```
-  `feed_type` function L103-105 — `(&self) -> &'static str` — ```
-  `row` function L106-126 — `(&self) -> ProjectionRow` — ```
-  `JiraHistoryProjection` type L142-177 — `impl Projection for JiraHistoryProjection` — ```
-  `feed_type` function L143-145 — `(&self) -> &'static str` — ```
-  `row` function L146-176 — `(&self) -> ProjectionRow` — ```
-  `ConfluencePageProjection` type L193-215 — `impl Projection for ConfluencePageProjection` — ```
-  `feed_type` function L194-196 — `(&self) -> &'static str` — ```
-  `row` function L197-214 — `(&self) -> ProjectionRow` — ```
-  `hash_id` function L217-224 — `(prefix: &str, feed_id: &str, source: &str) -> String` — ```
-  `parse_dt` function L226-230 — `(s: &str) -> DateTime<Utc>` — ```
-  `visit_jira` function L251-297 — `( feed_id: &str, dir: &Path, out: &mut JiraWalkResult, depth: usize, ) -> Result...` — ```
-  `read_jira_issue` function L299-394 — `( feed_id: &str, path: &Path, ) -> Result<Option<JiraIssueProjection>, Projectio...` — ```
-  `read_jira_comments` function L396-443 — `( feed_id: &str, issue_key: &str, path: &Path, out: &mut Vec<JiraCommentProjecti...` — ```
-  `read_jira_history` function L445-519 — `( feed_id: &str, issue_key: &str, path: &Path, out: &mut Vec<JiraHistoryProjecti...` — ```
-  `tests` module L605-712 — `-` — ```
-  `jira_issue_from_disk` function L610-642 — `()` — ```
-  `jira_comments_and_history` function L645-684 — `()` — ```
-  `confluence_page_from_disk` function L687-711 — `()` — ```

#### crates/arawn-projections/src/calendar.rs

- pub `FEED_TYPE` variable L20 — `: &str` — we store one projection row per file.
- pub `CalendarEventProjection` struct L23-39 — `{ id: String, feed_id: String, source_id: String, source_ts: DateTime<Utc>, cale...` — we store one projection row per file.
- pub `projection_id` function L81-88 — `(feed_id: &str, event_id: &str) -> String` — we store one projection row per file.
- pub `from_calendar_event` function L111-178 — `(feed_id: &str, v: &Value) -> Option<CalendarEventProjection>` — we store one projection row per file.
- pub `walk_feed_dir` function L180-210 — `( feed_id: &str, feed_dir: &Path, ) -> Result<Vec<CalendarEventProjection>, Proj...` — we store one projection row per file.
-  `CalendarEventProjection` type L41-79 — `impl Projection for CalendarEventProjection` — we store one projection row per file.
-  `feed_type` function L42-44 — `(&self) -> &'static str` — we store one projection row per file.
-  `row` function L46-78 — `(&self) -> ProjectionRow` — we store one projection row per file.
-  `parse_event_time` function L90-109 — `(v: Option<&Value>) -> (Option<DateTime<Utc>>, bool)` — we store one projection row per file.
-  `tests` module L213-280 — `-` — we store one projection row per file.
-  `parses_dated_event` function L218-236 — `()` — we store one projection row per file.
-  `parses_all_day_event` function L239-249 — `()` — we store one projection row per file.
-  `walks_events_dir` function L252-273 — `()` — we store one projection row per file.
-  `skips_event_without_start` function L276-279 — `()` — we store one projection row per file.

#### crates/arawn-projections/src/dispatch.rs

- pub `project_feed_dir` function L27-142 — `( store: &ProjectionStore, template_name: &str, feed_id: &str, feed_dir: &Path, ...` — Project every item under the on-disk mirror for `feed_id`, walking
-  `SubBatch` enum L144-148 — `Issues | Comments | History` — and after backfill.
-  `SubKind` enum L150-154 — `IssueKey | CommentId | HistoryId` — and after backfill.
-  `atlassian_write_subbatch` function L156-174 — `( store: &ProjectionStore, feed_type: &str, feed_id: &str, sub: SubBatch, _kind:...` — and after backfill.
-  `dedup_and_write_single_type` function L176-200 — `( store: &ProjectionStore, feed_type: &str, feed_id: &str, parsed: Vec<P>, sourc...` — and after backfill.

#### crates/arawn-projections/src/drive.rs

- pub `FEED_TYPE` variable L24 — `: &str` — body_hash is the file size + path so a re-run is still a no-op.
- pub `DriveFileProjection` struct L32-42 — `{ id: String, feed_id: String, source_id: String, source_ts: DateTime<Utc>, path...` — body_hash is the file size + path so a re-run is still a no-op.
- pub `projection_id` function L69-76 — `(feed_id: &str, file_id: &str) -> String` — body_hash is the file size + path so a re-run is still a no-op.
- pub `walk_feed_dir` function L78-135 — `( feed_id: &str, feed_dir: &Path, ) -> Result<Vec<DriveFileProjection>, Projecti...` — body_hash is the file size + path so a re-run is still a no-op.
-  `MAX_BODY_BYTES` variable L29 — `: usize` — Heuristic: only embed files whose body looks like text.
-  `DriveFileProjection` type L44-67 — `impl Projection for DriveFileProjection` — body_hash is the file size + path so a re-run is still a no-op.
-  `feed_type` function L45-47 — `(&self) -> &'static str` — body_hash is the file size + path so a re-run is still a no-op.
-  `row` function L49-66 — `(&self) -> ProjectionRow` — body_hash is the file size + path so a re-run is still a no-op.
-  `read_text_body` function L140-161 — `(path: &Path) -> (String, u64)` — Read a file as utf-8 text, truncated to `MAX_BODY_BYTES`.
-  `read_capped` function L163-179 — `(path: &Path, cap: usize) -> Result<Vec<u8>, std::io::Error>` — body_hash is the file size + path so a re-run is still a no-op.
-  `tests` module L182-260 — `-` — body_hash is the file size + path so a re-run is still a no-op.
-  `write_meta` function L186-188 — `(dir: &Path, meta: Value)` — body_hash is the file size + path so a re-run is still a no-op.
-  `walks_files_from_meta` function L191-224 — `()` — body_hash is the file size + path so a re-run is still a no-op.
-  `missing_meta_returns_empty` function L227-231 — `()` — body_hash is the file size + path so a re-run is still a no-op.
-  `tolerates_top_level_files_key` function L234-244 — `()` — body_hash is the file size + path so a re-run is still a no-op.
-  `missing_local_file_still_produces_metadata_row` function L247-259 — `()` — body_hash is the file size + path so a re-run is still a no-op.

#### crates/arawn-projections/src/embed.rs

- pub `EMBEDDABLE_FEED_TYPES` variable L24-33 — `: &[&str]` — Feed types whose body_text is worth embedding.
- pub `EmbedPassOutcome` struct L41-45 — `{ embedded: usize, skipped_empty: usize, errors: usize }` — `crates/arawn/src/main.rs`.
- pub `Embedder` interface L51-56 — `{ fn embed_batch() }` — Lightweight embedding interface this crate consumes.
- pub `run_embed_pass` function L60-104 — `( store: &ProjectionStore, embedder: &dyn Embedder, batch_size: usize, max_per_p...` — Run a single embed pass over every embeddable feed type, capped at
- pub `PendingEmbedRow` struct L178-181 — `{ projection_id: String, body_text: String }` — A row pending embedding: the `<feed_type>` row's projection id +
- pub `pending_embedding_rows` function L186-219 — `( &self, feed_type: &str, limit: usize, ) -> Result<Vec<PendingEmbedRow>, Projec...` — Find rows in `<feed_type>` whose embed status is `pending`,
- pub `write_embedding` function L225-280 — `( &self, feed_type: &str, projection_id: &str, vector: &[f32], ) -> Result<(), P...` — Write a freshly computed embedding for a projection row.
-  `MIN_BODY_CHARS` variable L38 — `: usize` — Minimum body length worth embedding.
-  `embed_batch` function L106-173 — `( store: &ProjectionStore, feed_type: &str, rows: &[PendingEmbedRow], embedder: ...` — `crates/arawn/src/main.rs`.
-  `ProjectionStore` type L183-281 — `= ProjectionStore` — `crates/arawn/src/main.rs`.

#### crates/arawn-projections/src/error.rs

- pub `ProjectionError` enum L4-13 — `Storage | Schema | Io`
-  `ProjectionError` type L15-19 — `= ProjectionError`
-  `from` function L16-18 — `(value: rusqlite::Error) -> Self`
-  `ProjectionError` type L21-25 — `= ProjectionError`
-  `from` function L22-24 — `(value: std::io::Error) -> Self`
-  `ProjectionError` type L27-31 — `= ProjectionError`
-  `from` function L28-30 — `(value: serde_json::Error) -> Self`

#### crates/arawn-projections/src/gmail.rs

- pub `FEED_TYPE` variable L17 — `: &str` — `GmailMessageProjection` row.
- pub `GmailMessageProjection` struct L20-31 — `{ id: String, feed_id: String, source_id: String, source_ts: DateTime<Utc>, send...` — `GmailMessageProjection` row.
- pub `projection_id` function L69-76 — `(feed_id: &str, message_id: &str) -> String` — Stable projection id from `(feed_id, message_id)`.
- pub `from_gmail_message` function L83-178 — `( feed_id: &str, msg: &Value, ) -> Result<Option<GmailMessageProjection>, Projec...` — Parse a single Gmail Message JSON value into a projection.
- pub `walk_feed_dir` function L183-227 — `( feed_id: &str, feed_dir: &Path, ) -> Result<Vec<GmailMessageProjection>, Proje...` — Walk the on-disk feed dir, parsing every `<YYYY-MM-DD>/<id>.json`
-  `GmailMessageProjection` type L33-65 — `impl Projection for GmailMessageProjection` — `GmailMessageProjection` row.
-  `feed_type` function L34-36 — `(&self) -> &'static str` — `GmailMessageProjection` row.
-  `row` function L38-64 — `(&self) -> ProjectionRow` — `GmailMessageProjection` row.
-  `extract_body_text` function L232-241 — `(payload: Option<&Value>) -> Option<String>` — Decode a gmail body part.
-  `extract_part` function L243-260 — `(part: &Value, mime: &str) -> Option<String>` — `GmailMessageProjection` row.
-  `decode_base64url` function L262-271 — `(s: &str) -> Result<String, ProjectionError>` — `GmailMessageProjection` row.
-  `base64_decode` function L275-311 — `(s: &str) -> Result<Vec<u8>, &'static str>` — Minimal base64 decoder (we don't have base64 as a workspace dep
-  `val` function L276-285 — `(c: u8) -> Result<u8, &'static str>` — `GmailMessageProjection` row.
-  `tests` module L314-400 — `-` — `GmailMessageProjection` row.
-  `parses_minimal_message` function L319-345 — `()` — `GmailMessageProjection` row.
-  `skips_missing_id` function L348-351 — `()` — `GmailMessageProjection` row.
-  `skips_bad_internaldate` function L354-357 — `()` — `GmailMessageProjection` row.
-  `projection_id_is_stable` function L360-366 — `()` — `GmailMessageProjection` row.
-  `snippet_fallback_when_no_body` function L369-378 — `()` — `GmailMessageProjection` row.
-  `walk_feed_dir_picks_up_files` function L381-399 — `()` — `GmailMessageProjection` row.

#### crates/arawn-projections/src/lib.rs

- pub `atlassian` module L15 — `-` — Projections sit between raw feed mirrors (on-disk files) and the
- pub `calendar` module L16 — `-` — - Decouples feed-side fidelity (raw mirror) from query-side shape.
- pub `dispatch` module L17 — `-` — - Decouples feed-side fidelity (raw mirror) from query-side shape.
- pub `drive` module L18 — `-` — - Decouples feed-side fidelity (raw mirror) from query-side shape.
- pub `embed` module L19 — `-` — - Decouples feed-side fidelity (raw mirror) from query-side shape.
- pub `error` module L20 — `-` — - Decouples feed-side fidelity (raw mirror) from query-side shape.
- pub `gmail` module L21 — `-` — - Decouples feed-side fidelity (raw mirror) from query-side shape.
- pub `schema` module L22 — `-` — - Decouples feed-side fidelity (raw mirror) from query-side shape.
- pub `slack` module L23 — `-` — - Decouples feed-side fidelity (raw mirror) from query-side shape.
- pub `store` module L24 — `-` — - Decouples feed-side fidelity (raw mirror) from query-side shape.
- pub `types` module L25 — `-` — - Decouples feed-side fidelity (raw mirror) from query-side shape.

#### crates/arawn-projections/src/schema.rs

- pub `EMBEDDING_DIMS` variable L27 — `: usize` — Embedding dimensionality.
- pub `init_vector_extension` function L32-39 — `()` — One-shot initialization of the sqlite-vec extension.
- pub `ensure_feed_type_tables` function L42-100 — `( conn: &Connection, feed_type: &str, ) -> Result<(), ProjectionError>` — Idempotently create all schema for a given feed type.
- pub `apply_pragmas` function L103-107 — `(conn: &Connection) -> Result<(), ProjectionError>` — Set basic pragmas for a projection database.

#### crates/arawn-projections/src/slack.rs

- pub `TOPLEVEL_FEED_TYPE` variable L21 — `: &str` — ```
- pub `THREAD_FEED_TYPE` variable L22 — `: &str` — ```
- pub `SlackMessageProjection` struct L25-36 — `{ id: String, feed_id: String, source_id: String, source_ts: DateTime<Utc>, chan...` — ```
- pub `projection_id` function L81-88 — `(feed_id: &str, slack_ts: &str) -> String` — ```
- pub `parse_slack_ts` function L92-97 — `(ts: &str) -> Option<DateTime<Utc>>` — Slack `ts` is `"<unix_secs>.<microseconds>"`.
- pub `from_slack_message` function L99-147 — `( feed_id: &str, msg: &Value, is_thread_reply: bool, ) -> Option<SlackMessagePro...` — ```
- pub `walk_feed_dir` function L166-225 — `( feed_id: &str, feed_dir: &Path, ) -> Result<Vec<SlackMessageProjection>, Proje...` — ```
-  `SlackMessageProjection` type L38-66 — `impl Projection for SlackMessageProjection` — ```
-  `feed_type` function L39-45 — `(&self) -> &'static str` — ```
-  `row` function L47-65 — `(&self) -> ProjectionRow` — ```
-  `synth_title` function L68-79 — `(p: &SlackMessageProjection) -> String` — ```
-  `parse_jsonl` function L149-164 — `(path: &Path) -> Result<Vec<Value>, ProjectionError>` — ```
-  `tests` module L228-297 — `-` — ```
-  `parses_ts` function L233-236 — `()` — ```
-  `from_message_basic` function L239-254 — `()` — ```
-  `thread_reply_routes_to_thread_table` function L257-267 — `()` — ```
-  `walks_top_level_and_threads` function L270-296 — `()` — ```

#### crates/arawn-projections/src/store.rs

- pub `ProjectionStore` struct L24-26 — `{ conn: Mutex<Connection> }` — Sqlite-backed projection store.
- pub `conn` function L35-37 — `(&self) -> &Mutex<Connection>` — Accessor for sibling modules (e.g.
- pub `open` function L39-51 — `(path: &Path) -> Result<Self, ProjectionError>` — detect stale entries cheaply.
- pub `in_memory` function L53-60 — `() -> Result<Self, ProjectionError>` — detect stale entries cheaply.
- pub `ensure_feed_type` function L63-66 — `(&self, feed_type: &str) -> Result<(), ProjectionError>` — Ensure schema for a feed type exists.
- pub `write` function L71-73 — `(&self, projection: &P) -> Result<WriteOutcome, ProjectionError>` — Write a single projection inside a transaction: row UPSERT,
- pub `write_batch` function L76-114 — `( &self, projections: &[P], ) -> Result<WriteOutcome, ProjectionError>` — Write many projections in one transaction.
- pub `missing_source_ids` function L119-158 — `( &self, feed_type: &str, feed_id: &str, candidate_source_ids: &[String], ) -> R...` — Returns ids that are NOT yet projected for a given feed.
- pub `count` function L161-168 — `(&self, feed_type: &str) -> Result<usize, ProjectionError>` — Total rows for a feed_type — useful for tests and ops.
- pub `vector_search` function L174-206 — `( &self, feed_type: &str, query_vec: &[f32], limit: usize, ) -> Result<Vec<Strin...` — Vector similarity search over a single feed type.
- pub `fts_search` function L210-232 — `( &self, feed_type: &str, query: &str, limit: usize, ) -> Result<Vec<String>, Pr...` — FTS search over a single feed type.
- pub `get_row` function L235-278 — `( &self, feed_type: &str, projection_id: &str, ) -> Result<Option<ProjectionRow>...` — Get a single projection row by primary key.
- pub `WriteOutcome` struct L282-286 — `{ inserted: usize, updated: usize, unchanged: usize }` — detect stale entries cheaply.
-  `ProjectionStore` type L28-279 — `= ProjectionStore` — detect stale entries cheaply.
-  `WriteAction` enum L288-292 — `Inserted | Updated | Unchanged` — detect stale entries cheaply.
-  `body_hash` function L294-299 — `(body_text: &str) -> String` — detect stale entries cheaply.
-  `write_row` function L301-395 — `( tx: &rusqlite::Transaction<'_>, feed_type: &str, row: &ProjectionRow, ) -> Res...` — detect stale entries cheaply.
-  `fts_upsert` function L397-415 — `( tx: &rusqlite::Transaction<'_>, feed_type: &str, projection_id: &str, title: &...` — detect stale entries cheaply.
-  `embedding_invalidate` function L420-442 — `( tx: &rusqlite::Transaction<'_>, feed_type: &str, projection_id: &str, body_has...` — Mark a projection row's embedding as pending re-compute.

#### crates/arawn-projections/src/types.rs

- pub `ProjectionRow` struct L13-22 — `{ id: String, feed_id: String, source_id: String, source_ts: DateTime<Utc>, titl...` — A single projection row, type-erased to the common fields every
- pub `Projection` interface L30-38 — `{ fn feed_type(), fn row() }` — Marker trait for type-specific projection structs.

### crates/arawn-projections/tests

> *Semantic summary to be generated by AI agent.*

#### crates/arawn-projections/tests/embed_pass.rs

-  `StubEmbedder` struct L12-15 — `{ calls: AtomicUsize, dim: usize }` — embedder, writes vectors back, skips short bodies.
-  `StubEmbedder` type L17-27 — `= StubEmbedder` — embedder, writes vectors back, skips short bodies.
-  `new` function L18-23 — `(dim: usize) -> Self` — embedder, writes vectors back, skips short bodies.
-  `calls` function L24-26 — `(&self) -> usize` — embedder, writes vectors back, skips short bodies.
-  `StubEmbedder` type L29-45 — `impl Embedder for StubEmbedder` — embedder, writes vectors back, skips short bodies.
-  `embed_batch` function L30-44 — `( &'a self, texts: &'a [&'a str], ) -> Pin<Box<dyn Future<Output = Result<Vec<Ve...` — embedder, writes vectors back, skips short bodies.
-  `fixture_message` function L47-60 — `(id: &str, body: &str) -> gmail::GmailMessageProjection` — embedder, writes vectors back, skips short bodies.
-  `embeds_rows_with_null_embedding` function L63-81 — `()` — embedder, writes vectors back, skips short bodies.
-  `skips_short_bodies_but_marks_them` function L84-102 — `()` — embedder, writes vectors back, skips short bodies.
-  `max_per_pass_caps_work` function L105-124 — `()` — embedder, writes vectors back, skips short bodies.
-  `known_feed_types_are_a_strict_subset_of_routed_types` function L127-143 — `()` — embedder, writes vectors back, skips short bodies.

#### crates/arawn-projections/tests/gmail_e2e.rs

-  `write_msg` function L8-12 — `(dir: &std::path::Path, day: &str, id: &str, msg: serde_json::Value)` — projections, search via FTS, re-run and confirm idempotency.
-  `fixture_msg` function L14-31 — `(id: &str, internal_date_ms: i64, subject: &str, body: &str) -> serde_json::Valu...` — projections, search via FTS, re-run and confirm idempotency.
-  `end_to_end_walk_write_search` function L34-71 — `()` — projections, search via FTS, re-run and confirm idempotency.
-  `rerun_is_idempotent` function L74-96 — `()` — projections, search via FTS, re-run and confirm idempotency.
-  `body_change_updates_and_refreshes_fts` function L99-126 — `()` — projections, search via FTS, re-run and confirm idempotency.
-  `missing_source_ids_returns_unprojected` function L129-153 — `()` — projections, search via FTS, re-run and confirm idempotency.
-  `rerun_after_partial_failure_picks_up_missing` function L156-190 — `()` — projections, search via FTS, re-run and confirm idempotency.

#### crates/arawn-projections/tests/hybrid_search.rs

-  `KeywordEmbedder` struct L13 — `-` — Embedder that maps text → unit vector along a content-derived
-  `KeywordEmbedder` type L15-33 — `= KeywordEmbedder` — sentinel-marked rows, and tolerates degenerate input.
-  `vec_for` function L16-32 — `(text: &str) -> Vec<f32>` — sentinel-marked rows, and tolerates degenerate input.
-  `normalize` function L35-47 — `(mut v: Vec<f32>) -> Vec<f32>` — sentinel-marked rows, and tolerates degenerate input.
-  `KeywordEmbedder` type L49-57 — `impl Embedder for KeywordEmbedder` — sentinel-marked rows, and tolerates degenerate input.
-  `embed_batch` function L50-56 — `( &'a self, texts: &'a [&'a str], ) -> Pin<Box<dyn Future<Output = Result<Vec<Ve...` — sentinel-marked rows, and tolerates degenerate input.
-  `fixture` function L59-72 — `(id: &str, body: &str) -> gmail::GmailMessageProjection` — sentinel-marked rows, and tolerates degenerate input.
-  `vector_search_ranks_by_cosine_similarity` function L75-90 — `()` — sentinel-marked rows, and tolerates degenerate input.
-  `vector_search_ignores_sentinel_and_null_rows` function L93-112 — `()` — sentinel-marked rows, and tolerates degenerate input.
-  `pending_rows_round_trip` function L115-127 — `()` — sentinel-marked rows, and tolerates degenerate input.
-  `empty_query_vec_returns_empty` function L130-135 — `()` — sentinel-marked rows, and tolerates degenerate input.

### crates/arawn-service/src

**Role**: The service contract (trait + types) shared between the backend implementation (`LocalService` in arawn-bin) and the WebSocket server — defines what the backend can do and the wire-serializable types for all operations.

**Key abstractions**:
- `ArawnService` trait — The complete backend API: workstream CRUD, session CRUD, `send_message` (returns a streaming `EngineEvent` pinned box), `cancel`, `promote_session`, `resolve_user_input` (modal responses), `query_inventory`, `list_available_commands`, `list_workflows`, `remember_fact`, `memory_summary`, `forget_entity`, `get_permission_mode`, `set_permission_mode`. Implemented only by `LocalService`.
- `ServiceError` — Error type with `#[from]` conversions for `EngineError`, `StorageError`, and `MemoryError` (enabling `?` propagation), plus `NotFound`, `InvalidOperation`, and `Internal` variants for string-only cases. `details()` emits a structured JSON `kind` tag for typed sub-sources so clients can do fine-grained dispatch. `error_code()` returns a stable string tag for the RPC `error.code` field.
- `EngineEvent` — The streaming payload type emitted during `send_message`: `StreamingText`, `ToolCallStart`, `ToolCallResult`, `Complete`, `Error`, `CompactionOccurred`, `ModalPromptRequest`, `MemoryStored`, `Warning`, `TaskCompleted`.
- Types in `types.rs` — All wire-serializable DTOs: `WorkstreamInfo`, `SessionInfo`, `SessionDetail`, `MemoryStoreResult`, `MemorySummary`, `ForgetResult`, `InventoryItem`, `CommandInfo`, `PromotionResult`, `WorkflowInfo`, `PermissionModeInfo`.

**Mixed concerns / gotchas**: `ServiceError` has typed `#[from]` conversions for the three subsystem error types, but `NotFound` and `InvalidOperation` are plain strings — the distinction matters for the `details()` method which only emits structured JSON for the typed variants.

**Dependencies**: `arawn-core` (Message), `arawn-engine` (EngineError), `arawn-storage` (StorageError), `arawn-memory` (MemoryError); `futures` (Stream), `serde`, `uuid`, `chrono`.

#### crates/arawn-service/src/error.rs

- pub `ServiceError` enum L4-22 — `NotFound | InvalidOperation | Engine | Storage | Memory | Internal`
- pub `error_code` function L26-35 — `(&self) -> &'static str` — Return a stable error code string for RPC responses.
- pub `details` function L41-54 — `(&self) -> Option<serde_json::Value>` — Structured detail suitable for RPC responses.
-  `ServiceError` type L24-55 — `= ServiceError`
-  `engine_error_kind` function L57-65 — `(e: &arawn_engine::EngineError) -> &'static str`
-  `storage_error_kind` function L67-76 — `(e: &arawn_storage::StorageError) -> &'static str`
-  `memory_error_kind` function L78-84 — `(e: &arawn_memory::MemoryError) -> &'static str`

#### crates/arawn-service/src/lib.rs

- pub `error` module L1 — `-`
- pub `types` module L2 — `-`
- pub `ArawnService` interface L27-199 — `{ fn list_workstreams(), fn create_workstream(), fn list_sessions(), fn create_s...` — The service contract between any UI client and the Arawn backend.

#### crates/arawn-service/src/types.rs

- pub `WorkstreamInfo` struct L11-16 — `{ id: Uuid, name: String, root_dir: PathBuf, created_at: DateTime<Utc> }` — Lightweight view of a workstream for API transport.
- pub `SessionInfo` struct L20-24 — `{ id: Uuid, workstream_id: Option<Uuid>, created_at: DateTime<Utc> }` — Lightweight view of a session (metadata only, no messages).
- pub `SessionDetail` struct L28-33 — `{ id: Uuid, workstream_id: Option<Uuid>, created_at: DateTime<Utc>, messages: Ve...` — Session with full message history.
- pub `ModalPromptOption` struct L37-41 — `{ label: String, description: Option<String> }` — An option in a modal prompt sent to the client.
- pub `EngineEvent` enum L46-93 — `StreamingText | ToolCallStart | ToolCallResult | Complete | Error | CompactionOc...` — Streaming event emitted during a conversation turn.
- pub `MemoryStoreResult` enum L98-117 — `Inserted | Reinforced | Superseded` — Result of storing a fact in the knowledge base.
- pub `MemorySummary` struct L121-124 — `{ global: MemoryStoreSummary, workstream: MemoryStoreSummary }` — Summary of the knowledge base.
- pub `MemoryStoreSummary` struct L127-130 — `{ total: u64, by_type: Vec<MemoryTypeCount> }`
- pub `MemoryTypeCount` struct L133-137 — `{ entity_type: String, count: u64 }`
- pub `ForgetResult` enum L142-151 — `Deleted | Ambiguous` — Result of forgetting an entity.
- pub `ForgetCandidate` struct L154-160 — `{ id: String, title: String, entity_type: String, scope: String }`
- pub `InventoryItem` struct L164-173 — `{ name: String, description: String, kind: Option<String>, enabled: Option<bool>...` — A single item in an inventory query result.
- pub `CommandInfo` struct L177-181 — `{ name: String, description: String, kind: String }` — A command available for autocomplete.
- pub `PromotionResult` struct L185-188 — `{ workstream_id: String, workstream_name: String }` — Result of promoting a scratch session to a workstream.
- pub `WorkflowInfo` struct L192-196 — `{ name: String, cron: Option<String> }` — Info about a workflow.
- pub `PermissionModeInfo` struct L200-202 — `{ mode: String }` — Result of getting or setting the permission mode.
- pub `ServerCapabilities` struct L209-215 — `{ server_version: String, embeddings_available: bool }` — Runtime capabilities advertised to clients on connect — what optional
- pub `PermissionsStatus` struct L221-227 — `{ mode: String, allow_rules: Vec<String>, deny_rules: Vec<String>, ask_rules: Ve...` — Read-only snapshot of the active permission configuration plus a
- pub `PermissionAuditEntry` struct L232-242 — `{ timestamp: String, tool_name: String, tool_input_summary: String, decision: St...` — One row of the permission audit — what the agent tried to do and how
- pub `ServerNotice` struct L249-261 — `{ level: String, category: String, message: String, timestamp: String }` — Server-wide event broadcast to every connected client.
- pub `IntegrationStatus` struct L265-268 — `{ name: String, connected: bool }` — One row of the integration registry as seen by clients.
- pub `OAuthFlowStarted` struct L274-279 — `{ service: String, auth_url: String }` — Returned by `start_oauth_flow` so the TUI knows what URL to open.
- pub `FeedRegisterSpec` struct L287-300 — `{ template: String, feed_id: String, params: serde_json::Value, cadence: Option<...` — Args for `ArawnService::feed_register`.
- pub `FeedSummaryDto` struct L306-318 — `{ id: String, template: String, cadence: String, enabled: bool, created_at: Stri...` — User-facing snapshot of one feed for the `/feeds` list.
- pub `FeedRemoveDto` struct L323-327 — `{ id: String, template: String, bytes_wiped: u64 }` — Returned by `feed_remove` so the TUI can confirm the wipe with a
- pub `FeedDiscoverRow` struct L331-340 — `{ label: String, hint: Option<String>, params: serde_json::Value }` — One pickable row from `feed_discover`.
- pub `FeedDiscoverDto` struct L346-350 — `{ template: String, picker_supported: bool, rows: Vec<FeedDiscoverRow> }` — Response from `feed_discover`.

### crates/arawn-steward/src

> *Semantic summary to be generated by AI agent.*

#### crates/arawn-steward/src/cursor.rs

- pub `CursorStore` struct L14-16 — `{ conn: Arc<Mutex<Connection>> }` — the last pass.
- pub `open` function L20-34 — `(data_dir: &Path, workstream_name: &str) -> Result<Self, StewardError>` — Open (or create) the cursor table inside `<data_dir>/workstreams/<name>/memory.db`.
- pub `get` function L36-53 — `(&self, subroutine: &str) -> Result<Option<DateTime<Utc>>, StewardError>` — the last pass.
- pub `advance` function L56-70 — `(&self, subroutine: &str, ts: DateTime<Utc>) -> Result<(), StewardError>` — Advance the cursor monotonically — never moves backwards.
-  `CursorStore` type L18-71 — `= CursorStore` — the last pass.
-  `tests` module L74-92 — `-` — the last pass.
-  `round_trip_and_monotonic` function L78-91 — `()` — the last pass.

#### crates/arawn-steward/src/doorwatch.rs

- pub `DoorWatchConfig` struct L33-38 — `{ focus_batch: usize, neighbors_per_workstream: usize }` — either side.
- pub `DoorWatchSubroutine` struct L49-56 — `{ client: Arc<dyn LlmClient>, model: String, config: DoorWatchConfig, cursor_fac...` — either side.
- pub `new` function L59-74 — `( client: Arc<dyn LlmClient>, model: impl Into<String>, cursor_factory: Arc<dyn ...` — either side.
- pub `with_config` function L76-79 — `(mut self, config: DoorWatchConfig) -> Self` — either side.
-  `SUBROUTINE_NAME` variable L30 — `: &str` — either side.
-  `DoorWatchConfig` type L40-47 — `impl Default for DoorWatchConfig` — either side.
-  `default` function L41-46 — `() -> Self` — either side.
-  `DoorWatchSubroutine` type L58-80 — `= DoorWatchSubroutine` — either side.
-  `IdentityMatch` struct L83-88 — `{ to_workstream: String, to_id: String, reason: String }` — either side.
-  `DoorWatchSubroutine` type L91-211 — `impl StewardSubroutine for DoorWatchSubroutine` — either side.
-  `name` function L92-94 — `(&self) -> &str` — either side.
-  `is_mutating` function L96-98 — `(&self) -> bool` — either side.
-  `run` function L100-210 — `(&self, ctx: &SubroutineCtx) -> Result<SubroutineOutcome, StewardError>` — either side.
-  `DoorWatchSubroutine` type L213-302 — `= DoorWatchSubroutine` — either side.
-  `classify` function L214-247 — `( &self, focus: &Entity, buckets: &[(String, Vec<Entity>)], ) -> Result<Vec<Iden...` — either side.
-  `record` function L249-301 — `( &self, focus: &Entity, m: &IdentityMatch, ctx: &SubroutineCtx, buckets: &[(Str...` — either side.
-  `brief` function L304-311 — `(e: &Entity) -> serde_json::Value` — either side.
-  `tests` module L314-490 — `-` — either side.
-  `ScriptedMock` struct L327-329 — `{ responses: Mutex<VecDeque<Value>> }` — either side.
-  `ScriptedMock` type L330-336 — `= ScriptedMock` — either side.
-  `new` function L331-335 — `(resp: Vec<Value>) -> Self` — either side.
-  `ScriptedMock` type L338-352 — `impl LlmClient for ScriptedMock` — either side.
-  `stream` function L339-351 — `( &self, _req: ChatRequest, ) -> Result< Pin<Box<dyn futures::Stream<Item = Resu...` — either side.
-  `setup_multi_workstream` function L354-380 — `() -> ( tempfile::TempDir, Arc<Mutex<Store>>, MemoryResolver, Arc<dyn Fn(&str) -...` — either side.
-  `proposes_identity_when_match_found` function L383-415 — `()` — either side.
-  `hallucinated_target_id_is_dropped` function L418-448 — `()` — either side.
-  `no_other_workstreams_means_zero_proposals` function L451-489 — `()` — either side.

#### crates/arawn-steward/src/error.rs

- pub `StewardError` enum L4-29 — `Storage | Memory | Journal | Subroutine | CapExceeded | NotFound | Parse`
-  `StewardError` type L31-35 — `= StewardError`
-  `from` function L32-34 — `(e: rusqlite::Error) -> Self`
-  `StewardError` type L37-41 — `= StewardError`
-  `from` function L38-40 — `(e: serde_json::Error) -> Self`
-  `StewardError` type L43-47 — `= StewardError`
-  `from` function L44-46 — `(e: arawn_memory::MemoryError) -> Self`

#### crates/arawn-steward/src/journal.rs

- pub `JournalRecord` struct L21-31 — `{ subroutine: String, action: String, inputs_json: String, outputs_json: String,...` — One row about to be (or already) written to the journal.
- pub `JournalRow` struct L35-46 — `{ id: i64, ts: DateTime<Utc>, subroutine: String, action: String, inputs_json: S...` — A journal row as read back from sqlite.
- pub `RevertResult` struct L52-57 — `{ row: JournalRow, newly_reverted: bool }` — Outcome of a `Journal::revert` call.
- pub `Journal` struct L63-67 — `{ conn: Arc<Mutex<Connection>>, workstream: String, path: PathBuf }` — Workstream-scoped journal.
- pub `open` function L75-88 — `(data_dir: &Path, workstream_name: &str) -> Result<Self, StewardError>` — Open (or create) the journal for `workstream_name` rooted at
- pub `workstream` function L90-92 — `(&self) -> &str` — `Journal::revert(action_id)` to reconstruct the inverse.
- pub `path` function L94-96 — `(&self) -> &Path` — `Journal::revert(action_id)` to reconstruct the inverse.
- pub `write_ahead` function L106-126 — `(&self, record: &JournalRecord) -> Result<i64, StewardError>` — Write a journal row *before* the mutation.
- pub `get` function L129-141 — `(&self, id: i64) -> Result<Option<JournalRow>, StewardError>` — Fetch one row by id.
- pub `recent` function L144-157 — `(&self, limit: usize) -> Result<Vec<JournalRow>, StewardError>` — Last `limit` rows, newest first.
- pub `pending_proposals` function L161-176 — `(&self, limit: usize) -> Result<Vec<JournalRow>, StewardError>` — Rows where `applied = 0` (proposals from map / door-watch) and
- pub `revert` function L182-208 — `(&self, id: i64) -> Result<RevertResult, StewardError>` — Mark a row reverted.
- pub `prompt_hash` function L213-216 — `(input: impl AsRef<[u8]>) -> String` — Build a deterministic prompt-hash id from arbitrary input bytes.
-  `Journal` type L69-217 — `= Journal` — `Journal::revert(action_id)` to reconstruct the inverse.
-  `ensure_schema` function L219-238 — `(conn: &Connection) -> Result<(), StewardError>` — `Journal::revert(action_id)` to reconstruct the inverse.
-  `row_to_record` function L240-266 — `(r: &rusqlite::Row<'_>) -> Result<JournalRow, StewardError>` — `Journal::revert(action_id)` to reconstruct the inverse.
-  `tests` module L269-358 — `-` — `Journal::revert(action_id)` to reconstruct the inverse.
-  `sample` function L272-282 — `() -> JournalRecord` — `Journal::revert(action_id)` to reconstruct the inverse.
-  `write_then_read` function L285-294 — `()` — `Journal::revert(action_id)` to reconstruct the inverse.
-  `revert_flips_metadata_idempotently` function L297-307 — `()` — `Journal::revert(action_id)` to reconstruct the inverse.
-  `recent_returns_newest_first` function L310-319 — `()` — `Journal::revert(action_id)` to reconstruct the inverse.
-  `pending_proposals_filters_applied_and_reverted` function L322-338 — `()` — `Journal::revert(action_id)` to reconstruct the inverse.
-  `prompt_hash_is_deterministic` function L341-347 — `()` — `Journal::revert(action_id)` to reconstruct the inverse.
-  `schema_idempotent_on_reopen` function L350-357 — `()` — `Journal::revert(action_id)` to reconstruct the inverse.

#### crates/arawn-steward/src/lib.rs

- pub `cursor` module L22 — `-` — The steward continuously re-reads each workstream's KB and applies
- pub `doorwatch` module L23 — `-` — T-0259 wires the /workstream refine / journal / rollback commands.
- pub `error` module L24 — `-` — T-0259 wires the /workstream refine / journal / rollback commands.
- pub `journal` module L25 — `-` — T-0259 wires the /workstream refine / journal / rollback commands.
- pub `llm_text` module L26 — `-` — T-0259 wires the /workstream refine / journal / rollback commands.
- pub `map` module L27 — `-` — T-0259 wires the /workstream refine / journal / rollback commands.
- pub `reshelve` module L28 — `-` — T-0259 wires the /workstream refine / journal / rollback commands.
- pub `rollback` module L29 — `-` — T-0259 wires the /workstream refine / journal / rollback commands.
- pub `runner` module L30 — `-` — T-0259 wires the /workstream refine / journal / rollback commands.
- pub `subroutine` module L31 — `-` — T-0259 wires the /workstream refine / journal / rollback commands.

#### crates/arawn-steward/src/llm_text.rs

- pub `complete_text` function L17-53 — `( client: &Arc<dyn LlmClient>, model: &str, system: &str, user: &str, ) -> Resul...` — `arawn-llm` once a third consumer appears.
- pub `extract_json_block` function L57-81 — `(raw: &str) -> Option<&str>` — First balanced `{...}` or `[...]` substring — same parser as

#### crates/arawn-steward/src/map.rs

- pub `MapConfig` struct L40-46 — `{ batch_size: usize, neighbors_per_focus: usize }` — Per ARAWN-A-0003 map never mutates the KB graph.
- pub `MapSubroutine` struct L57-62 — `{ client: Arc<dyn LlmClient>, model: String, config: MapConfig, cursor_factory: ...` — Per ARAWN-A-0003 map never mutates the KB graph.
- pub `new` function L65-76 — `( client: Arc<dyn LlmClient>, model: impl Into<String>, cursor_factory: Arc<dyn ...` — Per ARAWN-A-0003 map never mutates the KB graph.
- pub `with_config` function L78-81 — `(mut self, config: MapConfig) -> Self` — Per ARAWN-A-0003 map never mutates the KB graph.
-  `SUBROUTINE_NAME` variable L24 — `: &str` — Per ARAWN-A-0003 map never mutates the KB graph.
-  `is_proposable` function L28-37 — `(rel: RelationType) -> bool` — Relations map is allowed to propose.
-  `MapConfig` type L48-55 — `impl Default for MapConfig` — Per ARAWN-A-0003 map never mutates the KB graph.
-  `default` function L49-54 — `() -> Self` — Per ARAWN-A-0003 map never mutates the KB graph.
-  `MapSubroutine` type L64-82 — `= MapSubroutine` — Per ARAWN-A-0003 map never mutates the KB graph.
-  `ProposedEdge` struct L85-91 — `{ from_id: String, rel: String, to_id: String, reason: String }` — Per ARAWN-A-0003 map never mutates the KB graph.
-  `MapSubroutine` type L94-188 — `impl StewardSubroutine for MapSubroutine` — Per ARAWN-A-0003 map never mutates the KB graph.
-  `name` function L95-97 — `(&self) -> &str` — Per ARAWN-A-0003 map never mutates the KB graph.
-  `is_mutating` function L99-101 — `(&self) -> bool` — Per ARAWN-A-0003 map never mutates the KB graph.
-  `run` function L103-187 — `(&self, ctx: &SubroutineCtx) -> Result<SubroutineOutcome, StewardError>` — Per ARAWN-A-0003 map never mutates the KB graph.
-  `MapSubroutine` type L190-280 — `= MapSubroutine` — Per ARAWN-A-0003 map never mutates the KB graph.
-  `propose_for` function L191-220 — `( &self, focus: &Entity, neighbors: &[&Entity], _ctx: &SubroutineCtx, ) -> Resul...` — Per ARAWN-A-0003 map never mutates the KB graph.
-  `record_proposal` function L222-279 — `( &self, focus: &Entity, prop: &ProposedEdge, ctx: &SubroutineCtx, ) -> Result<(...` — Per ARAWN-A-0003 map never mutates the KB graph.
-  `brief` function L282-289 — `(e: &Entity) -> serde_json::Value` — Per ARAWN-A-0003 map never mutates the KB graph.
-  `tests` module L292-429 — `-` — Per ARAWN-A-0003 map never mutates the KB graph.
-  `ScriptedMock` struct L307-309 — `{ responses: Mutex<VecDeque<Value>> }` — Per ARAWN-A-0003 map never mutates the KB graph.
-  `ScriptedMock` type L310-316 — `= ScriptedMock` — Per ARAWN-A-0003 map never mutates the KB graph.
-  `new` function L311-315 — `(resp: Vec<Value>) -> Self` — Per ARAWN-A-0003 map never mutates the KB graph.
-  `ScriptedMock` type L318-333 — `impl LlmClient for ScriptedMock` — Per ARAWN-A-0003 map never mutates the KB graph.
-  `stream` function L319-332 — `( &self, _req: ChatRequest, ) -> Result< Pin<Box<dyn futures::Stream<Item = Resu...` — Per ARAWN-A-0003 map never mutates the KB graph.
-  `setup` function L335-346 — `() -> (tempfile::TempDir, Arc<MemoryManager>, Arc<Journal>, Arc< dyn Fn(&str) ->...` — Per ARAWN-A-0003 map never mutates the KB graph.
-  `ctx` function L348-360 — `( tmp: &tempfile::TempDir, mem: &Arc<MemoryManager>, j: &Arc<Journal>, cap: usiz...` — Per ARAWN-A-0003 map never mutates the KB graph.
-  `proposes_valid_edges_and_drops_invalid` function L363-393 — `()` — Per ARAWN-A-0003 map never mutates the KB graph.
-  `cap_stops_after_n_proposals` function L396-415 — `()` — Per ARAWN-A-0003 map never mutates the KB graph.
-  `cursor_advances_and_skips_on_rerun` function L418-428 — `()` — Per ARAWN-A-0003 map never mutates the KB graph.

#### crates/arawn-steward/src/reshelve.rs

- pub `ReshelveConfig` struct L31-36 — `{ batch_size: usize, candidates_per_focus: usize }` — LLM proposes the action; Rust picks the survivor.
- pub `ReshelveSubroutine` struct L47-54 — `{ client: Arc<dyn LlmClient>, model: String, config: ReshelveConfig, cursor_fact...` — LLM proposes the action; Rust picks the survivor.
- pub `new` function L57-68 — `( client: Arc<dyn LlmClient>, model: impl Into<String>, cursor_factory: Arc<dyn ...` — LLM proposes the action; Rust picks the survivor.
- pub `with_config` function L70-73 — `(mut self, config: ReshelveConfig) -> Self` — LLM proposes the action; Rust picks the survivor.
-  `SUBROUTINE_NAME` variable L28 — `: &str` — LLM proposes the action; Rust picks the survivor.
-  `ReshelveConfig` type L38-45 — `impl Default for ReshelveConfig` — LLM proposes the action; Rust picks the survivor.
-  `default` function L39-44 — `() -> Self` — LLM proposes the action; Rust picks the survivor.
-  `ReshelveSubroutine` type L56-74 — `= ReshelveSubroutine` — LLM proposes the action; Rust picks the survivor.
-  `PairVerdict` struct L79-98 — `{ action: String, reason: String, combined_content: Option<String>, delete_targe...` — LLM verdict on a (focus, candidate) pair.
-  `ReshelveSubroutine` type L101-184 — `impl StewardSubroutine for ReshelveSubroutine` — LLM proposes the action; Rust picks the survivor.
-  `name` function L102-104 — `(&self) -> &str` — LLM proposes the action; Rust picks the survivor.
-  `is_mutating` function L106-108 — `(&self) -> bool` — LLM proposes the action; Rust picks the survivor.
-  `run` function L110-183 — `(&self, ctx: &SubroutineCtx) -> Result<SubroutineOutcome, StewardError>` — LLM proposes the action; Rust picks the survivor.
-  `ReshelveSubroutine` type L186-422 — `= ReshelveSubroutine` — LLM proposes the action; Rust picks the survivor.
-  `process_focus` function L187-252 — `( &self, focus: &Entity, ctx: &SubroutineCtx, outcome: &mut SubroutineOutcome, )...` — LLM proposes the action; Rust picks the survivor.
-  `classify_pair` function L254-289 — `( &self, focus: &Entity, cand: &Entity, ) -> Result<PairVerdict, StewardError>` — LLM proposes the action; Rust picks the survivor.
-  `apply_merge` function L291-389 — `( &self, focus: &Entity, cand: &Entity, verdict: &PairVerdict, ctx: &SubroutineC...` — LLM proposes the action; Rust picks the survivor.
-  `apply_delete` function L391-421 — `( &self, focus: &Entity, verdict: &PairVerdict, ctx: &SubroutineCtx, outcome: &m...` — LLM proposes the action; Rust picks the survivor.
-  `fts_quote` function L427-429 — `(s: &str) -> String` — FTS5 phrase-quote helper.
-  `tests` module L432-683 — `-` — LLM proposes the action; Rust picks the survivor.
-  `ScriptedMock` struct L450-452 — `{ responses: Mutex<VecDeque<Value>> }` — Queue-based mock that returns scripted JSON for each call.
-  `ScriptedMock` type L454-460 — `= ScriptedMock` — LLM proposes the action; Rust picks the survivor.
-  `new` function L455-459 — `(responses: Vec<Value>) -> Self` — LLM proposes the action; Rust picks the survivor.
-  `ScriptedMock` type L463-484 — `impl LlmClient for ScriptedMock` — LLM proposes the action; Rust picks the survivor.
-  `stream` function L464-483 — `( &self, _req: ChatRequest, ) -> Result< Pin<Box<dyn futures::Stream<Item = Resu...` — LLM proposes the action; Rust picks the survivor.
-  `Fixture` struct L486-493 — `{ tmp: tempfile::TempDir, memory: Arc<MemoryManager>, journal: Arc<Journal>, cur...` — LLM proposes the action; Rust picks the survivor.
-  `setup` function L495-510 — `() -> Fixture` — LLM proposes the action; Rust picks the survivor.
-  `ctx` function L512-519 — `(fx: &Fixture, cap: usize) -> SubroutineCtx` — LLM proposes the action; Rust picks the survivor.
-  `fact` function L521-527 — `(title: &str, content: &str, reinforce: u32) -> Entity` — LLM proposes the action; Rust picks the survivor.
-  `merge_picks_most_reinforced_survivor` function L530-572 — `()` — LLM proposes the action; Rust picks the survivor.
-  `erroneous_deletes_focus` function L575-601 — `()` — LLM proposes the action; Rust picks the survivor.
-  `none_verdict_leaves_kb_untouched_but_advances_cursor` function L604-628 — `()` — LLM proposes the action; Rust picks the survivor.
-  `second_pass_skips_already_processed_entities` function L631-653 — `()` — LLM proposes the action; Rust picks the survivor.
-  `cap_stops_after_n_applied` function L656-682 — `()` — LLM proposes the action; Rust picks the survivor.

#### crates/arawn-steward/src/rollback.rs

- pub `apply_inverse` function L22-41 — `(row: &JournalRow, kb: &Arc<MemoryManager>) -> Result<(), StewardError>` — Apply the inverse mutation described by `row.outputs_json` to `kb`.
-  `MergeOutputs` struct L44-49 — `{ survivor_id: Uuid, deprecated_id: Uuid, pre_survivor: Entity, pre_deprecated: ...` — `(subroutine, action)` so the contract stays in one place.
-  `reshelve_merge_inverse` function L51-67 — `(row: &JournalRow, kb: &Arc<MemoryManager>) -> Result<(), StewardError>` — `(subroutine, action)` so the contract stays in one place.
-  `DeleteOutputs` struct L70-72 — `{ entity: Entity }` — `(subroutine, action)` so the contract stays in one place.
-  `reshelve_delete_inverse` function L74-80 — `(row: &JournalRow, kb: &Arc<MemoryManager>) -> Result<(), StewardError>` — `(subroutine, action)` so the contract stays in one place.
-  `tests` module L83-152 — `-` — `(subroutine, action)` so the contract stays in one place.
-  `setup_kb` function L87-91 — `() -> (tempfile::TempDir, Arc<MemoryManager>)` — `(subroutine, action)` so the contract stays in one place.
-  `proposal_inverse_is_noop` function L94-109 — `()` — `(subroutine, action)` so the contract stays in one place.
-  `reshelve_delete_inverse_reinserts_entity` function L112-132 — `()` — `(subroutine, action)` so the contract stays in one place.
-  `unknown_action_returns_error` function L135-151 — `()` — `(subroutine, action)` so the contract stays in one place.

#### crates/arawn-steward/src/runner.rs

- pub `SubroutineCaps` struct L24-27 — `{ per_subroutine: HashMap<String, usize>, default_cap: usize }` — Per-subroutine action caps.
- pub `new` function L48-53 — `(default_cap: usize) -> Self` — exercised end-to-end via `IdentitySubroutine`.
- pub `with_cap` function L55-58 — `(mut self, subroutine: impl Into<String>, cap: usize) -> Self` — exercised end-to-end via `IdentitySubroutine`.
- pub `cap_for` function L60-65 — `(&self, subroutine: &str) -> usize` — exercised end-to-end via `IdentitySubroutine`.
- pub `StewardStats` struct L71-79 — `{ workstreams_visited: usize, subroutine_runs: usize, actions_journaled: usize, ...` — Aggregate stats for one `run_pass` invocation across all
- pub `MemoryResolver` type L84-86 — `= Arc< dyn Fn(&str) -> Result<Arc<MemoryManager>, StewardError> + Send + Sync, >` — Function that materializes the `MemoryManager` for a workstream.
- pub `StewardRunner` struct L88-97 — `{ store: Arc<Mutex<Store>>, data_dir: PathBuf, memory: MemoryResolver, subroutin...` — exercised end-to-end via `IdentitySubroutine`.
- pub `new` function L100-114 — `( store: Arc<Mutex<Store>>, data_dir: impl Into<PathBuf>, memory: MemoryResolver...` — exercised end-to-end via `IdentitySubroutine`.
- pub `with_caps` function L116-119 — `(mut self, caps: SubroutineCaps) -> Self` — exercised end-to-end via `IdentitySubroutine`.
- pub `journal_for` function L122-132 — `(&self, workstream_name: &str) -> Result<Arc<Journal>, StewardError>` — Open / fetch the cached journal for a workstream.
- pub `run_pass_for_workstream` function L137-187 — `( &self, workstream: &Workstream, ) -> Result<StewardStats, StewardError>` — Run one pass over `workstream`: every subroutine, in declared
- pub `run_pass_for_all` function L190-229 — `(&self) -> Result<StewardStats, StewardError>` — Run one pass across every active (non-archived) workstream.
-  `SubroutineCaps` type L29-45 — `impl Default for SubroutineCaps` — exercised end-to-end via `IdentitySubroutine`.
-  `default` function L33-44 — `() -> Self` — Placeholder defaults that exist only so tests + first-boot don't
-  `SubroutineCaps` type L47-66 — `= SubroutineCaps` — exercised end-to-end via `IdentitySubroutine`.
-  `StewardRunner` type L99-230 — `= StewardRunner` — exercised end-to-end via `IdentitySubroutine`.
-  `tests` module L233-318 — `-` — exercised end-to-end via `IdentitySubroutine`.
-  `setup` function L237-253 — `() -> ( tempfile::TempDir, Arc<Mutex<Store>>, MemoryResolver, )` — exercised end-to-end via `IdentitySubroutine`.
-  `pass_visits_every_active_workstream` function L256-284 — `()` — exercised end-to-end via `IdentitySubroutine`.
-  `caps_override_takes_precedence` function L287-301 — `()` — exercised end-to-end via `IdentitySubroutine`.
-  `journal_persists_across_passes` function L304-317 — `()` — exercised end-to-end via `IdentitySubroutine`.

#### crates/arawn-steward/src/subroutine.rs

- pub `SubroutineCtx` struct L22-30 — `{ workstream: Workstream, memory: Arc<MemoryManager>, journal: Arc<Journal>, cap...` — Per-pass context handed to a subroutine.
- pub `SubroutineOutcome` struct L36-41 — `{ actions_journaled: usize, mutations_applied: usize, proposals_recorded: usize,...` — What a subroutine did.
- pub `StewardSubroutine` interface L44-59 — `{ fn name(), fn is_mutating(), fn run() }` — subroutine on this pass.
- pub `IdentitySubroutine` struct L64-66 — `{ name: String }` — No-op subroutine that writes exactly one journal row per invocation
- pub `new` function L75-77 — `(name: impl Into<String>) -> Self` — subroutine on this pass.
-  `IdentitySubroutine` type L68-72 — `impl Default for IdentitySubroutine` — subroutine on this pass.
-  `default` function L69-71 — `() -> Self` — subroutine on this pass.
-  `IdentitySubroutine` type L74-78 — `= IdentitySubroutine` — subroutine on this pass.
-  `IdentitySubroutine` type L81-117 — `impl StewardSubroutine for IdentitySubroutine` — subroutine on this pass.
-  `name` function L82-84 — `(&self) -> &str` — subroutine on this pass.
-  `is_mutating` function L86-91 — `(&self) -> bool` — subroutine on this pass.
-  `run` function L93-116 — `(&self, ctx: &SubroutineCtx) -> Result<SubroutineOutcome, StewardError>` — subroutine on this pass.

### crates/arawn-storage/src

**Role**: Dual-layer persistence: SQLite (via refinery migrations) for structured metadata (workstreams, session records, stats) and JSONL files for message history.

**Key abstractions**:
- `Store` — The public unified interface. Composes a `Database` (SQLite) and a `JsonlMessageStore` (JSONL). All workstream and session CRUD routes through here. `load_session` reads metadata from SQLite then loads messages from JSONL. `promote_session` does both the SQLite workstream_id update and the JSONL file move in a two-step sequence (the sync part and the async move are split into separate public methods to support the service layer's async context). `reconcile_sessions` removes SQLite records whose JSONL files no longer exist on disk.
- `Database` — Opens or creates an SQLite file and runs refinery migrations embedded from `migrations/` SQL files. Accessed via `conn()` to get the underlying `rusqlite::Connection`. Wrapped in `Mutex` at the service layer because `rusqlite::Connection` is not `Send`.
- `JsonlMessageStore` — Writes one JSON object per line to `{data_dir}/{workstream_dir}/{session_id}.jsonl`. Each new file gets a version header line. `load` skips malformed lines with a warning rather than failing. `move_session` renames the JSONL file between workstream directories. `sandbox_dir` computes the per-session sandbox root (scratch sessions get isolated dirs; named workstream sessions share the workstream dir).
- `SessionStore` / `WorkstreamStore` — Thin DAL types that borrow a `&Database` and perform CRUD SQL. `SessionMeta` is the SQLite row view (no messages); `into_session()` converts it to a `arawn_core::Session`.
- `DataLayout` — Declarative description of the `data_dir` tree: `v1()` lists every expected subdirectory. `ensure()` creates missing ones. Called once at startup.
- `workstream_dir_name(name, id)` — The canonical naming rule: use the workstream name if non-empty, otherwise fall back to the UUID string. This is the key link between a workstream's `id` and its on-disk directory name.

**Mixed concerns / gotchas**: Session promotion is split across two methods because the JSONL file move requires knowing the old and new workstream directory names, which must be resolved from the database before the file is moved. The `Store::promote_session` orchestrates both steps.

**Dependencies**: `rusqlite` (SQLite), `refinery` (migrations), `serde_json` (JSONL serialization), `arawn-core` (Message, Session, Workstream).

#### crates/arawn-storage/src/database.rs

- pub `Database` struct L14-16 — `{ conn: Connection }` — SQLite database with automatic schema migrations via refinery.
- pub `open` function L20-27 — `(path: &Path) -> Result<Self, StorageError>` — Open or create a database at the given path and run pending migrations.
- pub `in_memory` function L30-35 — `() -> Result<Self, StorageError>` — Create an in-memory database for testing.
- pub `conn` function L47-49 — `(&self) -> &Connection` — Get a reference to the underlying connection.
-  `embedded` module L8-11 — `-`
-  `Database` type L18-50 — `= Database`
-  `run_migrations` function L38-44 — `(&mut self) -> Result<(), StorageError>` — Run all pending refinery migrations.
-  `tests` module L53-107 — `-`
-  `in_memory_db_has_tables` function L58-82 — `()`
-  `migrations_are_idempotent` function L85-96 — `()`
-  `file_based_db_creates_file` function L99-106 — `()`

#### crates/arawn-storage/src/error.rs

- pub `StorageError` enum L4-22 — `Database | Migration | Io | Json | NotFound | InvalidOperation`

#### crates/arawn-storage/src/extractor_cursor_store.rs

- pub `ExtractorCursorStore` struct L13-15 — `{ db: &'a Database }` — next run and `advance` it monotonically as it makes progress.
- pub `ExtractorCursor` struct L18-23 — `{ workstream_name: String, feed_type: String, last_source_ts: Option<DateTime<Ut...` — next run and `advance` it monotonically as it makes progress.
- pub `new` function L26-28 — `(db: &'a Database) -> Self` — next run and `advance` it monotonically as it makes progress.
- pub `get` function L33-71 — `( &self, workstream_name: &str, feed_type: &str, ) -> Result<Option<ExtractorCur...` — Read the current cursor for (workstream, feed_type).
- pub `advance` function L75-99 — `( &self, workstream_name: &str, feed_type: &str, new_source_ts: DateTime<Utc>, )...` — Advance the cursor for (workstream, feed_type) to `new_source_ts`.
- pub `list_for_workstream` function L103-137 — `( &self, workstream_name: &str, ) -> Result<Vec<ExtractorCursor>, StorageError>` — List every cursor row for a workstream — used by
-  `parse_dt` function L140-144 — `(s: &str) -> Result<DateTime<Utc>, StorageError>` — next run and `advance` it monotonically as it makes progress.
-  `tests` module L147-207 — `-` — next run and `advance` it monotonically as it makes progress.
-  `db` function L150-152 — `() -> Database` — next run and `advance` it monotonically as it makes progress.
-  `get_returns_none_for_unknown` function L155-159 — `()` — next run and `advance` it monotonically as it makes progress.
-  `advance_inserts_then_updates` function L162-175 — `()` — next run and `advance` it monotonically as it makes progress.
-  `advance_refuses_to_go_backwards` function L178-188 — `()` — next run and `advance` it monotonically as it makes progress.
-  `list_for_workstream_returns_all_feed_types` function L191-206 — `()` — next run and `advance` it monotonically as it makes progress.

#### crates/arawn-storage/src/jsonl.rs

- pub `JsonlMessageStore` struct L17-19 — `{ data_dir: PathBuf }` — JSONL-based message persistence.
- pub `new` function L22-26 — `(data_dir: impl Into<PathBuf>) -> Self`
- pub `append` function L29-58 — `( &self, session_id: Uuid, workstream_dir: &str, msg: &Message, ) -> Result<(), ...` — Append a message to the session's JSONL file.
- pub `load` function L61-103 — `( &self, session_id: Uuid, workstream_dir: &str, ) -> Result<Vec<Message>, Stora...` — Load all messages for a session from its JSONL file.
- pub `truncate` function L113-153 — `( &self, session_id: Uuid, workstream_dir: &str, keep_count: usize, ) -> Result<...` — Atomically rewrite the session's JSONL file to keep only the first
- pub `move_session` function L157-177 — `( &self, session_id: Uuid, from_dir: &str, to_dir: &str, ) -> Result<(), Storage...` — Move a session's JSONL file from one workstream directory to another.
- pub `path_for` function L190-192 — `(&self, session_id: Uuid, workstream_dir: &str) -> PathBuf` — Get the path for a session (exposed for testing/debugging).
- pub `sandbox_dir` function L201-210 — `(&self, workstream_dir: &str, session_id: Uuid, is_scratch: bool) -> PathBuf` — Resolve the sandbox root for a session.
- pub `workstream_dir_name` function L214-220 — `(name: &str, id: Uuid) -> String` — Resolve a workstream directory name: use name if non-empty, fall back to UUID.
-  `JsonlMessageStore` type L21-211 — `= JsonlMessageStore`
-  `session_path` function L181-187 — `(&self, session_id: Uuid, workstream_dir: &str) -> PathBuf` — Resolve the filesystem path for a session's JSONL file.
-  `tests` module L223-562 — `-`
-  `setup` function L229-233 — `() -> (TempDir, JsonlMessageStore)`
-  `append_and_load_roundtrip` function L236-272 — `()`
-  `append_twice_accumulates` function L275-303 — `()`
-  `load_nonexistent_returns_empty` function L306-310 — `()`
-  `scratch_session_path` function L313-334 — `()`
-  `move_session_relocates_file` function L337-374 — `()`
-  `move_nonexistent_session_is_ok` function L377-383 — `()`
-  `jsonl_each_line_is_valid_json` function L386-422 — `()`
-  `sandbox_dir_scratch_is_per_session` function L425-433 — `()`
-  `sandbox_dir_named_is_shared` function L436-441 — `()`
-  `workstream_dir_name_prefers_name` function L444-448 — `()`
-  `workstream_dir_name_falls_back_to_uuid` function L451-454 — `()`
-  `load_skips_malformed_lines` function L457-485 — `()`
-  `new_file_has_version_header` function L488-510 — `()`
-  `truncate_keeps_only_first_n_messages` function L513-533 — `()`
-  `truncate_to_zero_drops_everything` function L536-543 — `()`
-  `truncate_beyond_length_is_no_op` function L546-553 — `()`
-  `truncate_nonexistent_session_is_ok` function L556-561 — `()`

#### crates/arawn-storage/src/layout.rs

- pub `DataLayout` struct L10-12 — `{ directories: Vec<PathBuf> }` — A declarative description of the expected directory tree.
- pub `v1` function L16-25 — `() -> Self` — The current layout version (V1).
- pub `ensure` function L29-38 — `(&self, data_dir: &Path) -> Result<(), StorageError>` — Reconcile the actual directory tree against the declaration.
- pub `directories` function L41-43 — `(&self) -> &[PathBuf]` — Return the list of declared directories (for testing/inspection).
-  `DataLayout` type L14-44 — `= DataLayout`
-  `tests` module L47-87 — `-`
-  `ensure_creates_directories_on_fresh_dir` function L52-62 — `()`
-  `ensure_is_idempotent` function L65-76 — `()`
-  `v1_declares_expected_directories` function L79-86 — `()`

#### crates/arawn-storage/src/lib.rs

- pub `database` module L1 — `-`
- pub `error` module L2 — `-`
- pub `extractor_cursor_store` module L3 — `-`
- pub `jsonl` module L4 — `-`
- pub `layout` module L5 — `-`
- pub `session_store` module L6 — `-`
- pub `store` module L7 — `-`
- pub `workstream_store` module L8 — `-`

#### crates/arawn-storage/src/session_store.rs

- pub `SessionStore` struct L10-12 — `{ db: &'a Database }` — CRUD operations for session metadata in SQLite.
- pub `new` function L15-17 — `(db: &'a Database) -> Self`
- pub `create` function L19-30 — `(&self, session: &Session) -> Result<(), StorageError>`
- pub `get` function L32-55 — `(&self, id: Uuid) -> Result<Option<SessionMeta>, StorageError>`
- pub `list_for_workstream` function L57-80 — `(&self, ws_id: Uuid) -> Result<Vec<SessionMeta>, StorageError>`
- pub `list_scratch` function L82-105 — `(&self) -> Result<Vec<SessionMeta>, StorageError>`
- pub `delete` function L108-114 — `(&self, session_id: Uuid) -> Result<bool, StorageError>` — Delete a session record from SQLite by ID.
- pub `update_stats` function L117-129 — `(&self, session_id: Uuid, stats: &SessionStats) -> Result<(), StorageError>` — Update session token/turn stats in SQLite.
- pub `update_workstream_id` function L131-141 — `( &self, session_id: Uuid, new_ws_id: Uuid, ) -> Result<bool, StorageError>`
- pub `update_workstream_name` function L146-156 — `( &self, session_id: Uuid, new_name: &str, ) -> Result<bool, StorageError>` — Update the persisted workstream slug for a session.
- pub `SessionMeta` struct L161-167 — `{ id: Uuid, workstream_id: Option<Uuid>, workstream_name: String, created_at: Da...` — Session metadata as stored in SQLite (no messages — those are in JSONL).
- pub `into_session` function L171-184 — `(self) -> Session` — Convert to an arawn_core::Session (without messages — load those separately).
-  `SessionMeta` type L169-185 — `= SessionMeta`
-  `SessionRow` struct L187-196 — `{ id: String, workstream_id: Option<String>, workstream_name: String, created_at...`
-  `SessionRow` type L198-226 — `= SessionRow`
-  `into_meta` function L199-225 — `(self) -> Result<SessionMeta, StorageError>`
-  `tests` module L229-354 — `-`
-  `setup` function L233-235 — `() -> Database`
-  `create_and_get_session` function L238-251 — `()`
-  `create_scratch_session` function L254-264 — `()`
-  `get_nonexistent_returns_none` function L267-271 — `()`
-  `list_for_workstream` function L274-296 — `()`
-  `list_scratch_sessions` function L299-317 — `()`
-  `update_workstream_id_promotes_scratch` function L320-335 — `()`
-  `update_workstream_id_on_bound_session_returns_false` function L338-353 — `()`

#### crates/arawn-storage/src/store.rs

- pub `Store` struct L16-20 — `{ db: Database, messages: JsonlMessageStore, data_dir: PathBuf }` — Unified persistence interface composing SQLite metadata + JSONL messages.
- pub `open` function L25-44 — `(data_dir: impl Into<PathBuf>) -> Result<Self, StorageError>` — Open or create a store at the given data directory.
- pub `database` function L50-52 — `(&self) -> &Database` — Data directory path.
- pub `data_dir` function L54-56 — `(&self) -> &Path`
- pub `message_store` function L59-61 — `(&self) -> &JsonlMessageStore` — Get the JSONL message store (for direct access in service layer).
- pub `create_workstream` function L65-81 — `(&self, ws: &Workstream) -> Result<(), StorageError>`
- pub `get_workstream` function L83-85 — `(&self, id: Uuid) -> Result<Option<Workstream>, StorageError>`
- pub `find_workstream_by_name` function L87-89 — `(&self, name: &str) -> Result<Option<Workstream>, StorageError>`
- pub `list_workstreams` function L91-93 — `(&self) -> Result<Vec<Workstream>, StorageError>`
- pub `list_all_workstreams` function L95-97 — `(&self) -> Result<Vec<Workstream>, StorageError>`
- pub `update_workstream_description` function L99-105 — `( &self, name: &str, description: &str, ) -> Result<(), StorageError>`
- pub `add_workstream_binding` function L107-109 — `(&self, name: &str, feed_id: &str) -> Result<(), StorageError>`
- pub `remove_workstream_binding` function L111-117 — `( &self, name: &str, feed_id: &str, ) -> Result<(), StorageError>`
- pub `soft_delete_workstream` function L119-121 — `(&self, name: &str) -> Result<(), StorageError>`
- pub `ensure_scratch_workstream` function L125-129 — `(&self) -> Result<Workstream, StorageError>` — Idempotently ensure the `scratch` workstream exists.
- pub `create_session` function L133-135 — `(&self, session: &Session) -> Result<(), StorageError>`
- pub `get_session_meta` function L137-139 — `(&self, id: Uuid) -> Result<Option<SessionMeta>, StorageError>`
- pub `list_sessions_for_workstream` function L141-146 — `( &self, ws_id: Uuid, ) -> Result<Vec<SessionMeta>, StorageError>`
- pub `list_scratch_sessions` function L148-150 — `(&self) -> Result<Vec<SessionMeta>, StorageError>`
- pub `reconcile_sessions` function L154-186 — `(&self) -> Result<usize, StorageError>` — Remove SQLite session records whose JSONL files no longer exist on disk.
- pub `load_session` function L203-220 — `(&self, id: Uuid) -> Result<Option<Session>, StorageError>` — Load a full session (metadata + messages) by ID.
- pub `update_session_stats` function L222-228 — `( &self, session_id: Uuid, stats: &arawn_core::SessionStats, ) -> Result<(), Sto...`
- pub `append_message` function L232-239 — `( &self, session_id: Uuid, workstream_dir: &str, msg: &Message, ) -> Result<(), ...`
- pub `load_messages` function L241-247 — `( &self, session_id: Uuid, workstream_dir: &str, ) -> Result<Vec<Message>, Stora...`
- pub `promote_session` function L253-306 — `( &self, session_id: Uuid, new_ws_id: Uuid, ) -> Result<(), StorageError>` — Promote a scratch session to a workstream.
- pub `sandbox_for` function L309-312 — `(&self, workstream_dir: &str, session_id: Uuid, is_scratch: bool) -> PathBuf` — Resolve the sandbox root for a session.
- pub `promote_session_metadata` function L316-328 — `( &self, session_id: Uuid, new_ws_id: Uuid, ) -> Result<(), StorageError>` — Sync-only part of session promotion: update SQLite workstream_id.
- pub `move_session_jsonl` function L331-340 — `( &self, session_id: Uuid, from_ws_dir: &str, to_ws_dir: &str, ) -> Result<(), S...` — Async part of session promotion: move the JSONL file between workstream dirs.
-  `Store` type L22-341 — `= Store`
-  `resolve_ws_dir` function L190-200 — `(&self, ws_id: Option<Uuid>) -> Result<String, StorageError>` — Resolve the directory name for a workstream by UUID.
-  `copy_dir_contents` function L344-357 — `(src: &Path, dst: &Path) -> Result<(), StorageError>` — Recursively copy directory contents from src to dst.
-  `tests` module L360-529 — `-`
-  `setup` function L364-368 — `() -> (TempDir, Store)`
-  `open_creates_directories_and_db` function L371-377 — `()`
-  `open_is_idempotent` function L380-385 — `()`
-  `create_and_list_workstreams` function L388-396 — `()`
-  `create_scratch_session_and_append_messages` function L399-417 — `()`
-  `load_full_session` function L420-443 — `()`
-  `promote_session_full_flow` function L446-486 — `()`
-  `promote_bound_session_fails` function L489-502 — `()`
-  `load_nonexistent_session_returns_none` function L505-509 — `()`
-  `sandbox_for_scratch_is_per_session` function L512-519 — `()`
-  `sandbox_for_named_is_shared` function L522-528 — `()`

#### crates/arawn-storage/src/workstream_store.rs

- pub `WorkstreamStore` struct L22-24 — `{ db: &'a Database }` — Workstream registry.
- pub `new` function L27-29 — `(db: &'a Database) -> Self` — for users.
- pub `ensure_scratch` function L33-40 — `(&self, scratch_root: &Path) -> Result<Workstream, StorageError>` — Idempotently create the `scratch` workstream at the given root.
- pub `create` function L44-58 — `(&self, ws: &Workstream) -> Result<(), StorageError>` — Create a new workstream.
- pub `get` function L88-96 — `(&self, id: Uuid) -> Result<Option<Workstream>, StorageError>` — for users.
- pub `find_by_name` function L98-106 — `(&self, name: &str) -> Result<Option<Workstream>, StorageError>` — for users.
- pub `list` function L109-111 — `(&self) -> Result<Vec<Workstream>, StorageError>` — List active (non-archived) workstreams, newest update first.
- pub `list_all` function L114-116 — `(&self) -> Result<Vec<Workstream>, StorageError>` — List all workstreams including soft-deleted (archived) ones.
- pub `update_description` function L137-152 — `( &self, name: &str, description: &str, ) -> Result<(), StorageError>` — for users.
- pub `set_bindings` function L154-167 — `(&self, name: &str, bindings: &[String]) -> Result<(), StorageError>` — for users.
- pub `add_binding` function L169-178 — `(&self, name: &str, feed_id: &str) -> Result<(), StorageError>` — for users.
- pub `remove_binding` function L180-186 — `(&self, name: &str, feed_id: &str) -> Result<(), StorageError>` — for users.
- pub `soft_delete` function L191-207 — `(&self, name: &str) -> Result<(), StorageError>` — Soft-delete: sets `archived = 1`.
- pub `delete` function L211-217 — `(&self, id: Uuid) -> Result<bool, StorageError>` — Hard-delete by id.
-  `insert_row` function L60-86 — `(&self, ws: &Workstream) -> Result<(), StorageError>` — for users.
-  `list_with_archived` function L118-135 — `(&self, include_archived: bool) -> Result<Vec<Workstream>, StorageError>` — for users.
-  `SELECT_COLS_WHERE_ID` variable L220-223 — `: &str` — for users.
-  `SELECT_COLS_WHERE_NAME` variable L225-228 — `: &str` — for users.
-  `row_to_workstream` function L230-263 — `(row: &rusqlite::Row<'_>) -> Result<Workstream, StorageError>` — for users.
-  `parse_dt` function L265-269 — `(s: &str) -> Result<DateTime<Utc>, StorageError>` — for users.
-  `rusqlite_map_err` function L271-273 — `(e: StorageError) -> rusqlite::Error` — for users.
-  `name_err` function L275-277 — `(e: WorkstreamNameError) -> StorageError` — for users.
-  `tests` module L280-403 — `-` — for users.
-  `setup` function L283-285 — `() -> Database` — for users.
-  `create_and_roundtrip` function L288-300 — `()` — for users.
-  `create_rejects_scratch` function L303-309 — `()` — for users.
-  `create_rejects_invalid_slug` function L312-323 — `()` — for users.
-  `create_rejects_duplicate` function L326-332 — `()` — for users.
-  `ensure_scratch_idempotent` function L335-342 — `()` — for users.
-  `update_description` function L345-352 — `()` — for users.
-  `bindings_add_and_remove` function L355-367 — `()` — for users.
-  `soft_delete_marks_archived` function L370-381 — `()` — for users.
-  `soft_delete_refuses_scratch` function L384-390 — `()` — for users.
-  `list_orders_by_updated_at_desc` function L393-402 — `()` — for users.

### crates/arawn-tests

**Role**: Integration and system test crate — exercises subsystem interactions that unit tests within individual crates cannot cover, including full pipeline wiring, WebSocket protocol, UAT scenarios, and cross-crate behaviors.

#### crates/arawn-tests/build.rs

-  `main` function L1-3 — `()`

### crates/arawn-tests/tests

**Role**: Integration test suite covering the full stack, WebSocket protocol compliance, permission enforcement, hook wiring, hot-reload, skills, plugin components, memory tools, workflow tooling, and UAT scenarios.

**Key abstractions**:
- `compaction.rs` — Tests the full compaction path: over-threshold detection → LLM summarization call → `Session::compact` → JSONL persistence of the Summary → correct resume after reload.
- `engine_persistence.rs` — Tests MockLLM → QueryEngine → Store → JSONL/SQLite round-trips: multi-turn persistence, session resume, tool result persistence, scratch → promoted session migration, session isolation.
- `full_pipeline.rs` — Single test wiring all subsystems simultaneously (compactor, permissions, hooks, skills, plugins, plan mode) to verify they compose without conflict.
- `hooks.rs` — Engine-level integration: pre-tool blocking, allowing, post-tool firing, content-pattern matching, multiple hook aggregation.
- `hot_reload.rs` — Tests `PermissionChecker::update_rules` and `update_mode` mid-session without restart.
- `local_service.rs` — Tests `LocalService` (the real `ArawnService` impl) including separate engine/compactor LLMs, workstream creation, session promotion, multi-turn history accumulation, engine error propagation.
- `permissions.rs` — Engine-level permission checks: deny/allow rules, mode switching, session grants, ask-with-mock.
- `websocket.rs` — Spins up a real WebSocket server on a random port and exercises the JSON-RPC protocol: session CRUD, message streaming, error responses, concurrent requests.
- `uat.rs` — `UatHarness` launches the actual `arawn serve` process, connects a WebSocket client, and drives multi-turn scenarios with an LLM judge evaluating correctness. Requires real API keys; run via `angreal test uat`.
- `tool_artifacts.rs` — Validates tool outputs: file_write/read round-trip, file_edit correctness, shell output capture, workflow scaffold compilation.
- `workflows.rs` / `skills.rs` — Verify workflow tools and skill invocation through the QueryEngine.

**Mixed concerns / gotchas**: `uat.rs` exports its types as `pub` because the `UatHarness` and scenario types are also referenced from the Python functional test script and may be invoked via external tooling. Tests that spin up real servers bind to port 0 (OS-assigned) to avoid conflicts.

#### crates/arawn-tests/tests/compaction.rs

-  `engine_with_compactor_compacts_when_over_threshold` function L18-73 — `()` — Tests the full stack: compaction detection, LLM summarization, session mutation, persistence.
-  `engine_without_compactor_no_compaction` function L76-92 — `()` — Tests the full stack: compaction detection, LLM summarization, session mutation, persistence.
-  `engine_under_threshold_no_compaction` function L95-122 — `()` — Tests the full stack: compaction detection, LLM summarization, session mutation, persistence.
-  `persistence_summary_survives_save_and_load` function L127-191 — `()` — Tests the full stack: compaction detection, LLM summarization, session mutation, persistence.
-  `persistence_no_summary_loads_all` function L194-227 — `()` — Tests the full stack: compaction detection, LLM summarization, session mutation, persistence.
-  `persistence_resume_after_compaction` function L230-290 — `()` — Tests the full stack: compaction detection, LLM summarization, session mutation, persistence.

#### crates/arawn-tests/tests/engine_persistence.rs

-  `Fixture` struct L16-21 — `{ _tmp: TempDir, store: Store, workstream: Workstream, ws_dir: String }` — Helper: set up a full stack with Store + Engine + MockLLM in a temp directory.
-  `Fixture` type L23-72 — `= Fixture` — These test the full stack: MockLLM → QueryEngine → Tools → Store → JSONL/SQLite.
-  `new` function L24-36 — `() -> Self` — These test the full stack: MockLLM → QueryEngine → Tools → Store → JSONL/SQLite.
-  `new_session` function L38-42 — `(&self) -> Session` — These test the full stack: MockLLM → QueryEngine → Tools → Store → JSONL/SQLite.
-  `scratch_session` function L44-48 — `(&self) -> Session` — These test the full stack: MockLLM → QueryEngine → Tools → Store → JSONL/SQLite.
-  `context` function L50-52 — `(&self, session: &Session) -> ToolContext` — These test the full stack: MockLLM → QueryEngine → Tools → Store → JSONL/SQLite.
-  `registry` function L54-60 — `(&self) -> Arc<ToolRegistry>` — These test the full stack: MockLLM → QueryEngine → Tools → Store → JSONL/SQLite.
-  `engine` function L62-71 — `(&self, mock: Arc<MockLlmClient>, registry: Arc<ToolRegistry>) -> QueryEngine` — These test the full stack: MockLLM → QueryEngine → Tools → Store → JSONL/SQLite.
-  `engine_run_persists_all_messages` function L75-116 — `()` — These test the full stack: MockLLM → QueryEngine → Tools → Store → JSONL/SQLite.
-  `session_resume_continues_conversation` function L119-186 — `()` — These test the full stack: MockLLM → QueryEngine → Tools → Store → JSONL/SQLite.
-  `tool_results_persisted_with_content` function L189-239 — `()` — These test the full stack: MockLLM → QueryEngine → Tools → Store → JSONL/SQLite.
-  `scratch_session_promotion_preserves_messages` function L242-298 — `()` — These test the full stack: MockLLM → QueryEngine → Tools → Store → JSONL/SQLite.
-  `multiple_sessions_isolated` function L301-367 — `()` — These test the full stack: MockLLM → QueryEngine → Tools → Store → JSONL/SQLite.

#### crates/arawn-tests/tests/full_pipeline.rs

-  `full_pipeline_all_subsystems_wired` function L18-167 — `()` — wired into the QueryEngine simultaneously.

#### crates/arawn-tests/tests/hooks.rs

-  `assert_tool_result_is_error` function L14-27 — `(msgs: &[Message], index: usize, substring: &str)` — Integration tests: hook system wired into the QueryEngine.
-  `assert_tool_result_ok` function L29-41 — `(msgs: &[Message], index: usize)` — Integration tests: hook system wired into the QueryEngine.
-  `make_hook_config` function L43-45 — `(json: serde_json::Value) -> HookConfig` — Integration tests: hook system wired into the QueryEngine.
-  `pre_tool_use_blocking_hook_stops_execution` function L50-73 — `()` — Integration tests: hook system wired into the QueryEngine.
-  `pre_tool_use_allowing_hook_permits_execution` function L76-99 — `()` — Integration tests: hook system wired into the QueryEngine.
-  `post_tool_use_hook_fires_after_tool` function L102-135 — `()` — Integration tests: hook system wired into the QueryEngine.
-  `hook_with_content_pattern_matching` function L138-170 — `()` — Integration tests: hook system wired into the QueryEngine.
-  `multiple_hooks_one_blocks_aggregated_block` function L173-200 — `()` — Integration tests: hook system wired into the QueryEngine.
-  `no_matching_hooks_tool_executes_normally` function L203-226 — `()` — Integration tests: hook system wired into the QueryEngine.

#### crates/arawn-tests/tests/hot_reload.rs

-  `assert_tool_result_is_error` function L15-28 — `(msgs: &[Message], index: usize, substring: &str)` — Integration tests: hot-reload APIs on PermissionChecker mid-session.
-  `assert_tool_result_ok` function L30-39 — `(msgs: &[Message], index: usize)` — Integration tests: hot-reload APIs on PermissionChecker mid-session.
-  `update_rules_changes_behavior` function L44-80 — `()` — Integration tests: hot-reload APIs on PermissionChecker mid-session.
-  `update_mode_changes_behavior` function L83-122 — `()` — Integration tests: hot-reload APIs on PermissionChecker mid-session.
-  `engine_uses_updated_rules_without_restart` function L125-167 — `()` — Integration tests: hot-reload APIs on PermissionChecker mid-session.

#### crates/arawn-tests/tests/local_service.rs

-  `setup_service` function L14-41 — `(responses: Vec<MockResponse>) -> (TempDir, arawn_bin::LocalService)` — Tests for LocalService — the ArawnService impl that wraps engine + store.
-  `separate_engine_and_compactor_llms_are_stored_distinctly` function L44-92 — `()` — Tests for LocalService — the ArawnService impl that wraps engine + store.
-  `list_workstreams_returns_scratch` function L95-100 — `()` — Tests for LocalService — the ArawnService impl that wraps engine + store.
-  `create_and_load_session_roundtrip` function L103-115 — `()` — Tests for LocalService — the ArawnService impl that wraps engine + store.
-  `send_message_text_only_returns_complete` function L118-140 — `()` — Tests for LocalService — the ArawnService impl that wraps engine + store.
-  `send_message_with_tool_call_returns_events` function L143-175 — `()` — Tests for LocalService — the ArawnService impl that wraps engine + store.
-  `send_message_persists_to_jsonl` function L178-200 — `()` — Tests for LocalService — the ArawnService impl that wraps engine + store.
-  `create_workstream_with_default_root_dir` function L203-223 — `()` — Tests for LocalService — the ArawnService impl that wraps engine + store.
-  `promote_scratch_session_to_workstream` function L226-271 — `()` — Tests for LocalService — the ArawnService impl that wraps engine + store.
-  `promote_non_scratch_session_fails` function L274-293 — `()` — Tests for LocalService — the ArawnService impl that wraps engine + store.
-  `multi_turn_conversation_accumulates_history` function L296-325 — `()` — Tests for LocalService — the ArawnService impl that wraps engine + store.
-  `list_sessions_returns_multiple` function L328-348 — `()` — Tests for LocalService — the ArawnService impl that wraps engine + store.
-  `engine_error_produces_error_event` function L351-372 — `()` — Tests for LocalService — the ArawnService impl that wraps engine + store.
-  `multi_turn_with_tool_calls_accumulates_full_history` function L375-410 — `()` — Tests for LocalService — the ArawnService impl that wraps engine + store.
-  `session_isolation_separate_histories` function L413-474 — `()` — Tests for LocalService — the ArawnService impl that wraps engine + store.
-  `large_conversation_five_turns_persisted` function L477-503 — `()` — Tests for LocalService — the ArawnService impl that wraps engine + store.
-  `error_after_successful_first_turn_preserves_history` function L506-547 — `()` — Tests for LocalService — the ArawnService impl that wraps engine + store.

#### crates/arawn-tests/tests/memory_stack.rs

-  `setup` function L12-17 — `() -> (TempDir, MemoryManager)` — shortcode compression, L2 topical injection, and deduplication.
-  `estimate_tokens` function L19-21 — `(text: &str) -> usize` — shortcode compression, L2 topical injection, and deduplication.
-  `wake_up_under_budget_with_many_entities` function L26-58 — `()` — shortcode compression, L2 topical injection, and deduplication.
-  `l1_ranks_stated_highest` function L61-86 — `()` — shortcode compression, L2 topical injection, and deduplication.
-  `empty_kb_produces_l0_only` function L89-97 — `()` — shortcode compression, L2 topical injection, and deduplication.
-  `tiny_budget_does_not_panic` function L100-111 — `()` — shortcode compression, L2 topical injection, and deduplication.
-  `shortcodes_applied_in_l1_output` function L116-134 — `()` — shortcode compression, L2 topical injection, and deduplication.
-  `shortcode_standalone_compression` function L137-149 — `()` — shortcode compression, L2 topical injection, and deduplication.
-  `shortcode_single_occurrence_unchanged` function L152-157 — `()` — shortcode compression, L2 topical injection, and deduplication.
-  `l2_retrieves_by_keyword` function L162-184 — `()` — shortcode compression, L2 topical injection, and deduplication.
-  `l2_deduplicates_against_l1` function L187-209 — `()` — shortcode compression, L2 topical injection, and deduplication.
-  `l2_empty_keywords_returns_none` function L212-217 — `()` — shortcode compression, L2 topical injection, and deduplication.
-  `retrieve_topical_respects_budget` function L220-241 — `()` — shortcode compression, L2 topical injection, and deduplication.

#### crates/arawn-tests/tests/memory_tools.rs

-  `MockEmbedder` struct L16-18 — `{ dims: usize }` — Bag-of-words embedder for deterministic testing.
-  `MockEmbedder` type L20-46 — `= MockEmbedder` — KB storage → retrieval → response.
-  `new` function L21-23 — `(dims: usize) -> Self` — KB storage → retrieval → response.
-  `embed_sync` function L25-45 — `(&self, text: &str) -> Vec<f32>` — KB storage → retrieval → response.
-  `MockEmbedder` type L49-57 — `impl Embedder for MockEmbedder` — KB storage → retrieval → response.
-  `embed` function L50-52 — `(&self, text: &str) -> Result<Vec<f32>, arawn_embed::EmbedError>` — KB storage → retrieval → response.
-  `dimensions` function L54-56 — `(&self) -> usize` — KB storage → retrieval → response.
-  `setup_memory_manager` function L59-69 — `() -> (Arc<MemoryManager>, Option<Arc<dyn Embedder>>)` — KB storage → retrieval → response.
-  `memory_store_inserts_entity` function L72-105 — `()` — KB storage → retrieval → response.
-  `memory_store_preference_goes_to_global` function L108-135 — `()` — KB storage → retrieval → response.
-  `memory_store_person_goes_to_global` function L138-158 — `()` — KB storage → retrieval → response.
-  `memory_store_deduplicates_on_reinsertion` function L161-196 — `()` — KB storage → retrieval → response.
-  `memory_search_finds_stored_entity` function L199-240 — `()` — KB storage → retrieval → response.
-  `memory_search_filters_by_type` function L243-289 — `()` — KB storage → retrieval → response.
-  `memory_store_then_search_roundtrip` function L292-346 — `()` — KB storage → retrieval → response.
-  `memory_search_empty_kb_returns_no_results` function L349-373 — `()` — KB storage → retrieval → response.
-  `memory_store_with_tags` function L376-396 — `()` — KB storage → retrieval → response.
-  `memory_store_explicit_scope_override` function L399-424 — `()` — KB storage → retrieval → response.

#### crates/arawn-tests/tests/permissions.rs

-  `assert_tool_result_is_error` function L15-28 — `(msgs: &[Message], index: usize, substring: &str)` — Integration tests: permission system wired into the QueryEngine.
-  `assert_tool_result_ok` function L30-42 — `(msgs: &[Message], index: usize)` — Integration tests: permission system wired into the QueryEngine.
-  `deny_rule_blocks_tool_call` function L47-65 — `()` — Integration tests: permission system wired into the QueryEngine.
-  `allow_rule_permits_tool_call` function L68-86 — `()` — Integration tests: permission system wired into the QueryEngine.
-  `bypass_mode_allows_all_tools` function L89-107 — `()` — Integration tests: permission system wired into the QueryEngine.
-  `accept_edits_mode_allows_file_write_but_asks_shell` function L110-144 — `()` — Integration tests: permission system wired into the QueryEngine.
-  `ask_rule_with_mock_allowing` function L147-166 — `()` — Integration tests: permission system wired into the QueryEngine.
-  `ask_rule_with_mock_denying` function L169-188 — `()` — Integration tests: permission system wired into the QueryEngine.
-  `session_grants_persist_across_turns` function L191-227 — `()` — Integration tests: permission system wired into the QueryEngine.

#### crates/arawn-tests/tests/plugin_components.rs

-  `write_plugin_json` function L15-26 — `(dir: &std::path::Path, name: &str)` — Create a minimal valid plugin directory with plugin.json.
-  `create_cache_plugin` function L29-38 — `(root: &std::path::Path, marketplace: &str, name: &str) -> std::path::PathBuf` — Create a plugin cache directory: cache/{marketplace}/{plugin}/{version}/
-  `write_skill` function L41-50 — `(dir: &std::path::Path, filename: &str, description: &str, prompt: &str)` — Write a skill markdown file into a directory.
-  `write_agent` function L53-62 — `(dir: &std::path::Path, filename: &str, name: &str, description: &str)` — Write an agent markdown file into a directory.
-  `write_hooks_json` function L65-73 — `(dir: &std::path::Path)` — Write a hooks.json file.
-  `discover_plugins_finds_cache_plugin` function L78-86 — `()` — Integration tests: plugin discovery, manifest parsing, and component loading.
-  `discover_plugins_finds_multiple` function L89-97 — `()` — Integration tests: plugin discovery, manifest parsing, and component loading.
-  `load_plugin_dir_parses_manifest` function L100-107 — `()` — Integration tests: plugin discovery, manifest parsing, and component loading.
-  `load_plugin_components_loads_skills` function L110-138 — `()` — Integration tests: plugin discovery, manifest parsing, and component loading.
-  `load_plugin_components_loads_agents` function L141-165 — `()` — Integration tests: plugin discovery, manifest parsing, and component loading.
-  `load_plugin_components_loads_hooks` function L168-210 — `()` — Integration tests: plugin discovery, manifest parsing, and component loading.
-  `register_plugin_skills_namespaces_into_registry` function L213-233 — `()` — Integration tests: plugin discovery, manifest parsing, and component loading.
-  `invalid_manifest_gracefully_skipped` function L236-255 — `()` — Integration tests: plugin discovery, manifest parsing, and component loading.
-  `plugin_with_mixed_valid_invalid_components` function L258-290 — `()` — Integration tests: plugin discovery, manifest parsing, and component loading.
-  `empty_cache_returns_no_plugins` function L293-298 — `()` — Integration tests: plugin discovery, manifest parsing, and component loading.

#### crates/arawn-tests/tests/skills.rs

-  `assert_tool_result_ok_contains` function L13-26 — `(msgs: &[Message], index: usize, substring: &str)` — Integration tests: skill loading and invocation through the QueryEngine.
-  `assert_tool_result_is_error` function L28-41 — `(msgs: &[Message], index: usize, substring: &str)` — Integration tests: skill loading and invocation through the QueryEngine.
-  `make_skill` function L43-54 — `(name: &str, prompt: &str, user_invocable: bool, source: SkillSource) -> SkillDe...` — Integration tests: skill loading and invocation through the QueryEngine.
-  `register_skill_in_memory_invoke_through_engine` function L59-80 — `()` — Integration tests: skill loading and invocation through the QueryEngine.
-  `load_skill_from_markdown_file_and_invoke` function L83-119 — `()` — Integration tests: skill loading and invocation through the QueryEngine.
-  `skill_not_found_returns_error` function L122-145 — `()` — Integration tests: skill loading and invocation through the QueryEngine.
-  `user_invocable_filtering` function L148-157 — `()` — Integration tests: skill loading and invocation through the QueryEngine.
-  `plugin_namespaced_skill_accessible` function L160-180 — `()` — Integration tests: skill loading and invocation through the QueryEngine.
-  `builtin_workflows_skill_loads_on_registry_creation` function L185-207 — `()` — Integration tests: skill loading and invocation through the QueryEngine.
-  `format_skill_listing_includes_builtins` function L212-225 — `()` — Integration tests: skill loading and invocation through the QueryEngine.
-  `skill_listing_appears_in_assembled_system_prompt` function L228-256 — `()` — Integration tests: skill loading and invocation through the QueryEngine.
-  `skill_descriptions_distinguish_different_use_cases` function L261-302 — `()` — Integration tests: skill loading and invocation through the QueryEngine.
-  `skill_invocation_chains_into_domain_tool` function L307-366 — `()` — Integration tests: skill loading and invocation through the QueryEngine.

#### crates/arawn-tests/tests/tool_artifacts.rs

-  `make_ctx` function L15-18 — `(tmp: &TempDir) -> EngineToolContext` — and validate the produced artifact (file exists, compiles, is searchable, etc.)
-  `file_write_read_roundtrip` function L25-65 — `()` — and validate the produced artifact (file exists, compiles, is searchable, etc.)
-  `file_edit_applies_correctly` function L72-126 — `()` — and validate the produced artifact (file exists, compiles, is searchable, etc.)
-  `shell_captures_output` function L133-148 — `()` — and validate the produced artifact (file exists, compiles, is searchable, etc.)
-  `shell_captures_exit_code_on_failure` function L151-165 — `()` — and validate the produced artifact (file exists, compiles, is searchable, etc.)
-  `workflow_create_minimal_compiles` function L173-225 — `()` — and validate the produced artifact (file exists, compiles, is searchable, etc.)
-  `workflow_create_with_cron_compiles` function L229-261 — `()` — and validate the produced artifact (file exists, compiles, is searchable, etc.)
-  `workflow_list_shows_installed` function L268-291 — `()` — and validate the produced artifact (file exists, compiles, is searchable, etc.)
-  `workflow_delete_removes_installed` function L294-315 — `()` — and validate the produced artifact (file exists, compiles, is searchable, etc.)

#### crates/arawn-tests/tests/uat.rs

- pub `Scenario` struct L25-30 — `{ name: String, objective: String, turns: Vec<ScenarioTurn>, mechanical: Mechani...` — Or via angreal: angreal test uat --model gemma4
- pub `ScenarioTurn` struct L33-36 — `{ user_message: String, judge_expectation: String }` — Or via angreal: angreal test uat --model gemma4
- pub `MechanicalThresholds` struct L39-45 — `{ min_files_created: usize, min_workflows_created: usize, min_memory_entities: u...` — Or via angreal: angreal test uat --model gemma4
- pub `TurnResult` struct L52-63 — `{ turn_number: usize, user_message: String, assistant_text: String, tool_calls: ...` — Or via angreal: angreal test uat --model gemma4
- pub `ToolCallRecord` struct L66-70 — `{ id: String, name: String, input: Value }` — Or via angreal: angreal test uat --model gemma4
- pub `ToolResultRecord` struct L73-77 — `{ id: String, content: String, is_error: bool }` — Or via angreal: angreal test uat --model gemma4
- pub `ScenarioResult` struct L84-91 — `{ scenario_name: String, model: String, turns: Vec<TurnResult>, mechanical: Mech...` — Or via angreal: angreal test uat --model gemma4
- pub `MechanicalCheckResult` struct L94-102 — `{ all_turns_completed: bool, no_errors: bool, tool_use_occurred: bool, files_cre...` — Or via angreal: angreal test uat --model gemma4
- pub `UatHarness` struct L185-189 — `{ data_dir: PathBuf, port: u16, server_process: Option<Child> }` — Or via angreal: angreal test uat --model gemma4
- pub `new` function L193-246 — `(base_dir: &Path, model: &str, provider: &str, api_key_env: &str) -> Self` — Create a new harness with an isolated data directory.
- pub `start_server` function L249-272 — `(&mut self) -> Result<(), String>` — Start the arawn server process.
- pub `wait_for_ready` function L275-299 — `(&self, timeout: Duration) -> Result<(), String>` — Wait for the server to be ready by polling the WebSocket endpoint.
- pub `ws_url` function L301-313 — `(&self) -> String` — Or via angreal: angreal test uat --model gemma4
- pub `run_scenario` function L316-375 — `(&self, scenario: &Scenario, model: &str) -> ScenarioResult` — Run a scenario: create session, drive all turns, collect results.
- pub `write_artifacts` function L481-529 — `(&self, result: &ScenarioResult, scenario: &Scenario)` — Write all artifacts to the results directory.
- pub `stop` function L532-538 — `(&mut self)` — Stop the server process.
-  `TurnAccumulator` struct L110-117 — `{ assistant_text: String, tool_calls: Vec<ToolCallRecord>, tool_results: Vec<Too...` — State accumulated while consuming engine events for a single turn.
-  `count_workflows_in` function L121-129 — `(dir: &Path) -> usize` — Count subdirectories of `dir`.
-  `apply_event` function L133-179 — `(event: &Value, acc: &mut TurnAccumulator) -> bool` — Apply one engine event JSON value to the accumulator.
-  `UatHarness` type L191-539 — `= UatHarness` — Or via angreal: angreal test uat --model gemma4
-  `rpc_create_session` function L377-403 — `( &self, write: &mut futures_util::stream::SplitSink< tokio_tungstenite::WebSock...` — Or via angreal: angreal test uat --model gemma4
-  `drive_turn` function L405-458 — `( &self, write: &mut futures_util::stream::SplitSink< tokio_tungstenite::WebSock...` — Or via angreal: angreal test uat --model gemma4
-  `list_workspace_files` function L460-473 — `(&self) -> Vec<String>` — Or via angreal: angreal test uat --model gemma4
-  `count_installed_workflows` function L476-478 — `(&self) -> usize` — Count installed workflows under `<data_dir>/workflows/`.
-  `UatHarness` type L541-545 — `impl Drop for UatHarness` — Or via angreal: angreal test uat --model gemma4
-  `drop` function L542-544 — `(&mut self)` — Or via angreal: angreal test uat --model gemma4
-  `walkdir` function L548-563 — `(dir: &Path) -> Result<Vec<PathBuf>, std::io::Error>` — Recursively list all files under a directory.
-  `github_monitor_scenario` function L569-598 — `() -> Scenario` — Or via angreal: angreal test uat --model gemma4
-  `work_signal_pipeline_scenario` function L600-633 — `() -> Scenario` — Or via angreal: angreal test uat --model gemma4
-  `all_scenarios` function L635-637 — `() -> Vec<Scenario>` — Or via angreal: angreal test uat --model gemma4
-  `uat_run` function L645-745 — `()` — Or via angreal: angreal test uat --model gemma4
-  `tests` module L753-900 — `-` — Or via angreal: angreal test uat --model gemma4
-  `count_workflows_returns_zero_for_missing_dir` function L759-762 — `()` — Or via angreal: angreal test uat --model gemma4
-  `count_workflows_returns_zero_for_empty_dir` function L765-768 — `()` — Or via angreal: angreal test uat --model gemma4
-  `count_workflows_counts_subdirs_only` function L771-779 — `()` — Or via angreal: angreal test uat --model gemma4
-  `apply_event_captures_error_message` function L784-800 — `()` — Or via angreal: angreal test uat --model gemma4
-  `apply_event_error_with_missing_message_field_keeps_none` function L803-809 — `()` — Or via angreal: angreal test uat --model gemma4
-  `apply_event_complete_sets_final_text` function L812-819 — `()` — Or via angreal: angreal test uat --model gemma4
-  `apply_event_streaming_text_appends` function L822-830 — `()` — Or via angreal: angreal test uat --model gemma4
-  `apply_event_ignores_rpc_ack` function L833-840 — `()` — Or via angreal: angreal test uat --model gemma4
-  `apply_event_records_tool_calls_and_results` function L843-863 — `()` — Or via angreal: angreal test uat --model gemma4
-  `turn_result_serializes_error_message_when_present` function L868-882 — `()` — Or via angreal: angreal test uat --model gemma4
-  `turn_result_omits_error_message_when_none` function L885-899 — `()` — Or via angreal: angreal test uat --model gemma4

#### crates/arawn-tests/tests/websocket.rs

-  `start_test_server` function L19-75 — `(mock_responses: Vec<MockResponse>) -> (String, TempDir)` — Spin up a test server on a random port and return the WS URL.
-  `send_request` function L78-100 — `( write: &mut futures_util::stream::SplitSink< tokio_tungstenite::WebSocketStrea...` — Helper: send a JSON request and get the response.
-  `list_workstreams_returns_scratch` function L103-119 — `()` — Spins up the server on a random port, connects a WS client, exercises the JSON protocol.
-  `create_and_load_session` function L122-148 — `()` — Spins up the server on a random port, connects a WS client, exercises the JSON protocol.
-  `unknown_method_returns_error` function L151-165 — `()` — Spins up the server on a random port, connects a WS client, exercises the JSON protocol.
-  `malformed_json_returns_error` function L168-182 — `()` — Spins up the server on a random port, connects a WS client, exercises the JSON protocol.
-  `send_message_streams_complete_event` function L187-240 — `()` — Spins up the server on a random port, connects a WS client, exercises the JSON protocol.
-  `send_message_with_tool_call_streams_events` function L243-310 — `()` — Spins up the server on a random port, connects a WS client, exercises the JSON protocol.
-  `list_sessions_via_ws` function L313-356 — `()` — Spins up the server on a random port, connects a WS client, exercises the JSON protocol.
-  `load_session_missing_id_returns_error` function L359-375 — `()` — Spins up the server on a random port, connects a WS client, exercises the JSON protocol.
-  `send_message_missing_id_returns_error` function L378-394 — `()` — Spins up the server on a random port, connects a WS client, exercises the JSON protocol.
-  `create_workstream_via_ws` function L397-438 — `()` — Spins up the server on a random port, connects a WS client, exercises the JSON protocol.
-  `get_and_set_permission_mode_via_ws` function L441-472 — `()` — Spins up the server on a random port, connects a WS client, exercises the JSON protocol.
-  `multi_turn_conversation_over_ws` function L475-553 — `()` — Spins up the server on a random port, connects a WS client, exercises the JSON protocol.
-  `send_and_wait_complete` function L494-544 — `( write: &mut futures_util::stream::SplitSink< tokio_tungstenite::WebSocketStrea...` — Spins up the server on a random port, connects a WS client, exercises the JSON protocol.
-  `rapid_fire_requests_same_connection` function L556-592 — `()` — Spins up the server on a random port, connects a WS client, exercises the JSON protocol.
-  `send_message_nonexistent_session_returns_error` function L595-638 — `()` — Spins up the server on a random port, connects a WS client, exercises the JSON protocol.

#### crates/arawn-tests/tests/workflows.rs

-  `assert_tool_result_ok_contains` function L12-25 — `(msgs: &[Message], index: usize, substring: &str)` — Integration tests: workflow tools and skill activation through the QueryEngine.
-  `assert_tool_result_is_error` function L27-36 — `(msgs: &[Message], index: usize)` — Integration tests: workflow tools and skill activation through the QueryEngine.
-  `workflows_skill_activates_on_workflow_request` function L41-63 — `()` — Integration tests: workflow tools and skill activation through the QueryEngine.
-  `workflows_skill_contains_decision_callback_pattern` function L66-82 — `()` — Integration tests: workflow tools and skill activation through the QueryEngine.
-  `workflow_list_empty_directory` function L87-102 — `()` — Integration tests: workflow tools and skill activation through the QueryEngine.
-  `workflow_list_shows_installed_packages` function L105-135 — `()` — Integration tests: workflow tools and skill activation through the QueryEngine.
-  `workflow_delete_removes_package` function L140-163 — `()` — Integration tests: workflow tools and skill activation through the QueryEngine.
-  `workflow_delete_nonexistent_errors` function L166-184 — `()` — Integration tests: workflow tools and skill activation through the QueryEngine.
-  `workflow_status_no_runner_returns_error` function L189-204 — `()` — Integration tests: workflow tools and skill activation through the QueryEngine.
-  `workflow_status_with_runner_returns_empty_list` function L207-226 — `()` — Integration tests: workflow tools and skill activation through the QueryEngine.
-  `scaffold_generates_compilable_project` function L231-281 — `()` — Integration tests: workflow tools and skill activation through the QueryEngine.
-  `skill_then_tool_workflow_creation_chain` function L286-318 — `()` — Integration tests: workflow tools and skill activation through the QueryEngine.

### crates/arawn-tool/src

**Role**: The foundational tool abstraction layer — defines the `Tool` trait, `PermissionCategory`, `ToolRegistry`, and the `ToolContext` that tools receive at execution time, plus LLM preference resolution types. This crate has no engine dependencies, enabling tool implementations to exist without circular dependencies.

**Key abstractions**:
- `Tool` trait — Async trait with `name()`, `description()`, `parameters_schema() -> Value`, `execute(ctx, params) -> Result<ToolOutput, ToolError>`, and defaulted methods: `is_read_only()` (false), `category() -> ToolCategory` (Core), `permission_category() -> PermissionCategory` (returns `ReadOnly` when `is_read_only()` is true, otherwise `Other`), `llm_preference() -> Option<LlmPreference>` (None). The `permission_category()` default is the key behavior: read-only tools automatically get `ReadOnly` without explicitly overriding; write tools must override to `FileWrite` or `Shell`.
- `PermissionCategory` — `ReadOnly | FileWrite | Shell | Other`. Used by `PermissionMode::fallback()` to decide whether to auto-allow, ask, or deny a tool when no explicit rule matches. Distinct from `ToolCategory` (which is about feature-area grouping for context filtering).
- `ToolRegistry` — Concurrent `RwLock<HashMap<String, Arc<dyn Tool>>>`. Tracks plugin tools separately (in `plugin_tools: HashSet<String>`) so they can be removed by name during hot-reload without touching built-in tools. `unregister_by_prefix` removes all tools with a given prefix — used to clean up a plugin's tools on disconnect.
- `ToolContext` trait — Runtime interface available to tools: `working_dir()`, `session_id()`, `validate_path()` (sandbox enforcement), `is_allowed_path()`, `mark_file_read()` / `has_read_file()`, `llm()`, `model()`, `model_limits()`, `data_dir()`, `agent_depth()`, `can_spawn_agent()`, `for_sub_agent()`, `workstream_name()`, `allowed_paths()`, `resolve_llm(&LlmPreference)`. The concrete impl is `EngineToolContext` in arawn-engine.
- `LlmPreference` / `LlmResolution` / `LlmResolverFn` — Types for tools and agents that want a specific LLM: a preference describes requirements (named entry, provider+model, or capabilities); a resolution carries the matched client. `LlmResolverFn = dyn Fn(&LlmPreference) -> LlmResolution + Send + Sync` — the closure alias stored in `EngineToolContext` (replacing the deleted `LlmResolver` trait). `ToolContext::resolve_llm` calls this closure.
- `ModelLimits` — Context window and compaction threshold for a known model, used by sub-agents. `for_model(name)` returns hard-coded limits for known Anthropic/OpenAI models.

**Dependencies**: `async-trait`, `serde`/`serde_json` (schema + output), `arawn-llm` (LlmClient for context methods), `uuid` (session ID).

#### crates/arawn-tool/src/context.rs

- pub `ModelLimits` struct L11-16 — `{ context_window: u32, compaction_threshold: f32 }` — Model context window limits — used by sub-agents for compaction decisions.
- pub `new` function L19-24 — `(context_window: u32, compaction_threshold: f32) -> Self`
- pub `for_model` function L27-42 — `(model: &str) -> Self` — Get default limits for a known model name.
- pub `should_compact` function L45-54 — `( &self, session_tokens: u32, tool_tokens: u32, system_tokens: u32, ) -> bool` — Check if the total estimated tokens exceed the compaction threshold.
- pub `available_for_messages` function L57-62 — `(&self, tool_tokens: u32, system_tokens: u32) -> u32` — The token budget available after accounting for tools and system prompt.
- pub `ToolContext` interface L78-132 — `{ fn working_dir(), fn session_id(), fn validate_path(), fn is_allowed_path(), f...` — Execution context provided to tools.
-  `ModelLimits` type L18-63 — `= ModelLimits`
-  `ModelLimits` type L65-72 — `impl Default for ModelLimits`
-  `default` function L66-71 — `() -> Self`
-  `resolve_llm` function L129-131 — `(&self, _preference: &LlmPreference) -> Option<LlmResolution>` — Resolve an [`LlmPreference`] against the runtime's LLM pool.

#### crates/arawn-tool/src/error.rs

- pub `ToolError` enum L8-24 — `ExecutionFailed | NotFound | Llm | Other` — Errors that tools can return from `execute()`.

#### crates/arawn-tool/src/lib.rs

-  `context` module L1 — `-`
-  `error` module L2 — `-`
-  `llm_preference` module L3 — `-`
-  `registry` module L4 — `-`
-  `tool` module L5 — `-`

#### crates/arawn-tool/src/llm_preference.rs

- pub `LlmPreference` struct L21-30 — `{ named: Option<String>, provider: Option<String>, model: Option<String>, capabi...` — What a tool or agent wants from an LLM.
- pub `any` function L34-36 — `() -> Self` — A preference that matches anything — resolves to the engine LLM.
- pub `named` function L39-44 — `(name: impl Into<String>) -> Self` — Request a specific named pool entry.
- pub `provider_model` function L47-53 — `(provider: impl Into<String>, model: impl Into<String>) -> Self` — Request a specific provider+model pair.
- pub `LlmCapabilities` struct L58-65 — `{ min_context_window: Option<u32>, tool_use: bool, vision: bool }` — Minimum capability requirements an LLM must satisfy.
- pub `satisfied_by` function L69-82 — `(&self, info: &ResolvedLlmInfo) -> bool` — Returns true if `info` meets every requirement.
- pub `is_empty` function L85-87 — `(&self) -> bool` — True if no capability constraints are set.
- pub `ResolvedLlmInfo` struct L94-100 — `{ provider: String, model: String, context_window: u32, tool_use: bool, vision: ...` — Static capability metadata for a resolved LLM.
- pub `LlmResolution` struct L103-107 — `{ client: Arc<dyn LlmClient>, info: ResolvedLlmInfo, match_quality: MatchQuality...` — The result of resolving an [`LlmPreference`] against a pool.
- pub `LlmResolverFn` type L123-124 — `= dyn Fn(&LlmPreference) -> LlmResolution + Send + Sync` — Type-erased resolver function.
- pub `MatchQuality` enum L128-135 — `Exact | Capability | Fallback` — How closely the resolved client matched the requested preference.
-  `LlmPreference` type L32-54 — `= LlmPreference` — them without pulling in `arawn-bin`.
-  `LlmCapabilities` type L67-88 — `= LlmCapabilities` — them without pulling in `arawn-bin`.
-  `LlmResolution` type L109-116 — `= LlmResolution` — them without pulling in `arawn-bin`.
-  `fmt` function L110-115 — `(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` — them without pulling in `arawn-bin`.
-  `tests` module L138-196 — `-` — them without pulling in `arawn-bin`.
-  `info` function L141-149 — `(provider: &str, model: &str, ctx: u32, tools: bool, vision: bool) -> ResolvedLl...` — them without pulling in `arawn-bin`.
-  `capabilities_default_is_satisfied_by_anything` function L152-155 — `()` — them without pulling in `arawn-bin`.
-  `capabilities_min_context_window_blocks_small_models` function L158-165 — `()` — them without pulling in `arawn-bin`.
-  `capabilities_tool_use_required` function L168-175 — `()` — them without pulling in `arawn-bin`.
-  `capabilities_vision_required` function L178-185 — `()` — them without pulling in `arawn-bin`.
-  `preference_constructors` function L188-195 — `()` — them without pulling in `arawn-bin`.

#### crates/arawn-tool/src/registry.rs

- pub `ToolRegistry` struct L8-12 — `{ tools: RwLock<HashMap<String, Arc<dyn Tool>>>, plugin_tools: RwLock<HashSet<St...` — Registry of available tools.
- pub `new` function L15-20 — `() -> Self`
- pub `register` function L23-26 — `(&self, tool: Box<dyn Tool>)` — Register a built-in tool.
- pub `register_plugin` function L29-36 — `(&self, tool: Box<dyn Tool>)` — Register a plugin-provided tool (tracked for hot-reload).
- pub `register_arc` function L39-42 — `(&self, tool: Arc<dyn Tool>)` — Register an already-Arc'd tool (used when building filtered registries).
- pub `unregister` function L44-47 — `(&self, name: &str) -> Option<Arc<dyn Tool>>`
- pub `plugin_tool_names` function L50-52 — `(&self) -> Vec<String>` — Returns the names of all currently loaded plugin tools.
- pub `get` function L55-57 — `(&self, name: &str) -> Option<Arc<dyn Tool>>` — Get a tool by name.
- pub `tool_definitions` function L59-69 — `(&self) -> Vec<arawn_llm::ToolDefinition>`
- pub `len` function L71-73 — `(&self) -> usize`
- pub `is_empty` function L75-77 — `(&self) -> bool`
- pub `unregister_by_prefix` function L80-95 — `(&self, prefix: &str) -> Vec<String>` — Unregister all tools whose names start with the given prefix.
-  `ToolRegistry` type L14-96 — `= ToolRegistry`
-  `ToolRegistry` type L98-102 — `impl Default for ToolRegistry`
-  `default` function L99-101 — `() -> Self`

#### crates/arawn-tool/src/tool.rs

- pub `ToolCategory` enum L12-31 — `Core | Task | Agent | Web | Memory | Plan | Workstream | Utility | BackgroundTas...` — Category of a tool — used for permission checking, context filtering, and
- pub `PermissionCategory` enum L37-48 — `ReadOnly | FileWrite | Shell | Other` — Risk class of a tool — used by the permission system to decide fallback
- pub `ToolOutput` struct L52-55 — `{ content: String, is_error: bool }` — Output from a tool execution.
- pub `success` function L58-63 — `(content: impl Into<String>) -> Self`
- pub `error` function L65-70 — `(content: impl Into<String>) -> Self`
- pub `Tool` interface L75-118 — `{ fn name(), fn description(), fn parameters_schema(), fn execute(), fn is_read_...` — A tool that can be invoked by the LLM.
-  `ToolOutput` type L57-71 — `= ToolOutput`
-  `is_read_only` function L86-88 — `(&self) -> bool` — Whether this tool is side-effect-free (observation only).
-  `category` function L91-93 — `(&self) -> ToolCategory` — Tool category for context filtering and feature-area grouping.
-  `permission_category` function L100-109 — `(&self) -> PermissionCategory` — Permission risk class for permission-mode fallback decisions.
-  `llm_preference` function L115-117 — `(&self) -> Option<LlmPreference>` — Optional preferred LLM for this tool.

### crates/arawn-tui/src

**Role**: Terminal user interface for Arawn — a Ratatui-based TUI that connects to the WebSocket server, renders a chat panel with markdown, a sidebar for workstreams/sessions, slash command autocomplete, and modal dialogs for permission prompts.

**Key abstractions**:
- `App` — All mutable TUI state: input buffer, cursor position, chat messages, workstream/session lists, focus, scroll offset, generating flag, modal, autocomplete, plan mode, sidebar section, token stats. `handle_action()` is the pure state mutator for keyboard events. `apply_engine_event()` is the pure state mutator for incoming WS events (both are fully testable without a terminal).
- `ChatMessage` / `ChatRole` — TUI message types. `rendered_lines()` caches the result of `markdown_to_lines_with_width` for assistant messages so re-renders don't re-parse markdown.
- `run_tui()` — The main event loop: sets up the terminal, connects `WsClient`, spawns a background task to receive WS messages, then drives the ratatui `Terminal::draw` / crossterm event loop. Mouse click handling targets sidebar regions using `LayoutRegions` from the last render.
- `CommandRegistry` / `AutocompleteState` — Slash command system. Built-in commands (`/help`, `/clear`, `/plan`, `/remember`, `/forget`, `@inventory`, `@memory`) plus dynamic skill commands registered from the server's skill list. `matching(prefix)` drives the autocomplete dropdown. `execute_command()` returns a `CommandResult` variant that `run_tui` dispatches into server RPC calls or local state mutations.
- `WsClient` — Typed wrapper over a `tokio-tungstenite` WebSocket. All methods are synchronous request/response except `send_message` which only sends (the response stream is read by the background task). `engine_event_to_update()` converts `EngineEvent` to `EventUpdate` (the TUI-local version of the event).
- `render.rs` — Pure rendering functions. `render_chat` handles scroll-aware message layout, tool call/result boxes with chrome, collapsed tool results (Ctrl+E toggle), truncation hints. `render_markdown` handles syntax-highlighted code blocks via `syntect`.
- `TuiModalPrompt` — Implements `ModalPrompt` by sending a `TuiModalRequest` through an mpsc channel to the TUI event loop, which renders a centered modal and blocks until the user selects an option. The response flows back through a `oneshot` channel.
- `theme.rs` — Centralized color palette; all colors are named constants. Change one file to restyle the entire TUI.

**Mixed concerns / gotchas**: `markdown.rs` implements a full recursive markdown renderer (headings, bold/italic, code blocks with syntax highlighting, tables with column-width alignment, lists, links, blockquotes). Tables are accumulated in a buffer and emitted all at once with computed column widths. The `snapshot_tests.rs` file keeps golden-output tests for the rendered terminal buffer using `TestBackend`.

**Dependencies**: `ratatui` (TUI framework), `crossterm` (terminal backend), `syntect` (syntax highlighting), `tokio-tungstenite` (WebSocket), `pulldown-cmark` (markdown parsing), `arawn-service` (EngineEvent, types).

#### crates/arawn-tui/src/action.rs

- pub `Action` enum L3-66 — `TypeChar | Backspace | Delete | CursorLeft | CursorRight | CursorHome | CursorEn...`

#### crates/arawn-tui/src/app.rs

- pub `LayoutRegions` struct L13-23 — `{ sidebar: Option<Rect>, chat: Rect, input: Rect, sidebar_ws: Option<Rect>, side...` — Tracks the screen regions of each panel from the last render.
- pub `Focus` enum L27-31 — `Main | Sidebar` — Which panel has focus.
- pub `SidebarSection` enum L35-38 — `Workstreams | Sessions` — Which sidebar section is active.
- pub `ChatMessage` struct L42-51 — `{ role: ChatRole, content: String, created_at: std::time::Instant, rendered_cach...` — A message displayed in the chat area.
- pub `new` function L54-62 — `(role: ChatRole, content: impl Into<String>) -> Self`
- pub `rendered_lines` function L66-76 — `(&mut self, width: usize) -> &[ratatui::text::Line<'static>]` — Get or compute the cached markdown rendering for assistant messages.
- pub `ChatRole` enum L80-86 — `User | Assistant | ToolCall | ToolResult | System`
- pub `App` struct L89-170 — `{ focus: Focus, input_buffer: String, cursor_pos: usize, messages: Vec<ChatMessa...` — All mutable TUI state.
- pub `DOUBLE_ESC_WINDOW` variable L175 — `: std::time::Duration` — Window for double-Esc detection.
- pub `HistoryEntry` struct L179-186 — `{ text: String, is_chat: bool }` — One entry in the per-session input history.
- pub `new` function L189-230 — `() -> Self`
- pub `handle_action` function L233-651 — `(&mut self, action: Action) -> bool` — Process an action and mutate state.
- pub `apply_engine_event` function L810-887 — `(&mut self, event: crate::ws_client::EventUpdate)` — Apply a streaming engine event to the app state (testable without network).
- pub `load_session_messages` function L891-931 — `(&mut self, detail: &serde_json::Value)` — Load messages from a session detail JSON response into the chat.
- pub `format_tool_input` function L951-999 — `(tool_name: &str, input: &serde_json::Value) -> String` — Format tool input args into a compact display string.
-  `ChatMessage` type L53-77 — `= ChatMessage`
-  `App` type L188-948 — `= App`
-  `record_input_history` function L657-668 — `(&mut self, text: &str, is_chat: bool)` — Append `text` to input history, skipping empty input and deduping
-  `history_recall_prev` function L672-687 — `(&mut self)` — Move backward in input history.
-  `history_recall_next` function L691-704 — `(&mut self)` — Move forward in input history.
-  `open_history_modal` function L711-765 — `(&mut self)` — Open a modal listing branchable history entries (chat prompts only,
-  `update_autocomplete` function L768-797 — `(&mut self)` — Update autocomplete suggestions based on current input buffer.
-  `accept_autocomplete` function L800-807 — `(&mut self)` — Accept the currently selected autocomplete suggestion.
-  `prev_char_boundary` function L933-939 — `(&self) -> usize`
-  `next_char_boundary` function L941-947 — `(&self) -> usize`
-  `App` type L1001-1005 — `impl Default for App`
-  `default` function L1002-1004 — `() -> Self`
-  `tests` module L1008-1492 — `-`
-  `type_chars_updates_buffer` function L1012-1018 — `()`
-  `backspace_removes_char` function L1021-1028 — `()`
-  `submit_moves_to_messages` function L1031-1043 — `()`
-  `submit_blocked_when_empty` function L1046-1052 — `()`
-  `submit_blocked_while_generating` function L1055-1061 — `()`
-  `tab_toggles_focus` function L1064-1071 — `()`
-  `scroll_updates_offset` function L1074-1082 — `()`
-  `cancel_stops_generation` function L1085-1094 — `()`
-  `quit_sets_flag` function L1097-1101 — `()`
-  `cursor_movement` function L1104-1125 — `()`
-  `full_conversation_flow` function L1130-1160 — `()`
-  `tool_call_flow` function L1163-1194 — `()`
-  `error_event_clears_generating` function L1197-1211 — `()`
-  `sidebar_navigation` function L1214-1245 — `()`
-  `submit_via_input` function L1247-1254 — `(app: &mut App, text: &str)`
-  `history_text` function L1256-1258 — `(app: &App) -> Vec<&str>`
-  `history_records_submitted_prompts` function L1261-1267 — `()`
-  `history_records_slash_commands_with_is_chat_false` function L1270-1280 — `()`
-  `history_dedupes_consecutive_duplicates` function L1283-1290 — `()`
-  `branch_modal_filters_out_slash_commands` function L1293-1307 — `()`
-  `branch_modal_skipped_when_no_chat_history` function L1310-1318 — `()`
-  `up_arrow_recalls_most_recent_when_input_empty` function L1321-1336 — `()`
-  `down_arrow_restores_draft_past_newest` function L1339-1357 — `()`
-  `double_esc_within_window_opens_history_modal` function L1360-1372 — `()`
-  `double_esc_outside_window_does_not_open_modal` function L1375-1383 — `()`
-  `history_recall_at_loads_entry_into_input` function L1386-1394 — `()`
-  `empty_history_modal_is_a_no_op` function L1397-1403 — `()`
-  `modal_select_index_picks_option_directly` function L1406-1428 — `()`
-  `cancel_marks_session_for_stale_event_drop` function L1431-1458 — `()`
-  `next_submit_clears_cancelled_session_marker` function L1461-1475 — `()`
-  `modal_select_out_of_range_is_no_op` function L1478-1491 — `()`

#### crates/arawn-tui/src/command.rs

- pub `CommandInfo` struct L11-15 — `{ name: String, description: String, kind: CommandKind }` — A registered slash command.
- pub `CommandKind` enum L19-26 — `BuiltIn | Inventory | Skill` — What kind of slash command this is.
- pub `ParsedCommand` struct L30-33 — `{ name: String, args: String }` — Result of parsing a slash command from the input buffer.
- pub `parse_command` function L37-57 — `(input: &str) -> Option<ParsedCommand>` — Parse a slash command from the input buffer.
- pub `CommandRegistry` struct L61-63 — `{ commands: Vec<CommandInfo> }` — The command registry — holds all available slash commands.
- pub `new` function L66-70 — `() -> Self` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
- pub `register_skills` function L194-204 — `(&mut self, skills: Vec<(String, String)>)` — Add skill commands from the server's cached skill list.
- pub `all` function L207-209 — `(&self) -> &[CommandInfo]` — Get all commands.
- pub `matching` function L212-218 — `(&self, prefix: &str) -> Vec<&CommandInfo>` — Find commands matching a prefix (for autocomplete).
- pub `find` function L221-224 — `(&self, name: &str) -> Option<&CommandInfo>` — Look up a command by exact name.
- pub `AutocompleteState` struct L229-234 — `{ suggestions: Vec<CommandInfo>, selected: usize }` — Autocomplete state for the slash command dropdown.
- pub `new` function L237-242 — `(suggestions: Vec<CommandInfo>) -> Self` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
- pub `next` function L244-248 — `(&mut self)` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
- pub `prev` function L250-258 — `(&mut self)` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
- pub `selected_command` function L260-262 — `(&self) -> Option<&CommandInfo>` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
- pub `is_empty` function L264-266 — `(&self) -> bool` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
- pub `CommandResult` enum L271-336 — `SystemMessage | ClearChat | EnterPlan | QueryInventory | InvokeSkill | RememberF...` — The result of executing a built-in command.
- pub `WatchSpec` struct L349-354 — `{ template: String, feed_id: String, params: serde_json::Value, cadence: Option<...` — Parsed args for the non-interactive form of `/watch`.
- pub `parse_watch_args` function L366-426 — `(args: &str) -> Result<WatchSpec, String>` — Parse the args body of `/watch`.
- pub `parse_feeds_args` function L528-570 — `(args: &str) -> CommandResult` — Parse the args of `/feeds` into a CommandResult.
- pub `execute_command` function L573-737 — `(cmd: &ParsedCommand, registry: &CommandRegistry) -> CommandResult` — Execute a parsed slash command against the registry.
-  `CommandRegistry` type L65-225 — `= CommandRegistry` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `register_builtins` function L72-191 — `(&mut self)` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `AutocompleteState` type L236-267 — `= AutocompleteState` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `parse_since` function L434-466 — `(s: &str) -> Result<String, String>` — Parse a `since=` value into a canonical RFC3339 UTC string.
-  `parse_relative_duration` function L470-482 — `(s: &str) -> Option<(i64, &str)>` — Pull `<digits><unit>` out of the input.
-  `tokenize_kv` function L487-518 — `(s: &str) -> Result<Vec<String>, String>` — Tokenizer that respects double-quoted runs so a param value can
-  `tests` module L740-1268 — `-` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `parse_simple_command` function L744-748 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `watch_parses_template_id_and_string_param` function L751-758 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `watch_parses_typed_and_quoted_params_and_cadence_override` function L761-772 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `watch_parses_since_relative_duration` function L775-783 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `watch_parses_since_iso_date` function L786-795 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `watch_parses_since_rfc3339` function L798-806 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `watch_rejects_garbage_since` function L809-818 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `watch_rejects_missing_args_and_bad_template` function L821-828 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `watch_command_dispatch_returns_feed_register` function L831-842 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `feeds_command_dispatch_returns_feed_list` function L845-852 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `feeds_pause_and_resume_dispatch` function L855-865 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `feeds_rm_requires_confirm_flag` function L868-881 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `feeds_pause_without_id_is_a_usage_message` function L884-890 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `watch_list_dispatches_to_feed_discover` function L893-911 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `watch_list_rejects_extra_args_with_hint` function L914-926 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `watch_list_doesnt_swallow_a_template_named_listed` function L929-945 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `feeds_unknown_subcommand_lists_usage` function L948-954 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `parse_command_with_args` function L957-961 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `parse_not_a_command` function L964-968 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `parse_slash_only` function L971-973 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `parse_with_leading_whitespace` function L976-979 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `registry_has_builtins` function L982-989 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `registry_matching_prefix` function L992-998 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `registry_matching_empty_returns_all` function L1001-1005 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `registry_skills` function L1008-1017 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `autocomplete_navigation` function L1020-1038 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `execute_help` function L1041-1048 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `execute_clear` function L1051-1055 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `execute_unknown` function L1058-1065 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `execute_inventory` function L1068-1075 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `execute_skill` function L1078-1089 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `execute_remember_with_text_returns_remember_fact` function L1096-1105 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `execute_remember_without_text_returns_usage_message` function L1108-1118 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `execute_memory_returns_memory_summary` function L1121-1128 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `execute_forget_with_query_returns_forget_entity` function L1131-1140 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `execute_forget_without_query_returns_usage_message` function L1143-1152 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `execute_workflows_list_returns_workflow_list` function L1155-1165 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `every_advertised_builtin_dispatches_or_explains` function L1173-1196 — `()` — Audit: every built-in command in /help must dispatch to a CommandResult
-  `execute_integrations_returns_list_variant` function L1201-1208 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `execute_connect_with_service_returns_connect_variant` function L1211-1218 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `execute_connect_without_service_returns_usage_message` function L1221-1231 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `execute_disconnect_with_service_returns_disconnect_variant` function L1234-1241 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `execute_disconnect_without_service_returns_usage_message` function L1244-1251 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `capabilities_banner_doc_path_pinned` function L1256-1267 — `()` — Capabilities banner copy in event_loop.rs points users at this docs
-  `PINNED` variable L1259 — `: &str` — - **Skill**: /skill-name — invoke a user-invocable skill via the server

#### crates/arawn-tui/src/event.rs

- pub `map_key_event` function L7-67 — `( key: KeyEvent, focus: Focus, is_generating: bool, has_modal: bool, has_autocom...` — Map a crossterm KeyEvent to an Action, given the current focus.
-  `map_main_key` function L69-85 — `(key: KeyEvent) -> Option<Action>`
-  `map_modal_key` function L87-102 — `(key: KeyEvent) -> Option<Action>`
-  `map_sidebar_key` function L104-112 — `(key: KeyEvent) -> Option<Action>`
-  `tests` module L115-228 — `-`
-  `key` function L117-119 — `(code: KeyCode) -> KeyEvent`
-  `ctrl` function L121-123 — `(c: char) -> KeyEvent`
-  `ctrl_c_quits_from_any_focus` function L126-135 — `()`
-  `tab_toggles_from_any_focus` function L138-147 — `()`
-  `esc_cancels_when_generating` function L150-161 — `()`
-  `main_focus_typing` function L164-177 — `()`
-  `main_focus_scrolling` function L180-193 — `()`
-  `ctrl_e_toggles_tool_results` function L196-207 — `()`
-  `sidebar_focus_navigation` function L210-227 — `()`

#### crates/arawn-tui/src/event_loop.rs

- pub `run_tui` function L64-1116 — `(url: &str, model_name: &str) -> Result<(), Box<dyn std::error::Error>>` — Run the TUI connected to the given WebSocket server URL.
-  `MIN_FRAME_INTERVAL` variable L29 — `: Duration` — Minimum interval between renders driven by streaming/event traffic.
-  `maybe_draw` function L33-45 — `( terminal: &mut Terminal<B>, app: &mut App, ) -> io::Result<()>` — Render if enough time has elapsed since the last draw.
-  `force_draw` function L49-57 — `( terminal: &mut Terminal<B>, app: &mut App, ) -> io::Result<()>` — Render now regardless of frame budget.
-  `rect_contains` function L59-61 — `(rect: Rect, col: u16, row: u16) -> bool`
-  `format_integrations_list` function L1119-1134 — `(items: &[serde_json::Value]) -> String` — Render a `list_integrations` response as a markdown table the user can scan.
-  `OpenAttempt` enum L1138-1142 — `Opened | NoOpener | Failed` — What `try_open_url` did.
-  `try_open_url` function L1146-1177 — `(url: &str) -> OpenAttempt` — Best-effort browser open.
-  `apply_system_notice` function L1182-1195 — `(notice: &arawn_service::ServerNotice, app: &mut crate::app::App)` — Push a server-side notice (plugin/config hot-reload outcome) into the
-  `format_permissions_status` function L1198-1238 — `(status: &serde_json::Value) -> String` — Render `get_permissions_status` JSON as a human-readable system message.
-  `format_feed_registered` function L1241-1252 — `(dto: &serde_json::Value) -> String` — Render a freshly-registered feed into a chat-ready system message.
-  `format_feed_list` function L1257-1286 — `(list: &[serde_json::Value]) -> String` — Render the `/feeds` listing as a markdown table-ish block.
-  `human_size` function L1288-1301 — `(bytes: u64) -> String`
-  `KB` variable L1289 — `: u64`
-  `MB` variable L1290 — `: u64`
-  `GB` variable L1291 — `: u64`
-  `format_feed_discover` function L1306-1363 — `(dto: &serde_json::Value) -> String` — Render `feed_discover` results into a chat-pane block.
-  `format_known_templates` function L1367-1380 — `() -> String` — Static help for `/watch list` with no template — points the user

#### crates/arawn-tui/src/lib.rs

- pub `action` module L1 — `-`
- pub `app` module L2 — `-`
- pub `command` module L3 — `-`
- pub `event` module L4 — `-`
- pub `event_loop` module L5 — `-`
- pub `markdown` module L6 — `-`
- pub `modal` module L7 — `-`
- pub `render` module L8 — `-`
- pub `theme` module L9 — `-`
- pub `tui_prompt` module L10 — `-`
- pub `wrap` module L15 — `-`
- pub `width` module L16 — `-`
- pub `ws_client` module L17 — `-`
-  `snapshot` module L12 — `-`
-  `snapshot_tests` module L14 — `-`

#### crates/arawn-tui/src/markdown.rs

- pub `markdown_to_lines` function L25-27 — `(text: &str) -> Vec<Line<'static>>` — Parse a markdown string into styled ratatui `Line`s.
- pub `markdown_to_lines_with_width` function L31-42 — `(text: &str, max_width: usize) -> Vec<Line<'static>>` — Parse a markdown string into styled ratatui `Line`s.
-  `SYNTAX_SET` variable L16 — `: LazyLock<SyntaxSet>` — suitable for rendering in the chat area.
-  `THEME` variable L17-20 — `: LazyLock<Theme>` — suitable for rendering in the chat area.
-  `CODE_STYLE` variable L44 — `: Style` — suitable for rendering in the chat area.
-  `MdRenderer` struct L46-68 — `{ lines: Vec<Line<'static>>, current_spans: Vec<Span<'static>>, style_stack: Vec...` — suitable for rendering in the chat area.
-  `MdRenderer` type L70-506 — `= MdRenderer` — suitable for rendering in the chat area.
-  `new` function L71-92 — `(max_width: usize) -> Self` — suitable for rendering in the chat area.
-  `process` function L94-110 — `(&mut self, event: Event)` — suitable for rendering in the chat area.
-  `start_tag` function L112-184 — `(&mut self, tag: Tag)` — suitable for rendering in the chat area.
-  `end_tag` function L186-271 — `(&mut self, tag: TagEnd)` — suitable for rendering in the chat area.
-  `text` function L273-295 — `(&mut self, text: &str)` — suitable for rendering in the chat area.
-  `inline_code` function L297-303 — `(&mut self, code: &str)` — suitable for rendering in the chat area.
-  `line_break` function L305-307 — `(&mut self)` — suitable for rendering in the chat area.
-  `flush_line` function L309-314 — `(&mut self)` — suitable for rendering in the chat area.
-  `push_blank` function L317-325 — `(&mut self)` — Push a blank line, but only if the last line wasn't already blank.
-  `push_style` function L327-330 — `(&mut self, style: Style)` — suitable for rendering in the chat area.
-  `pop_style` function L332-335 — `(&mut self)` — suitable for rendering in the chat area.
-  `recompute_style` function L337-343 — `(&mut self)` — suitable for rendering in the chat area.
-  `emit_full_table` function L347-456 — `(&mut self)` — suitable for rendering in the chat area.
-  `emit_padded_row` function L458-492 — `( &mut self, row: &[String], col_widths: &[usize], cell_style: Style, chrome_sty...` — suitable for rendering in the chat area.
-  `finish` function L494-505 — `(mut self) -> Vec<Line<'static>>` — suitable for rendering in the chat area.
-  `highlight_code` function L510-548 — `(code: &str, lang: Option<&str>) -> Vec<Line<'static>>` — Syntax-highlight a code block, returning one Line per source line.
-  `heading_style` function L550-565 — `(level: u8) -> Style` — suitable for rendering in the chat area.
-  `wrap_text` function L569-648 — `(text: &str, width: usize) -> Vec<String>` — Word-wrap text to fit within a given width.
-  `tests` module L651-827 — `-` — suitable for rendering in the chat area.
-  `spans_text` function L654-666 — `(lines: &[Line]) -> String` — suitable for rendering in the chat area.
-  `plain_text` function L669-673 — `()` — suitable for rendering in the chat area.
-  `heading_levels` function L676-685 — `()` — suitable for rendering in the chat area.
-  `bold_and_italic` function L688-702 — `()` — suitable for rendering in the chat area.
-  `inline_code` function L705-713 — `()` — suitable for rendering in the chat area.
-  `fenced_code_block` function L716-731 — `()` — suitable for rendering in the chat area.
-  `unordered_list` function L734-740 — `()` — suitable for rendering in the chat area.
-  `ordered_list` function L743-748 — `()` — suitable for rendering in the chat area.
-  `table_renders_aligned` function L751-771 — `()` — suitable for rendering in the chat area.
-  `link_shows_url` function L774-779 — `()` — suitable for rendering in the chat area.
-  `no_double_blank_lines` function L782-796 — `()` — suitable for rendering in the chat area.
-  `table_wide_content_preserves_short_columns` function L799-819 — `()` — suitable for rendering in the chat area.
-  `no_trailing_blanks` function L822-826 — `()` — suitable for rendering in the chat area.

#### crates/arawn-tui/src/modal.rs

- pub `ModalOption` struct L17-20 — `{ label: String, description: Option<String> }` — A single option in the modal.
- pub `new` function L23-28 — `(label: impl Into<String>) -> Self` — questions, and any future tool that needs user input.
- pub `with_description` function L30-33 — `(mut self, desc: impl Into<String>) -> Self` — questions, and any future tool that needs user input.
- pub `ModalState` struct L37-46 — `{ title: String, subtitle: Option<String>, options: Vec<ModalOption>, focused_in...` — Active modal state.
- pub `new` function L49-63 — `( title: impl Into<String>, options: Vec<ModalOption>, border_color: Color, resu...` — questions, and any future tool that needs user input.
- pub `with_subtitle` function L65-68 — `(mut self, subtitle: impl Into<String>) -> Self` — questions, and any future tool that needs user input.
- pub `focus_prev` function L71-75 — `(&mut self)` — Move focus up.
- pub `focus_next` function L78-82 — `(&mut self)` — Move focus down.
- pub `confirm` function L85-89 — `(&mut self)` — Confirm the focused option.
- pub `cancel` function L92-96 — `(&mut self)` — Cancel (Escape).
- pub `render_modal` function L100-184 — `(modal: &ModalState, frame: &mut Frame)` — Render the modal as a centered overlay.
-  `ModalOption` type L22-34 — `= ModalOption` — questions, and any future tool that needs user input.
-  `ModalState` type L48-97 — `= ModalState` — questions, and any future tool that needs user input.
-  `centered_rect` function L187-191 — `(width: u16, height: u16, area: Rect) -> Rect` — Calculate a centered rectangle within an area.
-  `tests` module L194-290 — `-` — questions, and any future tool that needs user input.
-  `make_modal` function L197-209 — `() -> ModalState` — questions, and any future tool that needs user input.
-  `navigation` function L212-235 — `()` — questions, and any future tool that needs user input.
-  `confirm_sends_index` function L238-250 — `()` — questions, and any future tool that needs user input.
-  `cancel_sends_none` function L253-264 — `()` — questions, and any future tool that needs user input.
-  `confirm_only_sends_once` function L267-279 — `()` — questions, and any future tool that needs user input.
-  `centered_rect_calculation` function L282-289 — `()` — questions, and any future tool that needs user input.

#### crates/arawn-tui/src/render.rs

- pub `render` function L13-94 — `(app: &mut App, frame: &mut Frame)` — Render function.
-  `SPINNER_FRAMES` variable L10 — `: &[char]`
-  `render_sidebar_tab` function L96-120 — `(frame: &mut Frame, area: ratatui::layout::Rect)`
-  `render_status_bar` function L122-207 — `(app: &App, frame: &mut Frame, area: ratatui::layout::Rect)`
-  `format_tokens` function L210-218 — `(n: u64) -> String` — Format a token count for display: 1234 → "1.2k", 12345 → "12.3k", 500 → "500"
-  `render_sidebar` function L220-294 — `(app: &App, frame: &mut Frame, area: ratatui::layout::Rect)`
-  `render_chat` function L296-680 — `(app: &mut App, frame: &mut Frame, area: ratatui::layout::Rect)`
-  `render_separator` function L682-686 — `(frame: &mut Frame, area: ratatui::layout::Rect)`
-  `render_input` function L688-751 — `(app: &App, frame: &mut Frame, area: ratatui::layout::Rect)`
-  `render_autocomplete` function L754-812 — `( ac: &crate::command::AutocompleteState, frame: &mut Frame, input_area: ratatui...` — Render the autocomplete dropdown above the input line.
-  `render_oauth_heartbeat` function L814-837 — `(app: &App, frame: &mut Frame, area: ratatui::layout::Rect)`
-  `render_idle_hero` function L839-873 — `(frame: &mut Frame, area: ratatui::layout::Rect)`
-  `truncate_to` function L876-878 — `(s: &str, max_cells: usize) -> String` — Truncate a string to fit within a display width, adding "…" if needed.
-  `compact_tool_summary` function L881-886 — `(content: &str) -> String` — Extract a compact summary from tool call content for inline display.
-  `truncate_for_display` function L888-892 — `(s: &str, max: usize) -> String`
-  `tests` module L895-1647 — `-`
-  `truncate_for_display_handles_utf8_at_boundary` function L902-912 — `()`
-  `truncate_for_display_passes_through_short_strings` function L915-917 — `()`
-  `buffer_to_string` function L919-934 — `(terminal: &Terminal<TestBackend>, row: u16) -> String`
-  `render_empty_app_has_status_bar` function L937-946 — `()`
-  `render_with_messages_shows_content` function L949-975 — `()`
-  `render_with_input_text` function L978-993 — `()`
-  `render_streaming_shows_cursor` function L996-1019 — `()`
-  `render_small_terminal` function L1022-1027 — `()`
-  `render_large_terminal` function L1030-1035 — `()`
-  `region_text` function L1040-1052 — `(terminal: &Terminal<TestBackend>, x: u16, y: u16, w: u16, h: u16) -> String` — Extract text from a rectangular region of the buffer.
-  `chat_region_for` function L1056-1069 — `(terminal: &Terminal<TestBackend>, sidebar_visible: bool) -> String` — Extract the chat area text.
-  `chat_region` function L1072-1074 — `(terminal: &Terminal<TestBackend>) -> String` — Convenience: chat region for default app (sidebar hidden).
-  `sidebar_region` function L1078-1086 — `(terminal: &Terminal<TestBackend>) -> String` — Extract the sidebar text (left 20%, rows 1..height-3).
-  `input_region` function L1089-1094 — `(terminal: &Terminal<TestBackend>) -> String` — Extract the input bar text (second from bottom row).
-  `chat_renders_user_message_with_prefix` function L1099-1113 — `()`
-  `chat_renders_assistant_message_with_prefix` function L1116-1130 — `()`
-  `chat_renders_tool_call_with_icon` function L1133-1158 — `()`
-  `chat_renders_tool_result_collapsed` function L1161-1189 — `()`
-  `chat_renders_tool_error_result` function L1192-1215 — `()`
-  `chat_renders_tool_result_truncated` function L1218-1245 — `()`
-  `chat_streaming_text_appears_in_chat_area` function L1248-1266 — `()`
-  `sidebar_renders_workstream_names` function L1269-1305 — `()`
-  `sidebar_does_not_leak_into_chat` function L1308-1342 — `()`
-  `input_shows_placeholder_when_empty` function L1345-1356 — `()`
-  `input_shows_generating_when_active` function L1359-1372 — `()`
-  `status_bar_shows_generating_indicator` function L1375-1389 — `()`
-  `status_bar_shows_workstream_name` function L1392-1416 — `()`
-  `messages_do_not_appear_in_input_area` function L1419-1442 — `()`
-  `chat_auto_scrolls_to_bottom_with_many_messages` function L1447-1477 — `()`
-  `chat_scroll_up_reveals_older_messages` function L1480-1508 — `()`
-  `chat_few_messages_all_visible` function L1511-1525 — `()`
-  `last_message_visible_above_input` function L1528-1581 — `()`
-  `last_tool_result_visible_above_input` function L1584-1646 — `()`

#### crates/arawn-tui/src/snapshot.rs

- pub `buffer_to_snapshot` function L6-26 — `(terminal: &ratatui::Terminal<ratatui::backend::TestBackend>) -> String` — Render a TestBackend buffer to a deterministic string for snapshot comparison.
- pub `buffer_to_styled_snapshot` function L33-71 — `( terminal: &ratatui::Terminal<ratatui::backend::TestBackend>, ) -> String` — Render a TestBackend buffer with inline style annotations.
-  `format_style_tag` function L74-110 — `(fg: Color, bg: Color, mods: Modifier) -> String`

#### crates/arawn-tui/src/snapshot_tests.rs

-  `tests` module L2-425 — `-`
-  `make_terminal` function L16-18 — `(w: u16, h: u16) -> Terminal<TestBackend>`
-  `draw` function L20-23 — `(app: &mut App, terminal: &mut Terminal<TestBackend>) -> String`
-  `draw_styled` function L25-28 — `(app: &mut App, terminal: &mut Terminal<TestBackend>) -> String`
-  `snapshot_empty_app` function L33-38 — `()`
-  `snapshot_chat_with_conversation` function L43-69 — `()`
-  `snapshot_streaming_response` function L74-83 — `()`
-  `snapshot_sidebar_with_workstreams` function L88-116 — `()`
-  `snapshot_focus_main` function L121-130 — `()`
-  `snapshot_focus_sidebar` function L133-146 — `()`
-  `snapshot_focus_main_with_messages` function L149-157 — `()`
-  `snapshot_input_placeholder` function L162-167 — `()`
-  `snapshot_input_generating` function L170-177 — `()`
-  `snapshot_idle_hero` function L182-193 — `()`
-  `snapshot_unicode_chrome_alignment` function L196-221 — `()`
-  `snapshot_speaker_gutters` function L224-254 — `()`
-  `snapshot_ten_tool_calls_collapsed` function L257-282 — `()`
-  `snapshot_error_in_chat` function L285-295 — `()`
-  `styled_snapshot_conversation` function L300-323 — `()`
-  `styled_snapshot_focus_borders` function L326-335 — `()`
-  `styled_snapshot_sidebar_focused` function L338-346 — `()`
-  `snapshot_rich_markdown` function L349-381 — `()`
-  `styled_snapshot_rich_markdown` function L384-412 — `()`
-  `styled_snapshot_generating_state` function L415-424 — `()`

#### crates/arawn-tui/src/theme.rs

- pub `CRUST` variable L26 — `: Color` — tool names, headings, links, etc.
- pub `MANTLE` variable L27 — `: Color` — tool names, headings, links, etc.
- pub `BASE` variable L28 — `: Color` — tool names, headings, links, etc.
- pub `SURFACE0` variable L29 — `: Color` — tool names, headings, links, etc.
- pub `SURFACE1` variable L30 — `: Color` — tool names, headings, links, etc.
- pub `SURFACE2` variable L31 — `: Color` — tool names, headings, links, etc.
- pub `OVERLAY0` variable L34 — `: Color` — tool names, headings, links, etc.
- pub `OVERLAY1` variable L35 — `: Color` — tool names, headings, links, etc.
- pub `OVERLAY2` variable L36 — `: Color` — tool names, headings, links, etc.
- pub `SUBTEXT0` variable L39 — `: Color` — tool names, headings, links, etc.
- pub `SUBTEXT1` variable L40 — `: Color` — tool names, headings, links, etc.
- pub `TEXT` variable L41 — `: Color` — tool names, headings, links, etc.
- pub `LAVENDER` variable L44 — `: Color` — tool names, headings, links, etc.
- pub `BLUE` variable L45 — `: Color` — tool names, headings, links, etc.
- pub `SAPPHIRE` variable L46 — `: Color` — tool names, headings, links, etc.
- pub `SKY` variable L47 — `: Color` — tool names, headings, links, etc.
- pub `TEAL` variable L48 — `: Color` — tool names, headings, links, etc.
- pub `GREEN` variable L49 — `: Color` — tool names, headings, links, etc.
- pub `YELLOW` variable L50 — `: Color` — tool names, headings, links, etc.
- pub `PEACH` variable L51 — `: Color` — tool names, headings, links, etc.
- pub `MAROON` variable L52 — `: Color` — tool names, headings, links, etc.
- pub `RED` variable L53 — `: Color` — tool names, headings, links, etc.
- pub `MAUVE` variable L54 — `: Color` — tool names, headings, links, etc.
- pub `PINK` variable L55 — `: Color` — tool names, headings, links, etc.
- pub `FLAMINGO` variable L56 — `: Color` — tool names, headings, links, etc.
- pub `ROSEWATER` variable L57 — `: Color` — tool names, headings, links, etc.
- pub `USER` variable L64 — `: Color` — User message prefix ("❯ ")
- pub `ASSISTANT` variable L67 — `: Color` — Assistant message body — the agent's prose, default reading color
- pub `SYSTEM` variable L70 — `: Color` — System / internal note prefix
- pub `ERROR` variable L73 — `: Color` — Errors and danger indicators
- pub `TOOL_NAME` variable L76 — `: Color` — Tool name in tool calls — interactive but not focused
- pub `GENERATING` variable L79 — `: Color` — In-progress / generating indicator (spinner, "thinking…")
- pub `SUCCESS` variable L82 — `: Color` — Success indicator (✓)
- pub `CHROME` variable L87 — `: Color` — Box borders around tool calls/results (┌│└)
- pub `SEPARATOR` variable L90 — `: Color` — Separator line between chat and input
- pub `STATUS_BAR_BG` variable L93 — `: Color` — Status bar background — Catppuccin Mantle (one shade darker than base)
- pub `STATUS_BAR_FG` variable L96 — `: Color` — Status bar foreground (default text color on the bar)
- pub `BORDER_INACTIVE` variable L99 — `: Color` — Sidebar border when not focused
- pub `BORDER_ACTIVE` variable L102 — `: Color` — Sidebar border when focused — accent.
- pub `SIDEBAR_TAB_BG` variable L105 — `: Color` — Sidebar tab strip background (collapsed sidebar) — Catppuccin Crust
- pub `RESULT_TEXT` variable L110 — `: Color` — Tool result content text
- pub `RESULT_LABEL` variable L113 — `: Color` — Tool result labels ("▸ shell result")
- pub `TOOL_SUMMARY` variable L116 — `: Color` — Tool input summary text (args after tool name)
- pub `RESULT_HINT` variable L119 — `: Color` — Truncation hints ("… 15 more")
- pub `INPUT_PROMPT` variable L124 — `: Color` — Input prompt "> "
- pub `PLACEHOLDER` variable L127 — `: Color` — Placeholder text ("Type your message...")
- pub `CODE_BG` variable L132 — `: Color` — Code block background
- pub `CODE_FG` variable L135 — `: Color` — Code block text (fallback when no syntax highlighting)
- pub `INLINE_CODE_FG` variable L138 — `: Color` — Inline code text — Catppuccin peach has a known "code" feel
- pub `INLINE_CODE_BG` variable L141 — `: Color` — Inline code background
- pub `CODE_LANG` variable L144 — `: Color` — Code block language label
- pub `HEADING_1` variable L153 — `: Color` — tool names, headings, links, etc.
- pub `HEADING_2` variable L154 — `: Color` — tool names, headings, links, etc.
- pub `HEADING_3` variable L155 — `: Color` — tool names, headings, links, etc.
- pub `HEADING_4` variable L156 — `: Color` — tool names, headings, links, etc.
- pub `RULE` variable L161 — `: Color` — Horizontal rules
- pub `LIST_BULLET` variable L164 — `: Color` — List bullet/number prefix
- pub `BLOCK_QUOTE` variable L167 — `: Color` — Block quote text
- pub `LINK` variable L170 — `: Color` — Link text — interactive, but not "focused", so not mauve
- pub `LINK_URL` variable L173 — `: Color` — Link URL shown after link text
- pub `TABLE_CHROME` variable L176 — `: Color` — Table chrome (│ ├ ┼ ┤)
- pub `bold` function L180-182 — `(color: Color) -> Style` — tool names, headings, links, etc.
- pub `italic` function L184-186 — `(color: Color) -> Style` — tool names, headings, links, etc.

#### crates/arawn-tui/src/tui_prompt.rs

- pub `TuiModalRequest` struct L15-17 — `{ modal: ModalState }` — A request to show a modal in the TUI event loop.
- pub `TuiModalPrompt` struct L21-23 — `{ tx: mpsc::Sender<TuiModalRequest> }` — TUI-based modal prompt.
- pub `new` function L26-28 — `(tx: mpsc::Sender<TuiModalRequest>) -> Self` — via a oneshot channel.
-  `TuiModalPrompt` type L25-29 — `= TuiModalPrompt` — via a oneshot channel.
-  `TuiModalPrompt` type L32-66 — `impl ModalPrompt for TuiModalPrompt` — via a oneshot channel.
-  `prompt` function L33-65 — `(&self, request: ModalRequest) -> Option<usize>` — via a oneshot channel.

#### crates/arawn-tui/src/width.rs

- pub `display_width` function L11-13 — `(s: &str) -> usize` — Display width (cells) of `s` in a fixed-width terminal.
- pub `truncate_display` function L17-37 — `(s: &str, max: usize) -> String` — Truncate `s` to fit within `max` display cells, appending `…` if truncated.

#### crates/arawn-tui/src/wrap.rs

- pub `wrap_lines` function L26-38 — `(input: Vec<Line<'a>>, width: usize) -> Vec<Line<'static>>` — Wrap input lines to `width`.
-  `into_owned` function L42-49 — `(line: Line<'_>) -> Line<'static>` — Force every span into an owned `Cow<'static, str>` so the resulting
-  `split_newlines` function L53-81 — `(line: Line<'static>) -> Vec<Line<'static>>` — If any span contains `\n`, split the line into multiple lines along
-  `Tok` struct L85-90 — `{ text: String, style: Style, is_ws: bool, width: usize }` — Token kind: a contiguous run of whitespace or non-whitespace chars,
-  `tokenize` function L92-125 — `(line: &Line<'static>) -> Vec<Tok>` — ratatui doesn't split spans on newlines.
-  `wrap_one` function L127-190 — `(line: Line<'static>, width: usize, out: &mut Vec<Line<'static>>)` — ratatui doesn't split spans on newlines.
-  `tests` module L193-297 — `-` — ratatui doesn't split spans on newlines.
-  `plain` function L197-199 — `(s: &str) -> Line<'static>` — ratatui doesn't split spans on newlines.
-  `line_text` function L201-203 — `(line: &Line) -> String` — ratatui doesn't split spans on newlines.
-  `passthrough_when_under_width` function L206-211 — `()` — ratatui doesn't split spans on newlines.
-  `word_wraps_at_whitespace` function L214-230 — `()` — ratatui doesn't split spans on newlines.
-  `hard_breaks_oversize_word` function L233-241 — `()` — ratatui doesn't split spans on newlines.
-  `splits_on_embedded_newlines` function L244-251 — `()` — ratatui doesn't split spans on newlines.
-  `preserves_span_styles_through_wrap` function L254-281 — `()` — ratatui doesn't split spans on newlines.
-  `empty_line_preserved` function L284-289 — `()` — ratatui doesn't split spans on newlines.
-  `zero_width_is_passthrough` function L292-296 — `()` — ratatui doesn't split spans on newlines.

#### crates/arawn-tui/src/ws_client.rs

- pub `WsEvent` enum L25-29 — `Text | Closed | Error` — A frame from the reader task.
- pub `WsClient` struct L40-49 — `{ write: futures_util::stream::SplitSink< tokio_tungstenite::WebSocketStream< to...` — A WebSocket connection to the Arawn server.
- pub `connect` function L52-75 — `(url: &str) -> Result<Self, Box<dyn std::error::Error>>`
- pub `events_take` function L80-82 — `(&mut self) -> Option<mpsc::Receiver<WsEvent>>` — Take ownership of the event receiver.
- pub `send_request` function L102-119 — `( &mut self, method: &str, params: Value, ) -> Result<u64, Box<dyn std::error::E...`
- pub `request_response` function L124-146 — `( &mut self, method: &str, params: Value, ) -> Result<Value, Box<dyn std::error:...` — Send a request and await its response via the pending-oneshot map.
- pub `list_workstreams` function L148-154 — `( &mut self, ) -> Result<Vec<WorkstreamInfo>, Box<dyn std::error::Error>>`
- pub `list_workflows` function L156-162 — `( &mut self, ) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>>`
- pub `get_capabilities` function L167-173 — `( &mut self, ) -> Result<serde_json::Value, Box<dyn std::error::Error>>` — Fetch server runtime capabilities.
- pub `get_permissions_status` function L176-182 — `( &mut self, ) -> Result<serde_json::Value, Box<dyn std::error::Error>>` — Fetch permission rules + recent audit.
- pub `list_integrations` function L185-191 — `( &mut self, ) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>>` — List registered integrations and their connection state.
- pub `start_oauth_flow` function L196-208 — `( &mut self, service: &str, ) -> Result<serde_json::Value, Box<dyn std::error::E...` — Begin the OAuth flow for a service.
- pub `disconnect_integration` function L211-222 — `( &mut self, service: &str, ) -> Result<(), Box<dyn std::error::Error>>` — Drop stored credentials for a service.
- pub `feed_register` function L225-235 — `( &mut self, spec: serde_json::Value, ) -> Result<serde_json::Value, Box<dyn std...` — Register a new feed at runtime.
- pub `feed_list` function L238-247 — `( &mut self, ) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>>` — List configured feeds.
- pub `feed_pause` function L250-262 — `( &mut self, feed_id: &str, ) -> Result<serde_json::Value, Box<dyn std::error::E...` — Pause a feed by id.
- pub `feed_resume` function L265-277 — `( &mut self, feed_id: &str, ) -> Result<serde_json::Value, Box<dyn std::error::E...` — Resume a paused feed by id.
- pub `feed_run` function L280-292 — `( &mut self, feed_id: &str, ) -> Result<serde_json::Value, Box<dyn std::error::E...` — Trigger a one-off run of a feed by id.
- pub `feed_discover` function L296-308 — `( &mut self, template: &str, ) -> Result<serde_json::Value, Box<dyn std::error::...` — Fetch discoverable params for a template.
- pub `feed_remove` function L311-323 — `( &mut self, feed_id: &str, ) -> Result<serde_json::Value, Box<dyn std::error::E...` — Decommission a feed by id.
- pub `get_permission_mode` function L325-331 — `( &mut self, ) -> Result<String, Box<dyn std::error::Error>>`
- pub `set_permission_mode` function L333-345 — `( &mut self, mode: &str, ) -> Result<String, Box<dyn std::error::Error>>`
- pub `list_sessions` function L347-358 — `( &mut self, ws_id: Option<uuid::Uuid>, ) -> Result<Vec<SessionInfo>, Box<dyn st...`
- pub `create_session` function L360-371 — `( &mut self, ws_id: Option<uuid::Uuid>, ) -> Result<SessionInfo, Box<dyn std::er...`
- pub `load_session` function L373-385 — `( &mut self, session_id: uuid::Uuid, ) -> Result<serde_json::Value, Box<dyn std:...`
- pub `truncate_session_at_user_message` function L390-409 — `( &mut self, session_id: uuid::Uuid, user_message_index: usize, ) -> Result<serd...` — Rewind a session back to before the Nth user message.
- pub `send_message` function L411-426 — `( &mut self, session_id: uuid::Uuid, content: &str, ) -> Result<(), Box<dyn std:...`
- pub `cancel` function L433-447 — `( &mut self, session_id: uuid::Uuid, ) -> Result<(), Box<dyn std::error::Error>>` — Tell the server to abort an in-flight generation on this session.
- pub `parse_engine_event` function L503-523 — `(text: &str) -> Option<EngineEvent>` — Parse a WS message as an EngineEvent.
- pub `EventUpdate` enum L526-553 — `AppendStreamingText | AddToolCall | AddToolResult | Complete | Error | Warning |...` — Convert an EngineEvent into App state updates.
- pub `parse_system_notice` function L559-565 — `(text: &str) -> Option<arawn_service::ServerNotice>` — Parse a server-wide notice (plugin/config hot-reload) from a raw WS text
- pub `engine_event_to_update` function L567-594 — `(event: EngineEvent) -> EventUpdate`
-  `REQUEST_ID` variable L13 — `: AtomicU64`
-  `next_id` function L15-17 — `() -> u64`
-  `Pending` type L31 — `= Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>>`
-  `WsClient` type L51-448 — `= WsClient`
-  `read_server_token` function L86-100 — `() -> Option<String>` — Read the server auth token from {data_dir}/server.token.
-  `spawn_reader` function L452-500 — `( mut read: futures_util::stream::SplitStream< tokio_tungstenite::WebSocketStrea...` — Spawn the reader task.
-  `tests` module L597-642 — `-`
-  `parses_well_formed_system_notice` function L604-619 — `()`
-  `rejects_engine_event_envelope` function L622-629 — `()`
-  `rejects_response_envelope` function L632-635 — `()`
-  `rejects_malformed_json` function L638-641 — `()`

### crates/arawn-workflow

**Role**: The workflow subsystem crate — wraps the cloacina DAG runner, provides code generation for workflow packages, exposes agent-facing management tools (create/list/delete/status), and handles decision requests from running pipelines.

#### crates/arawn-workflow/build.rs

-  `main` function L1-3 — `()`

### crates/arawn-workflow/src

**Role**: Workflow runtime integration: scaffolds new workflow Cargo projects from a `WorkflowDef`, wraps cloacina's `DefaultRunner`, and exposes agent-facing tools for managing the workflow lifecycle. Also handles decision callbacks from running pipelines via `DecisionService`.

**Key abstractions**:
- `WorkflowRunner` — Thin wrapper around cloacina's `DefaultRunner`. `new(config)` initializes the runner against a SQLite database and packages directory. `execute(name, context)` triggers a named workflow programmatically. `shutdown()` drains in-flight pipelines. The `inner()` accessor is used by `ws_server.rs` for the decision callback HTTP endpoint.
- `scaffold::generate(dir, def)` — Takes a `WorkflowDef` (name, description, tasks with bodies and dependencies, optional cron) and writes a complete Cargo workspace: `Cargo.toml`, `build.rs`, `package.toml` (cloacina metadata), and `src/lib.rs` (generated Rust code using cloacina-workflow macros). The generated code can be compiled to a `.cloacina` archive and hot-loaded by the runner.
- `DecisionService` — Called by the HTTP `/decision` endpoint in `ws_server.rs` when a workflow pipeline needs an AI decision. Creates a fresh session in the store, runs a `QueryEngine` loop with the decision prompt, and returns the assistant's final text as `DecisionResponse`. This bridges workflow execution back into the LLM agent.
- `WorkflowCreateTool` — The full create flow: takes name/description/tasks/cron from the LLM, calls `scaffold::generate` into a temp dir, runs `cargo build --release`, copies the compiled `.cloacina` archive to the packages dir. Long-running (compilation can take minutes).
- `WorkflowListTool` / `WorkflowDeleteTool` / `WorkflowStatusTool` — Read-only list of installed `.cloacina` packages, delete by name, and query the runner for active pipeline status.

**Internal flow**: The LLM calls `WorkflowCreateTool` to scaffold + compile a workflow. The compiled archive lands in `packages_dir`. The `WorkflowRunner` hot-loads it (cloacina polls the dir). When a pipeline runs, decision tasks POST to `/decision` in the arawn server, `DecisionService::execute` creates a session and runs the QueryEngine, returning the decision.

**Mixed concerns / gotchas**: `WorkflowCreateTool` must invoke `cargo build` as a subprocess, which means it depends on the build toolchain being available at runtime. The `scaffold::lib_rs` code generator produces compilable Rust using string templates — if cloacina API changes, this template breaks.

**Dependencies**: `cloacina` (workflow DAG runner), `arawn-engine` (QueryEngine for decision service), `arawn-storage` (Store for decision sessions), `arawn-llm`, `arawn-tool` (Tool trait).

#### crates/arawn-workflow/src/agent_executor.rs

- pub `DecisionRequest` struct L21-30 — `{ prompt: String, workstream: String, upstream_data: Value }` — Request from a workflow decision task.
- pub `DecisionResponse` struct L38-43 — `{ result: String, session_id: String }` — Response returned to the workflow decision task.
- pub `DecisionService` struct L46-51 — `{ store: Arc<Mutex<Store>>, llm: Arc<dyn LlmClient>, registry: Arc<ToolRegistry>...` — Service that handles decision task requests from workflow pipelines.
- pub `new` function L54-66 — `( store: Arc<Mutex<Store>>, llm: Arc<dyn LlmClient>, registry: Arc<ToolRegistry>...` — those requests, creating sessions and running the QueryEngine loop.
- pub `execute` function L70-136 — `(&self, req: DecisionRequest) -> Result<DecisionResponse, DecisionError>` — Execute a decision request — creates a session, runs the QueryEngine,
- pub `DecisionError` struct L141 — `-` — those requests, creating sessions and running the QueryEngine loop.
-  `default_workstream` function L32-34 — `() -> String` — those requests, creating sessions and running the QueryEngine loop.
-  `DecisionService` type L53-137 — `= DecisionService` — those requests, creating sessions and running the QueryEngine loop.

#### crates/arawn-workflow/src/lib.rs

- pub `agent_executor` module L5 — `-` — scheduled agent workflows with DAG execution, cron scheduling, and
- pub `runner` module L6 — `-` — hot-loaded .cloacina packages.
- pub `scaffold` module L7 — `-` — hot-loaded .cloacina packages.
- pub `tools` module L8 — `-` — hot-loaded .cloacina packages.

#### crates/arawn-workflow/src/runner.rs

- pub `WorkflowRunnerConfig` struct L10-17 — `{ database_path: PathBuf, packages_dir: PathBuf, max_concurrent_tasks: usize }` — Configuration for the workflow runner.
- pub `new` function L20-26 — `(data_dir: &Path) -> Self` — Wrapper around cloacina's DefaultRunner for arawn server integration.
- pub `WorkflowRunner` struct L33-35 — `{ runner: DefaultRunner }` — Arawn's workflow engine — wraps cloacina's DefaultRunner.
- pub `new` function L41-67 — `(config: WorkflowRunnerConfig) -> Result<Self, WorkflowError>` — Initialize the workflow runner with the given configuration.
- pub `execute` function L70-87 — `( &self, workflow_name: &str, context: serde_json::Value, ) -> Result<WorkflowEx...` — Execute a named workflow programmatically.
- pub `shutdown` function L90-95 — `(&self)` — Graceful shutdown — drains in-flight pipelines.
- pub `inner` function L98-100 — `(&self) -> &DefaultRunner` — Get a reference to the underlying DefaultRunner.
- pub `cloacina_runner` function L105-107 — `(&self) -> std::sync::Arc<DefaultRunner>` — Hand out an `Arc<DefaultRunner>` for callers that need to own
- pub `WorkflowError` enum L111-116 — `Init | Runtime` — Wrapper around cloacina's DefaultRunner for arawn server integration.
-  `WorkflowRunnerConfig` type L19-27 — `= WorkflowRunnerConfig` — Wrapper around cloacina's DefaultRunner for arawn server integration.
-  `WorkflowRunner` type L37-108 — `= WorkflowRunner` — Wrapper around cloacina's DefaultRunner for arawn server integration.
-  `tests` module L119-148 — `-` — Wrapper around cloacina's DefaultRunner for arawn server integration.
-  `runner_initializes_and_shuts_down` function L123-137 — `()` — Wrapper around cloacina's DefaultRunner for arawn server integration.
-  `runner_starts_with_empty_packages_dir` function L140-147 — `()` — Wrapper around cloacina's DefaultRunner for arawn server integration.

#### crates/arawn-workflow/src/scaffold.rs

- pub `TaskDef` struct L7-16 — `{ id: String, dependencies: Vec<String>, body: String, retry_attempts: Option<i3...` — Definition of a single task within a workflow.
- pub `WorkflowDef` struct L19-30 — `{ name: String, description: String, tasks: Vec<TaskDef>, cron: Option<String>, ...` — Definition of a workflow to scaffold.
- pub `generate` function L35-55 — `(dir: &Path, def: &WorkflowDef) -> Result<(), ScaffoldError>` — Generate a complete workflow Cargo project in the given directory.
- pub `ScaffoldError` struct L170 — `-` — using cloacina-workflow macros that can be packaged as a `.cloacina` archive.
-  `cargo_toml` function L57-88 — `(name: &str) -> String` — using cloacina-workflow macros that can be packaged as a `.cloacina` archive.
-  `BUILD_RS` variable L90-93 — `: &str` — using cloacina-workflow macros that can be packaged as a `.cloacina` archive.
-  `package_toml` function L95-107 — `(name: &str, workflow_name: &str, description: &str) -> String` — using cloacina-workflow macros that can be packaged as a `.cloacina` archive.
-  `lib_rs` function L109-166 — `(def: &WorkflowDef, crate_name: &str) -> String` — using cloacina-workflow macros that can be packaged as a `.cloacina` archive.
-  `tests` module L173-241 — `-` — using cloacina-workflow macros that can be packaged as a `.cloacina` archive.
-  `generates_valid_project_structure` function L177-218 — `()` — using cloacina-workflow macros that can be packaged as a `.cloacina` archive.
-  `no_trigger_when_no_cron` function L221-240 — `()` — using cloacina-workflow macros that can be packaged as a `.cloacina` archive.

#### crates/arawn-workflow/src/tools.rs

- pub `SharedWorkflowRunner` type L18 — `= Arc<RwLock<Option<Arc<WorkflowRunner>>>>` — Shared handle to the workflow runner (Option because it may not be available).
- pub `WorkflowCreateTool` struct L21-23 — `{ packages_dir: PathBuf }` — Tool for creating a new workflow — scaffolds, compiles, and installs.
- pub `new` function L26-28 — `(packages_dir: PathBuf) -> Self` — Agent-facing tools for workflow management: create, list, delete, status.
- pub `WorkflowListTool` struct L185-187 — `{ packages_dir: PathBuf }` — Tool for listing installed workflows.
- pub `new` function L190-192 — `(packages_dir: PathBuf) -> Self` — Agent-facing tools for workflow management: create, list, delete, status.
- pub `WorkflowDeleteTool` struct L258-260 — `{ packages_dir: PathBuf }` — Tool for deleting a workflow package.
- pub `new` function L263-265 — `(packages_dir: PathBuf) -> Self` — Agent-facing tools for workflow management: create, list, delete, status.
- pub `WorkflowStatusTool` struct L313-315 — `{ runner: SharedWorkflowRunner }` — Tool for checking workflow execution status.
- pub `new` function L318-320 — `(runner: SharedWorkflowRunner) -> Self` — Agent-facing tools for workflow management: create, list, delete, status.
-  `WorkflowCreateTool` type L25-29 — `= WorkflowCreateTool` — Agent-facing tools for workflow management: create, list, delete, status.
-  `WorkflowCreateTool` type L32-182 — `impl Tool for WorkflowCreateTool` — Agent-facing tools for workflow management: create, list, delete, status.
-  `name` function L33-35 — `(&self) -> &str` — Agent-facing tools for workflow management: create, list, delete, status.
-  `description` function L37-41 — `(&self) -> &str` — Agent-facing tools for workflow management: create, list, delete, status.
-  `parameters_schema` function L43-90 — `(&self) -> Value` — Agent-facing tools for workflow management: create, list, delete, status.
-  `execute` function L92-181 — `(&self, _ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput,...` — Agent-facing tools for workflow management: create, list, delete, status.
-  `WorkflowListTool` type L189-193 — `= WorkflowListTool` — Agent-facing tools for workflow management: create, list, delete, status.
-  `WorkflowListTool` type L196-255 — `impl Tool for WorkflowListTool` — Agent-facing tools for workflow management: create, list, delete, status.
-  `name` function L197-199 — `(&self) -> &str` — Agent-facing tools for workflow management: create, list, delete, status.
-  `description` function L201-203 — `(&self) -> &str` — Agent-facing tools for workflow management: create, list, delete, status.
-  `is_read_only` function L205-207 — `(&self) -> bool` — Agent-facing tools for workflow management: create, list, delete, status.
-  `parameters_schema` function L209-215 — `(&self) -> Value` — Agent-facing tools for workflow management: create, list, delete, status.
-  `execute` function L217-254 — `(&self, _ctx: &dyn arawn_tool::ToolContext, _params: Value) -> Result<ToolOutput...` — Agent-facing tools for workflow management: create, list, delete, status.
-  `WorkflowDeleteTool` type L262-266 — `= WorkflowDeleteTool` — Agent-facing tools for workflow management: create, list, delete, status.
-  `WorkflowDeleteTool` type L269-310 — `impl Tool for WorkflowDeleteTool` — Agent-facing tools for workflow management: create, list, delete, status.
-  `name` function L270-272 — `(&self) -> &str` — Agent-facing tools for workflow management: create, list, delete, status.
-  `description` function L274-276 — `(&self) -> &str` — Agent-facing tools for workflow management: create, list, delete, status.
-  `parameters_schema` function L278-289 — `(&self) -> Value` — Agent-facing tools for workflow management: create, list, delete, status.
-  `execute` function L291-309 — `(&self, _ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput,...` — Agent-facing tools for workflow management: create, list, delete, status.
-  `WorkflowStatusTool` type L317-321 — `= WorkflowStatusTool` — Agent-facing tools for workflow management: create, list, delete, status.
-  `WorkflowStatusTool` type L324-383 — `impl Tool for WorkflowStatusTool` — Agent-facing tools for workflow management: create, list, delete, status.
-  `name` function L325-327 — `(&self) -> &str` — Agent-facing tools for workflow management: create, list, delete, status.
-  `description` function L329-331 — `(&self) -> &str` — Agent-facing tools for workflow management: create, list, delete, status.
-  `is_read_only` function L333-335 — `(&self) -> bool` — Agent-facing tools for workflow management: create, list, delete, status.
-  `parameters_schema` function L337-348 — `(&self) -> Value` — Agent-facing tools for workflow management: create, list, delete, status.
-  `execute` function L350-382 — `(&self, _ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput,...` — Agent-facing tools for workflow management: create, list, delete, status.

### examples/workflows/daily-pr-summary

> *Semantic summary to be generated by AI agent.*

#### examples/workflows/daily-pr-summary/build.rs

-  `main` function L1-3 — `()`

### examples/workflows/daily-pr-summary/src

> *Semantic summary to be generated by AI agent.*

#### examples/workflows/daily-pr-summary/src/lib.rs

- pub `daily_pr_summary` module L28-110 — `-` — are replaced with stubs marked TODO.
- pub `fetch_prs` function L36-55 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — Fetch open PRs from the configured GitHub org.
- pub `summarize_prs` function L62-84 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — Summarize the fetched PRs into markdown sections.
- pub `save_briefing` function L91-109 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — Persist the briefing to disk.
- pub `scheduled` function L114 — `()` — Cron trigger — every weekday at 8:00 AM, server's local timezone.
-  `fail` function L20-25 — `(task_id: &str, message: impl Into<String>) -> TaskError` — Tiny helper — collapses cloacina's struct-shaped TaskError variants into

### examples/workflows/issue-triage

> *Semantic summary to be generated by AI agent.*

#### examples/workflows/issue-triage/lib.rs

- pub `issue_triage` module L21-147 — `-` — by copying boilerplate from ../daily-pr-summary/.
- pub `fetch_open_issues` function L27-54 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — Pull open issues from a GitHub repo.
- pub `classify_severity` function L81-111 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — Decision task — asks the agent to classify each issue's severity.
- pub `notify_if_p0` function L120-146 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — Action task — only does work if classifications include at least one P0.
-  `fail` function L10-15 — `(task_id: &str, message: impl Into<String>) -> TaskError` — by copying boilerplate from ../daily-pr-summary/.

### examples/workflows/work-signal-pipeline

> *Semantic summary to be generated by AI agent.*

#### examples/workflows/work-signal-pipeline/lib.rs

- pub `work_signal_pipeline` module L24-124 — `-` — by copying boilerplate from ../daily-pr-summary/.
- pub `fetch_meeting_notes` function L33-43 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — by copying boilerplate from ../daily-pr-summary/.
- pub `fetch_slack_digest` function L46-56 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — by copying boilerplate from ../daily-pr-summary/.
- pub `fetch_jira_updates` function L59-69 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — by copying boilerplate from ../daily-pr-summary/.
- pub `aggregate_signals` function L80-91 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — by copying boilerplate from ../daily-pr-summary/.
- pub `prioritize_signals` function L96-107 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — by copying boilerplate from ../daily-pr-summary/.
- pub `write_briefing` function L110-123 — `(context: &mut Context<Value>) -> Result<(), TaskError>` — by copying boilerplate from ../daily-pr-summary/.
- pub `scheduled` function L127 — `()` — by copying boilerplate from ../daily-pr-summary/.
-  `fail` function L13-18 — `(task_id: &str, message: impl Into<String>) -> TaskError` — by copying boilerplate from ../daily-pr-summary/.

### scripts

**Role**: Standalone functional test script for manual smoke-testing the running Arawn server over WebSocket — not part of the automated test suite.

#### scripts/functional_test.py

- pub `send_rpc` function L16-30 — `def send_rpc(ws, method, params=None)` — Send a JSON-RPC request and return the result.
- pub `send_and_wait` function L33-60 — `def send_and_wait(ws, session_id, prompt)` — Send a message and wait for the Complete event.
- pub `load_session_jsonl` function L63-71 — `def load_session_jsonl(session_id)` — Load the session JSONL from disk.
- pub `analyze` function L74-170 — `def analyze(messages, scenario_name)` — Analyze session messages and print a report.
- pub `run_scenario` function L173-189 — `def run_scenario(prompt, name="test")` — Connect, send prompt, wait, analyze.

