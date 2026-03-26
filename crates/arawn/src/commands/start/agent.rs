//! Agent construction, session indexer, MCP, plugin, and tool registry initialization.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;

use arawn_agent::{Agent, McpToolAdapter, PromptMode, SystemPromptBuilder, Tool, ToolRegistry};
#[cfg(feature = "gliner")]
use arawn_agent_indexing::{GlinerEngine, NerConfig};
use arawn_agent_indexing::{IndexerConfig, SessionIndexer};
use arawn_agent_tools as tools;
use arawn_config::ResolvedLlm;
use arawn_mcp::{McpManager, McpServerConfig};
use arawn_memory::MemoryStore;
use arawn_plugin::{HookDispatcher, PluginManager, PluginWatcher, SubscriptionManager, SyncAction};
use arawn_workstream::WorkstreamFsGate;

use super::Context;

/// Result of plugin system initialization.
pub(super) struct PluginInitResult {
    pub(super) prompts: Vec<(String, String)>,
    pub(super) hook_dispatcher: HookDispatcher,
    pub(super) agent_configs: HashMap<String, arawn_plugin::PluginAgentConfig>,
    pub(super) agent_sources: HashMap<String, String>,
    pub(super) _watcher: Option<arawn_plugin::WatcherHandle>,
}

/// Phase 13: Build the agent with all configuration, tools, prompts, hooks, and sandboxing.
#[allow(clippy::too_many_arguments)]
pub(super) async fn build_agent(
    config: &arawn_config::ArawnConfig,
    resolved: &ResolvedLlm,
    backend: arawn_llm::SharedBackend,
    tool_registry: arawn_agent::ToolRegistry,
    plugin_prompts: Vec<(String, String)>,
    shared_hook_dispatcher: &Option<arawn_types::SharedHookDispatcher>,
    workspace: &Option<PathBuf>,
    bootstrap_dir: &Option<PathBuf>,
    prompt_files: &[PathBuf],
    memory_store: &Option<Arc<MemoryStore>>,
    embedder: &Arc<dyn arawn_llm::Embedder>,
    data_dir: &std::path::Path,
    ctx: &Context,
) -> Result<arawn_agent::Agent> {
    let agent_profile = config.agent.get("default");

    let agent_name = agent_profile
        .and_then(|a| a.name.as_deref())
        .unwrap_or("Arawn");
    let agent_description = agent_profile
        .and_then(|a| a.description.as_deref())
        .unwrap_or("a capable AI assistant running on the user's local machine");

    let prompt_builder = SystemPromptBuilder::new()
        .with_mode(PromptMode::Full)
        .with_identity(agent_name, agent_description)
        .with_datetime(None)
        .with_memory_hints();

    if ctx.verbose {
        println!("Agent identity: {} — {}", agent_name, agent_description);
    }

    let mut builder = Agent::builder()
        .with_shared_backend(backend)
        .with_tools(tool_registry)
        .with_plugin_prompts(plugin_prompts)
        .with_prompt_builder(prompt_builder)
        .with_model(&resolved.model);

    if let Some(max_iter) = agent_profile.and_then(|a| a.max_iterations) {
        builder = builder.with_max_iterations(max_iter);
    }
    if let Some(max_tok) = agent_profile.and_then(|a| a.max_tokens) {
        builder = builder.with_max_tokens(max_tok);
    }
    if let Some(dispatcher) = shared_hook_dispatcher {
        builder = builder.with_hook_dispatcher(dispatcher.clone());
    }
    if let Some(ws) = workspace {
        if ctx.verbose {
            println!("Workspace: {}", ws.display());
        }
        builder = builder.with_workspace(ws);
    }
    if let Some(dir) = bootstrap_dir {
        if ctx.verbose {
            println!("Loading bootstrap files from: {}", dir.display());
        }
        builder = builder.with_bootstrap_dir(dir);
    }
    for file in prompt_files {
        if ctx.verbose {
            println!("Loading prompt file: {}", file.display());
        }
        builder = builder.with_prompt_file(file);
    }

    // Secret resolver
    match arawn_config::AgeSecretStore::open_default() {
        Ok(store) => {
            builder = builder.with_secret_resolver(std::sync::Arc::new(store));
        }
        Err(e) => {
            tracing::warn!(
                "Failed to open secret store (handles will not resolve): {}",
                e
            );
        }
    }

    // Filesystem gate resolver
    {
        use arawn_workstream::DirectoryManager;

        let ws_data_dir = config
            .workstream
            .as_ref()
            .and_then(|w| w.data_dir.clone())
            .map(|p| if p.is_relative() { data_dir.join(p) } else { p })
            .unwrap_or_else(|| data_dir.join("workstreams"));

        let dm = Arc::new(DirectoryManager::new(&ws_data_dir));

        let sandbox: Option<Arc<arawn_sandbox::SandboxManager>> =
            match arawn_sandbox::SandboxManager::new().await {
                Ok(mgr) => {
                    if ctx.verbose {
                        println!("Sandbox: {} platform detected", mgr.platform());
                    }
                    Some(Arc::new(mgr))
                }
                Err(e) => {
                    tracing::warn!(
                        "Sandbox unavailable (shell tool disabled, file tools still work): {}",
                        e
                    );
                    if ctx.verbose {
                        println!("Sandbox: unavailable — {}", e);
                    }
                    None
                }
            };

        let resolver: arawn_types::FsGateResolver =
            Arc::new(move |session_id: &str, workstream_id: &str| {
                let gate: Arc<dyn arawn_types::FsGate> = match &sandbox {
                    Some(sandbox) => Arc::new(WorkstreamFsGate::new(
                        &dm,
                        Arc::clone(sandbox),
                        workstream_id,
                        session_id,
                    )),
                    None => Arc::new(WorkstreamFsGate::path_only(&dm, workstream_id, session_id)),
                };
                Some(gate)
            });

        builder = builder.with_fs_gate_resolver(resolver);
    }

    if let Some(store) = memory_store {
        builder = builder.with_memory_store(store.clone());
        if ctx.verbose {
            println!("Active recall: memory store wired");
        }
    }
    builder = builder.with_embedder(embedder.clone());

    Ok(builder.build()?)
}

