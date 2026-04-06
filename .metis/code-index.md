# Code Index

> Generated: 2026-04-06T10:29:51Z | 144 files | Python, Rust

## Project Structure

```
├── crates/
│   ├── arawn/
│   │   └── src/
│   │       ├── channel_prompt.rs
│   │       ├── config.rs
│   │       ├── config_watcher.rs
│   │       ├── lib.rs
│   │       ├── local_service.rs
│   │       ├── main.rs
│   │       ├── plugin_cmd.rs
│   │       └── ws_server.rs
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
│   │       ├── plugin_adapter.rs
│   │       ├── plugin_loader.rs
│   │       ├── plugin_watcher.rs
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
│   │       └── tools/
│   │           ├── agent.rs
│   │           ├── ask_user.rs
│   │           ├── enter_plan_mode.rs
│   │           ├── exit_plan_mode.rs
│   │           ├── file_edit.rs
│   │           ├── file_read.rs
│   │           ├── file_write.rs
│   │           ├── glob.rs
│   │           ├── grep.rs
│   │           ├── memory_search.rs
│   │           ├── memory_store.rs
│   │           ├── mod.rs
│   │           ├── shell.rs
│   │           ├── skill.rs
│   │           ├── sleep.rs
│   │           ├── task_list.rs
│   │           ├── task_output.rs
│   │           ├── task_stop.rs
│   │           ├── think.rs
│   │           ├── web_fetch.rs
│   │           └── web_search.rs
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
│   │       └── types.rs
│   ├── arawn-mcp/
│   │   └── src/
│   │       ├── adapter.rs
│   │       ├── config.rs
│   │       ├── lib.rs
│   │       └── manager.rs
│   ├── arawn-memory/
│   │   └── src/
│   │       ├── error.rs
│   │       ├── inject.rs
│   │       ├── lib.rs
│   │       ├── manager.rs
│   │       ├── store.rs
│   │       ├── types.rs
│   │       └── vector.rs
│   ├── arawn-service/
│   │   └── src/
│   │       ├── error.rs
│   │       ├── lib.rs
│   │       └── types.rs
│   ├── arawn-storage/
│   │   └── src/
│   │       ├── database.rs
│   │       ├── error.rs
│   │       ├── jsonl.rs
│   │       ├── layout.rs
│   │       ├── lib.rs
│   │       ├── session_store.rs
│   │       ├── store.rs
│   │       └── workstream_store.rs
│   ├── arawn-tests/
│   │   ├── fixtures/
│   │   │   ├── arawn-plugin-web-fetch/
│   │   │   │   └── src/
│   │   │   │       └── lib.rs
│   │   │   └── arawn-plugin-web-search/
│   │   │       └── src/
│   │   │           └── lib.rs
│   │   ├── src/
│   │   │   └── lib.rs
│   │   └── tests/
│   │       ├── compaction.rs
│   │       ├── engine_persistence.rs
│   │       ├── full_pipeline.rs
│   │       ├── hooks.rs
│   │       ├── hot_reload.rs
│   │       ├── local_service.rs
│   │       ├── permissions.rs
│   │       ├── plugin_components.rs
│   │       ├── plugin_loading.rs
│   │       ├── skills.rs
│   │       └── websocket.rs
│   ├── arawn-tool-plugin/
│   │   └── src/
│   │       └── lib.rs
│   └── arawn-tui/
│       └── src/
│           ├── action.rs
│           ├── app.rs
│           ├── command.rs
│           ├── event.rs
│           ├── event_loop.rs
│           ├── lib.rs
│           ├── markdown.rs
│           ├── modal.rs
│           ├── render.rs
│           ├── snapshot.rs
│           ├── snapshot_tests.rs
│           ├── theme.rs
│           ├── tui_prompt.rs
│           └── ws_client.rs
└── scripts/
    └── functional_test.py
```

## Modules

### crates/arawn/src

> *Semantic summary to be generated by AI agent.*

#### crates/arawn/src/channel_prompt.rs

- pub `PendingModals` type L23 — `= Arc<Mutex<HashMap<String, oneshot::Sender<Option<usize>>>>>` — Shared map of pending modal responses.
- pub `new_pending_modals` function L26-28 — `() -> PendingModals` — Create a new empty pending modals map.
- pub `ChannelModalPrompt` struct L31-34 — `{ tx: mpsc::Sender<EngineEvent>, pending: PendingModals }` — ModalPrompt that sends via an EngineEvent channel and waits for response.
- pub `new` function L37-39 — `(tx: mpsc::Sender<EngineEvent>, pending: PendingModals) -> Self` — 6.
-  `ChannelModalPrompt` type L36-40 — `= ChannelModalPrompt` — 6.
-  `ChannelModalPrompt` type L43-84 — `impl ModalPrompt for ChannelModalPrompt` — 6.
-  `prompt` function L44-83 — `(&self, request: ModalRequest) -> Option<usize>` — 6.

#### crates/arawn/src/config.rs

- pub `LlmConfig` struct L9-22 — `{ provider: String, model: String, api_key_env: String, base_url: Option<String>...` — A named LLM provider configuration.
- pub `EngineConfig` struct L48-55 — `{ llm: String, max_iterations: usize, max_result_size: usize }`
- pub `CompactorConfig` struct L78-86 — `{ llm: Option<String>, compaction_threshold: f32, keep_recent: usize }`
- pub `ServerConfig` struct L106-111 — `{ host: String, port: u16 }`
- pub `StorageConfig` struct L130-133 — `{ data_dir: String }`
- pub `PromptsConfig` struct L148-151 — `{ token_budget: u32 }`
- pub `SandboxConfig` struct L167-173 — `{ network_tools: Vec<String> }` — Sandbox configuration for shell command execution.
- pub `ArawnConfig` struct L223-238 — `{ llm: HashMap<String, LlmConfig>, engine: EngineConfig, compactor: CompactorCon...` — Top-level configuration.
- pub `load` function L262-295 — `(data_dir: &Path) -> Self` — Load config from `data_dir/arawn.toml`, merging with env var overrides and defaults.
- pub `engine_llm` function L318-323 — `(&self) -> &LlmConfig` — Resolve the LLM config for the engine.
- pub `compactor_llm` function L326-333 — `(&self) -> &LlmConfig` — Resolve the LLM config for the compactor.
- pub `data_dir` function L336-338 — `(&self) -> PathBuf` — Resolve the data directory with ~ expansion.
- pub `prompts_dir` function L341-343 — `(&self) -> PathBuf` — Resolve the prompts directory.
- pub `resolve_api_key` function L346-350 — `(llm: &LlmConfig) -> Option<String>` — Resolve API key for an LLM config by reading the env var.
- pub `generate_default_toml` function L353-415 — `() -> String` — Generate a default config file string with comments.
-  `default_api_key_env` function L24-26 — `() -> String`
-  `default_context_window` function L27-29 — `() -> u32`
-  `default_max_tokens` function L30-32 — `() -> u32`
-  `LlmConfig` type L34-45 — `impl Default for LlmConfig`
-  `default` function L35-44 — `() -> Self`
-  `default_engine_llm` function L57-59 — `() -> String`
-  `default_max_iterations` function L60-62 — `() -> usize`
-  `default_max_result_size` function L63-65 — `() -> usize`
-  `EngineConfig` type L67-75 — `impl Default for EngineConfig`
-  `default` function L68-74 — `() -> Self`
-  `default_compaction_threshold` function L88-90 — `() -> f32`
-  `default_keep_recent` function L91-93 — `() -> usize`
-  `CompactorConfig` type L95-103 — `impl Default for CompactorConfig`
-  `default` function L96-102 — `() -> Self`
-  `default_host` function L113-115 — `() -> String`
-  `default_port` function L116-118 — `() -> u16`
-  `ServerConfig` type L120-127 — `impl Default for ServerConfig`
-  `default` function L121-126 — `() -> Self`
-  `default_data_dir` function L135-137 — `() -> String`
-  `StorageConfig` type L139-145 — `impl Default for StorageConfig`
-  `default` function L140-144 — `() -> Self`
-  `default_prompt_token_budget` function L153-155 — `() -> u32`
-  `PromptsConfig` type L157-163 — `impl Default for PromptsConfig`
-  `default` function L158-162 — `() -> Self`
-  `default_network_tools` function L175-211 — `() -> Vec<String>`
-  `SandboxConfig` type L213-219 — `impl Default for SandboxConfig`
-  `default` function L214-218 — `() -> Self`
-  `default_llm_configs` function L240-244 — `() -> HashMap<String, LlmConfig>`
-  `ArawnConfig` type L246-258 — `impl Default for ArawnConfig`
-  `default` function L247-257 — `() -> Self`
-  `ArawnConfig` type L260-416 — `= ArawnConfig`
-  `apply_env_overrides` function L297-315 — `(&mut self)`
-  `expand_tilde` function L418-425 — `(path: &str) -> PathBuf`
-  `tests` module L428-555 — `-`
-  `default_config_has_working_values` function L432-441 — `()`
-  `load_from_toml_string` function L444-464 — `()`
-  `compactor_falls_back_to_engine_llm` function L467-472 — `()`
-  `compactor_uses_own_llm_when_specified` function L475-494 — `()`
-  `missing_llm_name_falls_back_to_default_via_load` function L497-513 — `()`
-  `load_missing_file_uses_defaults` function L516-520 — `()`
-  `load_from_tempdir` function L523-541 — `()`
-  `generate_default_toml_is_parseable` function L544-548 — `()`
-  `tilde_expansion` function L551-554 — `()`

#### crates/arawn/src/config_watcher.rs

- pub `ConfigWatcher` struct L21-27 — `{ config_path: PathBuf, data_dir: PathBuf, permission_rules: Arc<std::sync::RwLo...` — Watches config files and dispatches live updates to running subsystems.
- pub `new` function L30-44 — `( config_path: PathBuf, data_dir: PathBuf, permission_rules: Arc<std::sync::RwLo...` — with debouncing.
- pub `spawn` function L47-53 — `(self) -> tokio::task::JoinHandle<()>` — Spawn the file watcher as a background tokio task.
-  `ConfigWatcher` type L29-146 — `= ConfigWatcher` — with debouncing.
-  `run` function L55-114 — `(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>` — with debouncing.
-  `reload` function L116-145 — `(&self)` — with debouncing.

#### crates/arawn/src/lib.rs

- pub `channel_prompt` module L1 — `-`
- pub `config` module L2 — `-`
- pub `config_watcher` module L3 — `-`
- pub `local_service` module L4 — `-`
- pub `plugin_cmd` module L5 — `-`
- pub `ws_server` module L6 — `-`

#### crates/arawn/src/local_service.rs

- pub `LocalService` struct L26-46 — `{ store: Arc<Mutex<Store>>, data_dir: PathBuf, llm: Arc<dyn LlmClient>, registry...` — In-process implementation of ArawnService.
- pub `new` function L49-70 — `( store: Store, data_dir: PathBuf, llm: Arc<dyn LlmClient>, registry: Arc<ToolRe...`
- pub `with_permission_rules` function L72-75 — `(mut self, rules: Vec<PermissionRule>) -> Self`
- pub `shared_permission_rules` function L78-80 — `(&self) -> Arc<std::sync::RwLock<Vec<PermissionRule>>>` — Get a reference to the shared permission rules for hot-reload.
- pub `with_skill_registry` function L82-85 — `(mut self, registry: Arc<arawn_engine::skills::SkillRegistry>) -> Self`
- pub `with_plugin_registry` function L87-90 — `(mut self, registry: Arc<arawn_engine::plugins::PluginRegistry>) -> Self`
- pub `with_plan_state` function L92-95 — `(mut self, state: Arc<PlanModeState>) -> Self`
- pub `with_background_tasks` function L97-100 — `(mut self, manager: Arc<BackgroundTaskManager>) -> Self`
- pub `with_memory_manager` function L102-105 — `(mut self, mgr: Arc<arawn_memory::MemoryManager>) -> Self`
- pub `query_inventory` function L109-181 — `(&self, kind: &str) -> serde_json::Value` — Query available inventory for slash commands.
- pub `list_available_commands` function L184-199 — `(&self) -> serde_json::Value` — List available commands (built-ins + user-invocable skills) for autocomplete cache.
- pub `remember_fact` function L202-248 — `(&self, text: &str) -> serde_json::Value` — Store a fact in the KB via /remember command.
- pub `memory_summary` function L251-289 — `(&self) -> serde_json::Value` — Get KB summary for /memory command.
- pub `forget_entity` function L292-341 — `(&self, query: &str) -> serde_json::Value` — Forget/delete an entity via /forget command.
-  `LocalService` type L48-342 — `= LocalService`
-  `infer_entity_type` function L345-358 — `(text: &str) -> (arawn_memory::EntityType, String)` — Infer entity type from text patterns.
-  `LocalService` type L363-730 — `impl ArawnService for LocalService`
-  `list_workstreams` function L364-379 — `(&self) -> Result<Vec<WorkstreamInfo>, ServiceError>`
-  `create_workstream` function L381-398 — `( &self, name: String, root_dir: PathBuf, ) -> Result<WorkstreamInfo, ServiceErr...`
-  `list_sessions` function L400-419 — `( &self, workstream_id: Option<Uuid>, ) -> Result<Vec<SessionInfo>, ServiceError...`
-  `create_session` function L421-442 — `( &self, workstream_id: Option<Uuid>, ) -> Result<SessionInfo, ServiceError>`
-  `load_session` function L444-471 — `(&self, id: Uuid) -> Result<SessionDetail, ServiceError>`
-  `send_message` function L473-723 — `( &self, session_id: Uuid, content: String, ) -> Result<Pin<Box<dyn futures::Str...`
-  `cancel` function L725-729 — `(&self, _session_id: Uuid) -> Result<(), ServiceError>`
-  `resolve_ws_dir_from_store` function L733-744 — `(store: &Store, ws_id: Option<Uuid>) -> Result<String, ServiceError>` — Resolve workstream directory name from store.
-  `first_sentence` function L748-759 — `(s: &str) -> String` — Extract the first sentence and sanitize for use in a markdown table cell.

#### crates/arawn/src/main.rs

-  `DEFAULT_MODEL` variable L21 — `: &str`
-  `main` function L24-380 — `() -> Result<()>`
-  `run_cli_via_server` function L383-494 — `( url: &str, prompt: &str, session_id: Option<Uuid>, ) -> Result<()>` — Run a CLI prompt by connecting to the running server via WebSocket.
-  `build_llm_client` function L497-519 — `( config: &arawn_bin::LlmConfig, ) -> Result<Arc<dyn arawn_llm::LlmClient>>` — Build the appropriate LLM client based on provider config.
-  `build_engine_config` function L521-553 — `( config: &arawn_bin::ArawnConfig, workstream: &arawn_core::Workstream, data_dir...`
-  `dirs_path` function L555-564 — `() -> Option<String>`

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

- pub `run_server` function L69-84 — `(service: LocalService, port: u16) -> anyhow::Result<()>` — Start the WebSocket server on the given port.
- pub `handle_connection_public` function L94-96 — `(socket: WebSocket, service: Arc<LocalService>)` — Handle a single WebSocket connection.
-  `Request` struct L24-29 — `{ id: u64, method: String, params: Value }` — JSON-RPC style request from client.
-  `Response` struct L33-39 — `{ id: u64, result: Option<Value>, error: Option<ErrorBody> }` — JSON-RPC style response to client.
-  `ErrorBody` struct L42-45 — `{ code: String, message: String }`
-  `Response` type L47-66 — `= Response`
-  `success` function L48-54 — `(id: u64, result: Value) -> Self`
-  `error` function L56-65 — `(id: u64, code: &str, message: String) -> Self`
-  `ws_handler` function L86-91 — `( ws: WebSocketUpgrade, State(service): State<Arc<LocalService>>, ) -> impl Into...`
-  `handle_connection` function L98-460 — `(socket: WebSocket, service: Arc<LocalService>)`

### crates/arawn-core/src

> *Semantic summary to be generated by AI agent.*

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

- pub `Session` struct L11-17 — `{ id: Uuid, workstream_id: Option<Uuid>, messages: Vec<Message>, created_at: Dat...` — A conversation session.
- pub `new` function L21-29 — `(workstream_id: Uuid) -> Self` — Create a session bound to a workstream.
- pub `from_parts` function L32-45 — `( id: Uuid, workstream_id: Option<Uuid>, created_at: DateTime<Utc>, messages: Ve...` — Reconstruct a session from persisted parts (DB load path).
- pub `from_parts_with_stats` function L48-62 — `( id: Uuid, workstream_id: Option<Uuid>, created_at: DateTime<Utc>, messages: Ve...` — Reconstruct a session with stats from persisted parts.
- pub `scratch` function L65-73 — `() -> Self` — Create a scratch session (no workstream binding yet).
- pub `workstream_id` function L75-77 — `(&self) -> Option<Uuid>`
- pub `is_scratch` function L80-82 — `(&self) -> bool` — Returns true if this is a scratch session (not yet promoted).
- pub `promote` function L85-92 — `(&mut self, workstream_id: Uuid)` — Promote a scratch session to a workstream.
- pub `add_message` function L94-96 — `(&mut self, msg: Message)`
- pub `messages` function L98-100 — `(&self) -> &[Message]`
- pub `microcompact` function L106-170 — `(&mut self, keep_recent: usize) -> usize` — Clear old tool results to save context space without an LLM call.
- pub `compact` function L174-207 — `(&mut self, summary_content: String, keep_recent: usize) -> usize` — Replace old messages with a Summary, keeping the last `keep_recent` messages verbatim.
- pub `load_compacted` function L211-221 — `(messages: Vec<Message>) -> Vec<Message>` — Load messages with compaction awareness — if a Summary exists, use the
-  `Session` type L19-222 — `= Session`
-  `TARGETED_TOOLS` variable L107-113 — `: &[&str]`
-  `STUB_THRESHOLD` variable L114 — `: usize`
-  `tests` module L225-531 — `-`
-  `session_bound_to_workstream` function L231-236 — `()`
-  `scratch_session_has_no_workstream` function L239-243 — `()`
-  `promote_scratch_session` function L246-252 — `()`
-  `promote_already_bound_panics` function L256-259 — `()`
-  `session_starts_with_no_messages` function L262-265 — `()`
-  `session_message_ordering_preserved` function L268-295 — `()`
-  `session_ids_are_unique` function L298-303 — `()`
-  `compact_replaces_old_with_summary` function L306-335 — `()`
-  `compact_too_few_messages_noop` function L338-350 — `()`
-  `load_compacted_skips_before_summary` function L353-378 — `()`
-  `load_compacted_no_summary_returns_all` function L381-393 — `()`
-  `microcompact_clears_old_tool_results` function L396-436 — `()`
-  `microcompact_preserves_recent_results` function L439-458 — `()`
-  `microcompact_skips_small_results` function L461-482 — `()`
-  `microcompact_skips_errors` function L485-506 — `()`
-  `microcompact_skips_non_targeted_tools` function L509-530 — `()`

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

- pub `Workstream` struct L9-14 — `{ id: Uuid, name: String, root_dir: PathBuf, created_at: DateTime<Utc> }` — A workstream — the primary organizational unit.
- pub `new` function L17-24 — `(name: impl Into<String>, root_dir: impl Into<PathBuf>) -> Self`
- pub `scratch` function L27-29 — `(root_dir: impl Into<PathBuf>) -> Self` — Create the default scratch workstream for ad-hoc sessions.
-  `Workstream` type L16-30 — `= Workstream`
-  `tests` module L33-56 — `-`
-  `workstream_creation` function L37-41 — `()`
-  `scratch_workstream` function L44-48 — `()`
-  `workstream_ids_are_unique` function L51-55 — `()`

### crates/arawn-embed/src

> *Semantic summary to be generated by AI agent.*

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
-  `resolve_model_dir` function L201-211 — `(config: &EmbeddingConfig) -> Result<PathBuf, EmbedError>` — Model files are downloaded to ~/.arawn/models/ on first use.
-  `download_model_files` function L213-245 — `(model_dir: &Path, model_name: &str) -> Result<(), EmbedError>` — Model files are downloaded to ~/.arawn/models/ on first use.
-  `tests` module L248-267 — `-` — Model files are downloaded to ~/.arawn/models/ on first use.
-  `resolve_default_dir` function L252-256 — `()` — Model files are downloaded to ~/.arawn/models/ on first use.
-  `resolve_custom_dir` function L259-266 — `()` — Model files are downloaded to ~/.arawn/models/ on first use.

### crates/arawn-engine/src

> *Semantic summary to be generated by AI agent.*

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
- pub `cancel` function L264-274 — `(&self, task_id: &str) -> bool` — Cancel a running task.
- pub `list` function L277-289 — `(&self) -> Vec<TaskSummary>` — List all tasks (for inventory/status display).
- pub `running_count` function L292-299 — `(&self) -> usize` — Number of currently running tasks.
- pub `TaskSummary` struct L310-315 — `{ id: String, description: String, status: String, elapsed_secs: u64 }` — Lightweight summary for listing/display.
-  `MAX_OUTPUT_BYTES` variable L18 — `: usize` — Maximum output buffer size per task (100 KB).
-  `generate_task_id` function L21-30 — `() -> String` — Generates a background task ID: "bg_" + 8 hex chars.
-  `rand_bytes` function L32-43 — `() -> [u8; 4]` — conversation so the LLM knows what finished.
-  `TaskNotification` type L55-67 — `= TaskNotification` — conversation so the LLM knows what finished.
-  `BackgroundTaskStatus` type L85-98 — `= BackgroundTaskStatus` — conversation so the LLM knows what finished.
-  `BackgroundTask` type L121-130 — `= BackgroundTask` — conversation so the LLM knows what finished.
-  `fmt` function L122-129 — `(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` — conversation so the LLM knows what finished.
-  `BackgroundTask` type L132-142 — `= BackgroundTask` — conversation so the LLM knows what finished.
-  `BackgroundTaskManager` type L165-300 — `= BackgroundTaskManager` — conversation so the LLM knows what finished.
-  `BackgroundTaskManager` type L302-306 — `impl Default for BackgroundTaskManager` — conversation so the LLM knows what finished.
-  `default` function L303-305 — `() -> Self` — conversation so the LLM knows what finished.
-  `tests` module L318-502 — `-` — conversation so the LLM knows what finished.
-  `generate_task_id_format` function L323-327 — `()` — conversation so the LLM knows what finished.
-  `task_status_labels` function L330-344 — `()` — conversation so the LLM knows what finished.
-  `task_status_is_terminal` function L347-352 — `()` — conversation so the LLM knows what finished.
-  `notification_to_message_format` function L355-365 — `()` — conversation so the LLM knows what finished.
-  `register_and_complete` function L368-400 — `()` — conversation so the LLM knows what finished.
-  `cancel_running_task` function L403-423 — `()` — conversation so the LLM knows what finished.
-  `output_buffer_bounded` function L426-435 — `()` — conversation so the LLM knows what finished.
-  `output_buffer_small_writes` function L438-444 — `()` — conversation so the LLM knows what finished.
-  `list_tasks` function L447-466 — `()` — conversation so the LLM knows what finished.
-  `complete_unknown_task_is_safe` function L469-473 — `()` — conversation so the LLM knows what finished.
-  `cancel_nonexistent_returns_false` function L476-479 — `()` — conversation so the LLM knows what finished.
-  `duplicate_complete_only_notifies_once` function L482-501 — `()` — conversation so the LLM knows what finished.

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

