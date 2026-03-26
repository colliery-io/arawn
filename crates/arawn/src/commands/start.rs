//! Start command - launches the Arawn server.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use clap::Args;

use arawn_agent::{Agent, McpToolAdapter, PromptMode, SystemPromptBuilder, Tool, ToolRegistry};
#[cfg(feature = "gliner")]
use arawn_agent_indexing::{GlinerEngine, NerConfig};
use arawn_agent_indexing::{IndexerConfig, SessionIndexer};
use arawn_agent_tools as tools;
use arawn_config::EmbeddingProvider;
use arawn_config::{self, Backend, LlmConfig, ResolvedLlm};
use arawn_llm::{
    AnthropicBackend, AnthropicConfig, ApiKeyProvider, EmbedderSpec, OpenAiBackend, OpenAiConfig,
    SharedBackend,
};
use arawn_mcp::{McpManager, McpServerConfig};
use arawn_memory::{MemoryStore, init_vector_extension};
use arawn_oauth;
use arawn_pipeline::sandbox::ScriptExecutor;
use arawn_pipeline::{
    CatalogEntry, PipelineConfig, PipelineEngine, RuntimeCatalog, RuntimeCategory, WorkflowEvent,
    WorkflowLoader, build_executor_factory,
};
use arawn_plugin::{HookDispatcher, PluginManager, PluginWatcher, SubscriptionManager, SyncAction};
use arawn_server::{AppState, Server, ServerConfig};
use arawn_workstream::{WorkstreamConfig as WsConfig, WorkstreamFsGate, WorkstreamManager};
use tokio::sync::RwLock;

use super::Context;

/// Arguments for the start command.
///
/// CLI arguments override config file values.
#[derive(Args, Debug)]
#[command(after_help = "\x1b[1mExamples:\x1b[0m
  arawn start                       Start with config file defaults
  arawn start -p 9090               Start on port 9090
  arawn start -d                    Start as a background daemon
  arawn start --backend anthropic   Start with a specific LLM backend
  arawn start --token my-secret     Start with an explicit auth token")]
pub struct StartArgs {
    /// Run server in background (daemon mode)
    #[arg(short, long)]
    pub daemon: bool,

    /// Port to listen on (overrides config)
    #[arg(short, long)]
    pub port: Option<u16>,

    /// Address to bind to (overrides config)
    #[arg(short, long)]
    pub bind: Option<String>,

    /// API token for authentication (or set ARAWN_API_TOKEN env var)
    #[arg(long, env = "ARAWN_API_TOKEN")]
    pub token: Option<String>,

    /// LLM backend (overrides config)
    #[arg(long)]
    pub backend: Option<String>,

    /// API key (overrides config and keyring)
    #[arg(long)]
    pub api_key: Option<String>,

    /// Custom base URL (overrides config)
    #[arg(long)]
    pub base_url: Option<String>,

    /// Model (overrides config)
    #[arg(long)]
    pub model: Option<String>,

    /// Working directory for file operations (overrides config)
    #[arg(long)]
    pub workspace: Option<PathBuf>,

    /// Path to directory containing bootstrap files (overrides config)
    #[arg(long)]
    pub bootstrap_dir: Option<PathBuf>,

    /// Additional prompt file to load (can be specified multiple times)
    #[arg(long)]
    pub prompt_file: Vec<PathBuf>,

    /// Path to config file (overrides default discovery)
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Seed the server with test workstreams and sessions (dev mode)
    #[arg(long)]
    pub seed: bool,
}

/// Run the start command.
/// Get the path to the PID file.
pub fn pid_file_path() -> std::path::PathBuf {
    arawn_config::xdg_config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("arawn.pid")
}

/// Stop a running daemon by sending SIGTERM to the PID in the PID file.
pub fn stop_daemon() -> Result<()> {
    let pid_path = pid_file_path();
    if !pid_path.exists() {
        anyhow::bail!(
            "No PID file found at {}. Is the daemon running?",
            pid_path.display()
        );
    }

    let pid_str = std::fs::read_to_string(&pid_path)
        .map_err(|e| anyhow::anyhow!("Failed to read PID file: {}", e))?;
    let pid: i32 = pid_str
        .trim()
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid PID in {}: {}", pid_path.display(), e))?;

    // Send SIGTERM via the kill command
    let status = std::process::Command::new("kill")
        .arg("-TERM")
        .arg(pid.to_string())
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("Sent SIGTERM to Arawn server (PID {})", pid);
            let _ = std::fs::remove_file(&pid_path);
        }
        _ => {
            let _ = std::fs::remove_file(&pid_path);
            anyhow::bail!("Process {} not running (stale PID file removed)", pid);
        }
    }

    Ok(())
}