/// Phase 14: Create the session indexer for background summarization.
pub(super) async fn init_session_indexer(
    memory_cfg: &arawn_config::MemoryConfig,
    memory_store: &Option<Arc<MemoryStore>>,
    backends: &HashMap<String, arawn_llm::SharedBackend>,
    embedder: &Arc<dyn arawn_llm::Embedder>,
    data_dir: &std::path::Path,
    ctx: &Context,
) -> Option<SessionIndexer> {
    if !memory_cfg.indexing.enabled {
        if ctx.verbose {
            println!("Session indexer: disabled");
        }
        return None;
    }

    let store = match memory_store {
        Some(s) => s,
        None => {
            tracing::warn!("memory store not available, indexer disabled");
            return None;
        }
    };

    let indexing_backend_name = &memory_cfg.indexing.backend;
    let indexing_backend = backends
        .get(indexing_backend_name)
        .or_else(|| backends.get("default"))
        .cloned();

    let ib = match indexing_backend {
        Some(ib) => ib,
        None => {
            tracing::warn!(
                " indexing backend '{}' not found, indexer disabled",
                indexing_backend_name
            );
            return None;
        }
    };

    let indexer_config = IndexerConfig {
        model: memory_cfg.indexing.model.clone(),
        ..Default::default()
    };

    #[allow(unused_mut)]
    let mut idx =
        SessionIndexer::with_backend(store.clone(), ib, Some(embedder.clone()), indexer_config);

    #[cfg(feature = "gliner")]
    {
        let ner_paths: Option<(String, String)> =
            if let Some(ref model_path) = memory_cfg.indexing.ner_model_path {
                let tok = memory_cfg
                    .indexing
                    .ner_tokenizer_path
                    .clone()
                    .unwrap_or_else(|| {
                        std::path::Path::new(model_path)
                            .parent()
                            .map(|p| p.join("tokenizer.json").to_string_lossy().into())
                            .unwrap_or_else(|| "tokenizer.json".to_string())
                    });
                Some((model_path.clone(), tok))
            } else {
                match arawn_llm::ensure_ner_model_files(
                    memory_cfg.indexing.ner_model_url.as_deref(),
                    memory_cfg.indexing.ner_tokenizer_url.as_deref(),
                )
                .await
                {
                    Some((m, t)) => Some((
                        m.to_string_lossy().into_owned(),
                        t.to_string_lossy().into_owned(),
                    )),
                    None => {
                        tracing::warn!(" GLiNER model download failed, NER disabled");
                        None
                    }
                }
            };

        if let Some((model_path, tokenizer_path)) = ner_paths {
            let ner_config = NerConfig {
                model_path: model_path.clone(),
                tokenizer_path,
                threshold: memory_cfg.indexing.ner_threshold,
            };
            match GlinerEngine::new(&ner_config) {
                Ok(engine) => {
                    idx = idx.with_ner_engine(Arc::new(engine));
                    if ctx.verbose {
                        println!("NER engine: GLiNER ({})", model_path);
                    }
                }
                Err(e) => tracing::warn!("failed to load GLiNER model: {}", e),
            }
        }
    }

    if ctx.verbose {
        println!(
            "Session indexer: enabled (backend={}, model={}, db={})",
            indexing_backend_name,
            memory_cfg.indexing.model,
            data_dir.join("memory.db").display()
        );
    }

    Some(idx)
}