- pub `ToolContext` struct L18-38 — `{ session_id: Uuid, working_dir: PathBuf, workstream_name: String, allowed_paths...` — Execution context provided to tools.
- pub `new` function L54-67 — `(workstream: &Workstream, session_id: Uuid) -> Self`
- pub `with_allowed_paths` function L70-73 — `(mut self, paths: Vec<PathBuf>) -> Self` — Set allowed paths that file tools can access outside the sandbox.
- pub `with_llm` function L76-80 — `(mut self, llm: Arc<dyn LlmClient>, model: String) -> Self` — Attach an LLM client and model for tools that need sub-queries.
- pub `with_model_limits` function L83-86 — `(mut self, limits: ModelLimits) -> Self` — Set model limits for sub-agent compaction.
- pub `with_data_dir` function L89-92 — `(mut self, dir: PathBuf) -> Self` — Set data directory for persisting large tool results.
- pub `is_allowed_path` function L95-104 — `(&self, path: &std::path::Path) -> bool` — Check if a path is in the allowed list (exact match on canonical paths).
- pub `workstream_name` function L106-108 — `(&self) -> &str`
- pub `llm` function L111-113 — `(&self) -> Option<&Arc<dyn LlmClient>>` — Get the LLM client if available.
- pub `model` function L116-118 — `(&self) -> Option<&str>` — Get the model name for sub-queries.
- pub `model_limits` function L121-123 — `(&self) -> &ModelLimits` — Get model limits (for sub-agent compaction).
- pub `data_dir` function L126-128 — `(&self) -> Option<&PathBuf>` — Get data directory for tool result persistence.
- pub `agent_depth` function L131-133 — `(&self) -> u8` — Current agent nesting depth.
- pub `can_spawn_agent` function L136-138 — `(&self) -> bool` — Whether another sub-agent can be spawned at this depth.
- pub `for_sub_agent` function L142-147 — `(&self) -> Self` — Create a child context for a sub-agent (increments depth).
- pub `mark_file_read` function L150-152 — `(&self, path: PathBuf)` — Record that a file has been read in this session.
- pub `has_read_file` function L155-157 — `(&self, path: &PathBuf) -> bool` — Check if a file has been read in this session.
-  `MAX_AGENT_DEPTH` variable L13 — `: u8` — Maximum sub-agent nesting depth.
-  `ToolContext` type L40-51 — `= ToolContext`
-  `fmt` function L41-50 — `(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result`
-  `ToolContext` type L53-158 — `= ToolContext`
-  `tests` module L161-184 — `-`
-  `context_from_workstream` function L166-174 — `()`
-  `context_is_clone` function L177-183 — `()`

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

- pub `EngineError` enum L4-19 — `Tool | ToolNotFound | Llm | MaxIterations | Other`
- pub `user_message` function L23-45 — `(&self) -> String` — Return a user-facing error message with actionable guidance.
-  `EngineError` type L21-46 — `= EngineError`

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
- pub `plugin_adapter` module L11 — `-`
- pub `plugins` module L12 — `-`
- pub `plugin_loader` module L13 — `-`
- pub `plugin_watcher` module L14 — `-`
- pub `query_engine` module L15 — `-`
- pub `skills` module L16 — `-`
- pub `system_prompt` module L17 — `-`
- pub `testing` module L18 — `-`
- pub `token_estimator` module L19 — `-`
- pub `tool` module L20 — `-`
- pub `tool_result_limiter` module L21 — `-`
- pub `tools` module L22 — `-`

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

#### crates/arawn-engine/src/plugin_adapter.rs

- pub `PluginToolAdapter` struct L13-18 — `{ handle: PluginHandle, cached_name: String, cached_description: String, cached_...` — Adapts a fides PluginHandle into an arawn Tool.
- pub `new` function L22-47 — `(handle: PluginHandle) -> Result<Self, EngineError>` — Create an adapter by calling the plugin's metadata methods once.
-  `PluginToolAdapter` type L20-48 — `= PluginToolAdapter`
-  `PluginToolAdapter` type L51-96 — `impl Tool for PluginToolAdapter`
-  `name` function L52-54 — `(&self) -> &str`
-  `description` function L56-58 — `(&self) -> &str`
-  `parameters_schema` function L60-62 — `(&self) -> Value`
-  `execute` function L64-95 — `(&self, ctx: &ToolContext, params: Value) -> Result<ToolOutput, EngineError>`
-  `ContextForPlugin` struct L100-104 — `{ working_dir: String, session_id: String, workstream_name: String }` — Serializable context sent to plugins across FFI.

#### crates/arawn-engine/src/plugin_loader.rs

- pub `PluginLoader` struct L11 — `-` — Scans a directory for `.arawn_tool` archives, unpacks, builds, loads,
- pub `load_tools` function L23-62 — `(tools_dir: &Path, build_dir: &Path) -> Vec<Box<dyn Tool>>` — Load all `.arawn_tool` plugins from `tools_dir`.
-  `PluginLoader` type L13-117 — `= PluginLoader`
-  `find_archives` function L64-82 — `(dir: &Path) -> Result<Vec<std::path::PathBuf>, std::io::Error>`
-  `load_single` function L84-116 — `( archive: &Path, build_dir: &Path, ) -> Result<Vec<Box<dyn Tool>>, Box<dyn std:...`

#### crates/arawn-engine/src/plugin_watcher.rs

- pub `PluginWatcher` struct L14-18 — `{ tools_dir: PathBuf, build_dir: PathBuf, registry: Arc<ToolRegistry> }` — Watches the plugin tools directory for `.arawn_tool` file changes
- pub `new` function L21-27 — `(tools_dir: PathBuf, build_dir: PathBuf, registry: Arc<ToolRegistry>) -> Self`
- pub `spawn` function L31-37 — `(self) -> tokio::task::JoinHandle<()>` — Spawn the file watcher as a background tokio task.
-  `PluginWatcher` type L20-129 — `= PluginWatcher`
-  `run` function L39-86 — `(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>`
-  `is_plugin_event` function L88-96 — `(event: &Event) -> bool`
-  `reload_plugins` function L98-128 — `(&self)`

#### crates/arawn-engine/src/query_engine.rs

- pub `ProgressEvent` enum L25-42 — `AssistantText | ToolCallStart | ToolCallResult` — Live progress events emitted during the engine loop.
- pub `PromptContext` struct L46-57 — `{ prompts_dir: Option<std::path::PathBuf>, os: String, shell: String, cwd: std::...` — Cached context for building system prompts per-turn.
- pub `QueryEngineConfig` struct L60-71 — `{ model: String, max_iterations: usize, system_prompt: String, max_tokens: Optio...` — Configuration for the query engine.
- pub `QueryEngine` struct L88-107 — `{ llm: Arc<dyn LlmClient>, registry: Arc<ToolRegistry>, config: QueryEngineConfi...` — The agentic loop: prompt → LLM → tool_use → execute → feed result → loop.
- pub `new` function L110-126 — `(llm: Arc<dyn LlmClient>, registry: Arc<ToolRegistry>) -> Self`
- pub `with_config` function L128-148 — `( llm: Arc<dyn LlmClient>, registry: Arc<ToolRegistry>, config: QueryEngineConfi...`
- pub `with_compactor` function L150-153 — `(mut self, compactor: Compactor) -> Self`
- pub `with_permission_checker` function L155-158 — `(mut self, checker: Arc<PermissionChecker>) -> Self`
- pub `with_hook_runner` function L160-163 — `(mut self, runner: Arc<HookRunner>) -> Self`
- pub `with_skill_registry` function L165-168 — `(mut self, registry: Arc<crate::skills::SkillRegistry>) -> Self`
- pub `with_plugin_registry` function L170-173 — `(mut self, registry: Arc<crate::plugins::PluginRegistry>) -> Self`
- pub `with_plan_state` function L175-178 — `(mut self, plan_state: Arc<PlanModeState>) -> Self`
- pub `plan_state` function L181-183 — `(&self) -> Option<&Arc<PlanModeState>>` — Get the plan mode state (if configured).
- pub `with_background_tasks` function L185-188 — `(mut self, manager: Arc<BackgroundTaskManager>) -> Self`
- pub `with_progress_sender` function L191-194 — `(mut self, tx: tokio::sync::mpsc::Sender<ProgressEvent>) -> Self` — Set a channel for live progress events during the engine loop.
- pub `fire_hook` function L208-214 — `(&self, input: &HookInput) -> Option<crate::hooks::AggregatedHookResult>` — Fire a hook event.
- pub `run` function L217-522 — `( &mut self, session: &mut Session, ctx: &ToolContext, ) -> Result<String, Engin...` — Run the agentic loop for a session.
-  `DEFAULT_MAX_ITERATIONS` variable L19 — `: usize`
-  `MAX_COMPACT_FAILURES` variable L20 — `: u32`
-  `DEFAULT_SYSTEM_PROMPT` variable L43 — `: &str`
-  `QueryEngineConfig` type L73-85 — `impl Default for QueryEngineConfig`
-  `default` function L74-84 — `() -> Self`
-  `QueryEngine` type L109-842 — `= QueryEngine`
-  `emit_progress` function L197-201 — `(&self, event: ProgressEvent)` — Emit a progress event if a sender is configured.
-  `build_request` function L524-614 — `(&self, session: &Session) -> ChatRequest`
-  `stream_response_with_retry` function L619-658 — `( &self, session: &Session, _ctx: &ToolContext, ) -> Result<AssembledResponse, E...` — Build the request and stream with up to 2 retries on transient LLM errors
-  `MAX_RETRIES` variable L624 — `: u32`
-  `stream_response` function L660-720 — `( &self, request: ChatRequest, ) -> Result<AssembledResponse, EngineError>`
-  `execute_tool` function L722-841 — `( &self, ctx: &ToolContext, tool_use_id: &str, name: &str, arguments: &serde_jso...`
-  `parse_arguments` function L844-849 — `(raw: &str) -> serde_json::Value`
-  `AssembledResponse` struct L852-856 — `{ text: String, tool_calls: Vec<AssembledToolCall>, usage: Option<arawn_llm::Usa...`
-  `AssembledToolCall` struct L858-862 — `{ id: String, name: String, arguments: serde_json::Value }`
-  `ToolResult` struct L864-867 — `{ content: String, is_error: bool }`
-  `CORE_TOOLS` variable L870-872 — `: &[&str]` — Core tools always included in every LLM request.
-  `WEB_TOOLS` variable L875 — `: &[&str]` — Web tools — included when conversation references URLs, web, search, fetch, APIs.
-  `PLAN_TOOLS` variable L878 — `: &[&str]` — Planning tools — included when in plan mode or conversation mentions planning.
-  `TASK_TOOLS` variable L881-883 — `: &[&str]` — Task management tools — included when conversation mentions tasks, background, todo.
-  `MEMORY_TOOLS` variable L886 — `: &[&str]` — Memory tools — included when conversation mentions memory, remember, recall.
-  `AGENT_TOOLS` variable L889 — `: &[&str]` — Agent/delegation tools — included when conversation mentions delegation, agent, subagent.
-  `ALWAYS_TOOLS` variable L892 — `: &[&str]` — Other tools always included.
-  `filter_tools_for_context` function L896-1004 — `( all_tools: &[arawn_llm::ToolDefinition], session: &Session, ) -> Vec<arawn_llm...` — Filter tool definitions to only contextually relevant ones for this turn.
-  `tests` module L1007-1194 — `-`
-  `MockLlm` struct L1018-1020 — `{ responses: Mutex<Vec<Vec<ChatChunk>>> }` — Mock LLM that returns pre-scripted responses.
-  `MockLlm` type L1022-1052 — `= MockLlm`
-  `new` function L1023-1027 — `(responses: Vec<Vec<ChatChunk>>) -> Self`
-  `text` function L1030-1037 — `(text: &str) -> Vec<ChatChunk>` — Convenience: text-only response
-  `tool_call` function L1040-1051 — `(id: &str, name: &str, args: &str) -> Vec<ChatChunk>` — Convenience: tool call then done
-  `MockLlm` type L1055-1071 — `impl LlmClient for MockLlm`
-  `stream` function L1056-1070 — `( &self, _request: ChatRequest, ) -> Result< Pin<Box<dyn futures::Stream<Item = ...`
-  `setup` function L1073-1078 — `() -> (Workstream, Session, ToolContext)`
-  `text_only_response` function L1081-1094 — `()`
-  `single_tool_call` function L1097-1115 — `()`
-  `tool_not_found` function L1118-1140 — `()`
-  `max_iterations_exceeded` function L1143-1170 — `()`
-  `multi_turn_tool_chain` function L1173-1192 — `()`

#### crates/arawn-engine/src/system_prompt.rs

- pub `SystemPromptBuilder` struct L134-137 — `{ sections: Vec<PromptSection>, token_budget: u32 }` — Builds a system prompt from static defaults (overridable) + dynamic context.
- pub `new` function L140-145 — `() -> Self`
- pub `with_token_budget` function L148-151 — `(mut self, budget: u32) -> Self` — Set a custom token budget.
- pub `load_static_sections` function L155-167 — `(mut self, prompts_dir: Option<&Path>) -> Self` — Load all 7 static sections, checking for user overrides in `prompts_dir`.
- pub `environment` function L170-181 — `(mut self, os: &str, shell: &str, cwd: &Path, model: &str) -> Self` — Add the environment section.
- pub `workstream` function L184-194 — `(mut self, name: &str, root_dir: &Path) -> Self` — Add the workstream section.
- pub `tools` function L204-219 — `(mut self, tool_defs: &[ToolDefinition]) -> Self` — Acknowledge tool availability in the system prompt.
- pub `context_files` function L222-245 — `(mut self, files: &[ContextFile]) -> Self` — Add context files (arawn.md at workstream and global levels).
- pub `memories` function L248-263 — `(mut self, memories: &[String]) -> Self` — Add relevant memories (future — currently a no-op if empty).
- pub `session_context` function L266-277 — `(mut self, summary: &str) -> Self` — Add session context (for resumed sessions).
- pub `plugin_prompts` function L280-296 — `(mut self, prompts: &[String]) -> Self` — Add plugin-contributed prompt fragments.
- pub `build` function L299-321 — `(mut self) -> String` — Build the final system prompt string, enforcing token budget.
- pub `ContextFile` struct L334-338 — `{ path: std::path::PathBuf, content: String, truncated: bool }` — A context file loaded from disk.
- pub `find_context_files` function L341-357 — `(workstream_root: &Path, global_dir: &Path) -> Vec<ContextFile>` — Load context files from workstream root and global config dir.
-  `DEFAULT_TOKEN_BUDGET` variable L6 — `: u32` — Default token budget for the system prompt (~24k chars).
-  `MAX_CONTEXT_FILE_CHARS` variable L9 — `: usize` — Max chars for a context file before truncation.
-  `DEFAULT_IDENTITY` variable L13 — `: &str`
-  `DEFAULT_SYSTEM` variable L15-20 — `: &str`
-  `DEFAULT_DOING_TASKS` variable L22-46 — `: &str`
-  `DEFAULT_ACTIONS` variable L48-56 — `: &str`
-  `DEFAULT_USING_TOOLS` variable L58-68 — `: &str`
-  `DEFAULT_TONE` variable L70-74 — `: &str`
-  `DEFAULT_OUTPUT_EFFICIENCY` variable L76-90 — `: &str`
-  `STATIC_SECTION_NAMES` variable L93-101 — `: &[&str]` — Names of the overridable static sections.
-  `STATIC_SECTION_DEFAULTS` variable L104-112 — `: &[&str]` — Compiled-in defaults for each static section.
-  `STATIC_SECTION_PRIORITIES` variable L115-123 — `: &[u8]` — Priority levels for sections.
-  `PromptSection` struct L127-131 — `{ name: String, content: String, priority: u8 }` — A section in the assembled prompt.
-  `SystemPromptBuilder` type L139-322 — `= SystemPromptBuilder`
-  `SystemPromptBuilder` type L324-328 — `impl Default for SystemPromptBuilder`
-  `default` function L325-327 — `() -> Self`
-  `load_context_file` function L359-378 — `(path: &Path, max_chars: usize) -> Option<ContextFile>`
-  `truncate_70_20` function L381-404 — `(content: &str, max_chars: usize) -> String` — Truncate keeping 70% from the head and 20% from the tail, with a marker in between.
-  `load_section` function L408-416 — `(name: &str, default: &str, prompts_dir: Option<&Path>) -> String`
-  `tests` module L419-734 — `-`
-  `default_assembly_includes_all_static_sections` function L426-442 — `()`
-  `sections_have_headers` function L446-457 — `()`
-  `empty_optional_sections_omitted` function L461-472 — `()`
-  `single_section_override` function L476-487 — `()`
-  `partial_overrides_other_sections_use_defaults` function L491-503 — `()`
-  `missing_override_dir_uses_defaults` function L507-513 — `()`
-  `empty_override_file_produces_empty_section` function L517-527 — `()`
-  `under_budget_all_sections_included` function L531-542 — `()`
-  `over_budget_drops_low_priority_sections` function L546-556 — `()`
-  `identity_survives_budget_cuts` function L560-569 — `()`
-  `truncation_produces_clean_sections` function L573-585 — `()`
-  `context_file_injected` function L589-600 — `()`
-  `context_file_missing_section_omitted` function L604-611 — `()`
-  `large_context_file_truncated` function L615-626 — `()`
-  `tools_section_reflects_tool_list` function L630-649 — `()`
-  `per_turn_freshness_different_tools` function L653-677 — `()`
-  `environment_section_contains_info` function L681-690 — `()`
-  `workstream_section_contains_info` function L694-701 — `()`
-  `snapshot_full_build` function L705-733 — `()`

#### crates/arawn-engine/src/testing.rs

- pub `HarnessResult` struct L15-18 — `{ final_text: String, session: Session }` — Result from running the test harness.
- pub `final_text` function L21-23 — `(&self) -> &str`
- pub `tool_calls` function L25-37 — `(&self) -> Vec<(&str, &serde_json::Value)>`
- pub `session_messages` function L39-41 — `(&self) -> &[Message]`
- pub `message_count` function L43-45 — `(&self) -> usize`
- pub `TestHarness` struct L49-58 — `{ _temp_dir: TempDir, workstream: Workstream, registry: Arc<ToolRegistry>, mock_...` — Builder for assembling a full engine test fixture.
- pub `TestHarnessBuilder` struct L61-70 — `{ temp_dir: TempDir, files: Vec<(String, String)>, tools: Vec<Box<dyn Tool>>, sc...` — Builder for constructing a TestHarness.
- pub `new` function L73-84 — `() -> Self`
- pub `with_workstream_file` function L87-94 — `( mut self, path: impl Into<String>, content: impl Into<String>, ) -> Self` — Pre-populate a file in the workstream directory.
- pub `with_tool` function L97-100 — `(mut self, tool: Box<dyn Tool>) -> Self` — Register a tool in the registry.
- pub `with_tools` function L103-106 — `(mut self, tools: impl IntoIterator<Item = Box<dyn Tool>>) -> Self` — Register multiple tools.
- pub `with_script` function L109-112 — `(mut self, script: Vec<MockResponse>) -> Self` — Set the scripted LLM responses.
- pub `with_max_iterations` function L115-118 — `(mut self, max: usize) -> Self` — Set max iterations for the engine.
- pub `with_permission_checker` function L121-124 — `(mut self, checker: Arc<PermissionChecker>) -> Self` — Wire a permission checker into the engine.
- pub `with_hook_runner` function L127-130 — `(mut self, runner: Arc<HookRunner>) -> Self` — Wire a hook runner into the engine.
- pub `with_skill_registry` function L133-136 — `(mut self, registry: Arc<SkillRegistry>) -> Self` — Wire a skill registry into the engine.
- pub `build` function L139-178 — `(self) -> TestHarness` — Build the harness.
- pub `builder` function L188-190 — `() -> TestHarnessBuilder`
- pub `run` function L193-212 — `(&self, user_input: impl Into<String>) -> HarnessResult` — Run the engine with the given user input and return results.
- pub `run_expect_error` function L215-232 — `( &self, user_input: impl Into<String>, ) -> crate::error::EngineError` — Run expecting an error (e.g., max iterations).
-  `HarnessResult` type L20-46 — `= HarnessResult`
-  `TestHarnessBuilder` type L72-179 — `= TestHarnessBuilder`
-  `TestHarnessBuilder` type L181-185 — `impl Default for TestHarnessBuilder`
-  `default` function L182-184 — `() -> Self`
-  `TestHarness` type L187-256 — `= TestHarness`
-  `build_engine` function L235-255 — `(&self) -> QueryEngine` — Build a QueryEngine with all configured subsystems wired in.
-  `tests` module L259-560 — `-`
-  `harness_text_only` function L265-274 — `()`
-  `harness_single_tool_call` function L277-293 — `()`
-  `harness_multi_step_tool_chain` function L296-314 — `()`
-  `harness_tool_not_found` function L317-339 — `()`
-  `harness_max_iterations` function L342-358 — `()`
-  `harness_shell_tool_receives_arguments` function L361-388 — `()`
-  `harness_raw_chunks_split_arguments` function L391-434 — `()`
-  `harness_tool_arguments_passed_correctly` function L437-460 — `()`
-  `harness_permission_checker_blocks_tool` function L463-499 — `()`
-  `harness_permission_checker_allows_tool` function L502-532 — `()`
-  `harness_file_read_with_real_filesystem` function L535-559 — `()`

#### crates/arawn-engine/src/token_estimator.rs

- pub `TokenEstimator` struct L6 — `-` — Fast, approximate token estimation using chars/4 heuristic.
- pub `estimate_message` function L10-26 — `(msg: &Message) -> u32` — Estimate tokens for a single message.
- pub `estimate_messages` function L29-31 — `(messages: &[Message]) -> u32` — Estimate total tokens for all messages in a session.
- pub `estimate_tools` function L34-40 — `(tools: &[ToolDefinition]) -> u32` — Estimate tokens for tool definitions (JSON schemas sent with each request).
- pub `estimate_system_prompt` function L43-45 — `(prompt: &str) -> u32` — Estimate tokens for a system prompt string.
- pub `ModelLimits` struct L50-55 — `{ context_window: u32, compaction_threshold: f32 }` — Model context window limits and compaction threshold.
- pub `new` function L58-63 — `(context_window: u32, compaction_threshold: f32) -> Self`
- pub `for_model` function L66-81 — `(model: &str) -> Self` — Get default limits for a known model name.
- pub `should_compact` function L84-93 — `( &self, session_tokens: u32, tool_tokens: u32, system_tokens: u32, ) -> bool` — Check if the total estimated tokens exceed the compaction threshold.
- pub `available_for_messages` function L96-101 — `(&self, tool_tokens: u32, system_tokens: u32) -> u32` — The token budget available after accounting for tools and system prompt.
-  `TokenEstimator` type L8-46 — `= TokenEstimator`
-  `ModelLimits` type L57-102 — `= ModelLimits`
-  `ModelLimits` type L104-111 — `impl Default for ModelLimits`
-  `default` function L105-110 — `() -> Self`
-  `tests` module L114-224 — `-`
-  `estimate_user_message` function L120-127 — `()`
-  `estimate_assistant_with_tool_uses` function L130-141 — `()`
-  `estimate_tool_result` function L144-152 — `()`
-  `estimate_messages_sums` function L155-171 — `()`
-  `estimate_tools` function L174-182 — `()`
-  `model_limits_for_known_models` function L185-202 — `()`
-  `should_compact_under_threshold` function L205-209 — `()`
-  `should_compact_over_threshold` function L212-215 — `()`
-  `available_for_messages` function L218-223 — `()`

