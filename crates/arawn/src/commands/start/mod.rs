//! Start command - launches the Arawn server.

mod agent;
mod config;
mod llm;
mod pipeline;
mod server;
mod storage;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use clap::Args;

use arawn_agent::ToolRegistry;
use arawn_agent_tools as tools;

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
    let loaded = config::load_and_validate_config(&args, ctx)?;
    let cfg = &loaded.config;

    // ── Clean up old log files ──────────────────────────────────────────
    if let Some(log_dir) = arawn_config::xdg_config_dir().map(|d| d.join("logs")) {
        server::cleanup_old_logs(&log_dir, 30, ctx.verbose);
    }

    // ── Resolve LLM backends ────────────────────────────────────────────
    let (resolved, backend, backends) = llm::init_llm_backends(cfg, &args, ctx).await?;

    // ── Server settings + auth ──────────────────────────────────────────
    let (addr, workspace, bootstrap_dir, auth_token) =
        config::resolve_server_settings(cfg, &args, ctx)?;
    let server_cfg = cfg.server.as_ref();

    // ── Build embedder ────────────────────────────────────────────────────
    let embedder = storage::init_embedder(cfg, ctx).await?;

    // ── Infrastructure init ─────────────────────────────────────────────
    let data_dir = arawn_config::xdg_config_dir().unwrap_or_else(|| PathBuf::from("."));

    let pipeline_cfg = cfg.pipeline.clone().unwrap_or_default();
    let (pipeline_engine, pipeline_workflow_dir, mut _workflow_watcher_handle) =
        pipeline::init_pipeline(&pipeline_cfg, &data_dir, ctx).await;

    // ── Memory store ───────────────────────────────────────────────────
    let memory_cfg = cfg.memory.clone().unwrap_or_default();
    let memory_store = storage::init_memory_store(&memory_cfg, &data_dir, &embedder, ctx);

    // ── Tool registry ──────────────────────────────────────────────────
    let tools_cfg = cfg.tools.clone().unwrap_or_default();
    let mut tool_registry = agent::init_tool_registry(&tools_cfg, &memory_store)?;

    // Register pipeline tools (catalog + workflow) if pipeline is enabled
    if let Some(ref engine) = pipeline_engine {
        _workflow_watcher_handle = pipeline::register_pipeline_tools(
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
    let plugins_cfg = cfg.plugins.clone().unwrap_or_default();
    let plugins = agent::init_plugins(&plugins_cfg, workspace.as_deref(), ctx).await;
    let plugin_prompts = plugins.prompts;
    let hook_dispatcher = plugins.hook_dispatcher;
    let plugin_agent_configs = plugins.agent_configs;
    let plugin_agent_sources = plugins.agent_sources;
    let _watcher_handle = plugins._watcher;

    // ── MCP servers ──────────────────────────────────────────────────────
    let mcp_cfg = cfg.mcp.clone().unwrap_or_default();
    let mut mcp_manager = agent::init_mcp(&mcp_cfg, &mut tool_registry, ctx);

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
        if let Some(ref rlm_toml) = cfg.rlm {
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
        if let Some(max_iter) = cfg.agent.get("default").and_then(|a| a.max_iterations) {
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
    let built_agent = agent::build_agent(
        cfg,
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
    let indexer = agent::init_session_indexer(
        &memory_cfg,
        &memory_store,
        &backends,
        &embedder,
        &data_dir,
        ctx,
    )
    .await;

    // ── Assemble + start server ─────────────────────────────────────────
    let srv = server::assemble_server(
        cfg,
        server_cfg,
        addr,
        auth_token,
        built_agent,
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

    srv.run().await?;

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