/// Phase 11: Connect to MCP servers and register discovered tools.
pub(super) fn init_mcp(
    mcp_cfg: &arawn_config::McpConfig,
    tool_registry: &mut ToolRegistry,
    ctx: &Context,
) -> Option<McpManager> {
    if !mcp_cfg.enabled {
        if ctx.verbose {
            println!("MCP: disabled");
        }
        return None;
    }

    let mut manager = McpManager::new();

    if mcp_cfg.servers.is_empty() {
        if ctx.verbose {
            println!("MCP: enabled (no servers configured)");
        }
        return Some(manager);
    }

    let enabled_servers: Vec<McpServerConfig> = mcp_cfg
        .servers
        .iter()
        .filter(|s| s.enabled)
        .filter_map(|entry| {
            if entry.is_http() {
                let url = match &entry.url {
                    Some(u) => u.clone(),
                    None => {
                        tracing::warn!(
                            " MCP server '{}' is HTTP but has no URL, skipping",
                            entry.name
                        );
                        return None;
                    }
                };
                let mut config = McpServerConfig::http(&entry.name, &url);
                for (k, v) in entry.header_tuples() {
                    config = config.with_header(k, v);
                }
                if let Some(timeout) = entry.timeout_secs {
                    config = config.with_timeout(std::time::Duration::from_secs(timeout));
                }
                if let Some(retries) = entry.retries {
                    config = config.with_retries(retries);
                }
                Some(config)
            } else {
                Some(
                    McpServerConfig::new(&entry.name, &entry.command)
                        .with_args(entry.args.clone())
                        .with_env(entry.env_tuples()),
                )
            }
        })
        .collect();

    if enabled_servers.is_empty() {
        if ctx.verbose {
            println!("MCP: enabled (no servers configured)");
        }
        return Some(manager);
    }

    for config in enabled_servers {
        manager.add_server(config);
    }
    if ctx.verbose {
        println!("MCP: connecting to {} server(s)...", manager.config_count());
    }

    match manager.connect_all() {
        Ok(connected) if connected > 0 => match manager.list_all_tools() {
            Ok(all_tools) => {
                let mut total_tools = 0;
                for server_name in all_tools.keys() {
                    if let Some(client) = manager.get_client(server_name) {
                        match McpToolAdapter::from_client(client) {
                            Ok(adapters) => {
                                for adapter in adapters {
                                    if ctx.verbose {
                                        println!("  Registered: {}", adapter.name());
                                    }
                                    tool_registry.register(adapter);
                                    total_tools += 1;
                                }
                            }
                            Err(e) => tracing::warn!(
                                " failed to create adapters for {}: {}",
                                server_name,
                                e
                            ),
                        }
                    }
                }
                println!(
                    "MCP: {} server(s) connected, {} tool(s) registered",
                    connected, total_tools
                );
            }
            Err(e) => tracing::warn!("failed to list MCP tools: {}", e),
        },
        Ok(_) => tracing::warn!("no MCP servers could be connected"),
        Err(e) => tracing::warn!("MCP connection failed: {}", e),
    }

    Some(manager)
}