#### crates/arawn-engine/src/tool.rs

- pub `ToolOutput` struct L13-16 — `{ content: String, is_error: bool }` — Output from a tool execution.
- pub `success` function L19-24 — `(content: impl Into<String>) -> Self`
- pub `error` function L26-31 — `(content: impl Into<String>) -> Self`
- pub `Tool` interface L36-50 — `{ fn name(), fn description(), fn parameters_schema(), fn execute(), fn is_read_...` — A tool that can be invoked by the LLM.
- pub `ToolRegistry` struct L54-58 — `{ tools: RwLock<HashMap<String, Arc<dyn Tool>>>, plugin_tools: RwLock<HashSet<St...` — Registry of available tools.
- pub `new` function L61-66 — `() -> Self`
- pub `register` function L69-72 — `(&self, tool: Box<dyn Tool>)` — Register a built-in tool.
- pub `register_plugin` function L75-82 — `(&self, tool: Box<dyn Tool>)` — Register a plugin-provided tool (tracked for hot-reload).
- pub `register_arc` function L85-88 — `(&self, tool: Arc<dyn Tool>)` — Register an already-Arc'd tool (used when building filtered registries).
- pub `unregister` function L90-93 — `(&self, name: &str) -> Option<Arc<dyn Tool>>`
- pub `plugin_tool_names` function L96-98 — `(&self) -> Vec<String>` — Returns the names of all currently loaded plugin tools.
- pub `get` function L101-103 — `(&self, name: &str) -> Option<Arc<dyn Tool>>` — Get a tool by name.
- pub `tool_definitions` function L105-115 — `(&self) -> Vec<arawn_llm::ToolDefinition>`
- pub `len` function L117-119 — `(&self) -> usize`
- pub `is_empty` function L121-123 — `(&self) -> bool`
- pub `unregister_by_prefix` function L127-142 — `(&self, prefix: &str) -> Vec<String>` — Unregister all tools whose names start with the given prefix.
-  `ToolOutput` type L18-32 — `= ToolOutput`
-  `is_read_only` function L47-49 — `(&self) -> bool` — Whether this tool is side-effect-free (observation only).
-  `ToolRegistry` type L60-143 — `= ToolRegistry`
-  `ToolRegistry` type L145-149 — `impl Default for ToolRegistry`
-  `default` function L146-148 — `() -> Self`
-  `tests` module L152-351 — `-`
-  `DummyTool` struct L158-160 — `{ tool_name: String }` — A minimal test tool for unit testing the registry.
-  `DummyTool` type L162-168 — `= DummyTool`
-  `new` function L163-167 — `(name: &str) -> Self`
-  `DummyTool` type L171-191 — `impl Tool for DummyTool`
-  `name` function L172-174 — `(&self) -> &str`
-  `description` function L176-178 — `(&self) -> &str`
-  `parameters_schema` function L180-182 — `(&self) -> Value`
-  `execute` function L184-190 — `( &self, _ctx: &ToolContext, _params: Value, ) -> Result<ToolOutput, EngineError...`
-  `registry_starts_empty` function L194-198 — `()`
-  `register_and_get_tool` function L201-211 — `()`
-  `get_nonexistent_tool_returns_none` function L214-217 — `()`
-  `unregister_tool` function L220-229 — `()`
-  `unregister_nonexistent_returns_none` function L232-235 — `()`
-  `hot_reload_register_unregister_cycle` function L238-256 — `()`
-  `tool_definitions_reflects_registered_tools` function L259-270 — `()`
-  `tool_definitions_updates_after_unregister` function L273-282 — `()`
-  `registry_is_send_sync` function L285-288 — `()`
-  `assert_send_sync` function L286 — `()`
-  `concurrent_access` function L291-309 — `()`
-  `unregister_by_prefix_removes_matching` function L312-327 — `()`
-  `unregister_by_prefix_no_match` function L330-336 — `()`
-  `tool_output_success` function L339-343 — `()`
-  `tool_output_error` function L346-350 — `()`

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

### crates/arawn-engine/src/hooks

> *Semantic summary to be generated by AI agent.*

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
- pub `matches` function L43-66 — `(&self, field_value: &str, content: &str) -> bool` — Check if this matcher matches a given field value and optional content string.
-  `HookMatcher` type L21-25 — `impl Serialize for HookMatcher`
-  `serialize` function L22-24 — `(&self, serializer: S) -> Result<S::Ok, S::Error>`
-  `HookMatcher` type L27-32 — `= HookMatcher`
-  `deserialize` function L28-31 — `(deserializer: D) -> Result<Self, D::Error>`
-  `HookMatcher` type L34-76 — `= HookMatcher`
-  `matches_alternatives` function L69-75 — `(&self, spec: &str, value: &str) -> bool` — Check pipe-separated alternatives: "Bash|Edit|Write"
-  `glob_match` function L80-84 — `(pattern: &str, text: &str) -> bool` — Simple glob matching supporting `*` (any chars) and `?` (single char).
-  `glob_match_inner` function L86-114 — `(pat: &[char], txt: &[char]) -> bool`
-  `tests` module L117-214 — `-`
-  `glob_exact` function L123-126 — `()`
-  `glob_star` function L129-133 — `()`
-  `glob_question_mark` function L136-139 — `()`
-  `empty_matcher_matches_everything` function L144-149 — `()`
-  `exact_tool_match` function L152-156 — `()`
-  `pipe_separated_alternatives` function L159-165 — `()`
-  `glob_tool_match` function L168-173 — `()`
-  `content_pattern` function L176-182 — `()`
-  `content_pattern_with_pipes` function L185-192 — `()`
-  `session_source_matching` function L195-199 — `()`
-  `wildcard_matches_any_tool` function L202-207 — `()`
-  `nested_parens_in_content` function L210-213 — `()`

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

> *Semantic summary to be generated by AI agent.*

#### crates/arawn-engine/src/permissions/checker.rs

- pub `PermissionMode` enum L11-26 — `Default | AcceptEdits | BypassPermissions | Plan` — Permission mode — controls fallback behavior when no explicit rule matches.
- pub `ToolCategory` enum L31-42 — `ReadOnly | FileWrite | Shell | Other` — Category of a tool for permission mode fallback decisions.
- pub `tool_category` function L46-69 — `(tool_name: &str) -> ToolCategory` — Determine the category of a tool by name.
- pub `fallback` function L73-100 — `(&self, tool_name: &str) -> PermissionDecision` — Determine the fallback decision for a tool when no explicit rule matched.
- pub `PermissionResponse` enum L105-109 — `AllowOnce | AllowAlways | Deny` — Response from a user when prompted for permission.
- pub `ModalOption` struct L113-116 — `{ label: String, description: Option<String> }` — A single option displayed in a modal prompt.
- pub `new` function L119-124 — `(label: impl Into<String>) -> Self`
- pub `with_description` function L126-129 — `(mut self, desc: impl Into<String>) -> Self`
- pub `ModalRequest` struct L134-138 — `{ title: String, subtitle: Option<String>, options: Vec<ModalOption> }` — A request to show a modal to the user and get a selection.
- pub `ModalPrompt` interface L144-146 — `{ fn prompt() }` — Generic trait for prompting the user with a modal dialog.
- pub `SessionGrants` struct L152-154 — `{ grants: std::collections::HashSet<String> }` — In-memory store for session-scoped permission grants.
- pub `new` function L157-159 — `() -> Self`
- pub `grant` function L162-164 — `(&mut self, tool_name: String)` — Record a session grant for a tool name.
- pub `is_granted` function L167-169 — `(&self, tool_name: &str) -> bool` — Check if a tool has been granted for this session.
- pub `clear` function L172-174 — `(&mut self)` — Clear all session grants.
- pub `PermissionChecker` struct L179-184 — `{ rules: std::sync::RwLock<Vec<PermissionRule>>, mode: std::sync::RwLock<Permiss...` — The central permission checker.
- pub `new` function L189-196 — `(rules: Vec<PermissionRule>) -> Self` — Create a new permission checker with the given rules and default mode.
- pub `with_mode` function L199-205 — `(self, mode: PermissionMode) -> Self` — Set the permission mode (Default, AcceptEdits, BypassPermissions).
- pub `with_prompter` function L208-211 — `(mut self, prompter: Box<dyn ModalPrompt>) -> Self` — Set the modal prompter for interactive permission requests.
- pub `update_rules` function L214-217 — `(&self, rules: Vec<PermissionRule>)` — Hot-reload: replace the current rules with new ones.
- pub `update_mode` function L220-223 — `(&self, mode: PermissionMode)` — Hot-reload: update the permission mode.
- pub `check` function L232-270 — `(&self, tool_name: &str, tool_input: &str) -> PermissionDecision` — Check if a tool call is permitted.
- pub `mode` function L306-308 — `(&self) -> PermissionMode` — Get the current permission mode.
- pub `clear_grants` function L311-313 — `(&self)` — Clear all session grants.
-  `PermissionMode` type L71-101 — `= PermissionMode`
-  `ModalOption` type L118-130 — `= ModalOption`
-  `SessionGrants` type L156-175 — `= SessionGrants`
-  `PermissionChecker` type L186-314 — `= PermissionChecker`
-  `prompt_user` function L273-303 — `(&self, tool_name: &str, tool_input: &str) -> PermissionDecision` — Prompt the user for permission (or deny if no prompter is configured).
-  `truncate_input` function L316-322 — `(input: &str, max_len: usize) -> String`
-  `tests` module L325-677 — `-`
-  `MockPrompter` struct L330-332 — `{ index: Option<usize> }` — Mock prompter that returns a fixed index (0=AllowOnce, 1=AllowAlways, 2/None=Deny).
-  `MockPrompter` type L334-338 — `= MockPrompter`
-  `allow_once` function L335 — `() -> Self`
-  `allow_always` function L336 — `() -> Self`
-  `deny` function L337 — `() -> Self`
-  `MockPrompter` type L341-345 — `impl ModalPrompt for MockPrompter`
-  `prompt` function L342-344 — `(&self, _request: ModalRequest) -> Option<usize>`
-  `allowed_by_rule` function L348-355 — `()`
-  `denied_by_rule` function L358-365 — `()`
-  `ask_without_prompter_denies` function L368-375 — `()`
-  `ask_with_allow_once` function L378-387 — `()`
-  `ask_with_allow_always_grants_session` function L390-403 — `()`
-  `ask_with_deny` function L406-413 — `()`
-  `default_mode_allows_read_only` function L416-435 — `()`
-  `default_mode_asks_for_writes` function L438-453 — `()`
-  `accept_edits_mode_allows_file_ops` function L456-476 — `()`
-  `bypass_mode_allows_everything` function L479-497 — `()`
-  `explicit_rules_override_mode` function L500-508 — `()`
-  `session_grant_short_circuits` function L511-520 — `()`
-  `clear_grants_resets` function L523-532 — `()`
-  `truncate_input_short` function L535-537 — `()`
-  `truncate_input_long` function L540-544 — `()`
-  `tool_categories` function L547-560 — `()`
-  `update_rules_hot_reload` function L563-584 — `()`
-  `update_mode_hot_reload` function L587-609 — `()`
-  `permission_mode_serde` function L612-621 — `()`
-  `plan_mode_allows_read_only` function L624-642 — `()`
-  `plan_mode_denies_writes` function L645-663 — `()`
-  `plan_mode_allows_plan_meta_tools` function L666-676 — `()`

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
- pub `parse` function L45-63 — `(kind: RuleKind, spec: &str) -> Self` — Parse a rule from the compact string format: `"ToolName"` or `"ToolName(content pattern)"`.
- pub `matches` function L66-74 — `(&self, tool_name: &str, tool_input: &str) -> bool` — Check if this rule matches a given tool name and input.
- pub `PermissionDecision` enum L79-88 — `Allowed | Denied | Ask | NoMatch` — The result of evaluating permission rules against a tool call.
- pub `RuleMatcher` struct L94 — `-` — Evaluates a list of permission rules against a tool call.
- pub `evaluate` function L100-127 — `( rules: &[PermissionRule], tool_name: &str, tool_input: &str, ) -> PermissionDe...` — Evaluate rules against a tool call.
-  `PermissionRule` type L30-75 — `= PermissionRule`
-  `RuleMatcher` type L96-128 — `= RuleMatcher`
-  `glob_match` function L132-136 — `(pattern: &str, text: &str) -> bool` — Simple glob matching supporting `*` (any chars) and `?` (single char).
-  `glob_match_inner` function L138-166 — `(pat: &[char], txt: &[char]) -> bool`
-  `tests` module L169-374 — `-`
-  `glob_exact_match` function L175-178 — `()`
-  `glob_star_match` function L181-186 — `()`
-  `glob_question_mark` function L189-192 — `()`
-  `glob_complex_patterns` function L195-200 — `()`
-  `glob_content_patterns` function L203-208 — `()`
-  `rule_exact_tool_match` function L213-217 — `()`
-  `rule_glob_tool_match` function L220-225 — `()`
-  `rule_with_content_pattern` function L228-233 — `()`
-  `rule_parse_simple` function L236-240 — `()`
-  `rule_parse_with_content` function L243-247 — `()`
-  `rule_parse_nested_parens` function L250-255 — `()`
-  `matcher_deny_takes_priority` function L260-269 — `()`
-  `matcher_allow_before_ask` function L272-281 — `()`
-  `matcher_ask_when_only_ask_rule` function L284-290 — `()`
-  `matcher_no_match_when_no_rules` function L293-298 — `()`
-  `matcher_no_match_when_rules_dont_apply` function L301-307 — `()`
-  `matcher_content_pattern_deny` function L310-325 — `()`
-  `matcher_mixed_rules_realistic` function L328-373 — `()`

### crates/arawn-engine/src/plugins

> *Semantic summary to be generated by AI agent.*

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
- pub `load_plugin_components` function L32-115 — `(plugin: &LoadedPlugin) -> PluginComponents` — Load all components from a plugin into a `PluginComponents` struct.
- pub `register_plugin_skills` function L118-122 — `(registry: &SkillRegistry, skills: Vec<SkillDefinition>)` — Register a plugin's skills into a SkillRegistry.
- pub `merge_plugin_hooks` function L125-127 — `(target: &mut HookConfig, plugin_hooks: HookConfig)` — Merge a plugin's hooks into an existing HookConfig.
-  `tests` module L130-380 — `-` — from a plugin's declared directories into the engine's registries.
-  `make_plugin` function L137-149 — `(dir: &TempDir, name: &str, paths: ResolvedPaths) -> LoadedPlugin` — from a plugin's declared directories into the engine's registries.
-  `load_agents_from_plugin` function L152-181 — `()` — from a plugin's declared directories into the engine's registries.
-  `load_skills_from_plugin` function L184-215 — `()` — from a plugin's declared directories into the engine's registries.
-  `load_hooks_from_file_path` function L218-256 — `()` — from a plugin's declared directories into the engine's registries.
-  `load_inline_hooks` function L259-286 — `()` — from a plugin's declared directories into the engine's registries.
-  `mcp_servers_extracted` function L289-314 — `()` — from a plugin's declared directories into the engine's registries.
-  `missing_dir_produces_error_not_panic` function L317-333 — `()` — from a plugin's declared directories into the engine's registries.
-  `empty_plugin_loads_nothing` function L336-346 — `()` — from a plugin's declared directories into the engine's registries.
-  `register_skills_into_registry` function L349-364 — `()` — from a plugin's declared directories into the engine's registries.
-  `merge_hooks_into_config` function L367-379 — `()` — from a plugin's declared directories into the engine's registries.

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
-  `clone_plugin_to_cache` function L215-323 — `( plugin: &MarketplacePlugin, market_source: &super::marketplace::MarketplaceSou...` — Clone a plugin's source into the cache directory.
-  `copy_dir_recursive` function L326-344 — `(src: &Path, dst: &Path) -> Result<(), String>` — Recursively copy a directory's contents.
-  `tests` module L347-509 — `-` — and track installations in installed_plugins.json.
-  `registry_roundtrip` function L352-376 — `()` — and track installations in installed_plugins.json.
-  `registry_replace_same_scope` function L379-405 — `()` — and track installations in installed_plugins.json.
-  `registry_multiple_scopes` function L408-433 — `()` — and track installations in installed_plugins.json.
-  `registry_remove_one_scope` function L436-462 — `()` — and track installations in installed_plugins.json.
-  `registry_remove_last_scope` function L465-481 — `()` — and track installations in installed_plugins.json.
-  `registry_load_missing` function L484-488 — `()` — and track installations in installed_plugins.json.
-  `copy_dir_skips_git` function L491-508 — `()` — and track installations in installed_plugins.json.

#### crates/arawn-engine/src/plugins/loader.rs

