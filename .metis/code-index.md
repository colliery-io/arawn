# Code Index

> Generated: 2026-04-18T13:48:23Z | 169 files | Python, Rust

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
│   │           ├── safe_env.rs
│   │           ├── sensitive_paths.rs
│   │           ├── shell.rs
│   │           ├── skill.rs
│   │           ├── sleep.rs
│   │           ├── task_list.rs
│   │           ├── task_output.rs
│   │           ├── task_stop.rs
│   │           ├── think.rs
│   │           ├── web_fetch.rs
│   │           ├── web_search.rs
│   │           └── workstream.rs
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
│   │   ├── src/
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
│   │       └── ws_client.rs
│   └── arawn-workflow/
│       ├── build.rs
│       └── src/
│           ├── agent_executor.rs
│           ├── lib.rs
│           ├── runner.rs
│           ├── scaffold.rs
│           └── tools.rs
└── scripts/
    └── functional_test.py
```

## Modules

### crates/arawn

> *Semantic summary to be generated by AI agent.*

#### crates/arawn/build.rs

-  `main` function L1-3 — `()`

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

- pub `LlmConfig` struct L9-30 — `{ provider: String, model: String, api_key_env: String, base_url: Option<String>...` — A named LLM provider configuration.
- pub `to_resolved_info` function L63-71 — `(&self) -> arawn_tool::ResolvedLlmInfo` — Project this config into the capability metadata used by
- pub `EngineConfig` struct L75-82 — `{ llm: String, max_iterations: usize, max_result_size: usize }`
- pub `CompactorConfig` struct L105-113 — `{ llm: Option<String>, compaction_threshold: f32, keep_recent: usize }`
- pub `ServerConfig` struct L133-138 — `{ host: String, port: u16 }`
- pub `StorageConfig` struct L157-160 — `{ data_dir: String }`
- pub `PromptsConfig` struct L175-178 — `{ token_budget: u32 }`
- pub `SandboxConfig` struct L194-200 — `{ network_tools: Vec<String> }` — Sandbox configuration for shell command execution.
- pub `ArawnConfig` struct L250-265 — `{ llm: HashMap<String, LlmConfig>, engine: EngineConfig, compactor: CompactorCon...` — Top-level configuration.
- pub `load` function L289-322 — `(data_dir: &Path) -> Self` — Load config from `data_dir/arawn.toml`, merging with env var overrides and defaults.
- pub `engine_llm` function L345-350 — `(&self) -> &LlmConfig` — Resolve the LLM config for the engine.
- pub `compactor_llm` function L353-360 — `(&self) -> &LlmConfig` — Resolve the LLM config for the compactor.
- pub `data_dir` function L363-365 — `(&self) -> PathBuf` — Resolve the data directory with ~ expansion.
- pub `prompts_dir` function L368-370 — `(&self) -> PathBuf` — Resolve the prompts directory.
- pub `resolve_api_key` function L373-377 — `(llm: &LlmConfig) -> Option<String>` — Resolve API key for an LLM config by reading the env var.
- pub `generate_default_toml` function L380-471 — `() -> String` — Generate a default config file string with comments.
-  `default_api_key_env` function L32-34 — `() -> String`
-  `default_context_window` function L35-37 — `() -> u32`
-  `default_max_tokens` function L38-40 — `() -> u32`
-  `default_tool_use` function L41-43 — `() -> bool`
-  `LlmConfig` type L45-58 — `impl Default for LlmConfig`
-  `default` function L46-57 — `() -> Self`
-  `LlmConfig` type L60-72 — `= LlmConfig`
-  `default_engine_llm` function L84-86 — `() -> String`
-  `default_max_iterations` function L87-89 — `() -> usize`
-  `default_max_result_size` function L90-92 — `() -> usize`
-  `EngineConfig` type L94-102 — `impl Default for EngineConfig`
-  `default` function L95-101 — `() -> Self`
-  `default_compaction_threshold` function L115-117 — `() -> f32`
-  `default_keep_recent` function L118-120 — `() -> usize`
-  `CompactorConfig` type L122-130 — `impl Default for CompactorConfig`
-  `default` function L123-129 — `() -> Self`
-  `default_host` function L140-142 — `() -> String`
-  `default_port` function L143-145 — `() -> u16`
-  `ServerConfig` type L147-154 — `impl Default for ServerConfig`
-  `default` function L148-153 — `() -> Self`
-  `default_data_dir` function L162-164 — `() -> String`
-  `StorageConfig` type L166-172 — `impl Default for StorageConfig`
-  `default` function L167-171 — `() -> Self`
-  `default_prompt_token_budget` function L180-182 — `() -> u32`
-  `PromptsConfig` type L184-190 — `impl Default for PromptsConfig`
-  `default` function L185-189 — `() -> Self`
-  `default_network_tools` function L202-238 — `() -> Vec<String>`
-  `SandboxConfig` type L240-246 — `impl Default for SandboxConfig`
-  `default` function L241-245 — `() -> Self`
-  `default_llm_configs` function L267-271 — `() -> HashMap<String, LlmConfig>`
-  `ArawnConfig` type L273-285 — `impl Default for ArawnConfig`
-  `default` function L274-284 — `() -> Self`
-  `ArawnConfig` type L287-472 — `= ArawnConfig`
-  `apply_env_overrides` function L324-342 — `(&mut self)`
-  `expand_tilde` function L474-481 — `(path: &str) -> PathBuf`
-  `tests` module L484-611 — `-`
-  `default_config_has_working_values` function L488-497 — `()`
-  `load_from_toml_string` function L500-520 — `()`
-  `compactor_falls_back_to_engine_llm` function L523-528 — `()`
-  `compactor_uses_own_llm_when_specified` function L531-550 — `()`
-  `missing_llm_name_falls_back_to_default_via_load` function L553-569 — `()`
-  `load_missing_file_uses_defaults` function L572-576 — `()`
-  `load_from_tempdir` function L579-597 — `()`
-  `generate_default_toml_is_parseable` function L600-604 — `()`
-  `tilde_expansion` function L607-610 — `()`

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
- pub `llm_pool` module L4 — `-`
- pub `local_service` module L5 — `-`
- pub `plugin_cmd` module L6 — `-`
- pub `ws_server` module L7 — `-`

#### crates/arawn/src/llm_pool.rs

- pub `LlmClientPool` struct L21-26 — `{ clients: HashMap<String, Arc<dyn LlmClient>>, configs: HashMap<String, LlmConf...` — A pool of named LLM clients built from an [`ArawnConfig`].
- pub `from_config` function L48-72 — `(config: &ArawnConfig, build: F) -> Result<Self>` — Build the pool from the given config.
- pub `from_clients` function L76-87 — `( clients: HashMap<String, Arc<dyn LlmClient>>, configs: HashMap<String, LlmConf...` — Construct a pool from a pre-built map of clients.
- pub `single` function L91-103 — `(client: Arc<dyn LlmClient>, model: impl Into<String>) -> Self` — Build a single-entry pool wrapping `client` as both engine and
- pub `get` function L106-108 — `(&self, name: &str) -> Option<Arc<dyn LlmClient>>` — Look up a client by name (e.g., "default", "cheap", "judge").
- pub `config` function L111-113 — `(&self, name: &str) -> Option<&LlmConfig>` — Get the [`LlmConfig`] for a named entry.
- pub `engine` function L116-118 — `(&self) -> Arc<dyn LlmClient>` — Engine LLM — never fails; falls back to whatever `engine_llm()` resolved.
- pub `engine_config` function L120-122 — `(&self) -> &LlmConfig` — surfaces here, not mid-session.
- pub `engine_name` function L124-126 — `(&self) -> &str` — surfaces here, not mid-session.
- pub `compactor` function L130-132 — `(&self) -> Arc<dyn LlmClient>` — Compactor LLM — never fails; falls back to engine LLM if `[compactor]`
- pub `compactor_config` function L134-136 — `(&self) -> &LlmConfig` — surfaces here, not mid-session.
- pub `compactor_name` function L138-140 — `(&self) -> &str` — surfaces here, not mid-session.
- pub `entries` function L143-145 — `(&self) -> impl Iterator<Item = (&String, &LlmConfig)>` — Iterator over (name, config) pairs.
- pub `resolve` function L155-216 — `(&self, preference: &LlmPreference) -> LlmResolution` — Resolve an [`LlmPreference`] against the pool.
- pub `len` function L218-220 — `(&self) -> usize` — surfaces here, not mid-session.
- pub `is_empty` function L222-224 — `(&self) -> bool` — surfaces here, not mid-session.
-  `LlmClientPool` type L28-32 — `impl LlmResolver for LlmClientPool` — surfaces here, not mid-session.
-  `resolve` function L29-31 — `(&self, preference: &LlmPreference) -> LlmResolution` — surfaces here, not mid-session.
-  `LlmClientPool` type L34-42 — `= LlmClientPool` — surfaces here, not mid-session.
-  `fmt` function L35-41 — `(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` — surfaces here, not mid-session.
-  `LlmClientPool` type L44-225 — `= LlmClientPool` — surfaces here, not mid-session.
-  `resolve_engine_name` function L227-241 — `( config: &ArawnConfig, clients: &HashMap<String, Arc<dyn LlmClient>>, ) -> Resu...` — surfaces here, not mid-session.
-  `resolve_compactor_name` function L243-251 — `(config: &ArawnConfig, engine_name: &str) -> String` — surfaces here, not mid-session.
-  `tests` module L254-514 — `-` — surfaces here, not mid-session.
-  `mock_builder` function L258-260 — `(_cfg: &LlmConfig) -> Result<Arc<dyn LlmClient>>` — surfaces here, not mid-session.
-  `cfg_from_toml` function L262-264 — `(toml_str: &str) -> ArawnConfig` — surfaces here, not mid-session.
-  `pool_builds_every_named_entry` function L267-287 — `()` — surfaces here, not mid-session.
-  `engine_and_compactor_resolve_distinct_clients_when_configured` function L290-314 — `()` — surfaces here, not mid-session.
-  `compactor_falls_back_to_engine_when_unconfigured` function L317-329 — `()` — surfaces here, not mid-session.
-  `compactor_falls_back_to_engine_when_pointing_at_missing_entry` function L332-345 — `()` — surfaces here, not mid-session.
-  `resolve_named_exact_match` function L348-364 — `()` — surfaces here, not mid-session.
-  `resolve_named_missing_falls_back` function L367-379 — `()` — surfaces here, not mid-session.
-  `resolve_provider_model_exact` function L382-401 — `()` — surfaces here, not mid-session.
-  `resolve_capability_match_when_no_exact` function L404-429 — `()` — surfaces here, not mid-session.
-  `resolve_capability_too_strict_falls_back` function L432-451 — `()` — surfaces here, not mid-session.
-  `resolve_empty_preference_is_fallback` function L454-465 — `()` — surfaces here, not mid-session.
-  `resolve_provider_only_uses_capability_path` function L468-488 — `()` — surfaces here, not mid-session.
-  `pool_construction_fails_fast_when_builder_errors` function L491-513 — `()` — surfaces here, not mid-session.

#### crates/arawn/src/local_service.rs

- pub `LocalService` struct L31-60 — `{ store: Arc<Mutex<Store>>, data_dir: PathBuf, llm_pool: Arc<LlmClientPool>, reg...` — In-process implementation of ArawnService.
- pub `new` function L63-87 — `( store: Store, data_dir: PathBuf, llm_pool: Arc<LlmClientPool>, registry: Arc<T...`
- pub `with_permission_rules` function L89-92 — `(self, rules: Vec<PermissionRule>) -> Self`
- pub `shared_store` function L96-98 — `(&self) -> Arc<Mutex<Store>>` — Get a reference to the shared permission rules for hot-reload.
- pub `shared_llm` function L100-102 — `(&self) -> Arc<dyn LlmClient>`
- pub `shared_compactor_llm` function L106-108 — `(&self) -> Arc<dyn LlmClient>` — Compactor LLM (separate client when `[compactor]` config selects a
- pub `compactor_model` function L111-113 — `(&self) -> &str` — Model name used by the compactor.
- pub `shared_llm_pool` function L117-119 — `(&self) -> Arc<LlmClientPool>` — Shared reference to the LLM pool — used by tools/agents that resolve
- pub `shared_registry` function L121-123 — `(&self) -> Arc<ToolRegistry>`
- pub `engine_config` function L125-127 — `(&self) -> &QueryEngineConfig`
- pub `shared_permission_rules` function L129-131 — `(&self) -> Arc<std::sync::RwLock<Vec<PermissionRule>>>`
- pub `shared_permission_mode` function L133-135 — `(&self) -> Arc<std::sync::RwLock<arawn_engine::permissions::PermissionMode>>`
- pub `with_skill_registry` function L137-140 — `(mut self, registry: Arc<arawn_engine::skills::SkillRegistry>) -> Self`
- pub `with_plugin_registry` function L142-145 — `(mut self, registry: Arc<arawn_engine::plugins::PluginRegistry>) -> Self`
- pub `with_plan_state` function L147-150 — `(mut self, state: Arc<PlanModeState>) -> Self`
- pub `with_background_tasks` function L152-155 — `(mut self, manager: Arc<BackgroundTaskManager>) -> Self`
- pub `with_memory_manager` function L157-160 — `(mut self, mgr: Arc<arawn_memory::MemoryManager>) -> Self`
-  `LocalService` type L62-317 — `= LocalService`
-  `load_session_state` function L164-193 — `( &self, session_id: Uuid, ) -> Result<(arawn_storage::SessionMeta, Workstream, ...` — Load session metadata, resolve workstream, and load message history.
-  `build_session_context` function L197-264 — `( &self, session_id: Uuid, workstream: &Workstream, ws_dir: &str, workspace_dir:...` — Build a ToolContext and per-session PromptContext for the engine.
-  `build_engine` function L268-316 — `( &self, prompt_context: Option<arawn_engine::PromptContext>, event_tx: &mpsc::S...` — Build a QueryEngine configured with compactor, skills, plugins, and plan state.
-  `infer_entity_type` function L321-334 — `(text: &str) -> (arawn_memory::EntityType, String)` — Infer entity type from text patterns.
-  `LocalService` type L339-1020 — `impl ArawnService for LocalService`
-  `list_workstreams` function L340-355 — `(&self) -> Result<Vec<WorkstreamInfo>, ServiceError>`
-  `create_workstream` function L357-374 — `( &self, name: String, root_dir: PathBuf, ) -> Result<WorkstreamInfo, ServiceErr...`
-  `list_sessions` function L376-395 — `( &self, workstream_id: Option<Uuid>, ) -> Result<Vec<SessionInfo>, ServiceError...`
-  `create_session` function L397-418 — `( &self, workstream_id: Option<Uuid>, ) -> Result<SessionInfo, ServiceError>`
-  `load_session` function L420-447 — `(&self, id: Uuid) -> Result<SessionDetail, ServiceError>`
-  `send_message` function L450-647 — `( &self, session_id: Uuid, content: String, ) -> Result<Pin<Box<dyn futures::Str...`
-  `cancel` function L649-662 — `(&self, session_id: Uuid) -> Result<(), ServiceError>`
-  `promote_session` function L664-715 — `( &self, session_id: Uuid, workstream_name: &str, ) -> Result<PromotionResult, S...`
-  `resolve_user_input` function L717-731 — `( &self, request_id: &str, selected_index: Option<usize>, ) -> Result<(), Servic...`
-  `query_inventory` function L733-798 — `(&self, kind: &str) -> Result<Vec<InventoryItem>, ServiceError>`
-  `list_available_commands` function L800-812 — `(&self) -> Result<Vec<CommandInfo>, ServiceError>`
-  `list_workflows` function L814-846 — `(&self) -> Result<Vec<WorkflowInfo>, ServiceError>`
-  `remember_fact` function L848-894 — `(&self, text: &str) -> Result<MemoryStoreResult, ServiceError>`
-  `memory_summary` function L896-943 — `(&self) -> Result<MemorySummary, ServiceError>`
-  `forget_entity` function L945-995 — `(&self, query: &str) -> Result<ForgetResult, ServiceError>`
-  `get_permission_mode` function L997-1005 — `(&self) -> Result<PermissionModeInfo, ServiceError>`
-  `set_permission_mode` function L1007-1019 — `(&self, mode_str: &str) -> Result<PermissionModeInfo, ServiceError>`
-  `resolve_ws_dir_from_store` function L1023-1034 — `(store: &Store, ws_id: Option<Uuid>) -> Result<String, ServiceError>` — Resolve workstream directory name from store.
-  `first_sentence` function L1038-1049 — `(s: &str) -> String` — Extract the first sentence and sanitize for use in a markdown table cell.

#### crates/arawn/src/main.rs

-  `DEFAULT_MODEL` variable L15 — `: &str`
-  `FILE_LOG_FILTER` variable L18 — `: &str` — Default file log filter: debug for arawn crates, warn for third-party.
-  `main` function L21-443 — `() -> Result<()>`
-  `Cli` struct L27-46 — `{ command: Option<Command>, data_dir: Option<String>, session: Option<Uuid>, lis...`
-  `Command` enum L49-68 — `Serve | Tui | Plugin`
-  `run_cli_via_server` function L446-552 — `( url: &str, prompt: &str, session_id: Option<Uuid>, ) -> Result<()>` — Run a CLI prompt by connecting to the running server via WebSocket.
-  `build_llm_client` function L555-577 — `( config: &arawn_bin::LlmConfig, ) -> Result<Arc<dyn arawn_llm::LlmClient>>` — Build the appropriate LLM client based on provider config.
-  `register_default_tools` function L580-626 — `( registry: &Arc<arawn_engine::ToolRegistry>, config: &arawn_bin::ArawnConfig, d...` — Register all default tools into the registry.
-  `connect_mcp_servers` function L629-677 — `( data_dir: &str, plugin_result: &arawn_engine::plugins::PluginLoadResult, regis...` — Connect to MCP servers from config and plugins.
-  `register_workflow_tools` function L680-697 — `( registry: &Arc<arawn_engine::ToolRegistry>, workflows_dir: std::path::PathBuf,...` — Register workflow management tools.
-  `build_engine_config` function L699-731 — `( config: &arawn_bin::ArawnConfig, workstream: &arawn_core::Workstream, data_dir...`
-  `dirs_path` function L733-742 — `() -> Option<String>`

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

- pub `read_token_file` function L119-130 — `() -> Option<String>` — Read the auth token from {data_dir}/server.token.
- pub `run_server` function L133-168 — `(service: LocalService, port: u16) -> anyhow::Result<()>` — Start the WebSocket server on the given port.
- pub `handle_connection_public` function L254-256 — `(socket: WebSocket, service: Arc<LocalService>)` — Handle a single WebSocket connection.
-  `PROTOCOL_VERSION` variable L24 — `: &str` — Protocol version reported by the `hello` handshake.
-  `RPC_METHODS` variable L27-46 — `: &[&str]` — Canonical RPC method names (returned by `hello`).
-  `Request` struct L50-55 — `{ id: u64, method: String, params: Value }` — JSON-RPC style request from client.
-  `Response` struct L59-65 — `{ id: u64, result: Option<Value>, error: Option<ErrorBody> }` — JSON-RPC style response to client.
-  `ErrorBody` struct L68-71 — `{ code: String, message: String }`
-  `Response` type L73-92 — `= Response`
-  `success` function L74-80 — `(id: u64, result: Value) -> Self`
-  `error` function L82-91 — `(id: u64, code: &str, message: String) -> Self`
-  `AppState` struct L96-101 — `{ service: Arc<LocalService>, auth_token: Option<String> }` — Shared app state for the WebSocket server.
-  `generate_auth_token` function L104-107 — `() -> String` — Generate a random auth token for WebSocket connections.
-  `write_token_file` function L110-115 — `(data_dir: &std::path::Path, token: &str) -> std::io::Result<std::path::PathBuf>` — Write the auth token to {data_dir}/server.token for clients to read.
-  `shutdown_signal` function L171-193 — `()` — Wait for a shutdown signal (Ctrl-C / SIGTERM).
-  `decision_handler` function L198-217 — `( State(AppState { service, .. }): State<AppState>, Json(req): Json<arawn_workfl...` — HTTP endpoint for workflow decision tasks.
-  `WsQueryParams` struct L221-223 — `{ token: Option<String> }` — Query parameters for WebSocket connection.
-  `ws_handler` function L225-251 — `( ws: WebSocketUpgrade, Query(params): Query<WsQueryParams>, State(state): State...`
-  `handle_connection` function L258-851 — `(socket: WebSocket, service: Arc<LocalService>)`

### crates/arawn-auth/src

> *Semantic summary to be generated by AI agent.*

#### crates/arawn-auth/src/error.rs

- pub `AuthError` enum L5-26 — `AuthExpired | ApiError | Network | InvalidConfig | Decode` — Errors raised by the auth primitives.

#### crates/arawn-auth/src/lib.rs

- pub `error` module L12 — `-` — Provides a provider-agnostic OAuth2 client (`OAuthClient`), a local
- pub `oauth2` module L13 — `-` — nothing else.
- pub `server` module L14 — `-` — nothing else.
- pub `token_store` module L15 — `-` — nothing else.

#### crates/arawn-auth/src/oauth2.rs

- pub `OAuthProviderConfig` struct L22-34 — `{ auth_url: Url, token_url: Url, client_id: String, client_secret: String, scope...` — Static configuration for an OAuth2 provider — not the user's credentials.
- pub `Token` struct L38-45 — `{ access: String, refresh: Option<String>, expires_at: Option<DateTime<Utc>>, sc...` — A user's OAuth credential — what `TokenStore` persists.
- pub `is_expired` function L52-57 — `(&self) -> bool` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.
- pub `AuthRequest` struct L62-69 — `{ authorization_url: Url, csrf_state: String, pkce_verifier: String }` — What `OAuthClient::start_flow` hands back.
- pub `OAuthClient` struct L71-74 — `{ config: OAuthProviderConfig, http: reqwest::Client }` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.
- pub `new` function L77-85 — `(config: OAuthProviderConfig) -> Self` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.
- pub `with_http` function L87-89 — `(config: OAuthProviderConfig, http: reqwest::Client) -> Self` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.
- pub `start_flow` function L97-124 — `(&self, redirect_uri: &Url) -> AuthRequest` — Generate a PKCE verifier + challenge + CSRF state and build the
- pub `exchange_code` function L127-144 — `( &self, code: &str, redirect_uri: &Url, pkce_verifier: &str, ) -> Result<Token,...` — Exchange an authorization code for a [`Token`].
- pub `refresh` function L147-170 — `(&self, refresh_token: &str) -> Result<Token, AuthError>` — Use a refresh token to mint a new access token.
-  `default_token_type` function L47-49 — `() -> String` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.
-  `Token` type L51-58 — `= Token` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.
-  `OAuthClient` type L76-206 — `= OAuthClient` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.
-  `post_token` function L172-205 — `(&self, form: &[(&str, &str)]) -> Result<Token, AuthError>` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.
-  `TokenResponse` struct L209-219 — `{ access_token: String, refresh_token: Option<String>, expires_in: Option<u64>, ...` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.
-  `generate_pkce_verifier` function L226-233 — `() -> String` — 64-character URL-safe random string.
-  `pkce_challenge_s256` function L235-238 — `(verifier: &str) -> String` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.
-  `generate_state` function L240-247 — `() -> String` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.
-  `tests` module L250-424 — `-` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.
-  `pkce_challenge_matches_rfc_7636_example` function L254-259 — `()` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.
-  `pkce_verifier_length` function L262-266 — `()` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.
-  `state_length` function L269-272 — `()` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.
-  `start_flow_includes_required_params` function L275-294 — `()` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.
-  `spawn_token_stub` function L299-343 — `( status: u16, body: &'static str, ) -> (Url, tokio::task::JoinHandle<Vec<u8>>)` — Tiny in-process HTTP stub for the OAuth token endpoint.
-  `client_with_token_url` function L345-353 — `(token_url: Url) -> OAuthClient` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.
-  `exchange_code_decodes_token_response` function L356-372 — `()` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.
-  `refresh_failure_with_400_returns_auth_expired` function L375-382 — `()` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.
-  `refresh_preserves_refresh_token_when_provider_omits_it` function L385-393 — `()` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.
-  `token_is_expired_respects_expiration_time` function L396-423 — `()` — Uses reqwest for HTTP, sha2 + base64 for the PKCE challenge.

#### crates/arawn-auth/src/server.rs

- pub `CallbackResult` struct L22-25 — `{ code: String, state: String }` — What the callback yielded.
- pub `CallbackServer` struct L27-30 — `{ listener: TcpListener, redirect_uri: Url }` — responds with a small HTML success page, then shuts down.
- pub `bind` function L35-48 — `(path: &str) -> Result<Self, AuthError>` — Bind to an OS-assigned port on `127.0.0.1`.
- pub `redirect_uri` function L50-52 — `(&self) -> &Url` — responds with a small HTML success page, then shuts down.
- pub `listen` function L56-58 — `(self) -> Result<CallbackResult, AuthError>` — Wait up to [`DEFAULT_TIMEOUT`] for a single redirect, parse it, and
- pub `listen_with_timeout` function L60-156 — `( self, timeout: Duration, ) -> Result<CallbackResult, AuthError>` — responds with a small HTML success page, then shuts down.
-  `DEFAULT_TIMEOUT` variable L16 — `: Duration` — responds with a small HTML success page, then shuts down.
-  `SUCCESS_PAGE` variable L18 — `: &str` — responds with a small HTML success page, then shuts down.
-  `CallbackServer` type L32-157 — `= CallbackServer` — responds with a small HTML success page, then shuts down.
-  `tests` module L160-229 — `-` — responds with a small HTML success page, then shuts down.
-  `simulate_browser` function L165-177 — `(server_url: &Url, query: &str)` — responds with a small HTML success page, then shuts down.
-  `happy_path_returns_code_and_state` function L180-188 — `()` — responds with a small HTML success page, then shuts down.
-  `missing_code_yields_invalid_config_error` function L191-201 — `()` — responds with a small HTML success page, then shuts down.
-  `provider_error_propagates` function L204-214 — `()` — responds with a small HTML success page, then shuts down.
-  `timeout_returns_error` function L217-221 — `()` — responds with a small HTML success page, then shuts down.
-  `redirect_uri_normalizes_path_with_or_without_slash` function L224-228 — `()` — responds with a small HTML success page, then shuts down.

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
-  `tests` module L185-301 — `-` — System spec's security contract and the sensitive-paths deny list.
-  `sample_token` function L190-198 — `() -> Token` — System spec's security contract and the sensitive-paths deny list.
-  `save_then_load_round_trip` function L201-209 — `()` — System spec's security contract and the sensitive-paths deny list.
-  `load_missing_returns_none` function L212-216 — `()` — System spec's security contract and the sensitive-paths deny list.
-  `delete_then_load_returns_none` function L219-225 — `()` — System spec's security contract and the sensitive-paths deny list.
-  `delete_nonexistent_is_idempotent` function L228-232 — `()` — System spec's security contract and the sensitive-paths deny list.
-  `tampered_ciphertext_fails_decrypt` function L235-250 — `()` — System spec's security contract and the sensitive-paths deny list.
-  `second_open_reuses_master_key` function L253-262 — `()` — System spec's security contract and the sensitive-paths deny list.
-  `missing_master_key_after_save_fails_clearly` function L265-279 — `()` — System spec's security contract and the sensitive-paths deny list.
-  `provider_name_sanitization_rejects_path_chars` function L282-289 — `()` — System spec's security contract and the sensitive-paths deny list.
-  `master_key_has_restrictive_permissions` function L293-300 — `()` — System spec's security contract and the sensitive-paths deny list.

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
-  `resolve_model_dir` function L201-214 — `(config: &EmbeddingConfig) -> Result<PathBuf, EmbedError>` — Model files are downloaded to ~/.arawn/models/ on first use.
-  `download_model_files` function L216-248 — `(model_dir: &Path, model_name: &str) -> Result<(), EmbedError>` — Model files are downloaded to ~/.arawn/models/ on first use.
-  `tests` module L251-270 — `-` — Model files are downloaded to ~/.arawn/models/ on first use.
-  `resolve_default_dir` function L255-259 — `()` — Model files are downloaded to ~/.arawn/models/ on first use.
-  `resolve_custom_dir` function L262-269 — `()` — Model files are downloaded to ~/.arawn/models/ on first use.

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

- pub `EngineToolContext` struct L22-46 — `{ session_id: Uuid, working_dir: PathBuf, workstream_name: String, allowed_paths...` — Concrete execution context provided to tools within the engine.
- pub `new` function L62-76 — `(workstream: &Workstream, session_id: Uuid) -> Self`
- pub `with_llm_resolver` function L80-83 — `(mut self, resolver: Arc<dyn LlmResolver>) -> Self` — Attach an LLM resolver (typically `arawn-bin`'s `LlmClientPool`).
- pub `with_allowed_paths` function L86-89 — `(mut self, paths: Vec<PathBuf>) -> Self` — Set allowed paths that file tools can access outside the sandbox.
- pub `with_llm` function L92-96 — `(mut self, llm: Arc<dyn LlmClient>, model: String) -> Self` — Attach an LLM client and model for tools that need sub-queries.
- pub `with_model_limits` function L99-102 — `(mut self, limits: ModelLimits) -> Self` — Set model limits for sub-agent compaction.
- pub `with_data_dir` function L105-108 — `(mut self, dir: PathBuf) -> Self` — Set data directory for persisting large tool results.
-  `MAX_AGENT_DEPTH` variable L13 — `: u8` — Maximum sub-agent nesting depth.
-  `EngineToolContext` type L48-59 — `= EngineToolContext`
-  `fmt` function L49-58 — `(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result`
-  `EngineToolContext` type L61-109 — `= EngineToolContext`
-  `EngineToolContext` type L115-212 — `= EngineToolContext`
-  `working_dir` function L116-118 — `(&self) -> &Path`
-  `session_id` function L120-122 — `(&self) -> Uuid`
-  `validate_path` function L124-147 — `(&self, path_str: &str) -> Result<PathBuf, String>`
-  `is_allowed_path` function L149-158 — `(&self, path: &Path) -> bool`
-  `mark_file_read` function L160-162 — `(&self, path: PathBuf)`
-  `has_read_file` function L164-166 — `(&self, path: &Path) -> bool`
-  `llm` function L168-170 — `(&self) -> Option<&Arc<dyn LlmClient>>`
-  `model` function L172-174 — `(&self) -> Option<&str>`
-  `model_limits` function L176-178 — `(&self) -> &ModelLimits`
-  `data_dir` function L180-182 — `(&self) -> Option<&PathBuf>`
-  `agent_depth` function L184-186 — `(&self) -> u8`
-  `can_spawn_agent` function L188-190 — `(&self) -> bool`
-  `for_sub_agent` function L192-197 — `(&self) -> Box<dyn arawn_tool::ToolContext>`
-  `workstream_name` function L199-201 — `(&self) -> &str`
-  `allowed_paths` function L203-205 — `(&self) -> &[PathBuf]`
-  `resolve_llm` function L207-211 — `(&self, preference: &LlmPreference) -> Option<LlmResolution>`
-  `tests` module L215-238 — `-`
-  `context_from_workstream` function L220-228 — `()`
-  `context_is_clone` function L231-237 — `()`
-  `normalize_path_components` function L241-254 — `(path: &Path) -> PathBuf` — Normalize a path by resolving .

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
- pub `PromptContext` struct L46-57 — `{ prompts_dir: Option<std::path::PathBuf>, os: String, shell: String, cwd: std::...` — Cached context for building system prompts per-turn.
- pub `QueryEngineConfig` struct L61-72 — `{ model: String, max_iterations: usize, system_prompt: String, max_tokens: Optio...` — Configuration for the query engine.
- pub `QueryEngine` struct L89-110 — `{ llm: Arc<dyn LlmClient>, registry: Arc<ToolRegistry>, config: QueryEngineConfi...` — The agentic loop: prompt → LLM → tool_use → execute → feed result → loop.
- pub `new` function L113-130 — `(llm: Arc<dyn LlmClient>, registry: Arc<ToolRegistry>) -> Self`
- pub `with_config` function L132-153 — `( llm: Arc<dyn LlmClient>, registry: Arc<ToolRegistry>, config: QueryEngineConfi...`
- pub `with_compactor` function L155-158 — `(mut self, compactor: Compactor) -> Self`
- pub `with_permission_checker` function L160-163 — `(mut self, checker: Arc<PermissionChecker>) -> Self`
- pub `with_hook_runner` function L165-168 — `(mut self, runner: Arc<HookRunner>) -> Self`
- pub `with_skill_registry` function L170-173 — `(mut self, registry: Arc<crate::skills::SkillRegistry>) -> Self`
- pub `with_plugin_registry` function L175-178 — `(mut self, registry: Arc<crate::plugins::PluginRegistry>) -> Self`
- pub `with_plan_state` function L180-183 — `(mut self, plan_state: Arc<PlanModeState>) -> Self`
- pub `plan_state` function L186-188 — `(&self) -> Option<&Arc<PlanModeState>>` — Get the plan mode state (if configured).
- pub `with_background_tasks` function L190-193 — `(mut self, manager: Arc<BackgroundTaskManager>) -> Self`
- pub `with_progress_sender` function L196-199 — `(mut self, tx: tokio::sync::mpsc::Sender<ProgressEvent>) -> Self` — Set a channel for live progress events during the engine loop.
- pub `with_cancel_token` function L202-205 — `(mut self, token: tokio_util::sync::CancellationToken) -> Self` — Set a cancellation token — checked at each loop iteration and before tool execution.
- pub `fire_hook` function L224-230 — `(&self, input: &HookInput) -> Option<crate::hooks::AggregatedHookResult>` — Fire a hook event.
- pub `run` function L233-553 — `( &mut self, session: &mut Session, ctx: &dyn arawn_tool::ToolContext, ) -> Resu...` — Run the agentic loop for a session.
-  `DEFAULT_MAX_ITERATIONS` variable L18 — `: usize`
-  `MAX_COMPACT_FAILURES` variable L19 — `: u32`
-  `DEFAULT_SYSTEM_PROMPT` variable L42 — `: &str`
-  `QueryEngineConfig` type L74-86 — `impl Default for QueryEngineConfig`
-  `default` function L75-85 — `() -> Self`
-  `QueryEngine` type L112-867 — `= QueryEngine`
-  `is_cancelled` function L208-210 — `(&self) -> bool` — Check if cancellation has been requested.
-  `emit_progress` function L213-217 — `(&self, event: ProgressEvent)` — Emit a progress event if a sender is configured.
-  `build_request` function L555-645 — `(&self, session: &Session) -> ChatRequest`
-  `stream_response_with_retry` function L650-683 — `( &self, session: &Session, _ctx: &dyn arawn_tool::ToolContext, ) -> Result<Asse...` — Build the request and stream with up to 2 retries on transient LLM errors
-  `MAX_RETRIES` variable L655 — `: u32`
-  `stream_response` function L685-745 — `( &self, request: ChatRequest, ) -> Result<AssembledResponse, EngineError>`
-  `execute_tool` function L747-866 — `( &self, ctx: &dyn arawn_tool::ToolContext, tool_use_id: &str, name: &str, argum...`
-  `parse_arguments` function L869-878 — `(raw: &str) -> serde_json::Value`
-  `AssembledResponse` struct L881-885 — `{ text: String, tool_calls: Vec<AssembledToolCall>, usage: Option<arawn_llm::Usa...`
-  `AssembledToolCall` struct L887-891 — `{ id: String, name: String, arguments: serde_json::Value }`
-  `ToolResult` struct L893-896 — `{ content: String, is_error: bool }`
-  `filter_tools_for_context` function L901-1013 — `( all_tools: &[arawn_llm::ToolDefinition], session: &Session, registry: &ToolReg...` — Filter tool definitions to only contextually relevant ones for this turn.
-  `tests` module L1016-1204 — `-`
-  `MockLlm` struct L1028-1030 — `{ responses: Mutex<Vec<Vec<ChatChunk>>> }` — Mock LLM that returns pre-scripted responses.
-  `MockLlm` type L1032-1062 — `= MockLlm`
-  `new` function L1033-1037 — `(responses: Vec<Vec<ChatChunk>>) -> Self`
-  `text` function L1040-1047 — `(text: &str) -> Vec<ChatChunk>` — Convenience: text-only response
-  `tool_call` function L1050-1061 — `(id: &str, name: &str, args: &str) -> Vec<ChatChunk>` — Convenience: tool call then done
-  `MockLlm` type L1065-1081 — `impl LlmClient for MockLlm`
-  `stream` function L1066-1080 — `( &self, _request: ChatRequest, ) -> Result< Pin<Box<dyn futures::Stream<Item = ...`
-  `setup` function L1083-1088 — `() -> (Workstream, Session, EngineToolContext)`
-  `text_only_response` function L1091-1104 — `()`
-  `single_tool_call` function L1107-1125 — `()`
-  `tool_not_found` function L1128-1150 — `()`
-  `max_iterations_exceeded` function L1153-1180 — `()`
-  `multi_turn_tool_chain` function L1183-1202 — `()`

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
- pub `plugin_prompts` function L297-313 — `(mut self, prompts: &[String]) -> Self` — Add plugin-contributed prompt fragments.
- pub `build` function L316-338 — `(mut self) -> String` — Build the final system prompt string, enforcing token budget.
- pub `ContextFile` struct L351-355 — `{ path: std::path::PathBuf, content: String, truncated: bool }` — A context file loaded from disk.
- pub `find_context_files` function L358-374 — `(workstream_root: &Path, global_dir: &Path) -> Vec<ContextFile>` — Load context files from workstream root and global config dir.
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
-  `SystemPromptBuilder` type L156-339 — `= SystemPromptBuilder`
-  `SystemPromptBuilder` type L341-345 — `impl Default for SystemPromptBuilder`
-  `default` function L342-344 — `() -> Self`
-  `load_context_file` function L376-395 — `(path: &Path, max_chars: usize) -> Option<ContextFile>`
-  `truncate_70_20` function L398-421 — `(content: &str, max_chars: usize) -> String` — Truncate keeping 70% from the head and 20% from the tail, with a marker in between.
-  `load_section` function L425-433 — `(name: &str, default: &str, prompts_dir: Option<&Path>) -> String`
-  `tests` module L436-751 — `-`
-  `default_assembly_includes_all_static_sections` function L443-459 — `()`
-  `sections_have_headers` function L463-474 — `()`
-  `empty_optional_sections_omitted` function L478-489 — `()`
-  `single_section_override` function L493-504 — `()`
-  `partial_overrides_other_sections_use_defaults` function L508-520 — `()`
-  `missing_override_dir_uses_defaults` function L524-530 — `()`
-  `empty_override_file_produces_empty_section` function L534-544 — `()`
-  `under_budget_all_sections_included` function L548-559 — `()`
-  `over_budget_drops_low_priority_sections` function L563-573 — `()`
-  `identity_survives_budget_cuts` function L577-586 — `()`
-  `truncation_produces_clean_sections` function L590-602 — `()`
-  `context_file_injected` function L606-617 — `()`
-  `context_file_missing_section_omitted` function L621-628 — `()`
-  `large_context_file_truncated` function L632-643 — `()`
-  `tools_section_reflects_tool_list` function L647-666 — `()`
-  `per_turn_freshness_different_tools` function L670-694 — `()`
-  `environment_section_contains_info` function L698-707 — `()`
-  `workstream_section_contains_info` function L711-718 — `()`
-  `snapshot_full_build` function L722-750 — `()`

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
- pub `check` function L233-273 — `(&self, tool_name: &str, tool_input: &str) -> PermissionDecision` — Check if a tool call is permitted.
- pub `mode` function L309-311 — `(&self) -> PermissionMode` — Get the current permission mode.
- pub `clear_grants` function L314-316 — `(&self)` — Clear all session grants.
-  `PermissionMode` type L71-101 — `= PermissionMode`
-  `ModalOption` type L118-130 — `= ModalOption`
-  `SessionGrants` type L156-175 — `= SessionGrants`
-  `PermissionChecker` type L186-317 — `= PermissionChecker`
-  `prompt_user` function L276-306 — `(&self, tool_name: &str, tool_input: &str) -> PermissionDecision` — Prompt the user for permission (or deny if no prompter is configured).
-  `truncate_input` function L319-327 — `(input: &str, max_len: usize) -> String`
-  `tests` module L330-705 — `-`
-  `MockPrompter` struct L335-337 — `{ index: Option<usize> }` — Mock prompter that returns a fixed index (0=AllowOnce, 1=AllowAlways, 2/None=Deny).
-  `MockPrompter` type L339-343 — `= MockPrompter`
-  `allow_once` function L340 — `() -> Self`
-  `allow_always` function L341 — `() -> Self`
-  `deny` function L342 — `() -> Self`
-  `MockPrompter` type L346-350 — `impl ModalPrompt for MockPrompter`
-  `prompt` function L347-349 — `(&self, _request: ModalRequest) -> Option<usize>`
-  `allowed_by_rule` function L353-360 — `()`
-  `denied_by_rule` function L363-370 — `()`
-  `ask_without_prompter_denies` function L373-380 — `()`
-  `ask_with_allow_once` function L383-392 — `()`
-  `ask_with_allow_always_grants_session` function L395-408 — `()`
-  `ask_with_deny` function L411-418 — `()`
-  `default_mode_allows_read_only` function L421-440 — `()`
-  `default_mode_asks_for_writes` function L443-458 — `()`
-  `accept_edits_mode_allows_file_ops` function L461-481 — `()`
-  `bypass_mode_allows_everything` function L484-502 — `()`
-  `explicit_rules_override_mode` function L505-513 — `()`
-  `deny_rules_override_session_grants` function L516-525 — `()`
-  `session_grant_works_for_non_denied_tools` function L528-537 — `()`
-  `clear_grants_resets` function L540-549 — `()`
-  `truncate_input_short` function L552-554 — `()`
-  `truncate_input_long` function L557-561 — `()`
-  `truncate_input_multibyte_utf8_no_panic` function L564-572 — `()`
-  `tool_categories` function L575-588 — `()`
-  `update_rules_hot_reload` function L591-612 — `()`
-  `update_mode_hot_reload` function L615-637 — `()`
-  `permission_mode_serde` function L640-649 — `()`
-  `plan_mode_allows_read_only` function L652-670 — `()`
-  `plan_mode_denies_writes` function L673-691 — `()`
-  `plan_mode_allows_plan_meta_tools` function L694-704 — `()`

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

- pub `PluginMcpServer` struct L26-32 — `{ name: String, command: String, args: Vec<String>, env: std::collections::HashM...` — An MCP server config extracted from a plugin manifest, ready for connection.
- pub `PluginLoadResult` struct L35-40 — `{ agents: Vec<AgentDefinition>, skills: Vec<SkillDefinition>, hooks: HookConfig,...` — Result of loading all plugins — the components ready to wire into the engine.
- pub `PluginRuntime` struct L43-52 — `{ plugins_root: PathBuf, settings_path: Option<PathBuf>, plugin_dirs: Vec<PathBu...` — Plugin runtime — manages plugin lifecycle for a running arawn instance.
- pub `new` function L55-62 — `(plugins_root: PathBuf) -> Self` — to hot-reload when plugins are installed or changed.
- pub `with_settings` function L64-67 — `(mut self, path: PathBuf) -> Self` — to hot-reload when plugins are installed or changed.
- pub `with_plugin_dir` function L69-72 — `(mut self, dir: PathBuf) -> Self` — to hot-reload when plugins are installed or changed.
- pub `load_all` function L75-162 — `(&self, skill_registry: &Arc<SkillRegistry>) -> PluginLoadResult` — Discover, load, and register all plugins.
- pub `watch` function L168-277 — `(&self, skill_registry: Arc<SkillRegistry>) -> tokio::task::JoinHandle<()>` — Spawn a file watcher that hot-reloads plugins when the cache directory changes.
-  `PluginRuntime` type L54-278 — `= PluginRuntime` — to hot-reload when plugins are installed or changed.

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
- pub `new` function L15-21 — `() -> Self`
- pub `register` function L43-46 — `(&self, skill: SkillDefinition)` — Register a skill.
- pub `get` function L49-61 — `(&self, name: &str) -> Option<SkillDefinition>` — Look up a skill by name (case-insensitive).
- pub `all` function L64-66 — `(&self) -> Vec<SkillDefinition>` — Get all registered skills.
- pub `user_invocable` function L69-77 — `(&self) -> Vec<SkillDefinition>` — Get only user-invocable skills.
- pub `len` function L80-82 — `(&self) -> usize` — Number of registered skills.
- pub `is_empty` function L84-86 — `(&self) -> bool`
- pub `load_skills_dir` function L94-136 — `(dir: &Path, source: SkillSource) -> Vec<SkillDefinition>` — Load skill definitions from a directory.
- pub `load_merged_skills` function L163-184 — `( project_dir: Option<&Path>, user_dir: Option<&Path>, ) -> SkillRegistry` — Load and merge skills from project and user directories.
- pub `format_skill_listing` function L190-226 — `(skills: &[SkillDefinition], budget_chars: usize, max_desc_chars: usize) -> Stri...` — Format skill listing for the system prompt, respecting a character budget.
-  `SkillRegistry` type L14-87 — `= SkillRegistry`
-  `register_builtins` function L24-40 — `(&self)` — Register built-in skills that ship with the arawn binary.
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

> *Semantic summary to be generated by AI agent.*

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
-  `tests` module L303-577 — `-`
-  `test_ctx_with_mock` function L312-321 — `( responses: Vec<MockResponse>, ) -> (EngineToolContext, Arc<MockLlmClient>, Arc...`
-  `schema_is_valid` function L324-333 — `()`
-  `text_only_sub_agent` function L336-353 — `()`
-  `TestResolver` struct L356-360 — `{ named_client: Arc<dyn arawn_llm::LlmClient>, named_model: String, named: Strin...` — Test resolver that knows about a single named entry.
-  `TestResolver` type L362-392 — `= TestResolver`
-  `resolve` function L363-391 — `(&self, pref: &arawn_tool::LlmPreference) -> arawn_tool::LlmResolution`
-  `sub_agent_uses_resolved_llm_preference` function L395-429 — `()`
-  `sub_agent_falls_back_to_parent_llm_when_resolution_unavailable` function L432-449 — `()`
-  `sub_agent_with_tool_call` function L452-469 — `()`
-  `sub_agent_no_llm_errors` function L472-481 — `()`
-  `sub_agent_max_iterations_returns_last_text` function L484-506 — `()`
-  `depth_limit_prevents_infinite_recursion` function L509-523 — `()`
-  `explore_agent_type_used` function L526-542 — `()`
-  `unknown_type_falls_back_to_general` function L545-559 — `()`
-  `for_sub_agent_increments_depth` function L562-576 — `()`

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

#### crates/arawn-engine/src/tools/file_edit.rs

- pub `FileEditTool` struct L8 — `-` — Edit a file by replacing a string.
-  `FileEditTool` type L11-159 — `impl Tool for FileEditTool`
-  `name` function L12-14 — `(&self) -> &str`
-  `description` function L16-26 — `(&self) -> &str`
-  `parameters_schema` function L28-51 — `(&self) -> Value`
-  `execute` function L53-158 — `(&self, ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput, ...`
-  `tests` module L162-338 — `-`
-  `test_ctx` function L170-173 — `(dir: &std::path::Path) -> EngineToolContext`
-  `mark_read` function L176-179 — `(ctx: &EngineToolContext, dir: &std::path::Path, name: &str)` — Mark a file as read in the context (simulates a prior file_read call).
-  `edit_replaces_string` function L182-203 — `()`
-  `edit_fails_on_missing_string` function L206-224 — `()`
-  `edit_fails_on_ambiguous_match` function L227-245 — `()`
-  `edit_replace_all` function L248-269 — `()`
-  `edit_rejects_path_traversal` function L272-286 — `()`
-  `edit_fails_without_prior_read` function L289-307 — `()`
-  `edit_rejects_secret_filename` function L310-328 — `()`
-  `schema_is_valid` function L331-337 — `()`

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
-  `FileWriteTool` type L12-145 — `impl Tool for FileWriteTool`
-  `name` function L13-15 — `(&self) -> &str`
-  `description` function L17-26 — `(&self) -> &str`
-  `parameters_schema` function L28-43 — `(&self) -> Value`
-  `execute` function L45-144 — `(&self, ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput, ...`
-  `normalize_path` function L147-159 — `(path: &std::path::Path) -> std::path::PathBuf`
-  `tests` module L162-311 — `-`
-  `test_ctx` function L170-173 — `(dir: &std::path::Path) -> EngineToolContext`
-  `mark_read` function L175-178 — `(ctx: &EngineToolContext, path: &std::path::Path)`
-  `write_creates_file` function L181-197 — `()`
-  `write_creates_parent_dirs` function L200-215 — `()`
-  `write_overwrites_existing` function L218-236 — `()`
-  `write_rejects_path_traversal` function L239-254 — `()`
-  `write_new_file_without_read_ok` function L257-268 — `()`
-  `write_existing_file_without_read_fails` function L271-286 — `()`
-  `write_rejects_secret_filename` function L289-301 — `()`
-  `schema_is_valid` function L304-310 — `()`

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
-  `run_rg` function L231-308 — `( cwd: &std::path::Path, pattern: &str, path: &str, glob: Option<&str>, file_typ...`
-  `run_grep_fallback` function L310-346 — `( cwd: &std::path::Path, pattern: &str, path: &str, case_insensitive: bool, outp...`
-  `tests` module L349-569 — `-`
-  `test_ctx` function L356-359 — `(dir: &std::path::Path) -> EngineToolContext`
-  `grep_finds_matches` function L362-380 — `()`
-  `grep_no_matches` function L383-397 — `()`
-  `grep_case_insensitive` function L400-414 — `()`
-  `grep_with_glob` function L417-432 — `()`
-  `grep_content_mode` function L435-453 — `()`
-  `grep_files_with_matches_mode` function L456-475 — `()`
-  `grep_head_limit` function L478-501 — `()`
-  `schema_is_valid` function L504-513 — `()`
-  `grep_path_traversal_rejected` function L516-534 — `()`
-  `grep_absolute_path_rejected` function L537-549 — `()`
-  `grep_relative_path_within_root_allowed` function L552-568 — `()`

#### crates/arawn-engine/src/tools/memory_search.rs

- pub `MemorySearchTool` struct L15-18 — `{ memory: Arc<MemoryManager>, embedder: Option<Arc<dyn Embedder>> }` — Tool that searches the knowledge base using composite retrieval:
- pub `new` function L21-23 — `(memory: Arc<MemoryManager>, embedder: Option<Arc<dyn Embedder>>) -> Self`
-  `MemorySearchTool` type L20-24 — `= MemorySearchTool`
-  `MemorySearchTool` type L27-261 — `impl Tool for MemorySearchTool`
-  `name` function L28-30 — `(&self) -> &str`
-  `description` function L32-36 — `(&self) -> &str`
-  `is_read_only` function L38-40 — `(&self) -> bool`
-  `category` function L42-44 — `(&self) -> ToolCategory`
-  `parameters_schema` function L46-80 — `(&self) -> Value`
-  `execute` function L82-260 — `(&self, _ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput,...`
-  `ScoredEntity` struct L263-269 — `{ entity: Entity, fts_score: f32, semantic_score: f32, confidence: f32, related:...`
-  `ScoredEntity` type L271-275 — `= ScoredEntity`
-  `composite` function L272-274 — `(&self) -> f32`
-  `tests` module L278-389 — `-`
-  `setup` function L285-292 — `() -> (TempDir, Arc<MemoryManager>, crate::context::EngineToolContext)`
-  `populate` function L294-316 — `(mgr: &MemoryManager)`
-  `search_fts_both_tiers` function L319-332 — `()`
-  `search_with_type_filter` function L335-347 — `()`
-  `search_global_only` function L350-361 — `()`
-  `search_no_results` function L364-374 — `()`
-  `search_with_tags` function L377-388 — `()`

#### crates/arawn-engine/src/tools/memory_store.rs

- pub `MemoryStoreTool` struct L15-18 — `{ memory: Arc<MemoryManager>, embedder: Option<Arc<dyn Embedder>> }` — Tool that stores knowledge in the KB with search-before-create deduplication.
- pub `new` function L21-23 — `(memory: Arc<MemoryManager>, embedder: Option<Arc<dyn Embedder>>) -> Self`
-  `MemoryStoreTool` type L20-24 — `= MemoryStoreTool`
-  `MemoryStoreTool` type L27-204 — `impl Tool for MemoryStoreTool`
-  `name` function L28-30 — `(&self) -> &str`
-  `description` function L32-43 — `(&self) -> &str`
-  `category` function L45-47 — `(&self) -> ToolCategory`
-  `parameters_schema` function L49-79 — `(&self) -> Value`
-  `execute` function L81-203 — `(&self, ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput, ...`
-  `tests` module L207-316 — `-`
-  `setup` function L214-223 — `() -> (TempDir, Arc<MemoryManager>, crate::context::EngineToolContext)`
-  `store_new_fact` function L226-238 — `()`
-  `store_preference_goes_global` function L241-251 — `()`
-  `store_decision_goes_workstream` function L254-264 — `()`
-  `store_reinforces_duplicate` function L267-282 — `()`
-  `store_with_tags` function L285-298 — `()`
-  `store_with_explicit_scope_override` function L301-315 — `()`

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
- pub `safe_env` module L12 — `-`
- pub `sensitive_paths` module L13 — `-`
- pub `shell` module L14 — `-`
- pub `skill` module L15 — `-`
- pub `sleep` module L16 — `-`
- pub `task_list` module L17 — `-`
- pub `task_output` module L18 — `-`
- pub `task_stop` module L19 — `-`
- pub `think` module L20 — `-`
- pub `web_fetch` module L21 — `-`
- pub `web_search` module L22 — `-`
- pub `workstream` module L23 — `-`

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

- pub `ShellTool` struct L23-28 — `{ network_tools: Vec<String>, bg_manager: Option<Arc<BackgroundTaskManager>> }` — Execute a shell command within an OS-level sandbox.
- pub `with_network_tools` function L43-48 — `(network_tools: Vec<String>) -> Self` — Create a ShellTool with the given list of network-allowed tool binaries.
- pub `with_background_manager` function L51-54 — `(mut self, mgr: Arc<BackgroundTaskManager>) -> Self` — Attach a background task manager for `run_in_background` support.
-  `DEFAULT_TIMEOUT_MS` variable L30 — `: u64`
-  `ShellTool` type L32-39 — `impl Default for ShellTool`
-  `default` function L33-38 — `() -> Self`
-  `ShellTool` type L41-220 — `= ShellTool`
-  `spawn_background` function L62-219 — `( &self, command: &str, working_dir: &std::path::Path, ) -> Result<ToolOutput, T...` — Spawn a shell command as a background task.
-  `init_sandbox_for_background` function L226-261 — `( command: &str, working_dir: &std::path::Path, network_tools: &[String], ) -> R...` — Initialize a sandbox manager for a background command and return it together
-  `command_needs_network` function L265-284 — `(command: &str, network_tools: &[String]) -> bool` — Check if a command invokes any tool that needs network access.
-  `build_sandbox_config` function L287-336 — `( command: &str, working_dir: &std::path::Path, network_tools: &[String], ) -> S...` — Build a sandbox config for executing a command in the given working directory.
-  `ShellTool` type L339-423 — `impl Tool for ShellTool`
-  `name` function L340-342 — `(&self) -> &str`
-  `description` function L344-359 — `(&self) -> &str`
-  `parameters_schema` function L361-380 — `(&self) -> Value`
-  `execute` function L382-422 — `(&self, ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput, ...`
-  `SandboxExecError` enum L425-430 — `Unavailable | Tool`
-  `execute_sandboxed` function L432-521 — `( command: &str, working_dir: &std::path::Path, timeout_ms: u64, network_tools: ...`
-  `execute_unsandboxed` function L523-569 — `( command: &str, working_dir: &std::path::Path, timeout_ms: u64, ) -> Result<Too...`
-  `tests` module L572-1001 — `-`
-  `test_ctx` function L580-583 — `() -> EngineToolContext`
-  `test_ctx_in` function L585-588 — `(dir: &std::path::Path) -> EngineToolContext`
-  `shell_echo` function L592-600 — `()`
-  `shell_nonzero_exit` function L604-612 — `()`
-  `shell_timeout` function L616-627 — `()`
-  `shell_missing_command` function L631-635 — `()`
-  `shell_env_does_not_leak_secrets` function L639-664 — `()`
-  `background_command_runs_sandboxed` function L668-702 — `()`
-  `background_command_sandbox_blocks_sensitive_read` function L706-752 — `()`
-  `shell_env_preserves_path` function L756-764 — `()`
-  `shell_schema_is_valid` function L767-772 — `()`
-  `sensitive_paths_includes_ssh` function L775-778 — `()`
-  `sensitive_paths_includes_aws` function L781-784 — `()`
-  `sandbox_config_allows_working_dir_and_tmp` function L787-798 — `()`
-  `network_detection_recognizes_tools` function L801-808 — `()`
-  `network_detection_blocks_unknown` function L811-816 — `()`
-  `network_detection_empty_list_blocks_all` function L819-822 — `()`
-  `sandbox_write_inside_allowed` function L828-847 — `()`
-  `sandbox_mkdir_inside_allowed` function L851-872 — `()`
-  `sandbox_unlink_inside_allowed` function L876-901 — `()`
-  `sandbox_build_tool_workflow` function L905-927 — `()`
-  `sandbox_write_outside_blocked` function L931-968 — `()`
-  `sandbox_read_sensitive_path_blocked` function L972-1000 — `()`

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
-  `TaskOutputTool` type L22-136 — `impl Tool for TaskOutputTool`
-  `name` function L23-25 — `(&self) -> &str`
-  `description` function L27-31 — `(&self) -> &str`
-  `is_read_only` function L33-35 — `(&self) -> bool`
-  `category` function L37-39 — `(&self) -> ToolCategory`
-  `parameters_schema` function L41-60 — `(&self) -> Value`
-  `execute` function L62-135 — `(&self, _ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput,...`
-  `tests` module L139-214 — `-`
-  `test_ctx` function L146-149 — `() -> crate::context::EngineToolContext`
-  `unknown_task_returns_error` function L152-161 — `()`
-  `completed_task_returns_output` function L164-189 — `()`
-  `running_task_non_blocking` function L192-213 — `()`

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

- pub `WorkstreamCreateTool` struct L12-14 — `{ store: Arc<Mutex<Store>> }` — Tool for creating a new workstream.
- pub `new` function L17-19 — `(store: Arc<Mutex<Store>>) -> Self`
- pub `WorkstreamListTool` struct L89-91 — `{ store: Arc<Mutex<Store>> }` — Tool for listing available workstreams.
- pub `new` function L94-96 — `(store: Arc<Mutex<Store>>) -> Self`
-  `WorkstreamCreateTool` type L16-20 — `= WorkstreamCreateTool`
-  `WorkstreamCreateTool` type L23-86 — `impl Tool for WorkstreamCreateTool`
-  `name` function L24-26 — `(&self) -> &str`
-  `description` function L28-32 — `(&self) -> &str`
-  `category` function L34-36 — `(&self) -> ToolCategory`
-  `parameters_schema` function L38-49 — `(&self) -> Value`
-  `execute` function L51-85 — `(&self, ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput, ...`
-  `WorkstreamListTool` type L93-97 — `= WorkstreamListTool`
-  `WorkstreamListTool` type L100-147 — `impl Tool for WorkstreamListTool`
-  `name` function L101-103 — `(&self) -> &str`
-  `description` function L105-107 — `(&self) -> &str`
-  `is_read_only` function L109-111 — `(&self) -> bool`
-  `category` function L113-115 — `(&self) -> ToolCategory`
-  `parameters_schema` function L117-123 — `(&self) -> Value`
-  `execute` function L125-146 — `(&self, _ctx: &dyn arawn_tool::ToolContext, _params: Value) -> Result<ToolOutput...`
-  `tests` module L150-216 — `-`
-  `setup` function L155-161 — `() -> (tempfile::TempDir, Arc<Mutex<Store>>)`
-  `test_ctx` function L163-167 — `(tmp: &tempfile::TempDir) -> crate::context::EngineToolContext`
-  `create_workstream_succeeds` function L170-179 — `()`
-  `create_duplicate_workstream_errors` function L182-192 — `()`
-  `create_workstream_empty_name_errors` function L195-203 — `()`
-  `list_workstreams_includes_scratch` function L206-215 — `()`

### crates/arawn-llm/src

> *Semantic summary to be generated by AI agent.*

#### crates/arawn-llm/src/anthropic.rs

- pub `AnthropicClient` struct L17-20 — `{ http: Client, api_key: String }` — Client for Anthropic's Claude API (Messages API).
- pub `new` function L23-28 — `(api_key: impl Into<String>) -> Self`
- pub `from_env` function L30-34 — `() -> Result<Self, LlmError>`
-  `API_URL` variable L13 — `: &str`
-  `API_VERSION` variable L14 — `: &str`
-  `AnthropicClient` type L22-57 — `= AnthropicClient`
-  `build_request_body` function L36-56 — `(&self, request: &ChatRequest) -> Value`
-  `AnthropicClient` type L60-200 — `impl LlmClient for AnthropicClient`
-  `stream` function L61-199 — `( &self, request: ChatRequest, ) -> Result<Pin<Box<dyn Stream<Item = Result<Chat...`
-  `build_messages` function L206-265 — `(messages: &[ChatMessage]) -> Vec<Value>` — Convert arawn messages to Anthropic format.
-  `merge_consecutive_roles` function L269-305 — `(messages: &mut Vec<Value>)` — Merge consecutive messages with the same role into a single message
-  `normalize_content` function L308-314 — `(content: &Value) -> Vec<Value>` — Normalize content to a Vec<Value> of content blocks.
-  `build_tools` function L317-328 — `(tools: &[ToolDefinition]) -> Vec<Value>` — Convert tool definitions to Anthropic format.
-  `tests` module L331-462 — `-`
-  `user_msg` function L335-342 — `(text: &str) -> ChatMessage`
-  `assistant_text` function L344-351 — `(text: &str) -> ChatMessage`
-  `assistant_with_tool` function L353-364 — `(text: &str, tool_id: &str, tool_name: &str, args: Value) -> ChatMessage`
-  `tool_result` function L366-374 — `(tool_use_id: &str, content: &str) -> ChatMessage`
-  `simple_conversation` function L377-386 — `()`
-  `tool_call_with_result` function L389-412 — `()`
-  `multi_turn_with_tools` function L415-438 — `()`
-  `consecutive_tool_results_merged` function L441-461 — `()`

#### crates/arawn-llm/src/client.rs

- pub `LlmClient` interface L12-17 — `{ fn stream() }` — Provider-agnostic LLM client trait.

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

- pub `McpToolAdapter` struct L14-23 — `{ arawn_name: String, mcp_name: String, mcp_tool: McpTool, peer: Arc<Peer<RoleCl...` — An arawn Tool backed by an MCP server tool.
- pub `new` function L26-38 — `(server_name: &str, mcp_tool: McpTool, peer: Arc<Peer<RoleClient>>) -> Self` — McpToolAdapter — wraps an MCP tool as an arawn Tool impl.
- pub `tool_name` function L41-43 — `(&self) -> &str` — Get the arawn tool name (for logging before registration).
-  `McpToolAdapter` type L25-44 — `= McpToolAdapter` — McpToolAdapter — wraps an MCP tool as an arawn Tool impl.
-  `McpToolAdapter` type L47-120 — `impl Tool for McpToolAdapter` — McpToolAdapter — wraps an MCP tool as an arawn Tool impl.
-  `name` function L48-50 — `(&self) -> &str` — McpToolAdapter — wraps an MCP tool as an arawn Tool impl.
-  `description` function L52-57 — `(&self) -> &str` — McpToolAdapter — wraps an MCP tool as an arawn Tool impl.
-  `parameters_schema` function L59-66 — `(&self) -> Value` — McpToolAdapter — wraps an MCP tool as an arawn Tool impl.
-  `is_read_only` function L68-74 — `(&self) -> bool` — McpToolAdapter — wraps an MCP tool as an arawn Tool impl.
-  `execute` function L76-119 — `(&self, _ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput,...` — McpToolAdapter — wraps an MCP tool as an arawn Tool impl.
-  `normalize_name` function L123-133 — `(name: &str) -> String` — Normalize a name for use in tool naming — replace non-alphanumeric chars with _
-  `tests` module L136-151 — `-` — McpToolAdapter — wraps an MCP tool as an arawn Tool impl.
-  `normalize_simple` function L140-143 — `()` — McpToolAdapter — wraps an MCP tool as an arawn Tool impl.
-  `normalize_special_chars` function L146-150 — `()` — McpToolAdapter — wraps an MCP tool as an arawn Tool impl.

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

- pub `error` module L6 — `-` — Provides graph-backed entity storage with FTS5 search, typed relations,
- pub `inject` module L7 — `-` — confidence scoring, tag support, and search-before-create deduplication.
- pub `manager` module L8 — `-` — confidence scoring, tag support, and search-before-create deduplication.
- pub `shortcodes` module L9 — `-` — confidence scoring, tag support, and search-before-create deduplication.
- pub `stack` module L10 — `-` — confidence scoring, tag support, and search-before-create deduplication.
- pub `store` module L11 — `-` — confidence scoring, tag support, and search-before-create deduplication.
- pub `types` module L12 — `-` — confidence scoring, tag support, and search-before-create deduplication.
- pub `vector` module L13 — `-` — confidence scoring, tag support, and search-before-create deduplication.

#### crates/arawn-memory/src/manager.rs

- pub `MemoryManager` struct L19-28 — `{ global: Arc<MemoryStore>, workstream: Arc<MemoryStore>, vectors_enabled: bool,...` — Two-tier memory manager holding global and workstream knowledge bases.
- pub `open` function L34-71 — `(data_dir: &Path, ws_dir: &str, embedding_dims: Option<usize>) -> Result<Self, M...` — Open both KB tiers.
- pub `open_with_stores` function L74-81 — `(global: Arc<MemoryStore>, workstream: Arc<MemoryStore>) -> Self` — Create a MemoryManager from pre-built stores (for testing).
- pub `with_embedder` function L84-87 — `(mut self, embedder: Arc<dyn Embedder>) -> Self` — Attach an embedder for automatic embedding on ingest and vector-enhanced retrieval.
- pub `embedder` function L90-92 — `(&self) -> Option<&Arc<dyn Embedder>>` — Get the embedder if available.
- pub `store_fact_embedded` function L97-131 — `( &self, entity: &Entity, scope: Option<Scope>, ) -> Result<StoreFactResult, Mem...` — Store a fact with automatic embedding.
- pub `store_for` function L134-139 — `(&self, scope: Scope) -> &Arc<MemoryStore>` — Get the store for a given scope.
- pub `store_for_type` function L142-144 — `(&self, entity_type: EntityType) -> &Arc<MemoryStore>` — Get the store for a given entity type (uses default scope).
- pub `vectors_enabled` function L147-149 — `(&self) -> bool` — Whether vector storage is available.
- pub `retrieve_topical` function L154-245 — `( &self, keywords: &[String], budget_tokens: usize, ) -> Vec<crate::types::Entit...` — Retrieve entities matching keywords from both tiers.
- pub `try_open_memory` function L249-261 — `( data_dir: &Path, ws_dir: &str, embedding_dims: Option<usize>, ) -> Option<Arc<...` — Try to open a MemoryManager, returning None on failure (graceful degradation).
-  `MemoryManager` type L30-246 — `= MemoryManager` — It abstracts the two-tier scoping and routes entities to the appropriate store.
-  `tests` module L264-371 — `-` — It abstracts the two-tier scoping and routes entities to the appropriate store.
-  `setup` function L269-274 — `() -> (TempDir, MemoryManager)` — It abstracts the two-tier scoping and routes entities to the appropriate store.
-  `setup_with_vectors` function L276-281 — `() -> (TempDir, MemoryManager)` — It abstracts the two-tier scoping and routes entities to the appropriate store.
-  `opens_both_stores` function L284-293 — `()` — It abstracts the two-tier scoping and routes entities to the appropriate store.
-  `scope_routing` function L296-326 — `()` — It abstracts the two-tier scoping and routes entities to the appropriate store.
-  `vectors_disabled_by_default` function L329-332 — `()` — It abstracts the two-tier scoping and routes entities to the appropriate store.
-  `vectors_enabled_with_dims` function L335-346 — `()` — It abstracts the two-tier scoping and routes entities to the appropriate store.
-  `graceful_degradation` function L349-353 — `()` — It abstracts the two-tier scoping and routes entities to the appropriate store.
-  `stores_are_independent` function L356-370 — `()` — It abstracts the two-tier scoping and routes entities to the appropriate store.

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
- pub `l1_entity_titles` function L129-141 — `(&self) -> Vec<String>` — Get the entity titles included in L1 (for L2 deduplication).
- pub `topical_context` function L145-171 — `( &self, keywords: &[String], l1_titles: &[String], budget_tokens: usize, ) -> O...` — L2: Topic-triggered context.
-  `estimate_tokens` function L11-13 — `(text: &str) -> usize` — Estimate token count from text length (matches arawn-engine's TokenEstimator).
-  `render_l0` function L55-74 — `(&self) -> String` — L0: Identity layer — workstream name + Person/Convention entities.
-  `render_l1_with_names` function L78-126 — `(&self, budget_tokens: usize) -> (String, Vec<String>)` — L1: Essential story — top-ranked entities grouped by type, within budget.
-  `format_entity_brief` function L174-184 — `(entity: &Entity) -> String` — L2: On-demand — topic-triggered retrieval (separate method)
-  `tests` module L187-257 — `-` — L2: On-demand — topic-triggered retrieval (separate method)
-  `setup` function L192-197 — `() -> (TempDir, MemoryManager)` — L2: On-demand — topic-triggered retrieval (separate method)
-  `wake_up_respects_budget` function L200-213 — `()` — L2: On-demand — topic-triggered retrieval (separate method)
-  `wake_up_empty_kb` function L216-223 — `()` — L2: On-demand — topic-triggered retrieval (separate method)
-  `l1_ranks_stated_before_inferred` function L226-244 — `()` — L2: On-demand — topic-triggered retrieval (separate method)
-  `tiny_budget_does_not_panic` function L247-256 — `()` — L2: On-demand — topic-triggered retrieval (separate method)

#### crates/arawn-memory/src/store.rs

- pub `MemoryStore` struct L16-18 — `{ conn: Mutex<Connection> }` — Knowledge base store backed by SQLite with FTS5 and relations.
- pub `open` function L22-40 — `(path: &Path) -> Result<Self, MemoryError>` — Open or create a memory database at the given path.
- pub `in_memory` function L43-51 — `() -> Result<Self, MemoryError>` — Create an in-memory store (for testing).
- pub `insert_entity` function L115-142 — `(&self, entity: &Entity) -> Result<(), MemoryError>` — SQLite-backed knowledge base store with FTS5 search and relations.
- pub `get_entity` function L144-160 — `(&self, id: Uuid) -> Result<Option<Entity>, MemoryError>` — SQLite-backed knowledge base store with FTS5 search and relations.
- pub `update_entity` function L162-184 — `(&self, entity: &Entity) -> Result<(), MemoryError>` — SQLite-backed knowledge base store with FTS5 search and relations.
- pub `delete_entity` function L186-205 — `(&self, id: Uuid) -> Result<bool, MemoryError>` — SQLite-backed knowledge base store with FTS5 search and relations.
- pub `list_by_type` function L207-235 — `( &self, entity_type: EntityType, limit: usize, ) -> Result<Vec<Entity>, MemoryE...` — SQLite-backed knowledge base store with FTS5 search and relations.
- pub `list_all_ranked` function L239-270 — `(&self, limit: usize) -> Result<Vec<Entity>, MemoryError>` — List all non-superseded entities ranked by confidence: stated > observed > inferred,
- pub `count_by_type` function L272-282 — `(&self, entity_type: EntityType) -> Result<usize, MemoryError>` — SQLite-backed knowledge base store with FTS5 search and relations.
- pub `count_all` function L284-294 — `(&self) -> Result<usize, MemoryError>` — SQLite-backed knowledge base store with FTS5 search and relations.
- pub `search` function L298-323 — `(&self, query: &str, limit: usize) -> Result<Vec<Entity>, MemoryError>` — SQLite-backed knowledge base store with FTS5 search and relations.
- pub `search_by_type` function L325-358 — `( &self, query: &str, entity_type: EntityType, limit: usize, ) -> Result<Vec<Ent...` — SQLite-backed knowledge base store with FTS5 search and relations.
- pub `add_relation` function L362-381 — `( &self, source_id: Uuid, relation_type: RelationType, target_id: Uuid, ) -> Res...` — SQLite-backed knowledge base store with FTS5 search and relations.
- pub `get_relations` function L383-424 — `(&self, entity_id: Uuid) -> Result<Vec<Relation>, MemoryError>` — SQLite-backed knowledge base store with FTS5 search and relations.
- pub `get_neighbors` function L426-454 — `(&self, entity_id: Uuid) -> Result<Vec<(Uuid, RelationType)>, MemoryError>` — SQLite-backed knowledge base store with FTS5 search and relations.
- pub `delete_relation` function L456-474 — `( &self, source_id: Uuid, relation_type: RelationType, target_id: Uuid, ) -> Res...` — SQLite-backed knowledge base store with FTS5 search and relations.
- pub `store_fact` function L481-503 — `(&self, entity: &Entity) -> Result<StoreFactResult, MemoryError>` — Store a fact with search-before-create deduplication.
- pub `supersede_entity` function L532-557 — `( &self, old_id: Uuid, new_entity: &Entity, ) -> Result<StoreFactResult, MemoryE...` — Supersede an existing entity with a new one.
- pub `init_vectors` function L563-567 — `(&self, dims: usize) -> Result<(), MemoryError>` — Initialize vector storage with the given dimensions.
- pub `store_embedding` function L570-573 — `(&self, entity_id: Uuid, embedding: &[f32]) -> Result<(), MemoryError>` — Store an embedding for an entity.
- pub `search_similar` function L576-583 — `( &self, query_embedding: &[f32], limit: usize, ) -> Result<Vec<vector::Similari...` — Search for entities similar to a query embedding.
- pub `search_similar_filtered` function L586-594 — `( &self, query_embedding: &[f32], entity_ids: &[Uuid], limit: usize, ) -> Result...` — Search for entities similar to a query, filtered to a subset.
- pub `has_embedding` function L597-600 — `(&self, entity_id: Uuid) -> Result<bool, MemoryError>` — Check if an entity has a stored embedding.
- pub `count_embeddings` function L603-606 — `(&self) -> Result<usize, MemoryError>` — Count total stored embeddings.
- pub `search_by_tags` function L610-659 — `( &self, tags: &[String], limit: usize, ) -> Result<Vec<Entity>, MemoryError>` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `MemoryStore` type L20-660 — `= MemoryStore` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `migrate` function L53-111 — `(&self) -> Result<(), MemoryError>` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `reinforce_entity` function L506-529 — `(&self, entity_id: Uuid) -> Result<StoreFactResult, MemoryError>` — Reinforce an existing entity (increment count, update timestamp).
-  `row_to_entity` function L664-699 — `(row: &rusqlite::Row) -> Result<Entity, MemoryError>` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `OptionalExt` interface L702-704 — `{ fn optional() }` — Extension trait for optional query results.
-  `optional` function L707-713 — `(self) -> Result<Option<T>, rusqlite::Error>` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `tests` module L717-959 — `-` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `test_store` function L720-722 — `() -> MemoryStore` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `insert_and_get` function L725-733 — `()` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `get_nonexistent` function L736-739 — `()` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `update_entity` function L742-753 — `()` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `delete_entity` function L756-763 — `()` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `list_by_type` function L766-777 — `()` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `count_by_type` function L780-789 — `()` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `fts5_search` function L792-805 — `()` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `fts5_search_by_type` function L808-818 — `()` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `relations_crud` function L821-840 — `()` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `store_fact_insert` function L843-851 — `()` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `store_fact_reinforce` function L854-868 — `()` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `store_fact_reinforce_case_insensitive` function L871-883 — `()` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `supersede_entity` function L886-909 — `()` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `tags_on_entity` function L912-920 — `()` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `search_by_tags` function L923-944 — `()` — SQLite-backed knowledge base store with FTS5 search and relations.
-  `superseded_excluded_from_search` function L947-958 — `()` — SQLite-backed knowledge base store with FTS5 search and relations.

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

### crates/arawn-memory/tests

> *Semantic summary to be generated by AI agent.*

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

### crates/arawn-service/src

> *Semantic summary to be generated by AI agent.*

#### crates/arawn-service/src/error.rs

- pub `ServiceError` enum L4-19 — `NotFound | InvalidOperation | Engine | Storage | Internal`
- pub `error_code` function L23-31 — `(&self) -> &'static str` — Return a stable error code string for RPC responses.
-  `ServiceError` type L21-32 — `= ServiceError`

#### crates/arawn-service/src/lib.rs

- pub `error` module L1 — `-`
- pub `types` module L2 — `-`
- pub `ArawnService` interface L24-111 — `{ fn list_workstreams(), fn create_workstream(), fn list_sessions(), fn create_s...` — The service contract between any UI client and the Arawn backend.

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

- pub `JsonlMessageStore` struct L17-19 — `{ data_dir: PathBuf }` — JSONL-based message persistence.
- pub `new` function L22-26 — `(data_dir: impl Into<PathBuf>) -> Self`
- pub `append` function L29-58 — `( &self, session_id: Uuid, workstream_dir: &str, msg: &Message, ) -> Result<(), ...` — Append a message to the session's JSONL file.
- pub `load` function L61-103 — `( &self, session_id: Uuid, workstream_dir: &str, ) -> Result<Vec<Message>, Stora...` — Load all messages for a session from its JSONL file.
- pub `move_session` function L107-127 — `( &self, session_id: Uuid, from_dir: &str, to_dir: &str, ) -> Result<(), Storage...` — Move a session's JSONL file from one workstream directory to another.
- pub `path_for` function L140-142 — `(&self, session_id: Uuid, workstream_dir: &str) -> PathBuf` — Get the path for a session (exposed for testing/debugging).
- pub `sandbox_dir` function L151-160 — `(&self, workstream_dir: &str, session_id: Uuid, is_scratch: bool) -> PathBuf` — Resolve the sandbox root for a session.
- pub `workstream_dir_name` function L164-170 — `(name: &str, id: Uuid) -> String` — Resolve a workstream directory name: use name if non-empty, fall back to UUID.
-  `JsonlMessageStore` type L21-161 — `= JsonlMessageStore`
-  `session_path` function L131-137 — `(&self, session_id: Uuid, workstream_dir: &str) -> PathBuf` — Resolve the filesystem path for a session's JSONL file.
-  `tests` module L173-461 — `-`
-  `setup` function L179-183 — `() -> (TempDir, JsonlMessageStore)`
-  `append_and_load_roundtrip` function L186-222 — `()`
-  `append_twice_accumulates` function L225-253 — `()`
-  `load_nonexistent_returns_empty` function L256-260 — `()`
-  `scratch_session_path` function L263-284 — `()`
-  `move_session_relocates_file` function L287-324 — `()`
-  `move_nonexistent_session_is_ok` function L327-333 — `()`
-  `jsonl_each_line_is_valid_json` function L336-372 — `()`
-  `sandbox_dir_scratch_is_per_session` function L375-383 — `()`
-  `sandbox_dir_named_is_shared` function L386-391 — `()`
-  `workstream_dir_name_prefers_name` function L394-398 — `()`
-  `workstream_dir_name_falls_back_to_uuid` function L401-404 — `()`
-  `load_skips_malformed_lines` function L407-435 — `()`
-  `new_file_has_version_header` function L438-460 — `()`

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
- pub `promote_session_metadata` function L266-278 — `( &self, session_id: Uuid, new_ws_id: Uuid, ) -> Result<(), StorageError>` — Sync-only part of session promotion: update SQLite workstream_id.
- pub `move_session_jsonl` function L281-290 — `( &self, session_id: Uuid, from_ws_dir: &str, to_ws_dir: &str, ) -> Result<(), S...` — Async part of session promotion: move the JSONL file between workstream dirs.
-  `Store` type L22-291 — `= Store`
-  `resolve_ws_dir` function L140-150 — `(&self, ws_id: Option<Uuid>) -> Result<String, StorageError>` — Resolve the directory name for a workstream by UUID.
-  `copy_dir_contents` function L294-307 — `(src: &Path, dst: &Path) -> Result<(), StorageError>` — Recursively copy directory contents from src to dst.
-  `tests` module L310-479 — `-`
-  `setup` function L314-318 — `() -> (TempDir, Store)`
-  `open_creates_directories_and_db` function L321-327 — `()`
-  `open_is_idempotent` function L330-335 — `()`
-  `create_and_list_workstreams` function L338-346 — `()`
-  `create_scratch_session_and_append_messages` function L349-367 — `()`
-  `load_full_session` function L370-393 — `()`
-  `promote_session_full_flow` function L396-436 — `()`
-  `promote_bound_session_fails` function L439-452 — `()`
-  `load_nonexistent_session_returns_none` function L455-459 — `()`
-  `sandbox_for_scratch_is_per_session` function L462-469 — `()`
-  `sandbox_for_named_is_shared` function L472-478 — `()`

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

### crates/arawn-tests

> *Semantic summary to be generated by AI agent.*

#### crates/arawn-tests/build.rs

-  `main` function L1-3 — `()`

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

- pub `Scenario` struct L26-31 — `{ name: String, objective: String, turns: Vec<ScenarioTurn>, mechanical: Mechani...` — Or via angreal: angreal test uat --model gemma4
- pub `ScenarioTurn` struct L34-37 — `{ user_message: String, judge_expectation: String }` — Or via angreal: angreal test uat --model gemma4
- pub `MechanicalThresholds` struct L40-44 — `{ min_files_created: usize, min_memory_entities: usize, max_tool_errors: usize }` — Or via angreal: angreal test uat --model gemma4
- pub `TurnResult` struct L51-60 — `{ turn_number: usize, user_message: String, assistant_text: String, tool_calls: ...` — Or via angreal: angreal test uat --model gemma4
- pub `ToolCallRecord` struct L63-67 — `{ id: String, name: String, input: Value }` — Or via angreal: angreal test uat --model gemma4
- pub `ToolResultRecord` struct L70-74 — `{ id: String, content: String, is_error: bool }` — Or via angreal: angreal test uat --model gemma4
- pub `ScenarioResult` struct L81-88 — `{ scenario_name: String, model: String, turns: Vec<TurnResult>, mechanical: Mech...` — Or via angreal: angreal test uat --model gemma4
- pub `MechanicalCheckResult` struct L91-98 — `{ all_turns_completed: bool, no_errors: bool, tool_use_occurred: bool, files_cre...` — Or via angreal: angreal test uat --model gemma4
- pub `UatHarness` struct L104-108 — `{ data_dir: PathBuf, port: u16, server_process: Option<Child> }` — Or via angreal: angreal test uat --model gemma4
- pub `new` function L112-165 — `(base_dir: &Path, model: &str, provider: &str, api_key_env: &str) -> Self` — Create a new harness with an isolated data directory.
- pub `start_server` function L168-191 — `(&mut self) -> Result<(), String>` — Start the arawn server process.
- pub `wait_for_ready` function L194-218 — `(&self, timeout: Duration) -> Result<(), String>` — Wait for the server to be ready by polling the WebSocket endpoint.
- pub `ws_url` function L220-232 — `(&self) -> String` — Or via angreal: angreal test uat --model gemma4
- pub `run_scenario` function L235-291 — `(&self, scenario: &Scenario, model: &str) -> ScenarioResult` — Run a scenario: create session, drive all turns, collect results.
- pub `write_artifacts` function L431-479 — `(&self, result: &ScenarioResult, scenario: &Scenario)` — Write all artifacts to the results directory.
- pub `stop` function L482-488 — `(&mut self)` — Stop the server process.
-  `UatHarness` type L110-489 — `= UatHarness` — Or via angreal: angreal test uat --model gemma4
-  `rpc_create_session` function L293-319 — `( &self, write: &mut futures_util::stream::SplitSink< tokio_tungstenite::WebSock...` — Or via angreal: angreal test uat --model gemma4
-  `drive_turn` function L321-413 — `( &self, write: &mut futures_util::stream::SplitSink< tokio_tungstenite::WebSock...` — Or via angreal: angreal test uat --model gemma4
-  `list_workspace_files` function L415-428 — `(&self) -> Vec<String>` — Or via angreal: angreal test uat --model gemma4
-  `UatHarness` type L491-495 — `impl Drop for UatHarness` — Or via angreal: angreal test uat --model gemma4
-  `drop` function L492-494 — `(&mut self)` — Or via angreal: angreal test uat --model gemma4
-  `walkdir` function L498-513 — `(dir: &Path) -> Result<Vec<PathBuf>, std::io::Error>` — Recursively list all files under a directory.
-  `github_monitor_scenario` function L519-547 — `() -> Scenario` — Or via angreal: angreal test uat --model gemma4
-  `work_signal_pipeline_scenario` function L549-581 — `() -> Scenario` — Or via angreal: angreal test uat --model gemma4
-  `all_scenarios` function L583-585 — `() -> Vec<Scenario>` — Or via angreal: angreal test uat --model gemma4
-  `uat_run` function L593-688 — `()` — Or via angreal: angreal test uat --model gemma4

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

> *Semantic summary to be generated by AI agent.*

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
- pub `LlmResolver` interface L122-124 — `{ fn resolve() }` — Anything that can resolve [`LlmPreference`] requests against a pool of
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
- pub `ToolOutput` struct L35-38 — `{ content: String, is_error: bool }` — Output from a tool execution.
- pub `success` function L41-46 — `(content: impl Into<String>) -> Self`
- pub `error` function L48-53 — `(content: impl Into<String>) -> Self`
- pub `Tool` interface L58-85 — `{ fn name(), fn description(), fn parameters_schema(), fn execute(), fn is_read_...` — A tool that can be invoked by the LLM.
-  `ToolOutput` type L40-54 — `= ToolOutput`
-  `is_read_only` function L69-71 — `(&self) -> bool` — Whether this tool is side-effect-free (observation only).
-  `category` function L74-76 — `(&self) -> ToolCategory` — Tool category for permission checking and context filtering.
-  `llm_preference` function L82-84 — `(&self) -> Option<LlmPreference>` — Optional preferred LLM for this tool.

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
- pub `rendered_lines` function L66-76 — `(&mut self, width: usize) -> &[ratatui::text::Line<'static>]` — Get or compute the cached markdown rendering for assistant messages.
- pub `ChatRole` enum L80-86 — `User | Assistant | ToolCall | ToolResult | System`
- pub `App` struct L89-134 — `{ focus: Focus, input_buffer: String, cursor_pos: usize, messages: Vec<ChatMessa...` — All mutable TUI state.
- pub `new` function L137-170 — `() -> Self`
- pub `handle_action` function L173-488 — `(&mut self, action: Action) -> bool` — Process an action and mutate state.
- pub `apply_engine_event` function L534-611 — `(&mut self, event: crate::ws_client::EventUpdate)` — Apply a streaming engine event to the app state (testable without network).
- pub `load_session_messages` function L615-655 — `(&mut self, detail: &serde_json::Value)` — Load messages from a session detail JSON response into the chat.
- pub `format_tool_input` function L675-723 — `(tool_name: &str, input: &serde_json::Value) -> String` — Format tool input args into a compact display string.
-  `ChatMessage` type L53-77 — `= ChatMessage`
-  `App` type L136-672 — `= App`
-  `update_autocomplete` function L491-520 — `(&mut self)` — Update autocomplete suggestions based on current input buffer.
-  `accept_autocomplete` function L523-531 — `(&mut self)` — Accept the currently selected autocomplete suggestion.
-  `prev_char_boundary` function L657-663 — `(&self) -> usize`
-  `next_char_boundary` function L665-671 — `(&self) -> usize`
-  `App` type L725-729 — `impl Default for App`
-  `default` function L726-728 — `() -> Self`
-  `tests` module L732-970 — `-`
-  `type_chars_updates_buffer` function L736-742 — `()`
-  `backspace_removes_char` function L745-752 — `()`
-  `submit_moves_to_messages` function L755-767 — `()`
-  `submit_blocked_when_empty` function L770-776 — `()`
-  `submit_blocked_while_generating` function L779-785 — `()`
-  `tab_toggles_focus` function L788-795 — `()`
-  `scroll_updates_offset` function L798-806 — `()`
-  `cancel_stops_generation` function L809-818 — `()`
-  `quit_sets_flag` function L821-825 — `()`
-  `cursor_movement` function L828-849 — `()`
-  `full_conversation_flow` function L854-884 — `()`
-  `tool_call_flow` function L887-918 — `()`
-  `error_event_clears_generating` function L921-935 — `()`
-  `sidebar_navigation` function L938-969 — `()`

#### crates/arawn-tui/src/command.rs

- pub `CommandInfo` struct L11-15 — `{ name: String, description: String, kind: CommandKind }` — A registered slash command.
- pub `CommandKind` enum L19-26 — `BuiltIn | Inventory | Skill` — What kind of slash command this is.
- pub `ParsedCommand` struct L30-33 — `{ name: String, args: String }` — Result of parsing a slash command from the input buffer.
- pub `parse_command` function L37-57 — `(input: &str) -> Option<ParsedCommand>` — Parse a slash command from the input buffer.
- pub `CommandRegistry` struct L61-63 — `{ commands: Vec<CommandInfo> }` — The command registry — holds all available slash commands.
- pub `new` function L66-70 — `() -> Self` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
- pub `register_skills` function L161-171 — `(&mut self, skills: Vec<(String, String)>)` — Add skill commands from the server's cached skill list.
- pub `all` function L174-176 — `(&self) -> &[CommandInfo]` — Get all commands.
- pub `matching` function L179-185 — `(&self, prefix: &str) -> Vec<&CommandInfo>` — Find commands matching a prefix (for autocomplete).
- pub `find` function L188-191 — `(&self, name: &str) -> Option<&CommandInfo>` — Look up a command by exact name.
- pub `AutocompleteState` struct L196-201 — `{ suggestions: Vec<CommandInfo>, selected: usize }` — Autocomplete state for the slash command dropdown.
- pub `new` function L204-209 — `(suggestions: Vec<CommandInfo>) -> Self` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
- pub `next` function L211-215 — `(&mut self)` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
- pub `prev` function L217-225 — `(&mut self)` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
- pub `selected_command` function L227-229 — `(&self) -> Option<&CommandInfo>` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
- pub `is_empty` function L231-233 — `(&self) -> bool` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
- pub `CommandResult` enum L238-273 — `SystemMessage | ClearChat | EnterPlan | QueryInventory | InvokeSkill | RememberF...` — The result of executing a built-in command.
- pub `execute_command` function L276-384 — `(cmd: &ParsedCommand, registry: &CommandRegistry) -> CommandResult` — Execute a parsed slash command against the registry.
-  `CommandRegistry` type L65-192 — `= CommandRegistry` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `register_builtins` function L72-158 — `(&mut self)` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `AutocompleteState` type L203-234 — `= AutocompleteState` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `tests` module L387-531 — `-` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `parse_simple_command` function L391-395 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `parse_command_with_args` function L398-402 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `parse_not_a_command` function L405-409 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `parse_slash_only` function L412-414 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `parse_with_leading_whitespace` function L417-420 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `registry_has_builtins` function L423-430 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `registry_matching_prefix` function L433-439 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `registry_matching_empty_returns_all` function L442-446 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `registry_skills` function L449-458 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `autocomplete_navigation` function L461-479 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `execute_help` function L482-489 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `execute_clear` function L492-496 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `execute_unknown` function L499-506 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `execute_inventory` function L509-516 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server
-  `execute_skill` function L519-530 — `()` — - **Skill**: /skill-name — invoke a user-invocable skill via the server

#### crates/arawn-tui/src/event.rs

- pub `map_key_event` function L7-66 — `( key: KeyEvent, focus: Focus, is_generating: bool, has_modal: bool, has_autocom...` — Map a crossterm KeyEvent to an Action, given the current focus.
-  `map_main_key` function L68-84 — `(key: KeyEvent) -> Option<Action>`
-  `map_modal_key` function L86-100 — `(key: KeyEvent) -> Option<Action>`
-  `map_sidebar_key` function L102-110 — `(key: KeyEvent) -> Option<Action>`
-  `tests` module L113-221 — `-`
-  `key` function L115-117 — `(code: KeyCode) -> KeyEvent`
-  `ctrl` function L119-121 — `(c: char) -> KeyEvent`
-  `ctrl_c_quits_from_any_focus` function L124-133 — `()`
-  `tab_toggles_from_any_focus` function L136-145 — `()`
-  `esc_cancels_when_generating` function L148-154 — `()`
-  `main_focus_typing` function L157-170 — `()`
-  `main_focus_scrolling` function L173-186 — `()`
-  `ctrl_e_toggles_tool_results` function L189-200 — `()`
-  `sidebar_focus_navigation` function L203-220 — `()`

#### crates/arawn-tui/src/event_loop.rs

- pub `run_tui` function L27-765 — `(url: &str, model_name: &str) -> Result<(), Box<dyn std::error::Error>>` — Run the TUI connected to the given WebSocket server URL.
-  `rect_contains` function L22-24 — `(rect: Rect, col: u16, row: u16) -> bool`

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
-  `render_status_bar` function L107-192 — `(app: &App, frame: &mut Frame, area: ratatui::layout::Rect)`
-  `format_tokens` function L195-203 — `(n: u64) -> String` — Format a token count for display: 1234 → "1.2k", 12345 → "12.3k", 500 → "500"
-  `render_sidebar` function L205-279 — `(app: &App, frame: &mut Frame, area: ratatui::layout::Rect)`
-  `render_chat` function L281-556 — `(app: &mut App, frame: &mut Frame, area: ratatui::layout::Rect)`
-  `render_separator` function L558-562 — `(frame: &mut Frame, area: ratatui::layout::Rect)`
-  `render_input` function L564-613 — `(app: &App, frame: &mut Frame, area: ratatui::layout::Rect)`
-  `render_autocomplete` function L616-674 — `( ac: &crate::command::AutocompleteState, frame: &mut Frame, input_area: ratatui...` — Render the autocomplete dropdown above the input line.
-  `truncate_to` function L677-686 — `(s: &str, max_chars: usize) -> String` — Truncate a string to fit within a display width, adding "…" if needed.
-  `compact_tool_summary` function L689-694 — `(content: &str) -> String` — Extract a compact summary from tool call content for inline display.
-  `truncate_for_display` function L696-702 — `(s: &str, max: usize) -> String`
-  `tests` module L705-1439 — `-`
-  `buffer_to_string` function L711-726 — `(terminal: &Terminal<TestBackend>, row: u16) -> String`
-  `render_empty_app_has_status_bar` function L729-738 — `()`
-  `render_with_messages_shows_content` function L741-767 — `()`
-  `render_with_input_text` function L770-785 — `()`
-  `render_streaming_shows_cursor` function L788-811 — `()`
-  `render_small_terminal` function L814-819 — `()`
-  `render_large_terminal` function L822-827 — `()`
-  `region_text` function L832-844 — `(terminal: &Terminal<TestBackend>, x: u16, y: u16, w: u16, h: u16) -> String` — Extract text from a rectangular region of the buffer.
-  `chat_region_for` function L848-861 — `(terminal: &Terminal<TestBackend>, sidebar_visible: bool) -> String` — Extract the chat area text.
-  `chat_region` function L864-866 — `(terminal: &Terminal<TestBackend>) -> String` — Convenience: chat region for default app (sidebar hidden).
-  `sidebar_region` function L870-878 — `(terminal: &Terminal<TestBackend>) -> String` — Extract the sidebar text (left 20%, rows 1..height-3).
-  `input_region` function L881-886 — `(terminal: &Terminal<TestBackend>) -> String` — Extract the input bar text (second from bottom row).
-  `chat_renders_user_message_with_prefix` function L891-905 — `()`
-  `chat_renders_assistant_message_with_prefix` function L908-922 — `()`
-  `chat_renders_tool_call_with_icon` function L925-950 — `()`
-  `chat_renders_tool_result_collapsed` function L953-981 — `()`
-  `chat_renders_tool_error_result` function L984-1007 — `()`
-  `chat_renders_tool_result_truncated` function L1010-1037 — `()`
-  `chat_streaming_text_appears_in_chat_area` function L1040-1058 — `()`
-  `sidebar_renders_workstream_names` function L1061-1097 — `()`
-  `sidebar_does_not_leak_into_chat` function L1100-1134 — `()`
-  `input_shows_placeholder_when_empty` function L1137-1148 — `()`
-  `input_shows_generating_when_active` function L1151-1164 — `()`
-  `status_bar_shows_generating_indicator` function L1167-1181 — `()`
-  `status_bar_shows_workstream_name` function L1184-1208 — `()`
-  `messages_do_not_appear_in_input_area` function L1211-1234 — `()`
-  `chat_auto_scrolls_to_bottom_with_many_messages` function L1239-1269 — `()`
-  `chat_scroll_up_reveals_older_messages` function L1272-1300 — `()`
-  `chat_few_messages_all_visible` function L1303-1317 — `()`
-  `last_message_visible_above_input` function L1320-1373 — `()`
-  `last_tool_result_visible_above_input` function L1376-1438 — `()`

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

- pub `WsClient` struct L17-29 — `{ write: futures_util::stream::SplitSink< tokio_tungstenite::WebSocketStream< to...` — A WebSocket connection to the Arawn server.
- pub `connect` function L32-46 — `(url: &str) -> Result<Self, Box<dyn std::error::Error>>`
- pub `send_request` function L66-83 — `( &mut self, method: &str, params: Value, ) -> Result<u64, Box<dyn std::error::E...`
- pub `list_workstreams` function L85-92 — `( &mut self, ) -> Result<Vec<WorkstreamInfo>, Box<dyn std::error::Error>>`
- pub `list_workflows` function L94-101 — `( &mut self, ) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>>`
- pub `get_permission_mode` function L103-110 — `( &mut self, ) -> Result<String, Box<dyn std::error::Error>>`
- pub `set_permission_mode` function L112-123 — `( &mut self, mode: &str, ) -> Result<String, Box<dyn std::error::Error>>`
- pub `list_sessions` function L125-137 — `( &mut self, ws_id: Option<uuid::Uuid>, ) -> Result<Vec<SessionInfo>, Box<dyn st...`
- pub `create_session` function L139-151 — `( &mut self, ws_id: Option<uuid::Uuid>, ) -> Result<SessionInfo, Box<dyn std::er...`
- pub `load_session` function L153-161 — `( &mut self, session_id: uuid::Uuid, ) -> Result<serde_json::Value, Box<dyn std:...`
- pub `send_message` function L163-179 — `( &mut self, session_id: uuid::Uuid, content: &str, ) -> Result<(), Box<dyn std:...`
- pub `read_response_raw` function L182-184 — `(&mut self) -> Result<Value, Box<dyn std::error::Error>>` — Read the next JSON response from the server (public for sidebar).
- pub `parse_engine_event` function L215-235 — `(text: &str) -> Option<EngineEvent>` — Parse a WS message as an EngineEvent.
- pub `EventUpdate` enum L238-265 — `AppendStreamingText | AddToolCall | AddToolResult | Complete | Error | Warning |...` — Convert an EngineEvent into App state updates.
- pub `engine_event_to_update` function L267-294 — `(event: EngineEvent) -> EventUpdate`
-  `REQUEST_ID` variable L10 — `: AtomicU64`
-  `next_id` function L12-14 — `() -> u64`
-  `WsClient` type L31-212 — `= WsClient`
-  `read_server_token` function L50-64 — `() -> Option<String>` — Read the server auth token from {data_dir}/server.token.
-  `read_response` function L187-211 — `(&mut self) -> Result<Value, Box<dyn std::error::Error>>` — Read the next JSON response from the server.

### crates/arawn-workflow

> *Semantic summary to be generated by AI agent.*

#### crates/arawn-workflow/build.rs

-  `main` function L1-3 — `()`

### crates/arawn-workflow/src

> *Semantic summary to be generated by AI agent.*

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

- pub `WorkflowRunnerConfig` struct L9-16 — `{ database_path: PathBuf, packages_dir: PathBuf, max_concurrent_tasks: usize }` — Configuration for the workflow runner.
- pub `new` function L19-25 — `(data_dir: &Path) -> Self` — Wrapper around cloacina's DefaultRunner for arawn server integration.
- pub `WorkflowRunner` struct L32-34 — `{ runner: DefaultRunner }` — Arawn's workflow engine — wraps cloacina's DefaultRunner.
- pub `new` function L40-65 — `(config: WorkflowRunnerConfig) -> Result<Self, WorkflowError>` — Initialize the workflow runner with the given configuration.
- pub `execute` function L68-85 — `( &self, workflow_name: &str, context: serde_json::Value, ) -> Result<PipelineRe...` — Execute a named workflow programmatically.
- pub `shutdown` function L88-93 — `(&self)` — Graceful shutdown — drains in-flight pipelines.
- pub `inner` function L96-98 — `(&self) -> &DefaultRunner` — Get a reference to the underlying DefaultRunner.
- pub `WorkflowError` enum L102-107 — `Init | Runtime` — Wrapper around cloacina's DefaultRunner for arawn server integration.
-  `WorkflowRunnerConfig` type L18-26 — `= WorkflowRunnerConfig` — Wrapper around cloacina's DefaultRunner for arawn server integration.
-  `WorkflowRunner` type L36-99 — `= WorkflowRunner` — Wrapper around cloacina's DefaultRunner for arawn server integration.
-  `tests` module L110-139 — `-` — Wrapper around cloacina's DefaultRunner for arawn server integration.
-  `runner_initializes_and_shuts_down` function L114-128 — `()` — Wrapper around cloacina's DefaultRunner for arawn server integration.
-  `runner_starts_with_empty_packages_dir` function L131-138 — `()` — Wrapper around cloacina's DefaultRunner for arawn server integration.

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
- pub `WorkflowDeleteTool` struct L259-261 — `{ packages_dir: PathBuf }` — Tool for deleting a workflow package.
- pub `new` function L264-266 — `(packages_dir: PathBuf) -> Self` — Agent-facing tools for workflow management: create, list, delete, status.
- pub `WorkflowStatusTool` struct L314-316 — `{ runner: SharedWorkflowRunner }` — Tool for checking workflow execution status.
- pub `new` function L319-321 — `(runner: SharedWorkflowRunner) -> Self` — Agent-facing tools for workflow management: create, list, delete, status.
-  `WorkflowCreateTool` type L25-29 — `= WorkflowCreateTool` — Agent-facing tools for workflow management: create, list, delete, status.
-  `WorkflowCreateTool` type L32-182 — `impl Tool for WorkflowCreateTool` — Agent-facing tools for workflow management: create, list, delete, status.
-  `name` function L33-35 — `(&self) -> &str` — Agent-facing tools for workflow management: create, list, delete, status.
-  `description` function L37-41 — `(&self) -> &str` — Agent-facing tools for workflow management: create, list, delete, status.
-  `parameters_schema` function L43-90 — `(&self) -> Value` — Agent-facing tools for workflow management: create, list, delete, status.
-  `execute` function L92-181 — `(&self, _ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput,...` — Agent-facing tools for workflow management: create, list, delete, status.
-  `WorkflowListTool` type L189-193 — `= WorkflowListTool` — Agent-facing tools for workflow management: create, list, delete, status.
-  `WorkflowListTool` type L196-256 — `impl Tool for WorkflowListTool` — Agent-facing tools for workflow management: create, list, delete, status.
-  `name` function L197-199 — `(&self) -> &str` — Agent-facing tools for workflow management: create, list, delete, status.
-  `description` function L201-203 — `(&self) -> &str` — Agent-facing tools for workflow management: create, list, delete, status.
-  `is_read_only` function L205-207 — `(&self) -> bool` — Agent-facing tools for workflow management: create, list, delete, status.
-  `parameters_schema` function L209-215 — `(&self) -> Value` — Agent-facing tools for workflow management: create, list, delete, status.
-  `execute` function L217-255 — `(&self, _ctx: &dyn arawn_tool::ToolContext, _params: Value) -> Result<ToolOutput...` — Agent-facing tools for workflow management: create, list, delete, status.
-  `WorkflowDeleteTool` type L263-267 — `= WorkflowDeleteTool` — Agent-facing tools for workflow management: create, list, delete, status.
-  `WorkflowDeleteTool` type L270-311 — `impl Tool for WorkflowDeleteTool` — Agent-facing tools for workflow management: create, list, delete, status.
-  `name` function L271-273 — `(&self) -> &str` — Agent-facing tools for workflow management: create, list, delete, status.
-  `description` function L275-277 — `(&self) -> &str` — Agent-facing tools for workflow management: create, list, delete, status.
-  `parameters_schema` function L279-290 — `(&self) -> Value` — Agent-facing tools for workflow management: create, list, delete, status.
-  `execute` function L292-310 — `(&self, _ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput,...` — Agent-facing tools for workflow management: create, list, delete, status.
-  `WorkflowStatusTool` type L318-322 — `= WorkflowStatusTool` — Agent-facing tools for workflow management: create, list, delete, status.
-  `WorkflowStatusTool` type L325-384 — `impl Tool for WorkflowStatusTool` — Agent-facing tools for workflow management: create, list, delete, status.
-  `name` function L326-328 — `(&self) -> &str` — Agent-facing tools for workflow management: create, list, delete, status.
-  `description` function L330-332 — `(&self) -> &str` — Agent-facing tools for workflow management: create, list, delete, status.
-  `is_read_only` function L334-336 — `(&self) -> bool` — Agent-facing tools for workflow management: create, list, delete, status.
-  `parameters_schema` function L338-349 — `(&self) -> Value` — Agent-facing tools for workflow management: create, list, delete, status.
-  `execute` function L351-383 — `(&self, _ctx: &dyn arawn_tool::ToolContext, params: Value) -> Result<ToolOutput,...` — Agent-facing tools for workflow management: create, list, delete, status.

### scripts

> *Semantic summary to be generated by AI agent.*

#### scripts/functional_test.py

- pub `send_rpc` function L16-30 — `def send_rpc(ws, method, params=None)` — Send a JSON-RPC request and return the result.
- pub `send_and_wait` function L33-60 — `def send_and_wait(ws, session_id, prompt)` — Send a message and wait for the Complete event.
- pub `load_session_jsonl` function L63-71 — `def load_session_jsonl(session_id)` — Load the session JSONL from disk.
- pub `analyze` function L74-170 — `def analyze(messages, scenario_name)` — Analyze session messages and print a report.
- pub `run_scenario` function L173-189 — `def run_scenario(prompt, name="test")` — Connect, send prompt, wait, analyze.