/// Phase 10: Load plugins, sync subscriptions, collect hooks + agent configs + skill prompts.
pub(super) async fn init_plugins(
    plugins_cfg: &arawn_config::PluginsConfig,
    workspace: Option<&std::path::Path>,
    ctx: &Context,
) -> PluginInitResult {
    let mut prompts: Vec<(String, String)> = Vec::new();
    let mut hook_dispatcher = HookDispatcher::new();
    let mut agent_configs: HashMap<String, arawn_plugin::PluginAgentConfig> = HashMap::new();
    let mut agent_sources: HashMap<String, String> = HashMap::new();

    let watcher_handle: Option<arawn_plugin::WatcherHandle> = if plugins_cfg.enabled {
        let mut plugin_dirs: Vec<PathBuf> = Vec::new();
        if let Some(config_dir) = dirs::config_dir() {
            plugin_dirs.push(config_dir.join("arawn").join("plugins"));
        }
        plugin_dirs.push(PathBuf::from("./plugins"));
        plugin_dirs.extend(plugins_cfg.dirs.clone());

        if !plugins_cfg.subscriptions.is_empty() {
            match SubscriptionManager::new(plugins_cfg.subscriptions.clone(), workspace) {
                Ok(sub_manager) => {
                    let should_update =
                        plugins_cfg.auto_update && !SubscriptionManager::is_auto_update_disabled();
                    if should_update {
                        if ctx.verbose {
                            println!(
                                "Syncing {} subscribed plugin(s)...",
                                sub_manager.all_subscriptions().len()
                            );
                        }
                        let results = sub_manager.sync_all_async().await;
                        for result in &results {
                            match result.action {
                                SyncAction::Cloned => {
                                    if ctx.verbose {
                                        println!("  Cloned: {}", result.subscription_id);
                                    }
                                }
                                SyncAction::Updated => {
                                    if ctx.verbose {
                                        println!("  Updated: {}", result.subscription_id);
                                    }
                                }
                                SyncAction::Skipped => {
                                    if ctx.verbose {
                                        println!("  Skipped: {}", result.subscription_id);
                                    }
                                }
                                SyncAction::CloneFailed | SyncAction::UpdateFailed => {
                                    let err = result.error.as_deref().unwrap_or("unknown error");
                                    tracing::warn!(
                                        " {} {}: {}",
                                        result.action,
                                        result.subscription_id,
                                        err
                                    );
                                }
                            }
                        }
                        let cloned = results
                            .iter()
                            .filter(|r| r.action == SyncAction::Cloned)
                            .count();
                        let updated = results
                            .iter()
                            .filter(|r| r.action == SyncAction::Updated)
                            .count();
                        let failed = results.iter().filter(|r| r.is_failure()).count();
                        if ctx.verbose || failed > 0 {
                            println!(
                                "Plugin sync: {} cloned, {} updated, {} failed",
                                cloned, updated, failed
                            );
                        }
                    } else if ctx.verbose {
                        println!("Plugin auto-update: disabled");
                    }
                    plugin_dirs.extend(sub_manager.plugin_dirs());
                }
                Err(e) => {
                    tracing::warn!("failed to load plugin subscriptions: {}", e);
                }
            }
        }

        let manager = PluginManager::new(plugin_dirs);
        let watcher = PluginWatcher::new(manager);
        let _events = watcher.load_initial().await;

        {
            let state = watcher.state();
            let st = state.read().await;

            for plugin in st.plugins() {
                if let Some(ref hooks_config) = plugin.hooks_config {
                    hook_dispatcher.register_from_config(hooks_config, &plugin.plugin_dir);
                    if ctx.verbose {
                        let hook_count =
                            hooks_config.hooks.values().map(|v| v.len()).sum::<usize>();
                        if hook_count > 0 {
                            println!(
                                "  Plugin '{}': {} hook(s) registered",
                                plugin.manifest.name, hook_count
                            );
                        }
                    }
                }
                for loaded_agent in &plugin.agent_configs {
                    agent_configs.insert(
                        loaded_agent.config.agent.name.clone(),
                        loaded_agent.config.clone(),
                    );
                    agent_sources.insert(
                        loaded_agent.config.agent.name.clone(),
                        plugin.manifest.name.clone(),
                    );
                    if ctx.verbose {
                        println!(
                            "  Plugin '{}': agent '{}' registered",
                            plugin.manifest.name, loaded_agent.config.agent.name
                        );
                    }
                }
                if !plugin.skill_contents.is_empty() {
                    let mut prompt_parts: Vec<String> = Vec::new();
                    prompt_parts.push("Available skills:".to_string());
                    for skill in &plugin.skill_contents {
                        prompt_parts.push(format!(
                            "- `/{name}`: {desc}",
                            name = skill.def.name,
                            desc = skill.def.description
                        ));
                    }
                    prompts.push((plugin.manifest.name.clone(), prompt_parts.join("\n")));
                    if ctx.verbose {
                        println!(
                            "  Plugin '{}': {} skill prompt(s) registered",
                            plugin.manifest.name,
                            plugin.skill_contents.len()
                        );
                    }
                }
            }

            if ctx.verbose || !st.is_empty() {
                println!(
                    "Plugins: {} loaded ({})",
                    st.len(),
                    st.plugins()
                        .iter()
                        .map(|p| p.manifest.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
            if !hook_dispatcher.is_empty() && ctx.verbose {
                println!("Hooks: {} total registered", hook_dispatcher.len());
            }
        }

        if plugins_cfg.hot_reload {
            match watcher.watch() {
                Ok((mut rx, handle)) => {
                    tokio::spawn(async move {
                        while let Some(event) = rx.recv().await {
                            match event {
                                arawn_plugin::PluginEvent::Reloaded { name, .. } => {
                                    tracing::info!(plugin = %name, "plugin reloaded");
                                }
                                arawn_plugin::PluginEvent::Removed { name, .. } => {
                                    tracing::info!(plugin = %name, "plugin removed");
                                }
                                arawn_plugin::PluginEvent::Error { plugin_dir, error } => {
                                    tracing::warn!(dir = %plugin_dir.display(), error = %error, "plugin reload failed");
                                }
                            }
                        }
                    });
                    Some(handle)
                }
                Err(e) => {
                    tracing::warn!("failed to start plugin watcher: {}", e);
                    None
                }
            }
        } else {
            if ctx.verbose {
                println!("Plugin hot-reload: disabled");
            }
            None
        }
    } else {
        if ctx.verbose {
            println!("Plugin system: disabled");
        }
        None
    };

    PluginInitResult {
        prompts,
        hook_dispatcher,
        agent_configs,
        agent_sources,
        _watcher: watcher_handle,
    }
}

/// Phase 8: Create tool registry with all built-in tools and output config.
pub(super) fn init_tool_registry(
    tools_cfg: &arawn_config::ToolsConfig,
    memory_store: &Option<Arc<MemoryStore>>,
) -> Result<ToolRegistry> {
    use arawn_agent::OutputConfig;

    tracing::debug!(
        shell_timeout_secs = tools_cfg.shell.timeout_secs,
        web_timeout_secs = tools_cfg.web.timeout_secs,
        max_output_bytes = tools_cfg.output.max_size_bytes,
        "Tool configuration loaded"
    );

    let shell_output_limit = tools_cfg.output.shell.unwrap_or(100 * 1024);
    let shell_config = tools::ShellConfig::new()
        .with_timeout(Duration::from_secs(tools_cfg.shell.timeout_secs))
        .with_max_output_size(shell_output_limit);

    let web_output_limit = tools_cfg.output.web_fetch.unwrap_or(200 * 1024);
    let web_config = tools::WebFetchConfig {
        timeout: Duration::from_secs(tools_cfg.web.timeout_secs),
        max_text_length: web_output_limit,
        ..Default::default()
    };

    let mut registry = ToolRegistry::new();
    registry.register(tools::ShellTool::with_config(shell_config));
    registry.register(tools::FileReadTool::new());
    registry.register(tools::FileWriteTool::new());
    registry.register(tools::GlobTool::new());
    registry.register(tools::GrepTool::new());
    registry.register(tools::WebFetchTool::with_config(web_config)?);
    registry.register(tools::WebSearchTool::new()?);
    registry.register(tools::NoteTool::new());
    match memory_store {
        Some(store) => registry.register(tools::MemorySearchTool::with_store(store.clone())),
        None => registry.register(tools::MemorySearchTool::new()),
    }

    // Wire per-tool output config overrides
    let output_cfg = &tools_cfg.output;
    if let Some(v) = output_cfg.shell {
        registry.set_output_config("shell", OutputConfig::with_max_size(v));
        registry.set_output_config("bash", OutputConfig::with_max_size(v));
    }
    if let Some(v) = output_cfg.file_read {
        registry.set_output_config("file_read", OutputConfig::with_max_size(v));
        registry.set_output_config("read_file", OutputConfig::with_max_size(v));
    }
    if let Some(v) = output_cfg.web_fetch {
        registry.set_output_config("web_fetch", OutputConfig::with_max_size(v));
        registry.set_output_config("fetch", OutputConfig::with_max_size(v));
    }
    if let Some(v) = output_cfg.search {
        registry.set_output_config("grep", OutputConfig::with_max_size(v));
        registry.set_output_config("glob", OutputConfig::with_max_size(v));
        registry.set_output_config("search", OutputConfig::with_max_size(v));
        registry.set_output_config("memory_search", OutputConfig::with_max_size(v));
    }

    Ok(registry)
}