pub async fn run(args: StartArgs, ctx: &Context) -> Result<()> {
    if args.daemon {
        // Re-exec ourselves without --daemon, in the background
        let exe = std::env::current_exe()
            .map_err(|e| anyhow::anyhow!("Failed to get current executable: {}", e))?;

        // Rebuild args without --daemon
        let mut new_args: Vec<String> = vec!["start".to_string()];
        if let Some(ref port) = args.port {
            new_args.extend(["--port".to_string(), port.to_string()]);
        }
        if let Some(ref bind) = args.bind {
            new_args.extend(["--bind".to_string(), bind.clone()]);
        }
        if let Some(ref config) = args.config {
            new_args.extend(["--config".to_string(), config.to_string_lossy().to_string()]);
        }

        // Redirect stdout/stderr to log files
        let log_dir = arawn_config::xdg_config_dir()
            .map(|d| d.join("logs"))
            .unwrap_or_else(|| std::path::PathBuf::from("logs"));
        std::fs::create_dir_all(&log_dir)?;

        let stdout_file = std::fs::File::create(log_dir.join("daemon-stdout.log"))?;
        let stderr_file = std::fs::File::create(log_dir.join("daemon-stderr.log"))?;

        let child = std::process::Command::new(&exe)
            .args(&new_args)
            .stdout(stdout_file)
            .stderr(stderr_file)
            .stdin(std::process::Stdio::null())
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to spawn daemon: {}", e))?;

        let pid = child.id();

        // Write PID file
        let pid_path = pid_file_path();
        std::fs::write(&pid_path, pid.to_string())
            .map_err(|e| anyhow::anyhow!("Failed to write PID file: {}", e))?;

        println!("Arawn daemon started (PID {})", pid);
        println!("PID file: {}", pid_path.display());
        println!("Stop with: arawn stop");
        return Ok(());
    }

    // ── Load configuration ──────────────────────────────────────────────
    let loaded = load_and_validate_config(&args, ctx)?;
    let config = &loaded.config;

    // ── Clean up old log files ──────────────────────────────────────────
    if let Some(log_dir) = arawn_config::xdg_config_dir().map(|d| d.join("logs")) {
        cleanup_old_logs(&log_dir, 30, ctx.verbose);
    }

    // ── Resolve LLM backends ────────────────────────────────────────────
    let (resolved, backend, backends) = init_llm_backends(config, &args, ctx).await?;

    // ── Server settings + auth ──────────────────────────────────────────
    let (addr, workspace, bootstrap_dir, auth_token) = resolve_server_settings(config, &args, ctx)?;
    let server_cfg = config.server.as_ref();

    // ── Build embedder ────────────────────────────────────────────────────
    let embedder = init_embedder(config, ctx).await?;

    // ── Infrastructure init ─────────────────────────────────────────────
    let data_dir = arawn_config::xdg_config_dir().unwrap_or_else(|| PathBuf::from("."));

    let pipeline_cfg = config.pipeline.clone().unwrap_or_default();
    let (pipeline_engine, pipeline_workflow_dir, mut _workflow_watcher_handle) =
        init_pipeline(&pipeline_cfg, &data_dir, ctx).await;

    // ── Memory store ───────────────────────────────────────────────────
    let memory_cfg = config.memory.clone().unwrap_or_default();
    let memory_store = init_memory_store(&memory_cfg, &data_dir, &embedder, ctx);

    // ── Tool registry ──────────────────────────────────────────────────
    let tools_cfg = config.tools.clone().unwrap_or_default();
    let mut tool_registry = init_tool_registry(&tools_cfg, &memory_store)?;

    // Register pipeline tools (catalog + workflow) if pipeline is enabled
    if let Some(ref engine) = pipeline_engine {
        _workflow_watcher_handle = register_pipeline_tools(
            engine,
            &pipeline_cfg,
            &pipeline_workflow_dir,
            &data_dir,
            &mut tool_registry,
            ctx,
        )
        .await;
    }

    // ── Plugin system ────────────────────────────────────────────────────
    let plugins_cfg = config.plugins.clone().unwrap_or_default();
    let plugins = init_plugins(&plugins_cfg, workspace.as_deref(), ctx).await;
    let plugin_prompts = plugins.prompts;
    let hook_dispatcher = plugins.hook_dispatcher;
    let plugin_agent_configs = plugins.agent_configs;
    let plugin_agent_sources = plugins.agent_sources;
    let _watcher_handle = plugins._watcher;

    // ── MCP servers ──────────────────────────────────────────────────────
    let mcp_cfg = config.mcp.clone().unwrap_or_default();
    let mut mcp_manager = init_mcp(&mcp_cfg, &mut tool_registry, ctx);

    // ── Hook dispatcher (shared between agent and subagent spawner) ─────────

    // Create the shared hook dispatcher early so it can be used by both
    // the agent and the subagent spawner for background execution events
    let shared_hook_dispatcher: Option<arawn_types::SharedHookDispatcher> =
        if !hook_dispatcher.is_empty() {
            Some(Arc::new(hook_dispatcher))
        } else {
            None
        };

    // ── Explore tool (RLM exploration agent) ────────────────────────────────

    {
        use arawn_agent::{RlmConfig, RlmSpawner};
        use arawn_agent_tools::ExploreTool;

        let mut rlm_config = RlmConfig::default();

        // Apply [rlm] config overrides from arawn.toml
        if let Some(ref rlm_toml) = config.rlm {
            if let Some(ref model) = rlm_toml.model {
                rlm_config.model = model.clone();
            }
            if let Some(max_turns) = rlm_toml.max_turns {
                rlm_config.max_turns = max_turns;
            }
            if let Some(max_ctx) = rlm_toml.max_context_tokens {
                rlm_config.max_context_tokens = max_ctx;
            }
            if let Some(threshold) = rlm_toml.compaction_threshold {
                rlm_config.compaction_threshold = threshold;
            }
            if let Some(max_c) = rlm_toml.max_compactions {
                rlm_config.max_compactions = max_c;
            }
            if let Some(max_t) = rlm_toml.max_total_tokens {
                rlm_config.max_total_tokens = Some(max_t);
            }
            if let Some(ref c_model) = rlm_toml.compaction_model {
                rlm_config.compaction_model = Some(c_model.clone());
            }
        }

        let spawner =
            RlmSpawner::new(backend.clone(), tool_registry.clone()).with_config(rlm_config);

        tool_registry.register(ExploreTool::new(Arc::new(spawner)));

        if ctx.verbose {
            println!("Explore tool: RLM exploration agent registered");
        }
    }

    // ── Delegate tool (subagent delegation) ────────────────────────────────

    // Create delegate tool if any plugin agents are defined
    if !plugin_agent_configs.is_empty() {
        let parent_tools = Arc::new(tool_registry);
        let mut spawner = arawn_plugin::PluginSubagentSpawner::with_sources(
            parent_tools.clone(),
            backend.clone(),
            plugin_agent_configs,
            plugin_agent_sources,
        );

        // Wire default max_iterations from [agent.default] config
        if let Some(max_iter) = config.agent.get("default").and_then(|a| a.max_iterations) {
            spawner = spawner.with_default_max_iterations(max_iter);
        }

        // Wire hook dispatcher for background subagent events
        if let Some(ref dispatcher) = shared_hook_dispatcher {
            spawner = spawner.with_hook_dispatcher(dispatcher.clone());
        }

        // Create a new mutable registry and copy tools from the Arc'd one
        let mut new_registry = ToolRegistry::new();
        for name in parent_tools.names() {
            if let Some(tool) = parent_tools.get(name) {
                new_registry.register_arc(tool);
            }
        }
        new_registry.register(tools::DelegateTool::new(Arc::new(spawner)));

        if ctx.verbose {
            println!(
                "Delegate tool: {} subagent(s) available",
                new_registry
                    .names()
                    .iter()
                    .filter(|n| *n != &"delegate")
                    .count()
            );
        }

        tool_registry = new_registry;
    }

    // ── Build agent ──────────────────────────────────────────────────────
    let agent = build_agent(
        config,
        &resolved,
        backend,
        tool_registry,
        plugin_prompts,
        &shared_hook_dispatcher,
        &workspace,
        &bootstrap_dir,
        &args.prompt_file,
        &memory_store,
        &embedder,
        &data_dir,
        ctx,
    )
    .await?;

    // ── Session indexer ──────────────────────────────────────────────────
    let indexer = init_session_indexer(
        &memory_cfg,
        &memory_store,
        &backends,
        &embedder,
        &data_dir,
        ctx,
    )
    .await;

    // ── Assemble + start server ─────────────────────────────────────────
    let server = assemble_server(
        config,
        server_cfg,
        addr,
        auth_token,
        agent,
        indexer,
        memory_store,
        shared_hook_dispatcher,
        &mut mcp_manager,
        &backends,
        &data_dir,
        args.seed,
        ctx,
    )
    .await;

    println!("Arawn server starting on http://{}", addr);
    println!("Press Ctrl+C to stop");

    server.run().await?;

    // ── Graceful shutdown ──────────────────────────────────────────────

    if let Some(engine) = pipeline_engine
        && let Ok(engine) = Arc::try_unwrap(engine)
        && let Err(e) = engine.shutdown().await
    {
        tracing::warn!("pipeline shutdown error: {}", e);
    }

    // Shutdown MCP servers
    if let Some(ref mut manager) = mcp_manager {
        if ctx.verbose {
            println!("Shutting down MCP servers...");
        }
        if let Err(e) = manager.shutdown_all() {
            tracing::warn!("MCP shutdown error: {}", e);
        }
    }

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Extracted initialization phases
// ─────────────────────────────────────────────────────────────────────────────

/// Phase 1-2: Load config from file or discovery, print warnings, validate.
fn load_and_validate_config(args: &StartArgs, ctx: &Context) -> Result<arawn_config::LoadedConfig> {
    let loaded = if let Some(ref config_path) = args.config {
        let config = arawn_config::load_config_file(config_path)?;
        let source = arawn_config::discovery::ConfigSource {
            path: config_path.clone(),
            loaded: true,
        };
        arawn_config::LoadedConfig {
            config,
            sources: vec![source.clone()],
            source: Some(source),
            warnings: Vec::new(),
        }
    } else {
        arawn_config::load_config(args.workspace.as_deref())?
    };

    for warning in &loaded.warnings {
        tracing::warn!("{}", warning);
    }

    if ctx.verbose {
        let sources = loaded.loaded_from();
        if sources.is_empty() {
            println!("No config files found, using defaults + CLI args");
        } else {
            for source in sources {
                println!("Loaded config: {}", source.display());
            }
        }
    }

    validate_config(&loaded.config)?;
    Ok(loaded)
}

/// Phase 3: Resolve LLM backends (default + named profiles).
async fn init_llm_backends(
    config: &arawn_config::ArawnConfig,
    args: &StartArgs,
    ctx: &Context,
) -> Result<(ResolvedLlm, SharedBackend, HashMap<String, SharedBackend>)> {
    let resolved = resolve_with_cli_overrides(config, args)?;

    if ctx.verbose {
        println!("Backend: {}", resolved.backend);
        println!("Model: {}", resolved.model);
        println!("Resolved via: {}", resolved.resolved_from);
        if let Some(ref source) = resolved.api_key_source {
            println!("API key from: {}", source);
        }
    }

    let backend = create_backend(&resolved, config.oauth.as_ref()).await?;

    let mut backends: HashMap<String, SharedBackend> = HashMap::new();
    backends.insert("default".to_string(), backend.clone());

    for (name, llm_config) in &config.llm_profiles {
        match resolve_profile(name, llm_config) {
            Ok(profile_resolved) => {
                match create_backend(&profile_resolved, config.oauth.as_ref()).await {
                    Ok(profile_backend) => {
                        if ctx.verbose {
                            println!(
                                "Backend '{}': {} / {}",
                                name, profile_resolved.backend, profile_resolved.model
                            );
                        }
                        backends.insert(name.clone(), profile_backend);
                    }
                    Err(e) => {
                        tracing::warn!("failed to create backend '{}': {}", name, e);
                    }
                }
            }
            Err(e) => {
                tracing::warn!("failed to resolve profile '{}': {}", name, e);
            }
        }
    }

    if ctx.verbose && backends.len() > 1 {
        println!(
            "Available backends: {}",
            backends
                .keys()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    Ok((resolved, backend, backends))
}

/// Phase 4: Resolve server bind address, workspace, bootstrap dir, and auth token.
/// Resolved server settings from config + CLI args.
type ServerSettings = (SocketAddr, Option<PathBuf>, Option<PathBuf>, Option<String>);

fn resolve_server_settings(
    config: &arawn_config::ArawnConfig,
    args: &StartArgs,
    ctx: &Context,
) -> Result<ServerSettings> {
    let server_cfg = config.server.as_ref();
    let port = args
        .port
        .or_else(|| server_cfg.map(|s| s.port))
        .unwrap_or(arawn_types::config::defaults::DEFAULT_PORT);
    let bind = args
        .bind
        .clone()
        .or_else(|| server_cfg.map(|s| s.bind.clone()))
        .unwrap_or_else(|| arawn_types::config::defaults::DEFAULT_BIND.to_string());
    let addr: SocketAddr = format!("{}:{}", bind, port).parse()?;

    let workspace = args
        .workspace
        .clone()
        .or_else(|| server_cfg.and_then(|s| s.workspace.clone()));
    let bootstrap_dir = args
        .bootstrap_dir
        .clone()
        .or_else(|| server_cfg.and_then(|s| s.bootstrap_dir.clone()));

    let explicit_token = args
        .token
        .clone()
        .or_else(|| std::env::var("ARAWN_API_TOKEN").ok());
    let auth_token: Option<String> = if let Some(token) = explicit_token {
        Some(token)
    } else if addr.ip().is_loopback() {
        None
    } else {
        let token = load_or_generate_server_token()?;
        println!("Server auth token: {}", token);
        Some(token)
    };

    if ctx.verbose {
        println!("Bind address: {}", addr);
        match &auth_token {
            Some(t) => println!("Auth token: {}...", &t[..8.min(t.len())]),
            None => println!("Auth: disabled (localhost)"),
        }
    }

    Ok((addr, workspace, bootstrap_dir, auth_token))
}

/// Phase 8: Create tool registry with all built-in tools and output config.
fn init_tool_registry(
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

/// Phase 5: Initialize the embedding provider.
async fn init_embedder(
    config: &arawn_config::ArawnConfig,
    ctx: &Context,
) -> Result<Arc<dyn arawn_llm::Embedder>> {
    let embedding_config = config.embedding.clone().unwrap_or_default();
    let embedder_spec = build_embedder_spec(&embedding_config);

    if ctx.verbose {
        println!(
            "Embedding provider: {:?} ({}d)",
            embedding_config.provider,
            embedding_config.effective_dimensions()
        );
    }

    let embedder = arawn_llm::build_embedder(&embedder_spec).await?;

    if ctx.verbose {
        println!("Embedder: {} ({}d)", embedder.name(), embedder.dimensions());
    }

    Ok(embedder)
}

/// Phase 6: Initialize the pipeline engine.
async fn init_pipeline(
    pipeline_cfg: &arawn_config::PipelineSection,
    data_dir: &std::path::Path,
    ctx: &Context,
) -> (
    Option<Arc<PipelineEngine>>,
    PathBuf,
    Option<arawn_pipeline::WatcherHandle>,
) {
    let resolve_path = |p: Option<PathBuf>, default: &str| -> PathBuf {
        let p = p.unwrap_or_else(|| PathBuf::from(default));
        if p.is_relative() { data_dir.join(p) } else { p }
    };

    let pipeline_db_path = resolve_path(pipeline_cfg.database.clone(), "pipeline.db");
    let pipeline_workflow_dir = resolve_path(pipeline_cfg.workflow_dir.clone(), "workflows");

    if !pipeline_cfg.enabled {
        if ctx.verbose {
            println!("Pipeline engine: disabled");
        }
        return (None, pipeline_workflow_dir, None);
    }

    let engine_config = PipelineConfig {
        max_concurrent_tasks: pipeline_cfg.max_concurrent_tasks,
        task_timeout_secs: pipeline_cfg.task_timeout_secs,
        pipeline_timeout_secs: pipeline_cfg.pipeline_timeout_secs,
        cron_enabled: pipeline_cfg.cron_enabled,
        triggers_enabled: pipeline_cfg.triggers_enabled,
    };

    if let Err(e) = std::fs::create_dir_all(&pipeline_workflow_dir) {
        tracing::warn!("failed to create workflow directory: {}", e);
    }

    match PipelineEngine::new(&pipeline_db_path, engine_config).await {
        Ok(engine) => {
            let engine = Arc::new(engine);
            if ctx.verbose {
                println!(
                    "Pipeline engine: enabled (db: {}, workflows: {})",
                    pipeline_db_path.display(),
                    pipeline_workflow_dir.display(),
                );
            }
            (Some(engine), pipeline_workflow_dir, None)
        }
        Err(e) => {
            tracing::warn!("failed to start pipeline engine: {}", e);
            (None, pipeline_workflow_dir, None)
        }
    }
}

/// Phase 7: Initialize the memory store with graph + vector extensions.
fn init_memory_store(
    memory_cfg: &arawn_config::MemoryConfig,
    data_dir: &std::path::Path,
    embedder: &Arc<dyn arawn_llm::Embedder>,
    ctx: &Context,
) -> Option<Arc<MemoryStore>> {
    let memory_db_path = memory_cfg
        .database
        .clone()
        .map(|p| if p.is_relative() { data_dir.join(p) } else { p })
        .unwrap_or_else(|| data_dir.join("memory.db"));

    init_vector_extension();
    match MemoryStore::open(&memory_db_path) {
        Ok(mut store) => {
            let graph_db_path = memory_db_path.with_extension("graph.db");
            if let Err(e) = store.init_graph_at_path(&graph_db_path) {
                tracing::warn!("failed to init knowledge graph: {}", e);
            }
            if let Err(e) = store.init_vectors(embedder.dimensions(), embedder.name()) {
                tracing::warn!("failed to init vector store: {}", e);
            }
            store.wal_checkpoint();

            if ctx.verbose {
                println!("Memory store: {}", memory_db_path.display());
            }
            Some(Arc::new(store))
        }
        Err(e) => {
            tracing::warn!("failed to open memory store: {}", e);
            None
        }
    }
}

/// Result of plugin system initialization.
struct PluginInitResult {
    prompts: Vec<(String, String)>,
    hook_dispatcher: HookDispatcher,
    agent_configs: HashMap<String, arawn_plugin::PluginAgentConfig>,
    agent_sources: HashMap<String, String>,
    _watcher: Option<arawn_plugin::WatcherHandle>,
}

/// Phase 9: Register pipeline tools (CatalogTool, WorkflowTool) and start workflow hot-reload watcher.
async fn register_pipeline_tools(
    engine: &Arc<PipelineEngine>,
    pipeline_cfg: &arawn_config::PipelineSection,
    pipeline_workflow_dir: &std::path::Path,
    data_dir: &std::path::Path,
    tool_registry: &mut ToolRegistry,
    ctx: &Context,
) -> Option<arawn_pipeline::WatcherHandle> {
    // Load runtime catalog + script executor (with fallbacks)
    let (executor, catalog) = {
        let runtimes_dir = data_dir.join("runtimes");
        let catalog = match RuntimeCatalog::load(&runtimes_dir) {
            Ok(c) => {
                if ctx.verbose {
                    println!("Runtime catalog: {}", runtimes_dir.display());
                }
                Arc::new(RwLock::new(c))
            }
            Err(e) => {
                tracing::warn!(
                    " failed to load runtime catalog at {}: {}",
                    runtimes_dir.display(),
                    e
                );
                let fallback = std::env::temp_dir().join("arawn-runtimes");
                match RuntimeCatalog::load(&fallback) {
                    Ok(c) => {
                        tracing::warn!("using fallback catalog at {}", fallback.display());
                        Arc::new(RwLock::new(c))
                    }
                    Err(e2) => {
                        tracing::error!("failed to create fallback catalog: {}", e2);
                        return None;
                    }
                }
            }
        };

        let cache_dir = data_dir.join("wasm-cache");
        let executor = match ScriptExecutor::new(
            cache_dir.clone(),
            std::time::Duration::from_secs(pipeline_cfg.task_timeout_secs),
        ) {
            Ok(e) => {
                if ctx.verbose {
                    println!("Script executor: cache at {}", cache_dir.display());
                }
                Arc::new(e)
            }
            Err(e) => {
                tracing::warn!("failed to create script executor: {}", e);
                let fallback_cache = std::env::temp_dir().join("arawn-wasm-cache");
                match ScriptExecutor::new(
                    fallback_cache,
                    std::time::Duration::from_secs(pipeline_cfg.task_timeout_secs),
                ) {
                    Ok(e2) => {
                        tracing::warn!("using fallback WASM cache");
                        Arc::new(e2)
                    }
                    Err(e2) => {
                        tracing::error!("failed to create fallback executor: {}", e2);
                        return None;
                    }
                }
            }
        };

        (executor, catalog)
    };

    // Auto-compile built-in WASM runtimes
    let runtimes_src_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .map(|p| p.join("runtimes"));
    if let Some(ref src_dir) = runtimes_src_dir
        && src_dir.is_dir()
    {
        register_builtin_runtimes(src_dir, &executor, &catalog, ctx.verbose).await;
    }

    // Register CatalogTool + WorkflowTool
    tool_registry.register(tools::CatalogTool::new(catalog.clone(), executor.clone()));
    tool_registry.register(tools::WorkflowTool::new(
        engine.clone(),
        pipeline_workflow_dir.to_path_buf(),
        executor.clone(),
        catalog.clone(),
    ));

    // Load existing workflows + start hot-reload watcher

    match WorkflowLoader::new(pipeline_workflow_dir) {
        Ok(loader) => {
            let factory = build_executor_factory(executor.clone(), catalog.clone());

            let events = loader.load_all().await;
            for event in &events {
                if let WorkflowEvent::Loaded { name, path } = event {
                    let wf = match arawn_pipeline::WorkflowFile::from_file(path) {
                        Ok(wf) => wf,
                        Err(e) => {
                            tracing::warn!(" failed to parse workflow {}: {}", path.display(), e);
                            continue;
                        }
                    };
                    match wf.workflow.to_dynamic_tasks(&factory) {
                        Ok(tasks) => {
                            if let Err(e) = engine
                                .register_dynamic_workflow(name, &wf.workflow.description, tasks)
                                .await
                            {
                                tracing::warn!(" failed to register workflow {}: {}", name, e);
                            }
                        }
                        Err(e) => {
                            tracing::warn!(" failed to convert workflow {} tasks: {}", name, e)
                        }
                    }
                }
            }

            if ctx.verbose {
                let loaded = events
                    .iter()
                    .filter(|e| matches!(e, WorkflowEvent::Loaded { .. }))
                    .count();
                if loaded > 0 {
                    println!("Workflow loader: {} workflows loaded", loaded);
                }
            }

            match loader.watch() {
                Ok((mut event_rx, handle)) => {
                    let engine_w = engine.clone();
                    let factory_w = build_executor_factory(executor, catalog);
                    tokio::spawn(async move {
                        while let Some(event) = event_rx.recv().await {
                            match event {
                                WorkflowEvent::Loaded { name, path } => {
                                    let wf = match arawn_pipeline::WorkflowFile::from_file(&path) {
                                        Ok(wf) => wf,
                                        Err(e) => {
                                            tracing::warn!(
                                                "Hot-reload: failed to parse {}: {}",
                                                path.display(),
                                                e
                                            );
                                            continue;
                                        }
                                    };
                                    match wf.workflow.to_dynamic_tasks(&factory_w) {
                                        Ok(tasks) => {
                                            if let Err(e) = engine_w
                                                .register_dynamic_workflow(
                                                    &name,
                                                    &wf.workflow.description,
                                                    tasks,
                                                )
                                                .await
                                            {
                                                tracing::warn!(
                                                    "Hot-reload: failed to register {}: {}",
                                                    name,
                                                    e
                                                );
                                            } else {
                                                tracing::info!(
                                                    "Hot-reload: workflow {} reloaded",
                                                    name
                                                );
                                            }
                                        }
                                        Err(e) => tracing::warn!(
                                            "Hot-reload: failed to convert {} tasks: {}",
                                            name,
                                            e
                                        ),
                                    }
                                }
                                WorkflowEvent::Removed { name, .. } => {
                                    tracing::info!("Hot-reload: workflow {} removed", name)
                                }
                                WorkflowEvent::Error { path, error } => tracing::warn!(
                                    "Hot-reload: error processing {}: {}",
                                    path.display(),
                                    error
                                ),
                            }
                        }
                    });
                    if ctx.verbose {
                        println!("Workflow watcher: enabled");
                    }
                    Some(handle)
                }
                Err(e) => {
                    tracing::warn!("failed to start workflow watcher: {}", e);
                    None
                }
            }
        }
        Err(e) => {
            tracing::warn!("failed to create workflow loader: {}", e);
            None
        }
    }
}

/// Phase 15: Assemble server config, AppState, workstreams, session cache, and compressor.
#[allow(clippy::too_many_arguments)]
async fn assemble_server(
    config: &arawn_config::ArawnConfig,
    server_cfg: Option<&arawn_config::ServerConfig>,
    addr: SocketAddr,
    auth_token: Option<String>,
    agent: arawn_agent::Agent,
    indexer: Option<SessionIndexer>,
    memory_store: Option<Arc<MemoryStore>>,
    shared_hook_dispatcher: Option<arawn_types::SharedHookDispatcher>,
    mcp_manager: &mut Option<McpManager>,
    backends: &HashMap<String, arawn_llm::SharedBackend>,
    data_dir: &std::path::Path,
    seed: bool,
    ctx: &Context,
) -> Server {
    let rate_limiting = server_cfg.map(|s| s.rate_limiting).unwrap_or(true);
    let request_logging = server_cfg.map(|s| s.request_logging).unwrap_or(true);
    let api_rpm = server_cfg
        .map(|s| s.api_rpm)
        .unwrap_or(arawn_types::config::defaults::REQUESTS_PER_MINUTE);

    let ws_allowed_origins = server_cfg
        .and_then(|s| {
            if s.ws_allowed_origins.is_empty() {
                None
            } else {
                Some(s.ws_allowed_origins.clone())
            }
        })
        .unwrap_or_default();

    let mut server_config = ServerConfig::new(auth_token)
        .with_bind_address(addr)
        .with_rate_limiting(rate_limiting)
        .with_request_logging(request_logging)
        .with_api_rpm(api_rpm)
        .with_trust_proxy(server_cfg.map(|s| s.trust_proxy).unwrap_or(false));

    if !ws_allowed_origins.is_empty() {
        server_config = server_config.with_ws_allowed_origins(ws_allowed_origins);
    } else if server_config.auth_token.is_some() {
        server_config = server_config.with_ws_allowed_origins(vec![
            "http://localhost".to_string(),
            "http://127.0.0.1".to_string(),
            "http://[::1]".to_string(),
            "https://localhost".to_string(),
            "https://127.0.0.1".to_string(),
            "https://[::1]".to_string(),
        ]);
    }

    let mut app_state = AppState::new(agent, server_config);
    if let Some(idx) = indexer {
        app_state = app_state.with_indexer(idx);
    }
    if let Some(store) = memory_store {
        app_state.services.memory_store = Some(store);
    }
    if let Some(dispatcher) = shared_hook_dispatcher {
        app_state = app_state.with_hook_dispatcher(dispatcher);
    }
    if let Some(manager) = mcp_manager.take() {
        app_state = app_state.with_mcp_manager(manager);
    }

    // Workstreams
    let ws_cfg = config.workstream.clone().unwrap_or_default();
    let ws_config = WsConfig {
        db_path: ws_cfg
            .database
            .map(|p| if p.is_relative() { data_dir.join(p) } else { p })
            .unwrap_or_else(|| data_dir.join("workstreams.db")),
        data_dir: ws_cfg
            .data_dir
            .map(|p| if p.is_relative() { data_dir.join(p) } else { p })
            .unwrap_or_else(|| data_dir.join("workstreams")),
        session_timeout_minutes: ws_cfg.session_timeout_minutes,
    };

    match WorkstreamManager::new(&ws_config) {
        Ok(mgr) => {
            if seed {
                seed_test_data(&mgr, ctx.verbose);
            }
            app_state = app_state.with_workstreams(mgr);
            if ctx.verbose {
                println!(
                    "Workstreams: db={}, data={}",
                    ws_config.db_path.display(),
                    ws_config.data_dir.display()
                );
            }
        }
        Err(e) => tracing::warn!("failed to init workstreams: {}", e),
    }

    // Session cache
    let session_cfg = config.session.clone().unwrap_or_default();
    app_state = app_state.with_session_config(&session_cfg);
    tracing::debug!(
        max_sessions = session_cfg.max_sessions,
        "Session cache configured"
    );

    // Session compressor
    if let Some(compression_cfg) = config
        .workstream
        .as_ref()
        .and_then(|ws| ws.compression.as_ref())
        && compression_cfg.enabled
        && app_state.workstreams().is_some()
    {
        let compression_backend = backends
            .get(&compression_cfg.backend)
            .or_else(|| backends.get("default"))
            .cloned();
        match compression_backend {
            Some(cb) => {
                let compressor_config = arawn_workstream::CompressorConfig {
                    model: compression_cfg.model.clone(),
                    max_summary_tokens: compression_cfg.max_summary_tokens,
                    token_threshold_chars: compression_cfg.token_threshold_chars,
                };
                app_state = app_state
                    .with_compressor(arawn_workstream::Compressor::new(cb, compressor_config));
                if ctx.verbose {
                    println!(
                        "Session compression: enabled (backend={}, model={})",
                        compression_cfg.backend, compression_cfg.model
                    );
                }
            }
            None => tracing::warn!(
                " compression backend '{}' not found",
                compression_cfg.backend
            ),
        }
    }

    Server::from_state(app_state)
}

/// Phase 13: Build the agent with all configuration, tools, prompts, hooks, and sandboxing.
#[allow(clippy::too_many_arguments)]
async fn build_agent(
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
async fn init_session_indexer(
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
fn init_mcp(
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
async fn init_plugins(
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

/// Resolve LLM config, applying CLI overrides on top of config file values.
fn resolve_with_cli_overrides(
    config: &arawn_config::ArawnConfig,
    args: &StartArgs,
) -> Result<ResolvedLlm> {
    // Try config-based resolution first
    let mut resolved = match arawn_config::resolve_for_agent(config, "default") {
        Ok(r) => r,
        Err(_) => {
            // No config — build from CLI args or fail
            let backend_str = args.backend.as_deref().unwrap_or("anthropic");
            let backend = parse_backend(backend_str)?;
            let model = args
                .model
                .clone()
                .unwrap_or_else(|| default_model(&backend));

            let api_key = args.api_key.clone().or_else(|| {
                let ref_name = backend.env_var();
                arawn_config::secrets::resolve_api_key_ref(ref_name).map(|r| r.value)
            });

            ResolvedLlm {
                backend,
                model,
                base_url: args.base_url.clone(),
                api_key,
                api_key_source: None,
                api_key_ref: Some(backend.env_var().to_string()),
                resolved_from: arawn_config::ResolvedFrom::GlobalDefault,
                retry_max: None,
                retry_backoff_ms: None,
            }
        }
    };

    // CLI overrides
    if let Some(ref backend_str) = args.backend {
        resolved.backend = parse_backend(backend_str)?;
    }
    if let Some(ref model) = args.model {
        resolved.model = model.clone();
    }
    if let Some(ref base_url) = args.base_url {
        resolved.base_url = Some(base_url.clone());
    }
    if let Some(ref api_key) = args.api_key {
        resolved.api_key = Some(api_key.clone());
        resolved.api_key_source = None; // CLI override, no tracked source
    }

    Ok(resolved)
}

/// Build an `ApiKeyProvider` that re-resolves from the secret store on each request.
///
/// This enables hot-loading: secrets stored after server startup are picked up
/// automatically without a restart.
fn make_api_key_provider(ref_name: String) -> ApiKeyProvider {
    ApiKeyProvider::dynamic(move || {
        arawn_config::secrets::resolve_api_key_ref(&ref_name).map(|r| r.value)
    })
}

/// Create an LLM backend from a resolved config.
async fn create_backend(
    resolved: &ResolvedLlm,
    oauth_overrides: Option<&arawn_config::OAuthConfigOverride>,
) -> Result<SharedBackend> {
    match resolved.backend {
        Backend::Anthropic => {
            let provider = make_api_key_provider(
                resolved
                    .api_key_ref
                    .clone()
                    .unwrap_or_else(|| resolved.backend.env_var().to_string()),
            );
            let mut config = AnthropicConfig::new("placeholder");
            config.api_key = provider;
            if let Some(max) = resolved.retry_max {
                config = config.with_max_retries(max);
            }
            if let Some(ms) = resolved.retry_backoff_ms {
                config = config.with_retry_backoff(Duration::from_millis(ms));
            }
            Ok(Arc::new(AnthropicBackend::new(config)?))
        }
        Backend::Openai => {
            let provider = make_api_key_provider(
                resolved
                    .api_key_ref
                    .clone()
                    .unwrap_or_else(|| resolved.backend.env_var().to_string()),
            );
            let mut config = OpenAiConfig::openai("placeholder");
            config.api_key = provider;
            if let Some(ref base_url) = resolved.base_url {
                config = config.with_base_url(base_url);
            }
            config = config.with_model(&resolved.model);
            if let Some(max) = resolved.retry_max {
                config = config.with_max_retries(max);
            }
            if let Some(ms) = resolved.retry_backoff_ms {
                config = config.with_retry_backoff(Duration::from_millis(ms));
            }
            Ok(Arc::new(OpenAiBackend::new(config)?))
        }
        Backend::Groq => {
            let provider = make_api_key_provider(
                resolved
                    .api_key_ref
                    .clone()
                    .unwrap_or_else(|| resolved.backend.env_var().to_string()),
            );
            let mut config = OpenAiConfig::groq("placeholder");
            config.api_key = provider;
            config = config.with_model(&resolved.model);
            if let Some(max) = resolved.retry_max {
                config = config.with_max_retries(max);
            }
            if let Some(ms) = resolved.retry_backoff_ms {
                config = config.with_retry_backoff(Duration::from_millis(ms));
            }
            Ok(Arc::new(OpenAiBackend::new(config)?))
        }
        Backend::Ollama => {
            let mut config = OpenAiConfig::ollama();
            if let Some(ref base_url) = resolved.base_url {
                config = config.with_base_url(base_url);
            }
            config = config.with_model(&resolved.model);
            if let Some(max) = resolved.retry_max {
                config = config.with_max_retries(max);
            }
            if let Some(ms) = resolved.retry_backoff_ms {
                config = config.with_retry_backoff(Duration::from_millis(ms));
            }
            Ok(Arc::new(OpenAiBackend::new(config)?))
        }
        Backend::Custom => {
            let base_url = resolved.base_url.as_deref().ok_or_else(|| {
                anyhow::anyhow!("Custom backend requires base_url in config or --base-url")
            })?;
            let mut config = OpenAiConfig::openai("")
                .with_base_url(base_url)
                .with_name("custom")
                .with_model(&resolved.model);
            if let Some(ref api_key) = resolved.api_key {
                config.api_key = ApiKeyProvider::from_static(api_key);
            } else {
                config.api_key = ApiKeyProvider::None;
            }
            if let Some(max) = resolved.retry_max {
                config = config.with_max_retries(max);
            }
            if let Some(ms) = resolved.retry_backoff_ms {
                config = config.with_retry_backoff(Duration::from_millis(ms));
            }
            Ok(Arc::new(OpenAiBackend::new(config)?))
        }
        Backend::ClaudeOauth => {
            // IMPORTANT: The claude-oauth backend proxies through Claude Code's OAuth
            // infrastructure. It supports conversation only — tool use is NOT available.
            // This backend cannot drive agentic workflows (tool calling, file operations,
            // etc.). For full agent capabilities, use the `anthropic` backend with an
            // API key.
            //
            // WARNING: Using this backend outside of Claude Code may violate Anthropic's
            // Terms of Service. Use at your own risk.
            //
            // Start the OAuth proxy on a random port, then point AnthropicBackend at it
            let data_dir = arawn_config::xdg_config_dir()
                .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;

            let oauth_config = {
                let base = arawn_oauth::OAuthConfig::default();
                match oauth_overrides {
                    Some(o) => base.with_overrides(
                        o.client_id.as_deref(),
                        o.authorize_url.as_deref(),
                        o.token_url.as_deref(),
                        o.redirect_uri.as_deref(),
                        o.scope.as_deref(),
                    ),
                    None => base,
                }
            };

            let token_manager = arawn_oauth::token_manager::create_token_manager_with_config(
                &data_dir,
                oauth_config,
            );
            if !token_manager.has_tokens() {
                return Err(anyhow::anyhow!(
                    "No OAuth tokens found. Run 'arawn auth login' first to authenticate."
                ));
            }

            let proxy_config =
                arawn_oauth::ProxyConfig::default().with_token_manager(token_manager);

            let proxy = arawn_oauth::ProxyServer::new(proxy_config);
            let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
            let proxy_addr = proxy
                .run_with_shutdown(async {
                    shutdown_rx.await.ok();
                })
                .await
                .map_err(|e| anyhow::anyhow!("Failed to start OAuth proxy: {}", e))?;

            // Leak the shutdown sender so the proxy lives for the process lifetime
            std::mem::forget(shutdown_tx);

            let proxy_url = format!("http://{}", proxy_addr);
            println!("OAuth proxy running on {}", proxy_url);

            // Point Anthropic backend at the proxy — no API key needed,
            // proxy handles auth via OAuth tokens
            let config = AnthropicConfig::new("oauth-proxy-managed").with_base_url(&proxy_url);
            Ok(Arc::new(AnthropicBackend::new(config)?))
        }
    }
}

fn parse_backend(s: &str) -> Result<Backend> {
    match s.to_lowercase().as_str() {
        "anthropic" => Ok(Backend::Anthropic),
        "openai" => Ok(Backend::Openai),
        "groq" => Ok(Backend::Groq),
        "ollama" => Ok(Backend::Ollama),
        "custom" => Ok(Backend::Custom),
        "claude-oauth" | "claudeoauth" => Ok(Backend::ClaudeOauth),
        other => Err(anyhow::anyhow!(
            "Unknown backend '{}'. Valid: anthropic, openai, groq, ollama, custom, claude-oauth",
            other
        )),
    }
}

/// Load a persisted server token, or generate and save a new one.
fn load_or_generate_server_token() -> Result<String> {
    let dir = arawn_config::xdg_config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
    let token_path = dir.join("server-token");

    if token_path.exists() {
        let token = std::fs::read_to_string(&token_path)?.trim().to_string();
        if !token.is_empty() {
            return Ok(token);
        }
    }

    let token = uuid::Uuid::new_v4().to_string();
    std::fs::create_dir_all(&dir)?;
    std::fs::write(&token_path, &token)?;
    Ok(token)
}

/// Resolve a named LLM profile into a ResolvedLlm ready for backend creation.
fn resolve_profile(name: &str, llm_config: &LlmConfig) -> Result<ResolvedLlm> {
    let backend = llm_config
        .backend
        .ok_or_else(|| anyhow::anyhow!("Profile '{}' is missing 'backend' field", name))?;

    let model = llm_config
        .model
        .clone()
        .ok_or_else(|| anyhow::anyhow!("Profile '{}' is missing 'model' field", name))?;

    let api_key_ref = llm_config
        .api_key_ref
        .as_deref()
        .unwrap_or(backend.env_var());
    let resolved_secret = arawn_config::secrets::resolve_api_key_ref(api_key_ref);

    let (api_key, api_key_source) = match resolved_secret {
        Some(s) => (Some(s.value), Some(s.source)),
        None => (None, None),
    };

    Ok(ResolvedLlm {
        backend,
        model,
        base_url: llm_config.base_url.clone(),
        api_key,
        api_key_source,
        api_key_ref: Some(api_key_ref.to_string()),
        resolved_from: arawn_config::ResolvedFrom::AgentSpecific {
            agent: "profile".to_string(),
            profile: name.to_string(),
        },
        retry_max: llm_config.retry_max,
        retry_backoff_ms: llm_config.retry_backoff_ms,
    })
}

/// Build an `EmbedderSpec` from the application's `EmbeddingConfig`.
fn build_embedder_spec(config: &arawn_config::EmbeddingConfig) -> EmbedderSpec {
    let provider = match config.provider {
        EmbeddingProvider::Local => "local",
        EmbeddingProvider::OpenAi => "openai",
        EmbeddingProvider::Mock => "mock",
    };

    let (openai_api_key, openai_model, openai_base_url) = config
        .openai
        .as_ref()
        .map(|c| {
            let ref_name = c.api_key_ref.as_deref().unwrap_or("OPENAI_API_KEY");
            let api_key = arawn_config::secrets::resolve_api_key_ref(ref_name).map(|r| r.value);
            (api_key, Some(c.model.clone()), c.base_url.clone())
        })
        .unwrap_or((None, None, None));

    let (local_model_path, local_tokenizer_path, local_model_url, local_tokenizer_url) = config
        .local
        .as_ref()
        .map(|c| {
            (
                c.model_path.clone(),
                c.tokenizer_path.clone(),
                c.model_url.clone(),
                c.tokenizer_url.clone(),
            )
        })
        .unwrap_or((None, None, None, None));

    EmbedderSpec {
        provider: provider.to_string(),
        openai_api_key,
        openai_model,
        openai_base_url,
        local_model_path,
        local_tokenizer_path,
        dimensions: Some(config.effective_dimensions()),
        local_model_url,
        local_tokenizer_url,
    }
}

fn default_model(backend: &Backend) -> String {
    match backend {
        Backend::Anthropic | Backend::ClaudeOauth => "claude-sonnet-4-20250514".to_string(),
        Backend::Openai => "gpt-4o".to_string(),
        Backend::Groq => "llama-3.1-70b-versatile".to_string(),
        Backend::Ollama => "llama3.2".to_string(),
        Backend::Custom => "default".to_string(),
    }
}

/// Compile and register built-in WASM runtimes from source crate directories.
///
/// Scans `runtimes_src_dir` for subdirectories, each expected to be a Cargo crate.
/// For each, if the runtime isn't already in the catalog, compiles it to wasm32-wasip1
/// and registers the `.wasm` as a builtin entry.
async fn register_builtin_runtimes(
    runtimes_src_dir: &std::path::Path,
    executor: &Arc<ScriptExecutor>,
    catalog: &Arc<RwLock<RuntimeCatalog>>,
    verbose: bool,
) {
    let entries = match std::fs::read_dir(runtimes_src_dir) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!("cannot read runtimes source dir: {e}");
            return;
        }
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_dir() || !path.join("Cargo.toml").exists() {
            continue;
        }

        let runtime_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        // Skip if already registered
        {
            let cat = catalog.read().await;
            if cat.get(&runtime_name).is_some() {
                if verbose {
                    println!("Runtime '{}' already registered, skipping", runtime_name);
                }
                continue;
            }
        }

        if verbose {
            println!("Compiling runtime '{}'...", runtime_name);
        }

        let wasm_path = match executor.compile_crate(&path).await {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!(" failed to compile runtime '{}': {}", runtime_name, e);
                continue;
            }
        };

        // Copy .wasm to catalog's builtin/ directory
        let mut cat = catalog.write().await;
        let builtin_dir = cat.root().join("builtin");
        if let Err(e) = std::fs::create_dir_all(&builtin_dir) {
            tracing::warn!("cannot create builtin dir: {e}");
            continue;
        }

        let dest = builtin_dir.join(format!("{runtime_name}.wasm"));
        if let Err(e) = std::fs::copy(&wasm_path, &dest) {
            tracing::warn!("failed to copy wasm for '{}': {}", runtime_name, e);
            continue;
        }

        if let Err(e) = cat.add(
            &runtime_name,
            CatalogEntry {
                description: format!("Built-in {runtime_name} runtime"),
                path: format!("builtin/{runtime_name}.wasm"),
                category: RuntimeCategory::Builtin,
            },
        ) {
            tracing::warn!(" failed to register runtime '{}': {}", runtime_name, e);
            continue;
        }

        if verbose {
            println!("Registered runtime '{}'", runtime_name);
        }
    }
}

/// Seed the database with test workstreams and sessions for development.
fn seed_test_data(manager: &WorkstreamManager, verbose: bool) {
    use arawn_workstream::types::MessageRole;

    let test_workstreams = [
        ("Project Alpha", "Research project for quantum computing"),
        ("Code Review", "Daily code review sessions"),
        ("Documentation", "Writing and updating docs"),
        ("Bug Fixes", "Tracking and fixing bugs"),
    ];

    let mut created_count = 0;

    for (title, summary) in &test_workstreams {
        // Check if workstream already exists by listing and checking titles
        let existing = manager.list_workstreams().unwrap_or_default();
        if existing.iter().any(|ws| ws.title == *title) {
            if verbose {
                println!("  Seed: workstream '{}' already exists, skipping", title);
            }
            continue;
        }

        match manager.create_workstream(title, None, &[]) {
            Ok(ws) => {
                // Update with summary
                if let Err(e) = manager.update_workstream(&ws.id, None, Some(summary), None)
                    && verbose
                {
                    tracing::warn!("Seed: failed to set summary for '{}': {}", title, e);
                }

                // Add some test messages (sessions are created automatically)
                let test_conversations = [
                    (
                        "Hello! I'm starting work on this project.",
                        "Great! I'm ready to help. What would you like to work on first?",
                    ),
                    (
                        "Can you help me understand the architecture?",
                        "Of course! Let me explain the high-level structure...",
                    ),
                    (
                        "What are the next steps?",
                        "Based on our discussion, I recommend we focus on the core components first.",
                    ),
                ];

                for (user_msg, assistant_msg) in &test_conversations {
                    // Send user message (creates session if needed)
                    if let Err(e) = manager.send_message(
                        Some(&ws.id),
                        None, // No specific session for seed data
                        MessageRole::User,
                        user_msg,
                        None,
                    ) {
                        if verbose {
                            tracing::warn!("Seed: failed to add user message: {}", e);
                        }
                        continue;
                    }

                    // Send assistant response
                    if let Err(e) = manager.send_message(
                        Some(&ws.id),
                        None,
                        MessageRole::Assistant,
                        assistant_msg,
                        None,
                    ) && verbose
                    {
                        tracing::warn!("Seed: failed to add assistant message: {}", e);
                    }
                }

                created_count += 1;
                if verbose {
                    println!("  Seed: created workstream '{}' with test messages", title);
                }
            }
            Err(e) => {
                if verbose {
                    tracing::warn!("Seed: failed to create workstream '{}': {}", title, e);
                }
            }
        }
    }

    if created_count > 0 {
        println!("Seed: created {} test workstream(s)", created_count);
    } else if verbose {
        println!("Seed: no new workstreams created (already seeded)");
    }
}

/// Delete log files older than `max_age_days` from the log directory.
///
/// Runs on server startup to prevent disk exhaustion from accumulated logs.
/// Only deletes files matching `arawn.log.*` (the daily-rotated log files).
fn cleanup_old_logs(log_dir: &std::path::Path, max_age_days: u64, verbose: bool) {
    let Ok(entries) = std::fs::read_dir(log_dir) else {
        return;
    };

    let cutoff =
        std::time::SystemTime::now() - std::time::Duration::from_secs(max_age_days * 24 * 60 * 60);

    let mut deleted = 0u32;
    let mut errors = 0u32;

    for entry in entries.flatten() {
        let path = entry.path();

        // Only clean up rotated log files (arawn.log.YYYY-MM-DD)
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if !name_str.starts_with("arawn.log.") {
            continue;
        }

        let Ok(metadata) = entry.metadata() else {
            continue;
        };

        let Ok(modified) = metadata.modified() else {
            continue;
        };

        if modified < cutoff {
            match std::fs::remove_file(&path) {
                Ok(()) => {
                    deleted += 1;
                    if verbose {
                        println!("Log cleanup: deleted {}", path.display());
                    }
                }
                Err(e) => {
                    errors += 1;
                    tracing::warn!(path = %path.display(), error = %e, "Failed to delete old log file");
                }
            }
        }
    }

    if deleted > 0 || errors > 0 {
        tracing::info!(deleted, errors, max_age_days, "Log cleanup complete");
    }
}

/// Validate configuration values at startup, failing fast with clear errors.
fn validate_config(config: &arawn_config::ArawnConfig) -> Result<()> {
    if let Some(ref server) = config.server {
        if server.port == 0 {
            anyhow::bail!("Invalid config: server.port cannot be 0");
        }
        if server.api_rpm == 0 {
            anyhow::bail!("Invalid config: server.api_rpm must be > 0");
        }
    }

    if let Some(ref embedding) = config.embedding
        && embedding.dimensions == Some(0)
    {
        anyhow::bail!("Invalid config: embedding.dimensions must be > 0");
    }

    Ok(())
}