- pub `PluginIdentifier` struct L15-18 — `{ name: String, marketplace: String }` — Plugin identifier in `name@marketplace` format.
- pub `new` function L21-26 — `(name: impl Into<String>, marketplace: impl Into<String>) -> Self` — Plugin discovery and loading — scans directories for plugin.json manifests.
- pub `parse` function L29-38 — `(s: &str) -> Option<Self>` — Parse from `name@marketplace` string.
- pub `inline` function L41-46 — `(name: impl Into<String>) -> Self` — For inline/session plugins loaded via --plugin-dir.
- pub `PluginSource` enum L57-64 — `Cache | Inline | BuiltIn` — Source of a loaded plugin.
- pub `LoadedPlugin` struct L68-81 — `{ id: PluginIdentifier, manifest: PluginManifest, plugin_dir: PathBuf, source: P...` — A discovered and validated plugin ready for component loading.
- pub `ResolvedPaths` struct L85-91 — `{ agents: Option<PathBuf>, skills: Option<PathBuf>, commands: Option<PathBuf>, t...` — Resolved absolute paths for plugin component directories.
- pub `name` function L95-97 — `(&self) -> &str` — Plugin name (convenience accessor).
- pub `discover_plugins` function L104-164 — `(plugins_root: &Path) -> Vec<LoadedPlugin>` — Discover plugins from the versioned cache directory.
- pub `load_plugin_dir` function L169-175 — `(dir: &Path) -> Option<LoadedPlugin>` — Load a single plugin from a directory (for --plugin-dir flag).
- pub `PluginRegistry` struct L268-270 — `{ plugins: RwLock<HashMap<String, LoadedPlugin>> }` — Registry of loaded plugins, queryable by identifier string.
- pub `new` function L273-277 — `() -> Self` — Plugin discovery and loading — scans directories for plugin.json manifests.
- pub `register` function L280-283 — `(&self, plugin: LoadedPlugin)` — Register a loaded plugin (keyed by id string: `name@marketplace`).
- pub `get` function L287-302 — `(&self, key: &str) -> Option<LoadedPlugin>` — Get a plugin by identifier string (e.g.
- pub `all` function L305-307 — `(&self) -> Vec<LoadedPlugin>` — Get all registered plugins.
- pub `enabled` function L310-318 — `(&self) -> Vec<LoadedPlugin>` — Get only enabled plugins.
- pub `len` function L320-322 — `(&self) -> usize` — Plugin discovery and loading — scans directories for plugin.json manifests.
- pub `is_empty` function L324-326 — `(&self) -> bool` — Plugin discovery and loading — scans directories for plugin.json manifests.
- pub `set_enabled` function L329-333 — `(&self, key: &str, enabled: bool)` — Set enable/disable state by identifier string.
-  `PluginIdentifier` type L20-47 — `= PluginIdentifier` — Plugin discovery and loading — scans directories for plugin.json manifests.
-  `PluginIdentifier` type L49-53 — `= PluginIdentifier` — Plugin discovery and loading — scans directories for plugin.json manifests.
-  `fmt` function L50-52 — `(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` — Plugin discovery and loading — scans directories for plugin.json manifests.
-  `LoadedPlugin` type L93-98 — `= LoadedPlugin` — Plugin discovery and loading — scans directories for plugin.json manifests.
-  `load_plugin_from_dir` function L178-219 — `( dir: &Path, default_name: &str, marketplace: &str, source: PluginSource, ) -> ...` — Load a plugin from a directory, reading .claude-plugin/plugin.json or plugin.json.
-  `resolve_paths` function L227-265 — `(manifest: &PluginManifest, plugin_dir: &Path) -> ResolvedPaths` — Resolve relative component paths against the plugin directory.
-  `PluginRegistry` type L272-334 — `= PluginRegistry` — Plugin discovery and loading — scans directories for plugin.json manifests.
-  `tests` module L337-462 — `-` — Plugin discovery and loading — scans directories for plugin.json manifests.
-  `write_cached_plugin` function L342-347 — `(root: &Path, marketplace: &str, name: &str, version: &str, extra: &str)` — Create a cache-structured plugin: cache/{marketplace}/{plugin}/{version}/plugin.json
-  `write_claude_plugin` function L350-356 — `(root: &Path, marketplace: &str, name: &str, version: &str)` — Create a .claude-plugin/plugin.json style plugin.
-  `discover_from_cache` function L359-370 — `()` — Plugin discovery and loading — scans directories for plugin.json manifests.
-  `latest_version_wins` function L373-381 — `()` — Plugin discovery and loading — scans directories for plugin.json manifests.
-  `claude_plugin_path_discovered` function L384-392 — `()` — Plugin discovery and loading — scans directories for plugin.json manifests.
-  `missing_cache_dir_returns_empty` function L395-398 — `()` — Plugin discovery and loading — scans directories for plugin.json manifests.
-  `load_plugin_dir_inline` function L401-409 — `()` — Plugin discovery and loading — scans directories for plugin.json manifests.
-  `identifier_parse_display` function L412-417 — `()` — Plugin discovery and loading — scans directories for plugin.json manifests.
-  `identifier_parse_invalid` function L420-424 — `()` — Plugin discovery and loading — scans directories for plugin.json manifests.
-  `registry_keyed_by_id` function L427-443 — `()` — Plugin discovery and loading — scans directories for plugin.json manifests.
-  `registry_enable_disable` function L446-461 — `()` — Plugin discovery and loading — scans directories for plugin.json manifests.

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
- pub `validate` function L190-218 — `(&self) -> Vec<PluginError>` — Validate the manifest and return any errors found.
-  `deserialize_hooks_field` function L114-132 — `(deserializer: D) -> Result<Option<HooksField>, D::Error>` — Plugin manifest — deserialization and validation of plugin.json.
-  `PluginError` type L145-155 — `= PluginError` — Plugin manifest — deserialization and validation of plugin.json.
-  `fmt` function L146-154 — `(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` — Plugin manifest — deserialization and validation of plugin.json.
-  `PluginManifest` type L157-237 — `= PluginManifest` — Plugin manifest — deserialization and validation of plugin.json.
-  `component_paths` function L221-236 — `(&self) -> Vec<(&str, &str)>` — Get all component path fields that are set.
-  `tests` module L240-424 — `-` — Plugin manifest — deserialization and validation of plugin.json.
-  `parse_full_manifest` function L244-286 — `()` — Plugin manifest — deserialization and validation of plugin.json.
-  `parse_minimal_manifest` function L289-297 — `()` — Plugin manifest — deserialization and validation of plugin.json.
-  `parse_hooks_inline` function L300-319 — `()` — Plugin manifest — deserialization and validation of plugin.json.
-  `parse_hooks_path` function L322-326 — `()` — Plugin manifest — deserialization and validation of plugin.json.
-  `validate_missing_name` function L329-336 — `()` — Plugin manifest — deserialization and validation of plugin.json.
-  `validate_invalid_paths` function L339-349 — `()` — Plugin manifest — deserialization and validation of plugin.json.
-  `validate_invalid_hooks_path` function L352-361 — `()` — Plugin manifest — deserialization and validation of plugin.json.
-  `validate_valid_manifest` function L364-374 — `()` — Plugin manifest — deserialization and validation of plugin.json.
-  `parse_error_on_invalid_json` function L377-380 — `()` — Plugin manifest — deserialization and validation of plugin.json.
-  `mcp_server_with_env` function L383-402 — `()` — Plugin manifest — deserialization and validation of plugin.json.
-  `user_config_with_default` function L405-423 — `()` — Plugin manifest — deserialization and validation of plugin.json.

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

- pub `PluginMcpServer` struct L27-33 — `{ name: String, command: String, args: Vec<String>, env: std::collections::HashM...` — An MCP server config extracted from a plugin manifest, ready for connection.
- pub `PluginLoadResult` struct L36-41 — `{ agents: Vec<AgentDefinition>, skills: Vec<SkillDefinition>, hooks: HookConfig,...` — Result of loading all plugins — the components ready to wire into the engine.
- pub `PluginRuntime` struct L44-53 — `{ plugins_root: PathBuf, settings_path: Option<PathBuf>, plugin_dirs: Vec<PathBu...` — Plugin runtime — manages plugin lifecycle for a running arawn instance.
- pub `new` function L56-63 — `(plugins_root: PathBuf) -> Self` — to hot-reload when plugins are installed or changed.
- pub `with_settings` function L65-68 — `(mut self, path: PathBuf) -> Self` — to hot-reload when plugins are installed or changed.
- pub `with_plugin_dir` function L70-73 — `(mut self, dir: PathBuf) -> Self` — to hot-reload when plugins are installed or changed.
- pub `load_all` function L76-167 — `( &self, tool_registry: &Arc<ToolRegistry>, skill_registry: &Arc<SkillRegistry>,...` — Discover, load, and register all plugins.
- pub `watch` function L173-286 — `( &self, tool_registry: Arc<ToolRegistry>, skill_registry: Arc<SkillRegistry>, )...` — Spawn a file watcher that hot-reloads plugins when the cache directory changes.
-  `PluginRuntime` type L55-287 — `= PluginRuntime` — to hot-reload when plugins are installed or changed.

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

> *Semantic summary to be generated by AI agent.*

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
- pub `new` function L15-19 — `() -> Self`
- pub `register` function L22-25 — `(&self, skill: SkillDefinition)` — Register a skill.
- pub `get` function L28-40 — `(&self, name: &str) -> Option<SkillDefinition>` — Look up a skill by name (case-insensitive).
- pub `all` function L43-45 — `(&self) -> Vec<SkillDefinition>` — Get all registered skills.
- pub `user_invocable` function L48-56 — `(&self) -> Vec<SkillDefinition>` — Get only user-invocable skills.
- pub `len` function L59-61 — `(&self) -> usize` — Number of registered skills.
- pub `is_empty` function L63-65 — `(&self) -> bool`
- pub `load_skills_dir` function L73-115 — `(dir: &Path, source: SkillSource) -> Vec<SkillDefinition>` — Load skill definitions from a directory.
- pub `load_merged_skills` function L142-163 — `( project_dir: Option<&Path>, user_dir: Option<&Path>, ) -> SkillRegistry` — Load and merge skills from project and user directories.
- pub `format_skill_listing` function L169-205 — `(skills: &[SkillDefinition], budget_chars: usize, max_desc_chars: usize) -> Stri...` — Format skill listing for the system prompt, respecting a character budget.
-  `SkillRegistry` type L14-66 — `= SkillRegistry`
-  `load_skill_file` function L117-137 — `(path: &Path, default_name: &str, source: SkillSource) -> Option<SkillDefinition...`
-  `tests` module L208-432 — `-`
-  `load_skills_from_files` function L213-243 — `()`
-  `load_skill_from_subdirectory` function L246-264 — `()`
-  `project_overrides_user` function L267-296 — `()`
-  `registry_case_insensitive_lookup` function L299-315 — `()`
-  `empty_dir_returns_no_skills` function L318-322 — `()`
-  `nonexistent_dir_returns_no_skills` function L325-328 — `()`
-  `format_listing_basic` function L331-358 — `()`
-  `format_listing_truncates_description` function L361-377 — `()`
-  `format_listing_respects_budget` function L380-396 — `()`
-  `format_listing_empty` function L399-402 — `()`
-  `user_invocable_filter` function L405-431 — `()`

#### crates/arawn-engine/src/skills/mod.rs

-  `definition` module L7 — `-` — Skills are markdown files with YAML frontmatter that define prompt templates
-  `loader` module L8 — `-` — execute a skill, which injects the skill's prompt into the conversation.

### crates/arawn-engine/src/tools

> *Semantic summary to be generated by AI agent.*

#### crates/arawn-engine/src/tools/agent.rs

- pub `AgentTool` struct L28-32 — `{ registry: Arc<ToolRegistry>, definitions: Vec<AgentDefinition>, bg_manager: Op...` — Spawns a sub-agent that runs a full `QueryEngine` loop in an isolated
- pub `new` function L35-41 — `(registry: Arc<ToolRegistry>, definitions: Vec<AgentDefinition>) -> Self`
- pub `with_background_manager` function L44-47 — `(mut self, mgr: Arc<BackgroundTaskManager>) -> Self` — Attach a background task manager for `run_in_background` support.
-  `DEFAULT_MAX_TURNS` variable L20 — `: usize`
-  `AgentTool` type L34-48 — `= AgentTool`
-  `AgentTool` type L51-266 — `impl Tool for AgentTool`
-  `name` function L52-54 — `(&self) -> &str`
-  `description` function L56-75 — `(&self) -> &str`
-  `parameters_schema` function L77-100 — `(&self) -> Value`
-  `execute` function L102-265 — `(&self, ctx: &ToolContext, params: Value) -> Result<ToolOutput, EngineError>`
-  `tests` module L269-445 — `-`
-  `test_ctx_with_mock` function L276-285 — `( responses: Vec<MockResponse>, ) -> (ToolContext, Arc<MockLlmClient>, Arc<ToolR...`
-  `schema_is_valid` function L288-297 — `()`
-  `text_only_sub_agent` function L300-317 — `()`
-  `sub_agent_with_tool_call` function L320-337 — `()`
-  `sub_agent_no_llm_errors` function L340-349 — `()`
-  `sub_agent_max_iterations_returns_last_text` function L352-374 — `()`
-  `depth_limit_prevents_infinite_recursion` function L377-391 — `()`
-  `explore_agent_type_used` function L394-410 — `()`
-  `unknown_type_falls_back_to_general` function L413-427 — `()`
-  `for_sub_agent_increments_depth` function L430-444 — `()`

#### crates/arawn-engine/src/tools/ask_user.rs

- pub `AskUserTool` struct L13 — `-` — Asks the user structured multiple-choice questions to gather requirements
-  `AskUserTool` type L16-135 — `impl Tool for AskUserTool`
-  `name` function L17-19 — `(&self) -> &str`
-  `description` function L21-30 — `(&self) -> &str`
-  `is_read_only` function L32-34 — `(&self) -> bool`
-  `parameters_schema` function L36-81 — `(&self) -> Value`
-  `execute` function L83-134 — `(&self, _ctx: &ToolContext, params: Value) -> Result<ToolOutput, EngineError>`
-  `tests` module L138-250 — `-`
-  `test_ctx` function L144-147 — `() -> ToolContext`
-  `schema_is_valid` function L150-157 — `()`
-  `is_read_only` function L160-162 — `()`
-  `single_question` function L165-189 — `()`
-  `multi_select_shows_hint` function L192-213 — `()`
-  `multiple_questions` function L216-241 — `()`
-  `empty_questions_errors` function L244-249 — `()`

#### crates/arawn-engine/src/tools/enter_plan_mode.rs

- pub `EnterPlanModeTool` struct L14-16 — `{ plan_state: Arc<PlanModeState> }` — Tool that enters plan mode — restricts the agent to observation-only tools
- pub `new` function L19-21 — `(plan_state: Arc<PlanModeState>) -> Self`
-  `EnterPlanModeTool` type L18-22 — `= EnterPlanModeTool`
-  `EnterPlanModeTool` type L25-90 — `impl Tool for EnterPlanModeTool`
-  `name` function L26-28 — `(&self) -> &str`
-  `description` function L30-40 — `(&self) -> &str`
-  `is_read_only` function L42-44 — `(&self) -> bool`
-  `parameters_schema` function L46-57 — `(&self) -> Value`
-  `execute` function L59-89 — `(&self, ctx: &ToolContext, params: Value) -> Result<ToolOutput, EngineError>`
-  `tests` module L93-147 — `-`
-  `test_ctx` function L99-102 — `(dir: &std::path::Path) -> ToolContext`
-  `enter_plan_mode_activates` function L105-120 — `()`
-  `enter_plan_mode_when_already_active` function L123-139 — `()`
-  `enter_plan_mode_is_read_only` function L142-146 — `()`

#### crates/arawn-engine/src/tools/exit_plan_mode.rs

- pub `ExitPlanModeTool` struct L14-16 — `{ plan_state: Arc<PlanModeState> }` — Tool that exits plan mode — writes the plan to disk and deactivates plan mode
- pub `new` function L19-21 — `(plan_state: Arc<PlanModeState>) -> Self`
-  `ExitPlanModeTool` type L18-22 — `= ExitPlanModeTool`
-  `ExitPlanModeTool` type L25-93 — `impl Tool for ExitPlanModeTool`
-  `name` function L26-28 — `(&self) -> &str`
-  `description` function L30-35 — `(&self) -> &str`
-  `is_read_only` function L37-40 — `(&self) -> bool`
-  `parameters_schema` function L42-53 — `(&self) -> Value`
-  `execute` function L55-92 — `(&self, _ctx: &ToolContext, params: Value) -> Result<ToolOutput, EngineError>`
-  `tests` module L96-174 — `-`
-  `test_ctx` function L103-106 — `() -> ToolContext`
-  `setup` function L108-116 — `() -> (Arc<PlanModeState>, ExitPlanModeTool, std::path::PathBuf)`
-  `exit_not_in_plan_mode` function L119-127 — `()`
-  `exit_with_empty_plan` function L130-137 — `()`
-  `exit_deactivates_plan_mode` function L140-153 — `()`
-  `plan_written_to_disk` function L156-166 — `()`
-  `exit_plan_mode_is_read_only` function L169-173 — `()`

#### crates/arawn-engine/src/tools/file_edit.rs

- pub `FileEditTool` struct L9 — `-` — Edit a file by replacing a string.
-  `FileEditTool` type L12-146 — `impl Tool for FileEditTool`
-  `name` function L13-15 — `(&self) -> &str`
-  `description` function L17-27 — `(&self) -> &str`
-  `parameters_schema` function L29-52 — `(&self) -> Value`
-  `execute` function L54-145 — `(&self, ctx: &ToolContext, params: Value) -> Result<ToolOutput, EngineError>`
-  `tests` module L149-302 — `-`
-  `test_ctx` function L155-158 — `(dir: &std::path::Path) -> ToolContext`
-  `mark_read` function L161-164 — `(ctx: &ToolContext, dir: &std::path::Path, name: &str)` — Mark a file as read in the context (simulates a prior file_read call).
-  `edit_replaces_string` function L167-188 — `()`
-  `edit_fails_on_missing_string` function L191-209 — `()`
-  `edit_fails_on_ambiguous_match` function L212-230 — `()`
-  `edit_replace_all` function L233-254 — `()`
-  `edit_rejects_path_traversal` function L257-271 — `()`
-  `edit_fails_without_prior_read` function L274-292 — `()`
-  `schema_is_valid` function L295-301 — `()`

#### crates/arawn-engine/src/tools/file_read.rs

- pub `FileReadTool` struct L12 — `-` — Read a file within the workstream's working directory.
-  `FileReadTool` type L15-124 — `impl Tool for FileReadTool`
-  `name` function L16-18 — `(&self) -> &str`
-  `description` function L20-29 — `(&self) -> &str`
-  `is_read_only` function L31-33 — `(&self) -> bool`
-  `parameters_schema` function L35-54 — `(&self) -> Value`
-  `execute` function L56-123 — `(&self, ctx: &ToolContext, params: Value) -> Result<ToolOutput, EngineError>`
-  `would_escape_root` function L129-134 — `(root: &Path, relative_path: &str) -> bool` — Check if a path would escape the root without requiring the file to exist.
-  `normalize_path` function L137-149 — `(path: &Path) -> std::path::PathBuf` — Normalize a path by resolving .
-  `tests` module L152-260 — `-`
-  `test_ctx_with_dir` function L159-162 — `(dir: &Path) -> ToolContext`
-  `read_existing_file` function L165-180 — `()`
-  `read_with_offset_and_limit` function L183-197 — `()`
-  `read_nonexistent_file` function L200-211 — `()`
-  `path_traversal_rejected` function L214-234 — `()`
-  `missing_path_param` function L237-243 — `()`
-  `schema_is_valid` function L246-251 — `()`
-  `would_escape_root_detects_traversal` function L254-259 — `()`

#### crates/arawn-engine/src/tools/file_write.rs

- pub `FileWriteTool` struct L10 — `-` — Write content to a file within the workstream's working directory.
-  `FileWriteTool` type L13-132 — `impl Tool for FileWriteTool`
-  `name` function L14-16 — `(&self) -> &str`
-  `description` function L18-27 — `(&self) -> &str`
-  `parameters_schema` function L29-44 — `(&self) -> Value`
-  `execute` function L46-131 — `(&self, ctx: &ToolContext, params: Value) -> Result<ToolOutput, EngineError>`
-  `normalize_path` function L134-146 — `(path: &std::path::Path) -> std::path::PathBuf`
-  `tests` module L149-281 — `-`
-  `test_ctx` function L155-158 — `(dir: &std::path::Path) -> ToolContext`
-  `mark_read` function L160-163 — `(ctx: &ToolContext, path: &std::path::Path)`
-  `write_creates_file` function L166-182 — `()`
-  `write_creates_parent_dirs` function L185-200 — `()`
-  `write_overwrites_existing` function L203-221 — `()`
-  `write_rejects_path_traversal` function L224-239 — `()`
-  `write_new_file_without_read_ok` function L242-253 — `()`
-  `write_existing_file_without_read_fails` function L256-271 — `()`
-  `schema_is_valid` function L274-280 — `()`

#### crates/arawn-engine/src/tools/glob.rs

- pub `GlobTool` struct L15 — `-` — Fast file pattern matching using globwalk.
-  `MAX_RESULTS` variable L11 — `: usize` — Maximum number of files to return before truncating.
-  `GlobTool` type L18-129 — `impl Tool for GlobTool`
-  `name` function L19-21 — `(&self) -> &str`
-  `description` function L23-29 — `(&self) -> &str`
-  `is_read_only` function L31-33 — `(&self) -> bool`
-  `parameters_schema` function L35-50 — `(&self) -> Value`
-  `execute` function L52-128 — `(&self, ctx: &ToolContext, params: Value) -> Result<ToolOutput, EngineError>`
-  `tests` module L132-212 — `-`
-  `schema_is_valid` function L139-146 — `()`
-  `is_read_only` function L149-151 — `()`
-  `glob_in_tempdir` function L154-173 — `()`
-  `glob_no_matches` function L176-188 — `()`
-  `glob_respects_gitignore` function L191-211 — `()`

#### crates/arawn-engine/src/tools/grep.rs

- pub `GrepTool` struct L16 — `-` — Search file contents using ripgrep (rg) or grep as fallback.
-  `DEFAULT_HEAD_LIMIT` variable L10 — `: usize` — Default cap on grep results when head_limit is unspecified.
-  `VCS_EXCLUDES` variable L13 — `: &[&str]` — VCS directories to exclude from searches.
-  `GrepTool` type L19-205 — `impl Tool for GrepTool`
-  `name` function L20-22 — `(&self) -> &str`
-  `description` function L24-34 — `(&self) -> &str`
-  `is_read_only` function L36-38 — `(&self) -> bool`
-  `parameters_schema` function L40-104 — `(&self) -> Value`
-  `execute` function L106-204 — `(&self, ctx: &ToolContext, params: Value) -> Result<ToolOutput, EngineError>`
-  `has_rg` function L207-209 — `() -> bool`
-  `run_rg` function L212-289 — `( cwd: &std::path::Path, pattern: &str, path: &str, glob: Option<&str>, file_typ...`
-  `run_grep_fallback` function L291-327 — `( cwd: &std::path::Path, pattern: &str, path: &str, case_insensitive: bool, outp...`
-  `tests` module L330-494 — `-`
-  `test_ctx` function L336-339 — `(dir: &std::path::Path) -> ToolContext`
-  `grep_finds_matches` function L342-360 — `()`
-  `grep_no_matches` function L363-377 — `()`
-  `grep_case_insensitive` function L380-394 — `()`
-  `grep_with_glob` function L397-412 — `()`
-  `grep_content_mode` function L415-433 — `()`
-  `grep_files_with_matches_mode` function L436-455 — `()`
-  `grep_head_limit` function L458-481 — `()`
-  `schema_is_valid` function L484-493 — `()`

#### crates/arawn-engine/src/tools/memory_search.rs

- pub `MemorySearchTool` struct L17-20 — `{ memory: Arc<MemoryManager>, embedder: Option<Arc<dyn Embedder>> }` — Tool that searches the knowledge base using composite retrieval:
- pub `new` function L23-25 — `(memory: Arc<MemoryManager>, embedder: Option<Arc<dyn Embedder>>) -> Self`
-  `MemorySearchTool` type L22-26 — `= MemorySearchTool`
-  `MemorySearchTool` type L29-267 — `impl Tool for MemorySearchTool`
-  `name` function L30-32 — `(&self) -> &str`
-  `description` function L34-38 — `(&self) -> &str`
-  `is_read_only` function L40-42 — `(&self) -> bool`
-  `parameters_schema` function L44-78 — `(&self) -> Value`
-  `execute` function L80-266 — `(&self, _ctx: &ToolContext, params: Value) -> Result<ToolOutput, EngineError>`
-  `ScoredEntity` struct L269-276 — `{ entity: Entity, fts_score: f32, semantic_score: f32, confidence: f32, source: ...`
-  `ScoredEntity` type L278-286 — `= ScoredEntity`
-  `composite` function L279-281 — `(&self) -> f32`
-  `compute_composite` function L283-285 — `(&mut self)`
-  `ScoredEntity` type L288-292 — `impl Default for ScoredEntity`
-  `default` function L289-291 — `() -> Self`
-  `tests` module L295-406 — `-`
-  `setup` function L302-309 — `() -> (TempDir, Arc<MemoryManager>, ToolContext)`
-  `populate` function L311-333 — `(mgr: &MemoryManager)`
-  `search_fts_both_tiers` function L336-349 — `()`
-  `search_with_type_filter` function L352-364 — `()`
-  `search_global_only` function L367-378 — `()`
-  `search_no_results` function L381-391 — `()`
-  `search_with_tags` function L394-405 — `()`

#### crates/arawn-engine/src/tools/memory_store.rs

- pub `MemoryStoreTool` struct L17-20 — `{ memory: Arc<MemoryManager>, embedder: Option<Arc<dyn Embedder>> }` — Tool that stores knowledge in the KB with search-before-create deduplication.
- pub `new` function L23-25 — `(memory: Arc<MemoryManager>, embedder: Option<Arc<dyn Embedder>>) -> Self`
-  `MemoryStoreTool` type L22-26 — `= MemoryStoreTool`
-  `MemoryStoreTool` type L29-202 — `impl Tool for MemoryStoreTool`
-  `name` function L30-32 — `(&self) -> &str`
-  `description` function L34-45 — `(&self) -> &str`
-  `parameters_schema` function L47-77 — `(&self) -> Value`
-  `execute` function L79-201 — `(&self, ctx: &ToolContext, params: Value) -> Result<ToolOutput, EngineError>`
-  `tests` module L205-314 — `-`
-  `setup` function L212-221 — `() -> (TempDir, Arc<MemoryManager>, ToolContext)`
-  `store_new_fact` function L224-236 — `()`
-  `store_preference_goes_global` function L239-249 — `()`
-  `store_decision_goes_workstream` function L252-262 — `()`
-  `store_reinforces_duplicate` function L265-280 — `()`
-  `store_with_tags` function L283-296 — `()`
-  `store_with_explicit_scope_override` function L299-313 — `()`

#### crates/arawn-engine/src/tools/mod.rs

- pub `agent` module L1 — `-`
- pub `ask_user` module L2 — `-`
- pub `enter_plan_mode` module L3 — `-`
- pub `exit_plan_mode` module L4 — `-`
- pub `file_edit` module L5 — `-`
- pub `file_read` module L6 — `-`
- pub `file_write` module L7 — `-`
- pub `glob` module L8 — `-`
- pub `grep` module L9 — `-`
- pub `memory_search` module L10 — `-`
- pub `memory_store` module L11 — `-`
- pub `shell` module L12 — `-`
- pub `skill` module L13 — `-`
- pub `sleep` module L14 — `-`
- pub `task_list` module L15 — `-`
- pub `task_output` module L16 — `-`
- pub `task_stop` module L17 — `-`
- pub `think` module L18 — `-`
- pub `web_fetch` module L19 — `-`
- pub `web_search` module L20 — `-`

#### crates/arawn-engine/src/tools/shell.rs

- pub `ShellTool` struct L23-28 — `{ network_tools: Vec<String>, bg_manager: Option<Arc<BackgroundTaskManager>> }` — Execute a shell command within an OS-level sandbox.
- pub `with_network_tools` function L43-48 — `(network_tools: Vec<String>) -> Self` — Create a ShellTool with the given list of network-allowed tool binaries.
- pub `with_background_manager` function L51-54 — `(mut self, mgr: Arc<BackgroundTaskManager>) -> Self` — Attach a background task manager for `run_in_background` support.
-  `DEFAULT_TIMEOUT_MS` variable L30 — `: u64`
-  `ShellTool` type L32-39 — `impl Default for ShellTool`
-  `default` function L33-38 — `() -> Self`
-  `ShellTool` type L41-198 — `= ShellTool`
-  `spawn_background` function L57-197 — `( &self, command: &str, working_dir: &std::path::Path, ) -> Result<ToolOutput, E...` — Spawn a shell command as a background task.
-  `sensitive_deny_read_paths` function L202-247 — `() -> Vec<String>` — Build the list of sensitive paths that should be denied for reading.
-  `command_needs_network` function L250-269 — `(command: &str, network_tools: &[String]) -> bool` — Check if a command invokes any tool that needs network access.
-  `build_sandbox_config` function L272-321 — `( command: &str, working_dir: &std::path::Path, network_tools: &[String], ) -> S...` — Build a sandbox config for executing a command in the given working directory.
-  `ShellTool` type L324-400 — `impl Tool for ShellTool`
-  `name` function L325-327 — `(&self) -> &str`
-  `description` function L329-344 — `(&self) -> &str`
-  `parameters_schema` function L346-365 — `(&self) -> Value`
-  `execute` function L367-399 — `(&self, ctx: &ToolContext, params: Value) -> Result<ToolOutput, EngineError>`
-  `SandboxExecError` enum L402-407 — `Unavailable | Tool`
-  `execute_sandboxed` function L409-496 — `( command: &str, working_dir: &std::path::Path, timeout_ms: u64, network_tools: ...`
-  `execute_unsandboxed` function L498-542 — `( command: &str, working_dir: &std::path::Path, timeout_ms: u64, ) -> Result<Too...`
-  `tests` module L545-844 — `-`
-  `test_ctx` function L552-555 — `() -> ToolContext`
-  `test_ctx_in` function L557-560 — `(dir: &std::path::Path) -> ToolContext`
-  `shell_echo` function L564-572 — `()`
-  `shell_nonzero_exit` function L576-584 — `()`
-  `shell_timeout` function L588-599 — `()`
-  `shell_missing_command` function L603-607 — `()`
-  `shell_schema_is_valid` function L610-615 — `()`
-  `sensitive_paths_includes_ssh` function L618-621 — `()`
-  `sensitive_paths_includes_aws` function L624-627 — `()`
-  `sandbox_config_allows_working_dir_and_tmp` function L630-641 — `()`
-  `network_detection_recognizes_tools` function L644-651 — `()`
-  `network_detection_blocks_unknown` function L654-659 — `()`
-  `network_detection_empty_list_blocks_all` function L662-665 — `()`
-  `sandbox_write_inside_allowed` function L671-690 — `()`
-  `sandbox_mkdir_inside_allowed` function L694-715 — `()`
-  `sandbox_unlink_inside_allowed` function L719-744 — `()`
-  `sandbox_build_tool_workflow` function L748-770 — `()`
-  `sandbox_write_outside_blocked` function L774-811 — `()`
-  `sandbox_read_sensitive_path_blocked` function L815-843 — `()`

#### crates/arawn-engine/src/tools/skill.rs

- pub `SkillTool` struct L16-18 — `{ registry: Arc<SkillRegistry> }` — Tool that executes skills (reusable prompt-based workflows).
- pub `new` function L21-23 — `(registry: Arc<SkillRegistry>) -> Self`
-  `SkillTool` type L20-24 — `= SkillTool`
-  `SkillTool` type L27-99 — `impl Tool for SkillTool`
-  `name` function L28-30 — `(&self) -> &str`
-  `description` function L32-37 — `(&self) -> &str`
-  `parameters_schema` function L39-54 — `(&self) -> Value`
-  `execute` function L56-93 — `(&self, _ctx: &ToolContext, params: Value) -> Result<ToolOutput, EngineError>`
-  `is_read_only` function L95-98 — `(&self) -> bool`
-  `tests` module L102-207 — `-`
-  `make_registry` function L106-139 — `() -> Arc<SkillRegistry>`
-  `ctx` function L141-144 — `() -> ToolContext`
-  `execute_existing_skill` function L147-155 — `()`
-  `execute_with_args` function L158-170 — `()`
-  `execute_missing_skill` function L173-183 — `()`
-  `execute_missing_param` function L186-190 — `()`
-  `tool_metadata` function L193-198 — `()`
-  `schema_has_required_skill` function L201-206 — `()`

#### crates/arawn-engine/src/tools/sleep.rs

- pub `SleepTool` struct L15 — `-` — Waits for a specified duration.
-  `MAX_SLEEP_SECS` variable L11 — `: u64` — Maximum sleep duration in seconds.
-  `SleepTool` type L18-70 — `impl Tool for SleepTool`
-  `name` function L19-21 — `(&self) -> &str`
-  `description` function L23-28 — `(&self) -> &str`
-  `is_read_only` function L30-32 — `(&self) -> bool`
-  `parameters_schema` function L34-45 — `(&self) -> Value`
-  `execute` function L47-69 — `(&self, _ctx: &ToolContext, params: Value) -> Result<ToolOutput, EngineError>`
-  `tests` module L73-140 — `-`
-  `test_ctx` function L79-82 — `() -> ToolContext`
-  `schema_is_valid` function L85-92 — `()`
-  `is_read_only` function L95-97 — `()`
-  `sleep_short_duration` function L100-112 — `()`
-  `sleep_negative_errors` function L115-123 — `()`
-  `sleep_clamped` function L126-139 — `()`

#### crates/arawn-engine/src/tools/task_list.rs

- pub `TaskStatus` enum L16-20 — `Pending | InProgress | Completed` — Session-scoped task status.
- pub `SessionTask` struct L34-42 — `{ id: String, subject: String, description: Option<String>, active_form: Option<...` — A single session-scoped task.
- pub `SessionTaskStore` struct L47-50 — `{ tasks: Arc<RwLock<HashMap<String, SessionTask>>>, order: Arc<RwLock<Vec<String...` — Shared in-memory task store for a session.
- pub `new` function L53-55 — `() -> Self`
- pub `TaskCreateTool` struct L131-133 — `{ store: SessionTaskStore }` — Creates a new session-scoped task for tracking work within the current session.
- pub `new` function L136-138 — `(store: SessionTaskStore) -> Self`
- pub `TaskUpdateTool` struct L210-212 — `{ store: SessionTaskStore }` — Updates a session task's status or details.
- pub `new` function L215-217 — `(store: SessionTaskStore) -> Self`
- pub `TaskListTool` struct L338-340 — `{ store: SessionTaskStore }` — Lists all session tasks with their status.
- pub `new` function L343-345 — `(store: SessionTaskStore) -> Self`
- pub `TaskGetTool` struct L401-403 — `{ store: SessionTaskStore }` — Gets full details of a session task by ID.
- pub `new` function L406-408 — `(store: SessionTaskStore) -> Self`
-  `TaskStatus` type L22-30 — `= TaskStatus`
-  `fmt` function L23-29 — `(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result`
-  `SessionTaskStore` type L52-117 — `= SessionTaskStore`
-  `create` function L57-74 — `( &self, subject: String, description: Option<String>, active_form: Option<Strin...`
-  `update` function L76-95 — `(&self, id: &str, updates: TaskUpdates) -> Option<SessionTask>`
-  `get` function L97-99 — `(&self, id: &str) -> Option<SessionTask>`
-  `delete` function L101-107 — `(&self, id: &str) -> bool`
-  `list` function L109-116 — `(&self) -> Vec<SessionTask>`
-  `TaskUpdates` struct L119-124 — `{ status: Option<TaskStatus>, subject: Option<String>, description: Option<Strin...`
-  `TaskCreateTool` type L135-139 — `= TaskCreateTool`
-  `TaskCreateTool` type L142-203 — `impl Tool for TaskCreateTool`
-  `name` function L143-145 — `(&self) -> &str`
-  `description` function L147-158 — `(&self) -> &str`
-  `parameters_schema` function L160-179 — `(&self) -> Value`
-  `execute` function L181-202 — `(&self, _ctx: &ToolContext, params: Value) -> Result<ToolOutput, EngineError>`
-  `TaskUpdateTool` type L214-218 — `= TaskUpdateTool`
-  `TaskUpdateTool` type L221-331 — `impl Tool for TaskUpdateTool`
-  `name` function L222-224 — `(&self) -> &str`
-  `description` function L226-235 — `(&self) -> &str`
-  `parameters_schema` function L237-265 — `(&self) -> Value`
-  `execute` function L267-330 — `(&self, _ctx: &ToolContext, params: Value) -> Result<ToolOutput, EngineError>`
-  `TaskListTool` type L342-346 — `= TaskListTool`
-  `TaskListTool` type L349-394 — `impl Tool for TaskListTool`
-  `name` function L350-352 — `(&self) -> &str`
-  `description` function L354-362 — `(&self) -> &str`
-  `is_read_only` function L364-366 — `(&self) -> bool`
-  `parameters_schema` function L368-373 — `(&self) -> Value`
-  `execute` function L375-393 — `(&self, _ctx: &ToolContext, _params: Value) -> Result<ToolOutput, EngineError>`
-  `TaskGetTool` type L405-409 — `= TaskGetTool`
-  `TaskGetTool` type L412-455 — `impl Tool for TaskGetTool`
-  `name` function L413-415 — `(&self) -> &str`
-  `description` function L417-423 — `(&self) -> &str`
-  `is_read_only` function L425-427 — `(&self) -> bool`
-  `parameters_schema` function L429-440 — `(&self) -> Value`
-  `execute` function L442-454 — `(&self, _ctx: &ToolContext, params: Value) -> Result<ToolOutput, EngineError>`
-  `tests` module L458-801 — `-`
-  `test_ctx` function L464-467 — `() -> ToolContext`
-  `store_create_and_list` function L470-480 — `()`
-  `store_update_status` function L483-498 — `()`
-  `store_update_subject_and_description` function L501-518 — `()`
-  `store_delete` function L521-526 — `()`
-  `store_delete_nonexistent` function L529-532 — `()`
-  `store_update_nonexistent` function L535-550 — `()`
-  `store_preserves_order` function L553-561 — `()`
-  `task_create_tool` function L564-581 — `()`
-  `task_create_with_active_form` function L584-600 — `()`
-  `task_update_status` function L603-616 — `()`
-  `task_update_delete` function L619-633 — `()`
-  `task_update_invalid_status` function L636-647 — `()`
-  `task_update_no_fields_errors` function L650-659 — `()`
-  `task_update_not_found` function L662-674 — `()`
-  `task_list_empty` function L677-684 — `()`
-  `task_list_with_tasks` function L687-707 — `()`
-  `full_lifecycle` function L710-745 — `()`
-  `schemas_are_valid` function L748-767 — `()`
-  `task_get_found` function L770-785 — `()`
-  `task_get_not_found` function L788-800 — `()`

#### crates/arawn-engine/src/tools/task_output.rs

- pub `TaskOutputTool` struct L14-16 — `{ bg_manager: Arc<BackgroundTaskManager> }` — Read the output and status of a background task.
- pub `new` function L19-21 — `(bg_manager: Arc<BackgroundTaskManager>) -> Self`
-  `TaskOutputTool` type L18-22 — `= TaskOutputTool`
-  `TaskOutputTool` type L25-135 — `impl Tool for TaskOutputTool`
-  `name` function L26-28 — `(&self) -> &str`
-  `description` function L30-34 — `(&self) -> &str`
-  `is_read_only` function L36-38 — `(&self) -> bool`
-  `parameters_schema` function L40-59 — `(&self) -> Value`
-  `execute` function L61-134 — `(&self, _ctx: &ToolContext, params: Value) -> Result<ToolOutput, EngineError>`
-  `tests` module L138-213 — `-`
-  `test_ctx` function L145-148 — `() -> ToolContext`
-  `unknown_task_returns_error` function L151-160 — `()`
-  `completed_task_returns_output` function L163-188 — `()`
-  `running_task_non_blocking` function L191-212 — `()`

#### crates/arawn-engine/src/tools/task_stop.rs

- pub `TaskStopTool` struct L13-15 — `{ bg_manager: Arc<BackgroundTaskManager> }` — Stop a running background task.
- pub `new` function L18-20 — `(bg_manager: Arc<BackgroundTaskManager>) -> Self`
-  `TaskStopTool` type L17-21 — `= TaskStopTool`
-  `TaskStopTool` type L24-76 — `impl Tool for TaskStopTool`
-  `name` function L25-27 — `(&self) -> &str`
-  `description` function L29-32 — `(&self) -> &str`
-  `is_read_only` function L34-36 — `(&self) -> bool`
-  `parameters_schema` function L38-49 — `(&self) -> Value`
-  `execute` function L51-75 — `(&self, _ctx: &ToolContext, params: Value) -> Result<ToolOutput, EngineError>`
-  `tests` module L79-154 — `-`
-  `test_ctx` function L86-89 — `() -> ToolContext`
-  `stop_unknown_task` function L92-101 — `()`
-  `stop_running_task` function L104-129 — `()`
-  `stop_already_completed_task` function L132-153 — `()`

#### crates/arawn-engine/src/tools/think.rs

- pub `ThinkTool` struct L10 — `-` — A no-op reasoning scratchpad tool.
-  `ThinkTool` type L13-52 — `impl Tool for ThinkTool`
-  `name` function L14-16 — `(&self) -> &str`
-  `description` function L18-25 — `(&self) -> &str`
-  `is_read_only` function L27-29 — `(&self) -> bool`
-  `parameters_schema` function L31-42 — `(&self) -> Value`
-  `execute` function L44-51 — `(&self, _ctx: &ToolContext, params: Value) -> Result<ToolOutput, EngineError>`
-  `tests` module L55-92 — `-`
-  `test_ctx` function L61-64 — `() -> ToolContext`
-  `think_returns_thought` function L67-75 — `()`
-  `think_with_empty_thought` function L78-83 — `()`
-  `think_schema_is_valid` function L86-91 — `()`

#### crates/arawn-engine/src/tools/web_fetch.rs

- pub `WebFetchTool` struct L39-41 — `{ cache: Arc<Mutex<LruCache<String, CacheEntry>>> }` — Fetches content from a URL, converts HTML to markdown, caches results,
- pub `new` function L44-50 — `() -> Self`
-  `CACHE_TTL` variable L16 — `: Duration` — Cache TTL: 15 minutes.
-  `CACHE_MAX_ENTRIES` variable L19 — `: usize` — Maximum cache entries.
-  `MAX_CONTENT_BYTES` variable L22 — `: usize` — Max content size before truncation (100KB).
-  `CacheEntry` struct L25-29 — `{ content: String, content_type: String, fetched_at: Instant }` — Cached fetch result.
-  `CacheEntry` type L31-35 — `= CacheEntry`
-  `is_expired` function L32-34 — `(&self) -> bool`
-  `WebFetchTool` type L43-51 — `= WebFetchTool`
-  `WebFetchTool` type L53-57 — `impl Default for WebFetchTool`
-  `default` function L54-56 — `() -> Self`
-  `WebFetchTool` type L60-167 — `impl Tool for WebFetchTool`
-  `name` function L61-63 — `(&self) -> &str`
-  `description` function L65-71 — `(&self) -> &str`
-  `parameters_schema` function L73-88 — `(&self) -> Value`
-  `execute` function L90-166 — `(&self, ctx: &ToolContext, params: Value) -> Result<ToolOutput, EngineError>`
-  `process_content` function L170-183 — `(body: &str, content_type: &str) -> String` — Convert HTML to markdown, or return non-HTML as-is.
-  `html_to_markdown` function L186-191 — `(html: &str) -> String` — Convert HTML to markdown using htmd (Turndown-equivalent).
-  `strip_html_tags` function L194-225 — `(html: &str) -> String` — Fallback: simple HTML tag stripper (used if htmd fails).
-  `finish` function L228-239 — `( ctx: &ToolContext, prompt: &str, url: &str, text: String, ) -> Result<ToolOutp...` — If we have an LLM and a prompt, summarize.
-  `summarize_with_llm` function L241-284 — `( llm: &Arc<dyn arawn_llm::LlmClient>, model: &str, prompt: &str, url: &str, con...`
-  `tests` module L287-521 — `-`
-  `test_ctx` function L296-299 — `() -> ToolContext`
-  `test_ctx_with_mock` function L301-307 — `(responses: Vec<MockResponse>) -> (ToolContext, Arc<MockLlmClient>)`
-  `html_to_markdown_headings` function L312-316 — `()`
-  `html_to_markdown_links` function L319-323 — `()`
-  `html_to_markdown_lists` function L326-330 — `()`
-  `html_to_markdown_code` function L333-336 — `()`
-  `non_html_passthrough` function L339-342 — `()`
-  `strip_tags_basic` function L347-349 — `()`
-  `strip_tags_collapses_whitespace` function L352-357 — `()`
-  `cache_entry_expiry` function L362-376 — `()`
-  `cache_stores_and_retrieves` function L379-398 — `()`
-  `large_content_truncated` function L403-408 — `()`
-  `schema_is_valid` function L413-422 — `()`
-  `http_upgraded_description` function L425-428 — `()`
-  `summarize_with_mock_llm` function L433-451 — `()`
-  `summarize_sends_correct_request_shape` function L454-469 — `()`
-  `execute_without_llm_returns_raw_text` function L472-475 — `()`
-  `summarize_empty_content` function L478-493 — `()`
-  `summarize_multipart_response` function L496-520 — `()`

#### crates/arawn-engine/src/tools/web_search.rs

- pub `WebSearchTool` struct L9 — `-` — Searches the web and returns results to inform responses.
-  `WebSearchTool` type L12-138 — `impl Tool for WebSearchTool`
-  `name` function L13-15 — `(&self) -> &str`
-  `description` function L17-24 — `(&self) -> &str`
-  `is_read_only` function L26-28 — `(&self) -> bool`
-  `parameters_schema` function L30-52 — `(&self) -> Value`
-  `execute` function L54-137 — `(&self, _ctx: &ToolContext, params: Value) -> Result<ToolOutput, EngineError>`
-  `SearchResult` struct L140-144 — `{ title: String, url: String, snippet: String }`
-  `parse_ddg_results` function L146-169 — `(html: &str, max: usize) -> Vec<SearchResult>`
-  `extract_tag_content` function L171-179 — `(html: &str, after: &str) -> String`
-  `extract_href` function L181-194 — `(html: &str) -> String`
-  `extract_after_class` function L196-208 — `(html: &str, class: &str) -> String`
-  `strip_tags` function L210-222 — `(html: &str) -> String`
-  `urlencod` function L224-232 — `(s: &str) -> String`
-  `urldecod` function L234-252 — `(s: &str) -> String`
-  `tests` module L255-394 — `-`
-  `urlencod_spaces` function L259-261 — `()`
-  `urlencod_special_chars` function L264-266 — `()`
-  `urldecod_percent` function L269-271 — `()`
-  `urldecod_stops_at_ampersand` function L274-276 — `()`
-  `urldecod_plus_to_space` function L279-281 — `()`
-  `strip_tags_removes_html` function L284-286 — `()`
-  `strip_tags_empty` function L289-291 — `()`
-  `schema_is_valid` function L294-303 — `()`
-  `parse_ddg_results_empty_html` function L306-309 — `()`
-  `parse_ddg_results_no_results` function L312-316 — `()`
-  `parse_ddg_results_respects_max` function L319-330 — `()`
-  `parse_ddg_results_extracts_fields` function L333-343 — `()`
-  `blocked_domains_filter` function L346-371 — `()`
-  `allowed_domains_builds_site_clause` function L374-387 — `()`
-  `is_read_only` function L390-393 — `()`

### crates/arawn-llm/src

> *Semantic summary to be generated by AI agent.*

#### crates/arawn-llm/src/anthropic.rs

- pub `AnthropicClient` struct L18-21 — `{ http: Client, api_key: String }` — Client for Anthropic's Claude API (Messages API).
- pub `new` function L24-29 — `(api_key: impl Into<String>) -> Self`
- pub `from_env` function L31-35 — `() -> Result<Self, LlmError>`
-  `API_URL` variable L14 — `: &str`
-  `API_VERSION` variable L15 — `: &str`
-  `AnthropicClient` type L23-58 — `= AnthropicClient`
-  `build_request_body` function L37-57 — `(&self, request: &ChatRequest) -> Value`
-  `AnthropicClient` type L61-201 — `impl LlmClient for AnthropicClient`
-  `stream` function L62-200 — `( &self, request: ChatRequest, ) -> Result<Pin<Box<dyn Stream<Item = Result<Chat...`
-  `build_messages` function L207-269 — `(messages: &[ChatMessage]) -> Vec<Value>` — Convert arawn messages to Anthropic format.
-  `merge_consecutive_roles` function L273-309 — `(messages: &mut Vec<Value>)` — Merge consecutive messages with the same role into a single message
-  `normalize_content` function L312-318 — `(content: &Value) -> Vec<Value>` — Normalize content to a Vec<Value> of content blocks.
-  `build_tools` function L321-332 — `(tools: &[ToolDefinition]) -> Vec<Value>` — Convert tool definitions to Anthropic format.
-  `tests` module L335-466 — `-`
-  `user_msg` function L339-346 — `(text: &str) -> ChatMessage`
-  `assistant_text` function L348-355 — `(text: &str) -> ChatMessage`
-  `assistant_with_tool` function L357-368 — `(text: &str, tool_id: &str, tool_name: &str, args: Value) -> ChatMessage`
-  `tool_result` function L370-378 — `(tool_use_id: &str, content: &str) -> ChatMessage`
-  `simple_conversation` function L381-390 — `()`
-  `tool_call_with_result` function L393-416 — `()`
-  `multi_turn_with_tools` function L419-442 — `()`
-  `consecutive_tool_results_merged` function L445-465 — `()`

#### crates/arawn-llm/src/client.rs

- pub `LlmClient` interface L12-17 — `{ fn stream() }` — Provider-agnostic LLM client trait.

#### crates/arawn-llm/src/error.rs

- pub `LlmError` enum L4-31 — `Api | Auth | ModelNotFound | RateLimited | ServerError | Stream | Config | Reque...`
- pub `is_retryable` function L35-47 — `(&self) -> bool` — Returns true if this error is transient and the request should be retried.
- pub `from_status` function L50-62 — `(status: u16, body: String) -> Self` — Create from an HTTP status code + body.
- pub `user_message` function L65-117 — `(&self) -> String` — Return a user-facing error message with actionable guidance.
-  `LlmError` type L33-118 — `= LlmError`
-  `extract_api_message` function L122-129 — `(body: &str) -> Option<String>` — Try to extract a clean message from a JSON error body.
-  `tests` module L132-206 — `-`
-  `from_status_401_is_auth` function L136-141 — `()`
-  `from_status_403_is_auth` function L144-147 — `()`
-  `from_status_404_is_model_not_found` function L150-158 — `()`
-  `from_status_429_is_rate_limited` function L161-166 — `()`
-  `from_status_500_is_server_error` function L169-174 — `()`
-  `from_status_400_is_api_error` function L177-181 — `()`
-  `extract_message_from_json_body` function L184-188 — `()`
-  `extract_message_from_plain_text_returns_none` function L191-193 — `()`
-  `config_error_user_message` function L196-199 — `()`
-  `stream_error_user_message` function L202-205 — `()`

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

#### crates/arawn-llm/src/mock.rs

- pub `MockResponse` enum L12-23 — `Text | ToolCall | Raw` — A scripted response for one LLM turn.
- pub `text` function L26-28 — `(text: impl Into<String>) -> Self`
- pub `tool_call` function L30-40 — `( id: impl Into<String>, name: impl Into<String>, arguments: impl Into<String>, ...`
- pub `raw` function L42-44 — `(chunks: Vec<ChatChunk>) -> Self`
- pub `MockLlmClient` struct L69-72 — `{ responses: Mutex<Vec<MockResponse>>, call_count: Mutex<usize> }` — Mock LLM client that returns pre-scripted responses.
- pub `new` function L75-80 — `(responses: Vec<MockResponse>) -> Self`
- pub `call_count` function L83-85 — `(&self) -> usize` — How many times `stream()` has been called.
-  `MockResponse` type L25-64 — `= MockResponse`
-  `into_chunks` function L46-63 — `(self) -> Vec<ChatChunk>`
-  `MockLlmClient` type L74-86 — `= MockLlmClient`
-  `MockLlmClient` type L89-110 — `impl LlmClient for MockLlmClient`
-  `stream` function L90-109 — `( &self, _request: ChatRequest, ) -> Result<Pin<Box<dyn futures::Stream<Item = R...`
-  `tests` module L113-219 — `-`
-  `mock_text_response` function L118-137 — `()`
-  `mock_tool_call_response` function L140-172 — `()`
-  `mock_multiple_responses_consumed_in_order` function L175-204 — `()`
-  `mock_panics_when_exhausted` function L208-218 — `()`

#### crates/arawn-llm/src/openai_compat.rs

- pub `OpenAICompatibleClient` struct L18-23 — `{ http: Client, base_url: String, api_key: Option<String>, provider_name: String...` — Generic client for any OpenAI-compatible API (Groq, Ollama, OpenAI, vLLM,
- pub `new` function L26-40 — `( base_url: impl Into<String>, api_key: Option<String>, provider_name: impl Into...`
- pub `groq` function L43-49 — `(api_key: impl Into<String>) -> Self` — Create a client for Groq.
- pub `groq_from_env` function L52-56 — `() -> Result<Self, LlmError>` — Create a client for Groq from the GROQ_API_KEY env var.
- pub `ollama` function L59-61 — `() -> Self` — Create a client for Ollama (local, no API key needed).
- pub `ollama_at` function L64-66 — `(base_url: impl Into<String>) -> Self` — Create a client for Ollama with a custom host/port.
- pub `openai` function L69-75 — `(api_key: impl Into<String>) -> Self` — Create a client for OpenAI.
- pub `openai_from_env` function L78-82 — `() -> Result<Self, LlmError>` — Create a client for OpenAI from the OPENAI_API_KEY env var.
- pub `from_config` function L85-113 — `( provider: &str, base_url: Option<&str>, api_key_env: &str, ) -> Result<Self, L...` — Create from explicit config values.
-  `OpenAICompatibleClient` type L25-139 — `= OpenAICompatibleClient`
-  `build_request_body` function L115-134 — `(&self, request: &ChatRequest) -> Value`
-  `completions_url` function L136-138 — `(&self) -> String`
-  `OpenAICompatibleClient` type L142-177 — `impl LlmClient for OpenAICompatibleClient`
-  `stream` function L143-176 — `( &self, request: ChatRequest, ) -> Result<Pin<Box<dyn Stream<Item = Result<Chat...`
-  `SseParser` struct L181-186 — `{ inner: S, buffer: String, pending_chunks: Vec<ChatChunk>, provider: String }`
-  `new` function L189-196 — `(inner: S, provider: String) -> Self`
-  `Item` type L203 — `= Result<ChatChunk, LlmError>`
-  `poll_next` function L205-241 — `( mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>, ) -> std::task::Pol...`
-  `try_parse_buffer` function L245-291 — `(&mut self) -> Option<Result<ChatChunk, LlmError>>`
-  `parse_stream_chunk` function L294-339 — `(chunk: &StreamChunk) -> Vec<ChatChunk>`
-  `build_messages` function L343-406 — `(system_prompt: &Option<String>, messages: &[ChatMessage]) -> Vec<Value>`
-  `build_tools` function L408-422 — `(tools: &[ToolDefinition]) -> Vec<Value>`
-  `ApiErrorResponse` struct L427-429 — `{ error: Option<ApiError> }`
-  `ApiError` struct L432-436 — `{ message: String, code: Option<String> }`
-  `StreamChunk` struct L439-444 — `{ choices: Vec<StreamChoice>, usage: Option<StreamUsage> }`
-  `StreamChoice` struct L447-449 — `{ delta: StreamDelta }`
-  `StreamDelta` struct L452-455 — `{ content: Option<String>, tool_calls: Option<Vec<StreamToolCall>> }`
-  `StreamToolCall` struct L458-461 — `{ id: Option<String>, function: Option<StreamFunction> }`
-  `StreamFunction` struct L464-467 — `{ name: Option<String>, arguments: Option<String> }`
-  `StreamUsage` struct L470-473 — `{ prompt_tokens: u32, completion_tokens: u32 }`
-  `tests` module L476-618 — `-`
-  `groq_convenience_constructor` function L481-486 — `()`
-  `ollama_convenience_constructor` function L489-494 — `()`
-  `openai_convenience_constructor` function L497-501 — `()`
-  `custom_base_url` function L504-511 — `()`
-  `from_config_known_providers` function L514-518 — `()`
-  `from_config_custom_url_override` function L521-528 — `()`
-  `build_messages_with_system_prompt` function L531-544 — `()`
-  `parse_text_delta` function L547-560 — `()`
-  `parse_tool_use_start` function L563-582 — `()`
-  `parse_usage` function L585-596 — `()`
-  `no_auth_header_when_no_api_key` function L599-617 — `()`

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

### crates/arawn-mcp/src

> *Semantic summary to be generated by AI agent.*

#### crates/arawn-mcp/src/adapter.rs

- pub `McpToolAdapter` struct L16-25 — `{ arawn_name: String, mcp_name: String, mcp_tool: McpTool, peer: Arc<Peer<RoleCl...` — An arawn Tool backed by an MCP server tool.
- pub `new` function L28-40 — `(server_name: &str, mcp_tool: McpTool, peer: Arc<Peer<RoleClient>>) -> Self` — McpToolAdapter — wraps an MCP tool as an arawn Tool impl.
- pub `tool_name` function L43-45 — `(&self) -> &str` — Get the arawn tool name (for logging before registration).
-  `McpToolAdapter` type L27-46 — `= McpToolAdapter` — McpToolAdapter — wraps an MCP tool as an arawn Tool impl.
-  `McpToolAdapter` type L49-122 — `impl Tool for McpToolAdapter` — McpToolAdapter — wraps an MCP tool as an arawn Tool impl.
-  `name` function L50-52 — `(&self) -> &str` — McpToolAdapter — wraps an MCP tool as an arawn Tool impl.
-  `description` function L54-59 — `(&self) -> &str` — McpToolAdapter — wraps an MCP tool as an arawn Tool impl.
-  `parameters_schema` function L61-68 — `(&self) -> Value` — McpToolAdapter — wraps an MCP tool as an arawn Tool impl.
-  `is_read_only` function L70-76 — `(&self) -> bool` — McpToolAdapter — wraps an MCP tool as an arawn Tool impl.
-  `execute` function L78-121 — `(&self, _ctx: &ToolContext, params: Value) -> Result<ToolOutput, EngineError>` — McpToolAdapter — wraps an MCP tool as an arawn Tool impl.
-  `normalize_name` function L125-135 — `(name: &str) -> String` — Normalize a name for use in tool naming — replace non-alphanumeric chars with _
-  `tests` module L138-153 — `-` — McpToolAdapter — wraps an MCP tool as an arawn Tool impl.
-  `normalize_simple` function L142-145 — `()` — McpToolAdapter — wraps an MCP tool as an arawn Tool impl.
-  `normalize_special_chars` function L148-152 — `()` — McpToolAdapter — wraps an MCP tool as an arawn Tool impl.

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
- pub `new` function L45-49 — `() -> Self` — registers them in the ToolRegistry, and handles reconnection.
- pub `connect_all` function L52-64 — `( &mut self, configs: &[McpServerConfig], registry: &Arc<ToolRegistry>, )` — Connect to all enabled servers and discover their tools.
- pub `connect_server` function L67-105 — `( &mut self, config: &McpServerConfig, registry: &Arc<ToolRegistry>, )` — Connect to a single MCP server.
- pub `disconnect_server` function L108-119 — `(&mut self, name: &str, registry: &Arc<ToolRegistry>)` — Disconnect a server and unregister its tools.
- pub `sync_servers` function L122-146 — `( &mut self, configs: &[McpServerConfig], registry: &Arc<ToolRegistry>, )` — Diff current servers against a new config and connect/disconnect as needed.
- pub `reconnect` function L149-196 — `( &mut self, server_name: &str, registry: &Arc<ToolRegistry>, ) -> bool` — Attempt to reconnect a failed server with exponential backoff.
- pub `connected_servers` function L199-201 — `(&self) -> Vec<&str>` — Get the names of all connected servers.
- pub `tool_count` function L204-206 — `(&self) -> usize` — Get tool count across all servers.
- pub `system_prompt` function L209-248 — `(&self) -> String` — Generate a system prompt section describing connected MCP servers and their tools.
-  `ArawnClientHandler` struct L19 — `-` — Handler for MCP client notifications.
-  `ArawnClientHandler` type L21-28 — `impl ClientHandler for ArawnClientHandler` — registers them in the ToolRegistry, and handles reconnection.
-  `get_info` function L22-27 — `(&self) -> ClientInfo` — registers them in the ToolRegistry, and handles reconnection.
-  `ConnectedServer` struct L31-37 — `{ config: McpServerConfig, _service: RunningService<RoleClient, ArawnClientHandl...` — State of a connected MCP server.
-  `McpManager` type L44-249 — `= McpManager` — registers them in the ToolRegistry, and handles reconnection.
-  `MAX_ATTEMPTS` variable L161 — `: u32` — registers them in the ToolRegistry, and handles reconnection.
-  `normalize_name` function L251-255 — `(name: &str) -> String` — registers them in the ToolRegistry, and handles reconnection.
-  `spawn_and_connect` function L258-286 — `( config: &McpServerConfig, ) -> Result< ( RunningService<RoleClient, ArawnClien...` — Spawn an MCP server process, connect via stdio, initialize, and discover tools.

### crates/arawn-memory/src

> *Semantic summary to be generated by AI agent.*

#### crates/arawn-memory/src/error.rs

- pub `MemoryError` enum L4-13 — `Storage | NotFound | Validation`

#### crates/arawn-memory/src/inject.rs

- pub `load_memories_for_injection` function L16-92 — `( memory: &MemoryManager, global_limit: Option<usize>, workstream_limit: Option<...` — Load relevant entities from both KB tiers and format as strings
-  `DEFAULT_GLOBAL_LIMIT` variable L8 — `: usize` — Default limits for entities injected per tier.
-  `DEFAULT_WORKSTREAM_LIMIT` variable L9 — `: usize` — Session injection — format KB entities for system prompt context.
-  `format_entity_line` function L94-115 — `(entity: &crate::types::Entity) -> String` — Session injection — format KB entities for system prompt context.
-  `tests` module L118-197 — `-` — Session injection — format KB entities for system prompt context.
-  `setup` function L123-128 — `() -> (TempDir, MemoryManager)` — Session injection — format KB entities for system prompt context.
-  `empty_kb_returns_empty` function L131-135 — `()` — Session injection — format KB entities for system prompt context.
-  `injects_global_preferences` function L138-152 — `()` — Session injection — format KB entities for system prompt context.
-  `injects_workstream_conventions` function L155-170 — `()` — Session injection — format KB entities for system prompt context.
-  `both_tiers_injected` function L173-184 — `()` — Session injection — format KB entities for system prompt context.
-  `reinforcement_shown` function L187-196 — `()` — Session injection — format KB entities for system prompt context.

#### crates/arawn-memory/src/lib.rs

- pub `error` module L6 — `-` — Provides graph-backed entity storage with FTS5 search, typed relations,
- pub `inject` module L7 — `-` — confidence scoring, tag support, and search-before-create deduplication.
- pub `manager` module L8 — `-` — confidence scoring, tag support, and search-before-create deduplication.
- pub `store` module L9 — `-` — confidence scoring, tag support, and search-before-create deduplication.
- pub `types` module L10 — `-` — confidence scoring, tag support, and search-before-create deduplication.
- pub `vector` module L11 — `-` — confidence scoring, tag support, and search-before-create deduplication.

#### crates/arawn-memory/src/manager.rs

- pub `MemoryManager` struct L17-24 — `{ global: Arc<MemoryStore>, workstream: Arc<MemoryStore>, vectors_enabled: bool ...` — Two-tier memory manager holding global and workstream knowledge bases.
- pub `open` function L30-66 — `(data_dir: &Path, ws_dir: &str, embedding_dims: Option<usize>) -> Result<Self, M...` — Open both KB tiers.
- pub `store_for` function L69-74 — `(&self, scope: Scope) -> &Arc<MemoryStore>` — Get the store for a given scope.
- pub `store_for_type` function L77-79 — `(&self, entity_type: EntityType) -> &Arc<MemoryStore>` — Get the store for a given entity type (uses default scope).
- pub `vectors_enabled` function L82-84 — `(&self) -> bool` — Whether vector storage is available.
- pub `try_open_memory` function L88-100 — `( data_dir: &Path, ws_dir: &str, embedding_dims: Option<usize>, ) -> Option<Arc<...` — Try to open a MemoryManager, returning None on failure (graceful degradation).
-  `MemoryManager` type L26-85 — `= MemoryManager` — It abstracts the two-tier scoping and routes entities to the appropriate store.
-  `tests` module L103-210 — `-` — It abstracts the two-tier scoping and routes entities to the appropriate store.
-  `setup` function L108-113 — `() -> (TempDir, MemoryManager)` — It abstracts the two-tier scoping and routes entities to the appropriate store.
-  `setup_with_vectors` function L115-120 — `() -> (TempDir, MemoryManager)` — It abstracts the two-tier scoping and routes entities to the appropriate store.
-  `opens_both_stores` function L123-132 — `()` — It abstracts the two-tier scoping and routes entities to the appropriate store.
-  `scope_routing` function L135-165 — `()` — It abstracts the two-tier scoping and routes entities to the appropriate store.
-  `vectors_disabled_by_default` function L168-171 — `()` — It abstracts the two-tier scoping and routes entities to the appropriate store.
-  `vectors_enabled_with_dims` function L174-185 — `()` — It abstracts the two-tier scoping and routes entities to the appropriate store.
-  `graceful_degradation` function L188-192 — `()` — It abstracts the two-tier scoping and routes entities to the appropriate store.
-  `stores_are_independent` function L195-209 — `()` — It abstracts the two-tier scoping and routes entities to the appropriate store.

#### crates/arawn-memory/src/store.rs

- pub `MemoryStore` struct L16-18 — `{ conn: Mutex<Connection> }` — Knowledge base store backed by SQLite with FTS5 and relations.
- pub `open` function L22-40 — `(path: &Path) -> Result<Self, MemoryError>` — Open or create a memory database at the given path.
- pub `in_memory` function L43-51 — `() -> Result<Self, MemoryError>` — Create an in-memory store (for testing).
- pub `insert_entity` function L115-142 — `(&self, entity: &Entity) -> Result<(), MemoryError>` — SQLite-backed knowledge base store with FTS5 search and relations.
- pub `get_entity` function L144-160 — `(&self, id: Uuid) -> Result<Option<Entity>, MemoryError>` — SQLite-backed knowledge base store with FTS5 search and relations.
- pub `update_entity` function L162-184 — `(&self, entity: &Entity) -> Result<(), MemoryError>` — SQLite-backed knowledge base store with FTS5 search and relations.
- pub `delete_entity` function L186-205 — `(&self, id: Uuid) -> Result<bool, MemoryError>` — SQLite-backed knowledge base store with FTS5 search and relations.
- pub `list_by_type` function L207-235 — `( &self, entity_type: EntityType, limit: usize, ) -> Result<Vec<Entity>, MemoryE...` — SQLite-backed knowledge base store with FTS5 search and relations.
- pub `count_by_type` function L237-247 — `(&self, entity_type: EntityType) -> Result<usize, MemoryError>` — SQLite-backed knowledge base store with FTS5 search and relations.
- pub `count_all` function L249-259 — `(&self) -> Result<usize, MemoryError>` — SQLite-backed knowledge base store with FTS5 search and relations.
- pub `search` function L263-288 — `(&self, query: &str, limit: usize) -> Result<Vec<Entity>, MemoryError>` — SQLite-backed knowledge base store with FTS5 search and relations.
- pub `search_by_type` function L290-323 — `( &self, query: &str, entity_type: EntityType, limit: usize, ) -> Result<Vec<Ent...` — SQLite-backed knowledge base store with FTS5 search and relations.
- pub `add_relation` function L327-346 — `( &self, source_id: Uuid, relation_type: RelationType, target_id: Uuid, ) -> Res...` — SQLite-backed knowledge base store with FTS5 search and relations.
- pub `get_relations` function L348-389 — `(&self, entity_id: Uuid) -> Result<Vec<Relation>, MemoryError>` — SQLite-backed knowledge base store with FTS5 search and relations.
- pub `get_neighbors` function L391-419 — `(&self, entity_id: Uuid) -> Result<Vec<(Uuid, RelationType)>, MemoryError>` — SQLite-backed knowledge base store with FTS5 search and relations.
- pub `delete_relation` function L421-439 — `( &self, source_id: Uuid, relation_type: RelationType, target_id: Uuid, ) -> Res...` — SQLite-backed knowledge base store with FTS5 search and relations.
- pub `store_fact` function L446-466 — `(&self, entity: &Entity) -> Result<StoreFactResult, MemoryError>` — Store a fact with search-before-create deduplication.
- pub `supersede_entity` function L495-520 — `( &self, old_id: Uuid, new_entity: &Entity, ) -> Result<StoreFactResult, MemoryE...` — Supersede an existing entity with a new one.
- pub `init_vectors` function L526-530 — `(&self, dims: usize) -> Result<(), MemoryError>` — Initialize vector storage with the given dimensions.
- pub `store_embedding` function L533-536 — `(&self, entity_id: Uuid, embedding: &[f32]) -> Result<(), MemoryError>` — Store an embedding for an entity.
- pub `search_similar` function L539-546 — `( &self, query_embedding: &[f32], limit: usize, ) -> Result<Vec<vector::Similari...` — Search for entities similar to a query embedding.
- pub `search_similar_filtered` function L549-557 — `( &self, query_embedding: &[f32], entity_ids: &[Uuid], limit: usize, ) -> Result...` — Search for entities similar to a query, filtered to a subset.
- pub `has_embedding` function L560-563 — `(&self, entity_id: Uuid) -> Result<bool, MemoryError>` — Check if an entity has a stored embedding.
- pub `count_embeddings` function L566-569 — `(&self) -> Result<usize, MemoryError>` — Count total stored embeddings.
- pub `search_by_tags` function L573-622 — `( &self, tags: &[String], limit: usize, ) -> Result<Vec<Entity>, MemoryError>` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `MemoryStore` type L20-623 — `= MemoryStore` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `migrate` function L53-111 — `(&self) -> Result<(), MemoryError>` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `reinforce_entity` function L469-492 — `(&self, entity_id: Uuid) -> Result<StoreFactResult, MemoryError>` — Reinforce an existing entity (increment count, update timestamp).
-  `row_to_entity` function L627-662 — `(row: &rusqlite::Row) -> Result<Entity, MemoryError>` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `OptionalExt` interface L665-667 — `{ fn optional() }` — Extension trait for optional query results.
-  `optional` function L670-676 — `(self) -> Result<Option<T>, rusqlite::Error>` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `tests` module L680-922 — `-` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `test_store` function L683-685 — `() -> MemoryStore` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `insert_and_get` function L688-696 — `()` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `get_nonexistent` function L699-702 — `()` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `update_entity` function L705-716 — `()` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `delete_entity` function L719-726 — `()` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `list_by_type` function L729-740 — `()` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `count_by_type` function L743-752 — `()` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `fts5_search` function L755-768 — `()` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `fts5_search_by_type` function L771-781 — `()` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `relations_crud` function L784-803 — `()` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `store_fact_insert` function L806-814 — `()` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `store_fact_reinforce` function L817-831 — `()` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `store_fact_reinforce_case_insensitive` function L834-846 — `()` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `supersede_entity` function L849-872 — `()` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `tags_on_entity` function L875-883 — `()` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `search_by_tags` function L886-907 — `()` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `superseded_excluded_from_search` function L910-921 — `()` — SQLite-backed knowledge base store with FTS5 search and relations.

#### crates/arawn-memory/src/types.rs

- pub `EntityType` enum L10-17 — `Fact | Decision | Convention | Preference | Person | Note` — Type of entity stored in the knowledge base.
- pub `as_str` function L20-29 — `(&self) -> &'static str` — Core types for the knowledge base memory system.
- pub `from_str` function L31-41 — `(s: &str) -> Option<Self>` — Core types for the knowledge base memory system.
- pub `default_scope` function L44-49 — `(&self) -> Scope` — Default scope for this entity type.
- pub `Scope` enum L55-58 — `Global | Workstream` — Which KB tier an entity belongs to.
- pub `RelationType` enum L63-71 — `RelatesTo | Contradicts | Supports | Supersedes | ExtractedFrom | Mentions | Bel...` — Type of relationship between entities.
- pub `as_str` function L74-84 — `(&self) -> &'static str` — Core types for the knowledge base memory system.
- pub `from_str` function L86-97 — `(s: &str) -> Option<Self>` — Core types for the knowledge base memory system.
- pub `ConfidenceSource` enum L103-110 — `Stated | Observed | Inferred` — How confident we are in this entity's accuracy.
- pub `base_score` function L113-119 — `(&self) -> f32` — Core types for the knowledge base memory system.
- pub `as_str` function L121-127 — `(&self) -> &'static str` — Core types for the knowledge base memory system.
- pub `from_str` function L129-136 — `(s: &str) -> Option<Self>` — Core types for the knowledge base memory system.
- pub `compute_confidence` function L140-165 — `( source: ConfidenceSource, reinforcement_count: u32, days_since_update: f64, su...` — Compute confidence score with reinforcement and staleness.
- pub `Entity` struct L169-182 — `{ id: Uuid, entity_type: EntityType, title: String, content: Option<String>, con...` — A knowledge entity stored in the KB.
- pub `new` function L185-201 — `(entity_type: EntityType, title: impl Into<String>) -> Self` — Core types for the knowledge base memory system.
- pub `with_content` function L203-206 — `(mut self, content: impl Into<String>) -> Self` — Core types for the knowledge base memory system.
- pub `with_confidence` function L208-211 — `(mut self, source: ConfidenceSource) -> Self` — Core types for the knowledge base memory system.
- pub `with_tags` function L213-216 — `(mut self, tags: Vec<String>) -> Self` — Core types for the knowledge base memory system.
- pub `with_session` function L218-221 — `(mut self, session_id: Uuid) -> Self` — Core types for the knowledge base memory system.
- pub `confidence_score` function L224-232 — `(&self) -> f32` — Compute the current confidence score.
- pub `Relation` struct L237-242 — `{ source_id: Uuid, relation_type: RelationType, target_id: Uuid, created_at: Dat...` — A directed relation between two entities.
- pub `StoreFactResult` enum L246-259 — `Inserted | Reinforced | Superseded` — Result of a store_fact operation (search-before-create).
-  `EntityType` type L19-50 — `= EntityType` — Core types for the knowledge base memory system.
-  `RelationType` type L73-98 — `= RelationType` — Core types for the knowledge base memory system.
-  `ConfidenceSource` type L112-137 — `= ConfidenceSource` — Core types for the knowledge base memory system.
-  `Entity` type L184-233 — `= Entity` — Core types for the knowledge base memory system.
-  `tests` module L262-342 — `-` — Core types for the knowledge base memory system.
-  `entity_type_roundtrip` function L266-277 — `()` — Core types for the knowledge base memory system.
-  `relation_type_roundtrip` function L280-292 — `()` — Core types for the knowledge base memory system.
-  `confidence_stated_fresh` function L295-298 — `()` — Core types for the knowledge base memory system.
-  `confidence_reinforced` function L301-305 — `()` — Core types for the knowledge base memory system.
-  `confidence_stale` function L308-312 — `()` — Core types for the knowledge base memory system.
-  `confidence_superseded_is_zero` function L315-318 — `()` — Core types for the knowledge base memory system.
-  `entity_builder` function L321-331 — `()` — Core types for the knowledge base memory system.
-  `default_scopes` function L334-341 — `()` — Core types for the knowledge base memory system.

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

### crates/arawn-service/src

> *Semantic summary to be generated by AI agent.*

#### crates/arawn-service/src/error.rs

- pub `ServiceError` enum L4-19 — `NotFound | InvalidOperation | Engine | Storage | Internal`

#### crates/arawn-service/src/lib.rs

- pub `error` module L1 — `-`
- pub `types` module L2 — `-`
- pub `ArawnService` interface L20-61 — `{ fn list_workstreams(), fn create_workstream(), fn list_sessions(), fn create_s...` — The service contract between any UI client and the Arawn backend.

#### crates/arawn-service/src/types.rs

- pub `WorkstreamInfo` struct L11-16 — `{ id: Uuid, name: String, root_dir: PathBuf, created_at: DateTime<Utc> }` — Lightweight view of a workstream for API transport.
- pub `SessionInfo` struct L20-24 — `{ id: Uuid, workstream_id: Option<Uuid>, created_at: DateTime<Utc> }` — Lightweight view of a session (metadata only, no messages).
- pub `SessionDetail` struct L28-33 — `{ id: Uuid, workstream_id: Option<Uuid>, created_at: DateTime<Utc>, messages: Ve...` — Session with full message history.
- pub `ModalPromptOption` struct L37-41 — `{ label: String, description: Option<String> }` — An option in a modal prompt sent to the client.
- pub `EngineEvent` enum L46-90 — `StreamingText | ToolCallStart | ToolCallResult | Complete | Error | CompactionOc...` — Streaming event emitted during a conversation turn.

### crates/arawn-storage/src

> *Semantic summary to be generated by AI agent.*

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

#### crates/arawn-storage/src/jsonl.rs

- pub `JsonlMessageStore` struct L16-18 — `{ data_dir: PathBuf }` — JSONL-based message persistence.
- pub `new` function L21-25 — `(data_dir: impl Into<PathBuf>) -> Self`
- pub `append` function L28-51 — `( &self, session_id: Uuid, workstream_dir: &str, msg: &Message, ) -> Result<(), ...` — Append a message to the session's JSONL file.
- pub `load` function L54-79 — `( &self, session_id: Uuid, workstream_dir: &str, ) -> Result<Vec<Message>, Stora...` — Load all messages for a session from its JSONL file.
- pub `move_session` function L83-103 — `( &self, session_id: Uuid, from_dir: &str, to_dir: &str, ) -> Result<(), Storage...` — Move a session's JSONL file from one workstream directory to another.
- pub `path_for` function L116-118 — `(&self, session_id: Uuid, workstream_dir: &str) -> PathBuf` — Get the path for a session (exposed for testing/debugging).
- pub `sandbox_dir` function L127-136 — `(&self, workstream_dir: &str, session_id: Uuid, is_scratch: bool) -> PathBuf` — Resolve the sandbox root for a session.
- pub `workstream_dir_name` function L140-146 — `(name: &str, id: Uuid) -> String` — Resolve a workstream directory name: use name if non-empty, fall back to UUID.
-  `JsonlMessageStore` type L20-137 — `= JsonlMessageStore`
-  `session_path` function L107-113 — `(&self, session_id: Uuid, workstream_dir: &str) -> PathBuf` — Resolve the filesystem path for a session's JSONL file.
-  `tests` module L149-381 — `-`
-  `setup` function L155-159 — `() -> (TempDir, JsonlMessageStore)`
-  `append_and_load_roundtrip` function L162-198 — `()`
-  `append_twice_accumulates` function L201-229 — `()`
-  `load_nonexistent_returns_empty` function L232-236 — `()`
-  `scratch_session_path` function L239-260 — `()`
-  `move_session_relocates_file` function L263-300 — `()`
-  `move_nonexistent_session_is_ok` function L303-309 — `()`
-  `jsonl_each_line_is_valid_json` function L312-348 — `()`
-  `sandbox_dir_scratch_is_per_session` function L351-359 — `()`
-  `sandbox_dir_named_is_shared` function L362-367 — `()`
-  `workstream_dir_name_prefers_name` function L370-374 — `()`
-  `workstream_dir_name_falls_back_to_uuid` function L377-380 — `()`

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
- pub `jsonl` module L3 — `-`
- pub `layout` module L4 — `-`
- pub `session_store` module L5 — `-`
- pub `store` module L6 — `-`
- pub `workstream_store` module L7 — `-`

#### crates/arawn-storage/src/session_store.rs

- pub `SessionStore` struct L10-12 — `{ db: &'a Database }` — CRUD operations for session metadata in SQLite.
- pub `new` function L15-17 — `(db: &'a Database) -> Self`
- pub `create` function L19-29 — `(&self, session: &Session) -> Result<(), StorageError>`
- pub `get` function L31-53 — `(&self, id: Uuid) -> Result<Option<SessionMeta>, StorageError>`
- pub `list_for_workstream` function L55-77 — `(&self, ws_id: Uuid) -> Result<Vec<SessionMeta>, StorageError>`
- pub `list_scratch` function L79-101 — `(&self) -> Result<Vec<SessionMeta>, StorageError>`
- pub `delete` function L104-110 — `(&self, session_id: Uuid) -> Result<bool, StorageError>` — Delete a session record from SQLite by ID.
- pub `update_stats` function L113-125 — `(&self, session_id: Uuid, stats: &SessionStats) -> Result<(), StorageError>` — Update session token/turn stats in SQLite.
- pub `update_workstream_id` function L127-137 — `( &self, session_id: Uuid, new_ws_id: Uuid, ) -> Result<bool, StorageError>`
- pub `SessionMeta` struct L142-147 — `{ id: Uuid, workstream_id: Option<Uuid>, created_at: DateTime<Utc>, stats: Sessi...` — Session metadata as stored in SQLite (no messages — those are in JSONL).
- pub `into_session` function L153-158 — `(self) -> Session` — Convert to an arawn_core::Session (without messages — load those separately).
-  `SessionMeta` type L149-159 — `= SessionMeta`
-  `SessionRow` struct L161-169 — `{ id: String, workstream_id: Option<String>, created_at: String, input_tokens: i...`
-  `SessionRow` type L171-198 — `= SessionRow`
-  `into_meta` function L172-197 — `(self) -> Result<SessionMeta, StorageError>`
-  `tests` module L201-326 — `-`
-  `setup` function L205-207 — `() -> Database`
-  `create_and_get_session` function L210-223 — `()`
-  `create_scratch_session` function L226-236 — `()`
-  `get_nonexistent_returns_none` function L239-243 — `()`
-  `list_for_workstream` function L246-268 — `()`
-  `list_scratch_sessions` function L271-289 — `()`
-  `update_workstream_id_promotes_scratch` function L292-307 — `()`
-  `update_workstream_id_on_bound_session_returns_false` function L310-325 — `()`

#### crates/arawn-storage/src/store.rs

- pub `Store` struct L16-20 — `{ db: Database, messages: JsonlMessageStore, data_dir: PathBuf }` — Unified persistence interface composing SQLite metadata + JSONL messages.
- pub `open` function L25-44 — `(data_dir: impl Into<PathBuf>) -> Result<Self, StorageError>` — Open or create a store at the given data directory.
- pub `data_dir` function L47-49 — `(&self) -> &Path` — Data directory path.
- pub `message_store` function L52-54 — `(&self) -> &JsonlMessageStore` — Get the JSONL message store (for direct access in service layer).
- pub `create_workstream` function L58-67 — `(&self, ws: &Workstream) -> Result<(), StorageError>`
- pub `get_workstream` function L69-71 — `(&self, id: Uuid) -> Result<Option<Workstream>, StorageError>`
- pub `find_workstream_by_name` function L73-75 — `(&self, name: &str) -> Result<Option<Workstream>, StorageError>`
- pub `list_workstreams` function L77-79 — `(&self) -> Result<Vec<Workstream>, StorageError>`
- pub `create_session` function L83-85 — `(&self, session: &Session) -> Result<(), StorageError>`
- pub `get_session_meta` function L87-89 — `(&self, id: Uuid) -> Result<Option<SessionMeta>, StorageError>`
- pub `list_sessions_for_workstream` function L91-96 — `( &self, ws_id: Uuid, ) -> Result<Vec<SessionMeta>, StorageError>`
- pub `list_scratch_sessions` function L98-100 — `(&self) -> Result<Vec<SessionMeta>, StorageError>`
- pub `reconcile_sessions` function L104-136 — `(&self) -> Result<usize, StorageError>` — Remove SQLite session records whose JSONL files no longer exist on disk.
- pub `load_session` function L153-170 — `(&self, id: Uuid) -> Result<Option<Session>, StorageError>` — Load a full session (metadata + messages) by ID.
- pub `update_session_stats` function L172-178 — `( &self, session_id: Uuid, stats: &arawn_core::SessionStats, ) -> Result<(), Sto...`
- pub `append_message` function L182-189 — `( &self, session_id: Uuid, workstream_dir: &str, msg: &Message, ) -> Result<(), ...`
- pub `load_messages` function L191-197 — `( &self, session_id: Uuid, workstream_dir: &str, ) -> Result<Vec<Message>, Stora...`
- pub `promote_session` function L203-256 — `( &self, session_id: Uuid, new_ws_id: Uuid, ) -> Result<(), StorageError>` — Promote a scratch session to a workstream.
- pub `sandbox_for` function L259-262 — `(&self, workstream_dir: &str, session_id: Uuid, is_scratch: bool) -> PathBuf` — Resolve the sandbox root for a session.
-  `Store` type L22-263 — `= Store`
-  `resolve_ws_dir` function L140-150 — `(&self, ws_id: Option<Uuid>) -> Result<String, StorageError>` — Resolve the directory name for a workstream by UUID.
-  `copy_dir_contents` function L266-279 — `(src: &Path, dst: &Path) -> Result<(), StorageError>` — Recursively copy directory contents from src to dst.
-  `tests` module L282-451 — `-`
-  `setup` function L286-290 — `() -> (TempDir, Store)`
-  `open_creates_directories_and_db` function L293-299 — `()`
-  `open_is_idempotent` function L302-307 — `()`
-  `create_and_list_workstreams` function L310-318 — `()`
-  `create_scratch_session_and_append_messages` function L321-339 — `()`
-  `load_full_session` function L342-365 — `()`
-  `promote_session_full_flow` function L368-408 — `()`
-  `promote_bound_session_fails` function L411-424 — `()`
-  `load_nonexistent_session_returns_none` function L427-431 — `()`
-  `sandbox_for_scratch_is_per_session` function L434-441 — `()`
-  `sandbox_for_named_is_shared` function L444-450 — `()`

#### crates/arawn-storage/src/workstream_store.rs

- pub `WorkstreamStore` struct L12-14 — `{ db: &'a Database }` — CRUD operations for workstream metadata in SQLite.
- pub `new` function L17-19 — `(db: &'a Database) -> Self`
- pub `create` function L21-32 — `(&self, ws: &Workstream) -> Result<(), StorageError>`
- pub `get` function L34-54 — `(&self, id: Uuid) -> Result<Option<Workstream>, StorageError>`
- pub `find_by_name` function L56-76 — `(&self, name: &str) -> Result<Option<Workstream>, StorageError>`
- pub `list` function L78-97 — `(&self) -> Result<Vec<Workstream>, StorageError>`
- pub `delete` function L99-105 — `(&self, id: Uuid) -> Result<bool, StorageError>`
-  `WorkstreamRow` struct L108-113 — `{ id: String, name: String, root_dir: String, created_at: String }`
-  `WorkstreamRow` type L115-130 — `= WorkstreamRow`
-  `into_workstream` function L116-129 — `(self) -> Result<Workstream, StorageError>`
-  `tests` module L133-204 — `-`
-  `setup` function L136-138 — `() -> Database`
-  `create_and_get_roundtrip` function L141-152 — `()`
-  `get_nonexistent_returns_none` function L155-159 — `()`
-  `find_by_name` function L162-172 — `()`
-  `list_workstreams` function L175-185 — `()`
-  `delete_workstream` function L188-196 — `()`
-  `delete_nonexistent_returns_false` function L199-203 — `()`

### crates/arawn-tests/fixtures/arawn-plugin-web-fetch/src

> *Semantic summary to be generated by AI agent.*

#### crates/arawn-tests/fixtures/arawn-plugin-web-fetch/src/lib.rs

- pub `WebFetchTool` struct L3 — `-`
-  `WebFetchTool` type L6-94 — `impl ArawnTool for WebFetchTool`
-  `name` function L7-9 — `(&self) -> String`
-  `description` function L11-14 — `(&self) -> String`
-  `parameters_schema` function L16-32 — `(&self) -> String`
-  `execute` function L34-93 — `(&self, _context_json: String, params_json: String) -> ToolExecuteOutput`
-  `strip_html_tags` function L96-127 — `(html: &str) -> String`

### crates/arawn-tests/fixtures/arawn-plugin-web-search/src

> *Semantic summary to be generated by AI agent.*

#### crates/arawn-tests/fixtures/arawn-plugin-web-search/src/lib.rs

- pub `WebSearchTool` struct L3 — `-`
-  `WebSearchTool` type L6-98 — `impl ArawnTool for WebSearchTool`
-  `name` function L7-9 — `(&self) -> String`
-  `description` function L11-14 — `(&self) -> String`
-  `parameters_schema` function L16-32 — `(&self) -> String`
-  `execute` function L34-97 — `(&self, _context_json: String, params_json: String) -> ToolExecuteOutput`
-  `SearchResult` struct L100-104 — `{ title: String, url: String, snippet: String }`
-  `parse_ddg_results` function L106-131 — `(html: &str, max: usize) -> Vec<SearchResult>`
-  `extract_tag_content` function L133-141 — `(html: &str, after: &str) -> String`
-  `extract_href` function L143-157 — `(html: &str) -> String`
-  `extract_after_class` function L159-171 — `(html: &str, class: &str) -> String`
-  `strip_tags` function L173-185 — `(html: &str) -> String`
-  `urlencod` function L187-195 — `(s: &str) -> String`
-  `urldecod` function L197-216 — `(s: &str) -> String`

### crates/arawn-tests/tests

> *Semantic summary to be generated by AI agent.*

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

-  `setup_service` function L14-35 — `(responses: Vec<MockResponse>) -> (TempDir, arawn_bin::LocalService)` — Tests for LocalService — the ArawnService impl that wraps engine + store.
-  `list_workstreams_returns_scratch` function L38-43 — `()` — Tests for LocalService — the ArawnService impl that wraps engine + store.
-  `create_and_load_session_roundtrip` function L46-58 — `()` — Tests for LocalService — the ArawnService impl that wraps engine + store.
-  `send_message_text_only_returns_complete` function L61-83 — `()` — Tests for LocalService — the ArawnService impl that wraps engine + store.
-  `send_message_with_tool_call_returns_events` function L86-118 — `()` — Tests for LocalService — the ArawnService impl that wraps engine + store.
-  `send_message_persists_to_jsonl` function L121-143 — `()` — Tests for LocalService — the ArawnService impl that wraps engine + store.

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
-  `register_plugin_skills_namespaces_into_registry` function L213-234 — `()` — Integration tests: plugin discovery, manifest parsing, and component loading.
-  `invalid_manifest_gracefully_skipped` function L237-256 — `()` — Integration tests: plugin discovery, manifest parsing, and component loading.
-  `plugin_with_mixed_valid_invalid_components` function L259-291 — `()` — Integration tests: plugin discovery, manifest parsing, and component loading.
-  `empty_cache_returns_no_plugins` function L294-299 — `()` — Integration tests: plugin discovery, manifest parsing, and component loading.

#### crates/arawn-tests/tests/plugin_loading.rs

-  `web_fetch_dylib_dir` function L10-13 — `() -> std::path::PathBuf` — Path to the pre-built web-fetch dylib (debug profile).
-  `load_web_fetch_plugin_and_read_metadata` function L16-64 — `()` — cd crates/arawn-tests/fixtures/arawn-plugin-web-fetch && cargo build
-  `web_fetch_plugin_execute_fetches_url` function L67-113 — `()` — cd crates/arawn-tests/fixtures/arawn-plugin-web-fetch && cargo build
-  `web_search_dylib_dir` function L116-119 — `() -> std::path::PathBuf` — Path to the pre-built web-search dylib (debug profile).
-  `load_web_search_plugin_and_read_metadata` function L122-145 — `()` — cd crates/arawn-tests/fixtures/arawn-plugin-web-fetch && cargo build
-  `web_search_plugin_execute_searches` function L148-192 — `()` — cd crates/arawn-tests/fixtures/arawn-plugin-web-fetch && cargo build

#### crates/arawn-tests/tests/skills.rs

-  `assert_tool_result_ok_contains` function L13-26 — `(msgs: &[Message], index: usize, substring: &str)` — Integration tests: skill loading and invocation through the QueryEngine.
-  `assert_tool_result_is_error` function L28-41 — `(msgs: &[Message], index: usize, substring: &str)` — Integration tests: skill loading and invocation through the QueryEngine.
-  `make_skill` function L43-54 — `(name: &str, prompt: &str, user_invocable: bool, source: SkillSource) -> SkillDe...` — Integration tests: skill loading and invocation through the QueryEngine.
-  `register_skill_in_memory_invoke_through_engine` function L59-80 — `()` — Integration tests: skill loading and invocation through the QueryEngine.
-  `load_skill_from_markdown_file_and_invoke` function L83-119 — `()` — Integration tests: skill loading and invocation through the QueryEngine.
-  `skill_not_found_returns_error` function L122-145 — `()` — Integration tests: skill loading and invocation through the QueryEngine.
-  `user_invocable_filtering` function L148-157 — `()` — Integration tests: skill loading and invocation through the QueryEngine.
-  `plugin_namespaced_skill_accessible` function L160-180 — `()` — Integration tests: skill loading and invocation through the QueryEngine.

#### crates/arawn-tests/tests/websocket.rs

-  `start_test_server` function L19-69 — `(mock_responses: Vec<MockResponse>) -> (String, TempDir)` — Spin up a test server on a random port and return the WS URL.
-  `send_request` function L72-94 — `( write: &mut futures_util::stream::SplitSink< tokio_tungstenite::WebSocketStrea...` — Helper: send a JSON request and get the response.
-  `list_workstreams_returns_scratch` function L97-113 — `()` — Spins up the server on a random port, connects a WS client, exercises the JSON protocol.
-  `create_and_load_session` function L116-142 — `()` — Spins up the server on a random port, connects a WS client, exercises the JSON protocol.
-  `unknown_method_returns_error` function L145-159 — `()` — Spins up the server on a random port, connects a WS client, exercises the JSON protocol.
-  `malformed_json_returns_error` function L162-176 — `()` — Spins up the server on a random port, connects a WS client, exercises the JSON protocol.
-  `send_message_streams_complete_event` function L181-234 — `()` — Spins up the server on a random port, connects a WS client, exercises the JSON protocol.
-  `send_message_with_tool_call_streams_events` function L237-304 — `()` — Spins up the server on a random port, connects a WS client, exercises the JSON protocol.

### crates/arawn-tool-plugin/src

> *Semantic summary to be generated by AI agent.*

#### crates/arawn-tool-plugin/src/lib.rs

- pub `ArawnTool` interface L26-40 — `{ fn name(), fn description(), fn parameters_schema(), fn execute() }` — The plugin interface for Arawn tools.
- pub `ToolExecuteOutput` struct L44-47 — `{ content: String, is_error: bool }` — Output from the `execute` method.
- pub `success` function L50-55 — `(content: impl Into<String>) -> Self`
- pub `error` function L57-62 — `(content: impl Into<String>) -> Self`
-  `ToolExecuteOutput` type L49-63 — `= ToolExecuteOutput`

### crates/arawn-tui/src

> *Semantic summary to be generated by AI agent.*

#### crates/arawn-tui/src/action.rs

- pub `Action` enum L3-49 — `TypeChar | Backspace | Delete | CursorLeft | CursorRight | CursorHome | CursorEn...`

#### crates/arawn-tui/src/app.rs

- pub `LayoutRegions` struct L13-23 — `{ sidebar: Option<Rect>, chat: Rect, input: Rect, sidebar_ws: Option<Rect>, side...` — Tracks the screen regions of each panel from the last render.
- pub `Focus` enum L27-31 — `Main | Sidebar` — Which panel has focus.
- pub `SidebarSection` enum L35-38 — `Workstreams | Sessions` — Which sidebar section is active.
- pub `ChatMessage` struct L42-51 — `{ role: ChatRole, content: String, created_at: std::time::Instant, rendered_cach...` — A message displayed in the chat area.
- pub `new` function L54-62 — `(role: ChatRole, content: impl Into<String>) -> Self`
- pub `rendered_lines` function L66-78 — `(&mut self, width: usize) -> &[ratatui::text::Line<'static>]` — Get or compute the cached markdown rendering for assistant messages.
- pub `ChatRole` enum L82-88 — `User | Assistant | ToolCall | ToolResult | System`
- pub `App` struct L91-132 — `{ focus: Focus, input_buffer: String, cursor_pos: usize, messages: Vec<ChatMessa...` — All mutable TUI state.
- pub `new` function L135-166 — `() -> Self`
- pub `handle_action` function L169-474 — `(&mut self, action: Action) -> bool` — Process an action and mutate state.
- pub `apply_engine_event` function L520-587 — `(&mut self, event: crate::ws_client::EventUpdate)` — Apply a streaming engine event to the app state (testable without network).
- pub `format_tool_input` function L607-655 — `(tool_name: &str, input: &serde_json::Value) -> String` — Format tool input args into a compact display string.
-  `ChatMessage` type L53-79 — `= ChatMessage`
-  `App` type L134-604 — `= App`
-  `update_autocomplete` function L477-506 — `(&mut self)` — Update autocomplete suggestions based on current input buffer.
-  `accept_autocomplete` function L509-517 — `(&mut self)` — Accept the currently selected autocomplete suggestion.
-  `prev_char_boundary` function L589-595 — `(&self) -> usize`
-  `next_char_boundary` function L597-603 — `(&self) -> usize`
-  `App` type L657-661 — `impl Default for App`
-  `default` function L658-660 — `() -> Self`
-  `tests` module L664-902 — `-`
-  `type_chars_updates_buffer` function L668-674 — `()`
-  `backspace_removes_char` function L677-684 — `()`
-  `submit_moves_to_messages` function L687-699 — `()`
-  `submit_blocked_when_empty` function L702-708 — `()`
-  `submit_blocked_while_generating` function L711-717 — `()`
-  `tab_toggles_focus` function L720-727 — `()`
-  `scroll_updates_offset` function L730-738 — `()`
-  `cancel_stops_generation` function L741-750 — `()`
-  `quit_sets_flag` function L753-757 — `()`
-  `cursor_movement` function L760-781 — `()`
-  `full_conversation_flow` function L786-816 — `()`
-  `tool_call_flow` function L819-850 — `()`
-  `error_event_clears_generating` function L853-867 — `()`
-  `sidebar_navigation` function L870-901 — `()`

#### crates/arawn-tui/src/command.rs

- pub `CommandInfo` struct L11-15 — `{ name: String, description: String, kind: CommandKind }` — A registered slash command.
- pub `CommandKind` enum L19-26 — `BuiltIn | Inventory | Skill` — What kind of slash command this is.
- pub `ParsedCommand` struct L30-33 — `{ name: String, args: String }` — Result of parsing a slash command from the input buffer.
- pub `parse_command` function L37-57 — `(input: &str) -> Option<ParsedCommand>` — Parse a slash command from the input buffer.
- pub `CommandRegistry` struct L61-63 — `{ commands: Vec<CommandInfo> }` — The command registry — holds all available slash commands.
- pub `new` function L66-70 — `() -> Self` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
- pub `register_skills` function L133-143 — `(&mut self, skills: Vec<(String, String)>)` — Add skill commands from the server's cached skill list.
- pub `all` function L146-148 — `(&self) -> &[CommandInfo]` — Get all commands.
- pub `matching` function L151-157 — `(&self, prefix: &str) -> Vec<&CommandInfo>` — Find commands matching a prefix (for autocomplete).
- pub `find` function L160-163 — `(&self, name: &str) -> Option<&CommandInfo>` — Look up a command by exact name.
- pub `AutocompleteState` struct L168-173 — `{ suggestions: Vec<CommandInfo>, selected: usize }` — Autocomplete state for the slash command dropdown.
- pub `new` function L176-181 — `(suggestions: Vec<CommandInfo>) -> Self` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
- pub `next` function L183-187 — `(&mut self)` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
- pub `prev` function L189-197 — `(&mut self)` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
- pub `selected_command` function L199-201 — `(&self) -> Option<&CommandInfo>` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
- pub `is_empty` function L203-205 — `(&self) -> bool` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
- pub `CommandResult` enum L210-227 — `SystemMessage | ClearChat | EnterPlan | QueryInventory | InvokeSkill | RememberF...` — The result of executing a built-in command.
- pub `execute_command` function L230-272 — `(cmd: &ParsedCommand, registry: &CommandRegistry) -> CommandResult` — Execute a parsed slash command against the registry.
-  `CommandRegistry` type L65-164 — `= CommandRegistry` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `register_builtins` function L72-130 — `(&mut self)` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `AutocompleteState` type L175-206 — `= AutocompleteState` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `tests` module L275-419 — `-` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `parse_simple_command` function L279-283 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `parse_command_with_args` function L286-290 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `parse_not_a_command` function L293-297 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `parse_slash_only` function L300-302 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `parse_with_leading_whitespace` function L305-308 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `registry_has_builtins` function L311-318 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `registry_matching_prefix` function L321-327 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `registry_matching_empty_returns_all` function L330-334 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `registry_skills` function L337-346 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `autocomplete_navigation` function L349-367 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `execute_help` function L370-377 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `execute_clear` function L380-384 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `execute_unknown` function L387-394 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `execute_inventory` function L397-404 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `execute_skill` function L407-418 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server

#### crates/arawn-tui/src/event.rs

- pub `map_key_event` function L7-66 — `( key: KeyEvent, focus: Focus, is_generating: bool, has_modal: bool, has_autocom...` — Map a crossterm KeyEvent to an Action, given the current focus.
-  `map_main_key` function L68-84 — `(key: KeyEvent) -> Option<Action>`
-  `map_modal_key` function L86-102 — `(key: KeyEvent) -> Option<Action>`
-  `map_sidebar_key` function L104-112 — `(key: KeyEvent) -> Option<Action>`
-  `tests` module L115-223 — `-`
-  `key` function L117-119 — `(code: KeyCode) -> KeyEvent`
-  `ctrl` function L121-123 — `(c: char) -> KeyEvent`
-  `ctrl_c_quits_from_any_focus` function L126-135 — `()`
-  `tab_toggles_from_any_focus` function L138-147 — `()`
-  `esc_cancels_when_generating` function L150-156 — `()`
-  `main_focus_typing` function L159-172 — `()`
-  `main_focus_scrolling` function L175-188 — `()`
-  `ctrl_e_toggles_tool_results` function L191-202 — `()`
-  `sidebar_focus_navigation` function L205-222 — `()`

#### crates/arawn-tui/src/event_loop.rs

- pub `run_tui` function L28-578 — `(url: &str, model_name: &str) -> Result<(), Box<dyn std::error::Error>>` — Run the TUI connected to the given WebSocket server URL.
-  `rect_contains` function L23-25 — `(rect: Rect, col: u16, row: u16) -> bool`

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
- pub `ws_client` module L15 — `-`
-  `snapshot` module L12 — `-`
-  `snapshot_tests` module L14 — `-`

#### crates/arawn-tui/src/markdown.rs

- pub `markdown_to_lines` function L23-25 — `(text: &str) -> Vec<Line<'static>>` — Parse a markdown string into styled ratatui `Line`s.
- pub `markdown_to_lines_with_width` function L29-40 — `(text: &str, max_width: usize) -> Vec<Line<'static>>` — Parse a markdown string into styled ratatui `Line`s.
-  `SYNTAX_SET` variable L14 — `: LazyLock<SyntaxSet>` — suitable for rendering in the chat area.
-  `THEME` variable L15-18 — `: LazyLock<Theme>` — suitable for rendering in the chat area.
-  `CODE_STYLE` variable L42-44 — `: Style` — suitable for rendering in the chat area.
-  `MdRenderer` struct L46-68 — `{ lines: Vec<Line<'static>>, current_spans: Vec<Span<'static>>, style_stack: Vec...` — suitable for rendering in the chat area.
-  `MdRenderer` type L70-497 — `= MdRenderer` — suitable for rendering in the chat area.
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
-  `emit_full_table` function L347-447 — `(&mut self)` — suitable for rendering in the chat area.
-  `emit_padded_row` function L449-483 — `( &mut self, row: &[String], col_widths: &[usize], cell_style: Style, chrome_sty...` — suitable for rendering in the chat area.
-  `finish` function L485-496 — `(mut self) -> Vec<Line<'static>>` — suitable for rendering in the chat area.
-  `highlight_code` function L501-539 — `(code: &str, lang: Option<&str>) -> Vec<Line<'static>>` — Syntax-highlight a code block, returning one Line per source line.
-  `heading_style` function L541-549 — `(level: u8) -> Style` — suitable for rendering in the chat area.
-  `wrap_text` function L553-632 — `(text: &str, width: usize) -> Vec<String>` — Word-wrap text to fit within a given width.
-  `tests` module L635-811 — `-` — suitable for rendering in the chat area.
-  `spans_text` function L638-650 — `(lines: &[Line]) -> String` — suitable for rendering in the chat area.
-  `plain_text` function L653-657 — `()` — suitable for rendering in the chat area.
-  `heading_levels` function L660-669 — `()` — suitable for rendering in the chat area.
-  `bold_and_italic` function L672-686 — `()` — suitable for rendering in the chat area.
-  `inline_code` function L689-697 — `()` — suitable for rendering in the chat area.
-  `fenced_code_block` function L700-715 — `()` — suitable for rendering in the chat area.
-  `unordered_list` function L718-724 — `()` — suitable for rendering in the chat area.
-  `ordered_list` function L727-732 — `()` — suitable for rendering in the chat area.
-  `table_renders_aligned` function L735-755 — `()` — suitable for rendering in the chat area.
-  `link_shows_url` function L758-763 — `()` — suitable for rendering in the chat area.
-  `no_double_blank_lines` function L766-780 — `()` — suitable for rendering in the chat area.
-  `table_wide_content_preserves_short_columns` function L783-803 — `()` — suitable for rendering in the chat area.
-  `no_trailing_blanks` function L806-810 — `()` — suitable for rendering in the chat area.

#### crates/arawn-tui/src/modal.rs

- pub `ModalOption` struct L15-18 — `{ label: String, description: Option<String> }` — A single option in the modal.
- pub `new` function L21-26 — `(label: impl Into<String>) -> Self` — questions, and any future tool that needs user input.
- pub `with_description` function L28-31 — `(mut self, desc: impl Into<String>) -> Self` — questions, and any future tool that needs user input.
- pub `ModalState` struct L35-44 — `{ title: String, subtitle: Option<String>, options: Vec<ModalOption>, focused_in...` — Active modal state.
- pub `new` function L47-61 — `( title: impl Into<String>, options: Vec<ModalOption>, border_color: Color, resu...` — questions, and any future tool that needs user input.
- pub `with_subtitle` function L63-66 — `(mut self, subtitle: impl Into<String>) -> Self` — questions, and any future tool that needs user input.
- pub `focus_prev` function L69-73 — `(&mut self)` — Move focus up.
- pub `focus_next` function L76-80 — `(&mut self)` — Move focus down.
- pub `confirm` function L83-87 — `(&mut self)` — Confirm the focused option.
- pub `cancel` function L90-94 — `(&mut self)` — Cancel (Escape).
- pub `render_modal` function L98-182 — `(modal: &ModalState, frame: &mut Frame)` — Render the modal as a centered overlay.
-  `ModalOption` type L20-32 — `= ModalOption` — questions, and any future tool that needs user input.
-  `ModalState` type L46-95 — `= ModalState` — questions, and any future tool that needs user input.
-  `centered_rect` function L185-189 — `(width: u16, height: u16, area: Rect) -> Rect` — Calculate a centered rectangle within an area.
-  `tests` module L192-288 — `-` — questions, and any future tool that needs user input.
-  `make_modal` function L195-207 — `() -> ModalState` — questions, and any future tool that needs user input.
-  `navigation` function L210-233 — `()` — questions, and any future tool that needs user input.
-  `confirm_sends_index` function L236-248 — `()` — questions, and any future tool that needs user input.
-  `cancel_sends_none` function L251-262 — `()` — questions, and any future tool that needs user input.
-  `confirm_only_sends_once` function L265-277 — `()` — questions, and any future tool that needs user input.
-  `centered_rect_calculation` function L280-287 — `()` — questions, and any future tool that needs user input.

#### crates/arawn-tui/src/render.rs

- pub `render` function L12-80 — `(app: &mut App, frame: &mut Frame)` — Render function.
-  `SPINNER_FRAMES` variable L9 — `: &[char]`
-  `render_sidebar_tab` function L82-105 — `(frame: &mut Frame, area: ratatui::layout::Rect)`
-  `render_status_bar` function L107-157 — `(app: &App, frame: &mut Frame, area: ratatui::layout::Rect)`
-  `format_tokens` function L160-168 — `(n: u64) -> String` — Format a token count for display: 1234 → "1.2k", 12345 → "12.3k", 500 → "500"
-  `render_sidebar` function L170-244 — `(app: &App, frame: &mut Frame, area: ratatui::layout::Rect)`
-  `render_chat` function L246-504 — `(app: &mut App, frame: &mut Frame, area: ratatui::layout::Rect)`
-  `render_separator` function L506-510 — `(frame: &mut Frame, area: ratatui::layout::Rect)`
-  `render_input` function L512-561 — `(app: &App, frame: &mut Frame, area: ratatui::layout::Rect)`
-  `render_autocomplete` function L564-622 — `( ac: &crate::command::AutocompleteState, frame: &mut Frame, input_area: ratatui...` — Render the autocomplete dropdown above the input line.
-  `truncate_to` function L625-634 — `(s: &str, max_chars: usize) -> String` — Truncate a string to fit within a display width, adding "…" if needed.
-  `compact_tool_summary` function L637-642 — `(content: &str) -> String` — Extract a compact summary from tool call content for inline display.
-  `truncate_for_display` function L644-650 — `(s: &str, max: usize) -> String`
-  `tests` module L653-1387 — `-`
-  `buffer_to_string` function L659-674 — `(terminal: &Terminal<TestBackend>, row: u16) -> String`
-  `render_empty_app_has_status_bar` function L677-686 — `()`
-  `render_with_messages_shows_content` function L689-715 — `()`
-  `render_with_input_text` function L718-733 — `()`
-  `render_streaming_shows_cursor` function L736-759 — `()`
-  `render_small_terminal` function L762-767 — `()`
-  `render_large_terminal` function L770-775 — `()`
-  `region_text` function L780-792 — `(terminal: &Terminal<TestBackend>, x: u16, y: u16, w: u16, h: u16) -> String` — Extract text from a rectangular region of the buffer.
-  `chat_region_for` function L796-809 — `(terminal: &Terminal<TestBackend>, sidebar_visible: bool) -> String` — Extract the chat area text.
-  `chat_region` function L812-814 — `(terminal: &Terminal<TestBackend>) -> String` — Convenience: chat region for default app (sidebar hidden).
-  `sidebar_region` function L818-826 — `(terminal: &Terminal<TestBackend>) -> String` — Extract the sidebar text (left 20%, rows 1..height-3).
-  `input_region` function L829-834 — `(terminal: &Terminal<TestBackend>) -> String` — Extract the input bar text (second from bottom row).
-  `chat_renders_user_message_with_prefix` function L839-853 — `()`
-  `chat_renders_assistant_message_with_prefix` function L856-870 — `()`
-  `chat_renders_tool_call_with_icon` function L873-898 — `()`
-  `chat_renders_tool_result_collapsed` function L901-929 — `()`
-  `chat_renders_tool_error_result` function L932-955 — `()`
-  `chat_renders_tool_result_truncated` function L958-985 — `()`
-  `chat_streaming_text_appears_in_chat_area` function L988-1010 — `()`
-  `sidebar_renders_workstream_names` function L1013-1049 — `()`
-  `sidebar_does_not_leak_into_chat` function L1052-1086 — `()`
-  `input_shows_placeholder_when_empty` function L1089-1100 — `()`
-  `input_shows_generating_when_active` function L1103-1116 — `()`
-  `status_bar_shows_generating_indicator` function L1119-1133 — `()`
-  `status_bar_shows_workstream_name` function L1136-1160 — `()`
-  `messages_do_not_appear_in_input_area` function L1163-1186 — `()`
-  `chat_auto_scrolls_to_bottom_with_many_messages` function L1191-1221 — `()`
-  `chat_scroll_up_reveals_older_messages` function L1224-1252 — `()`
-  `chat_few_messages_all_visible` function L1255-1269 — `()`
-  `last_message_visible_above_input` function L1272-1325 — `()`
-  `last_tool_result_visible_above_input` function L1328-1386 — `()`

#### crates/arawn-tui/src/snapshot.rs

- pub `buffer_to_snapshot` function L6-26 — `(terminal: &ratatui::Terminal<ratatui::backend::TestBackend>) -> String` — Render a TestBackend buffer to a deterministic string for snapshot comparison.
- pub `buffer_to_styled_snapshot` function L33-71 — `( terminal: &ratatui::Terminal<ratatui::backend::TestBackend>, ) -> String` — Render a TestBackend buffer with inline style annotations.
-  `format_style_tag` function L74-110 — `(fg: Color, bg: Color, mods: Modifier) -> String`

#### crates/arawn-tui/src/snapshot_tests.rs

-  `tests` module L2-322 — `-`
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
-  `snapshot_error_in_chat` function L182-192 — `()`
-  `styled_snapshot_conversation` function L197-220 — `()`
-  `styled_snapshot_focus_borders` function L223-232 — `()`
-  `styled_snapshot_sidebar_focused` function L235-243 — `()`
-  `snapshot_rich_markdown` function L246-278 — `()`
-  `styled_snapshot_rich_markdown` function L281-309 — `()`
-  `styled_snapshot_generating_state` function L312-321 — `()`

#### crates/arawn-tui/src/theme.rs

- pub `USER` variable L10 — `: Color` — User message prefix ("You:")
- pub `ASSISTANT` variable L13 — `: Color` — Assistant message prefix ("Arawn:")
- pub `SYSTEM` variable L16 — `: Color` — System message prefix
- pub `ERROR` variable L19 — `: Color` — Error text and indicators
- pub `TOOL_NAME` variable L22 — `: Color` — Tool name in tool calls
- pub `GENERATING` variable L25 — `: Color` — Generating / in-progress indicator
- pub `SUCCESS` variable L28 — `: Color` — Success indicator (✓)
- pub `CHROME` variable L33 — `: Color` — Box borders around tool calls/results (┌│└)
- pub `SEPARATOR` variable L36 — `: Color` — Separator line between chat and input
- pub `STATUS_BAR_BG` variable L39 — `: Color` — Status bar background
- pub `STATUS_BAR_FG` variable L42 — `: Color` — Status bar text
- pub `BORDER_INACTIVE` variable L45 — `: Color` — Sidebar border (unfocused)
- pub `BORDER_ACTIVE` variable L48 — `: Color` — Sidebar border (focused)
- pub `SIDEBAR_TAB_BG` variable L51 — `: Color` — Sidebar tab strip background
- pub `RESULT_TEXT` variable L56 — `: Color` — Tool result content text
- pub `RESULT_LABEL` variable L59 — `: Color` — Tool result labels ("▸ shell result")
- pub `TOOL_SUMMARY` variable L62 — `: Color` — Tool input summary text (args after tool name)
- pub `RESULT_HINT` variable L65 — `: Color` — Truncation hints ("… 15 more")
- pub `INPUT_PROMPT` variable L70 — `: Color` — Input prompt "> "
- pub `PLACEHOLDER` variable L73 — `: Color` — Placeholder text ("Type your message...")
- pub `CODE_BG` variable L78 — `: Color` — Code block background
- pub `CODE_FG` variable L81 — `: Color` — Code block text (fallback when no syntax highlighting)
- pub `INLINE_CODE_FG` variable L84 — `: Color` — Inline code text
- pub `INLINE_CODE_BG` variable L87 — `: Color` — Inline code background
- pub `CODE_LANG` variable L90 — `: Color` — Code block language label
- pub `HEADING_1` variable L94 — `: Color` — Change colors here to restyle the entire TUI in one place.
- pub `HEADING_2` variable L95 — `: Color` — Change colors here to restyle the entire TUI in one place.
- pub `HEADING_3` variable L96 — `: Color` — Change colors here to restyle the entire TUI in one place.
- pub `HEADING_4` variable L97 — `: Color` — Change colors here to restyle the entire TUI in one place.
- pub `RULE` variable L102 — `: Color` — Horizontal rules
- pub `LIST_BULLET` variable L105 — `: Color` — List bullet/number prefix
- pub `BLOCK_QUOTE` variable L108 — `: Color` — Block quote text
- pub `LINK` variable L111 — `: Color` — Link text
- pub `LINK_URL` variable L114 — `: Color` — Link URL shown after link text
- pub `TABLE_CHROME` variable L117 — `: Color` — Table chrome (│ ├ ┼ ┤)
- pub `bold` function L121-123 — `(color: Color) -> Style` — Change colors here to restyle the entire TUI in one place.
- pub `italic` function L125-127 — `(color: Color) -> Style` — Change colors here to restyle the entire TUI in one place.

#### crates/arawn-tui/src/tui_prompt.rs

- pub `TuiModalRequest` struct L15-17 — `{ modal: ModalState }` — A request to show a modal in the TUI event loop.
- pub `TuiModalPrompt` struct L21-23 — `{ tx: mpsc::Sender<TuiModalRequest> }` — TUI-based modal prompt.
- pub `new` function L26-28 — `(tx: mpsc::Sender<TuiModalRequest>) -> Self` — via a oneshot channel.
-  `TuiModalPrompt` type L25-29 — `= TuiModalPrompt` — via a oneshot channel.
-  `TuiModalPrompt` type L32-66 — `impl ModalPrompt for TuiModalPrompt` — via a oneshot channel.
-  `prompt` function L33-65 — `(&self, request: ModalRequest) -> Option<usize>` — via a oneshot channel.

#### crates/arawn-tui/src/ws_client.rs

- pub `WsClient` struct L16-28 — `{ write: futures_util::stream::SplitSink< tokio_tungstenite::WebSocketStream< to...` — A WebSocket connection to the Arawn server.
- pub `connect` function L31-35 — `(url: &str) -> Result<Self, Box<dyn std::error::Error>>`
- pub `send_request` function L37-52 — `( &mut self, method: &str, params: Value, ) -> Result<u64, Box<dyn std::error::E...`
- pub `list_workstreams` function L54-61 — `( &mut self, ) -> Result<Vec<WorkstreamInfo>, Box<dyn std::error::Error>>`
- pub `list_sessions` function L63-75 — `( &mut self, ws_id: Option<uuid::Uuid>, ) -> Result<Vec<SessionInfo>, Box<dyn st...`
- pub `create_session` function L77-89 — `( &mut self, ws_id: Option<uuid::Uuid>, ) -> Result<SessionInfo, Box<dyn std::er...`
- pub `send_message` function L91-107 — `( &mut self, session_id: uuid::Uuid, content: &str, ) -> Result<(), Box<dyn std:...`
- pub `read_response_raw` function L110-112 — `(&mut self) -> Result<Value, Box<dyn std::error::Error>>` — Read the next JSON response from the server (public for sidebar).
- pub `parse_engine_event` function L128-137 — `(text: &str) -> Option<EngineEvent>` — Parse a WS message as an EngineEvent.
- pub `EventUpdate` enum L140-166 — `AppendStreamingText | AddToolCall | AddToolResult | Complete | Error | Compactio...` — Convert an EngineEvent into App state updates.
- pub `engine_event_to_update` function L168-194 — `(event: EngineEvent) -> EventUpdate`
-  `REQUEST_ID` variable L9 — `: AtomicU64`
-  `next_id` function L11-13 — `() -> u64`
-  `WsClient` type L30-125 — `= WsClient`
-  `read_response` function L115-124 — `(&mut self) -> Result<Value, Box<dyn std::error::Error>>` — Read the next JSON response from the server.

### scripts

> *Semantic summary to be generated by AI agent.*

#### scripts/functional_test.py

- pub `send_rpc` function L16-30 — `def send_rpc(ws, method, params=None)` — Send a JSON-RPC request and return the result.
- pub `send_and_wait` function L33-60 — `def send_and_wait(ws, session_id, prompt)` — Send a message and wait for the Complete event.
- pub `load_session_jsonl` function L63-71 — `def load_session_jsonl(session_id)` — Load the session JSONL from disk.
- pub `analyze` function L74-170 — `def analyze(messages, scenario_name)` — Analyze session messages and print a report.
- pub `run_scenario` function L173-189 — `def run_scenario(prompt, name="test")` — Connect, send prompt, wait, analyze.

