use std::sync::Arc;

use anyhow::Result;
use tracing::{debug, info, warn};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

use arawn_core::Workstream;
use arawn_engine::{
    AgentTool, AskUserTool, BackgroundTaskManager, EnterPlanModeTool, ExitPlanModeTool,
    FileEditTool, FileReadTool, FileWriteTool, GlobTool, GrepTool, PlanModeState, PluginLoader,
    PluginWatcher, QueryEngineConfig, SessionTaskStore, ShellTool, SkillTool,
    SleepTool, TaskCreateTool, TaskGetTool, TaskListTool, TaskOutputTool, TaskStopTool,
    TaskUpdateTool, ThinkTool, ToolRegistry, WebFetchTool, WebSearchTool,
};
use arawn_engine::plugins::PluginRuntime;
use arawn_engine::skills::SkillRegistry;
use arawn_llm::RetryClient;
use arawn_storage::Store;

const DEFAULT_MODEL: &str = "llama-3.3-70b-versatile";

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .with_writer(std::io::stderr)
        .init();

    // Parse CLI args
    let args: Vec<String> = std::env::args().collect();
    let mut session_id: Option<Uuid> = None;
    let mut list_sessions = false;
    let mut serve_mode = false;
    let mut tui_mode = false;
    let mut serve_port: u16 = 3100;
    let mut tui_url = "ws://127.0.0.1:3100/ws".to_string();
    let mut prompt_parts: Vec<String> = Vec::new();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "serve" => {
                serve_mode = true;
            }
            "tui" => {
                tui_mode = true;
            }
            "plugin" => {
                let plugin_args: Vec<String> = args[i + 1..].to_vec();
                let plugins_root = dirs_path()
                    .map(|d| std::path::PathBuf::from(d).join("plugins"))
                    .unwrap_or_else(|| std::path::PathBuf::from(".arawn/plugins"));
                match arawn_bin::plugin_cmd::run_plugin_command(&plugin_args, &plugins_root) {
                    Ok(()) => std::process::exit(0),
                    Err(e) => {
                        eprintln!("Error: {e}");
                        std::process::exit(1);
                    }
                }
            }
            "--url" => {
                i += 1;
                if i < args.len() {
                    tui_url = args[i].clone();
                }
            }
            "--port" => {
                i += 1;
                if i < args.len() {
                    serve_port = args[i].parse().unwrap_or(3100);
                }
            }
            "--session" => {
                i += 1;
                if i < args.len() {
                    session_id = Some(
                        Uuid::parse_str(&args[i])
                            .map_err(|e| anyhow::anyhow!("invalid session UUID: {e}"))?,
                    );
                }
            }
            "--list-sessions" => {
                list_sessions = true;
            }
            _ => {
                prompt_parts.push(args[i].clone());
            }
        }
        i += 1;
    }

    // Load config — try ARAWN_DATA_DIR first for bootstrap, then load arawn.toml
    let bootstrap_dir = std::env::var("ARAWN_DATA_DIR")
        .unwrap_or_else(|_| dirs_path().unwrap_or_else(|| ".arawn".into()));
    let config = arawn_bin::ArawnConfig::load(std::path::Path::new(&bootstrap_dir));
    let data_dir = config.data_dir().to_string_lossy().to_string();

    let store = Store::open(std::path::Path::new(&data_dir))?;
    info!(data_dir = %data_dir, "store opened");

    // Clean up sessions whose JSONL files were deleted from disk
    if let Err(e) = store.reconcile_sessions() {
        warn!(error = %e, "failed to reconcile sessions");
    }

    // Generate default config file if it doesn't exist
    let config_path = std::path::PathBuf::from(&data_dir).join("arawn.toml");
    if !config_path.exists() {
        if let Err(e) = std::fs::write(
            &config_path,
            arawn_bin::ArawnConfig::generate_default_toml(),
        ) {
            debug!(error = %e, "could not write default arawn.toml");
        } else {
            info!("generated default arawn.toml");
        }
    }

    // Ensure scratch workstream exists
    // Note: scratch sessions get per-session workspaces, but the workstream root_dir
    // is a placeholder — the actual workspace is resolved per-session at runtime.
    let scratch_dir = std::path::PathBuf::from(&data_dir).join("workstreams/scratch");
    let workstream = match store.find_workstream_by_name("scratch")? {
        Some(ws) => {
            debug!("reusing existing scratch workstream");
            ws
        }
        None => {
            let ws = Workstream::scratch(&scratch_dir);
            store.create_workstream(&ws)?;
            info!("created scratch workstream");
            ws
        }
    };

    // Handle --list-sessions
    if list_sessions {
        let sessions = store.list_sessions_for_workstream(workstream.id)?;
        let scratch = store.list_scratch_sessions()?;
        if sessions.is_empty() && scratch.is_empty() {
            println!("No sessions found.");
        } else {
            println!("Sessions:");
            for s in scratch {
                println!(
                    "  {} (scratch) — {}",
                    s.id,
                    s.created_at.format("%Y-%m-%d %H:%M")
                );
            }
            for s in sessions {
                println!("  {} — {}", s.id, s.created_at.format("%Y-%m-%d %H:%M"));
            }
        }
        return Ok(());
    }

    // Handle serve mode
    if serve_mode {
        let engine_llm_config = config.engine_llm();
        let llm_client = Arc::new(arawn_llm::OpenAICompatibleClient::from_config(
            &engine_llm_config.provider,
            engine_llm_config.base_url.as_deref(),
            &engine_llm_config.api_key_env,
        )?);
        let llm = Arc::new(arawn_llm::RetryClient::new(llm_client));
        info!(
            provider = %engine_llm_config.provider,
            model = %engine_llm_config.model,
            "LLM provider configured"
        );

        // Initialize memory system (two-tier KB)
        let ws_dir = arawn_storage::workstream_dir_name(&workstream.name, workstream.id);
        let memory_manager = arawn_memory::try_open_memory(
            std::path::Path::new(&data_dir),
            &ws_dir,
            Some(384), // default embedding dimensions
        );
        if memory_manager.is_some() {
            info!("memory system initialized (global + workstream KB)");
        }

        let registry = Arc::new(arawn_engine::ToolRegistry::new());
        registry.register(Box::new(arawn_engine::ThinkTool));
        // Background task manager (shared across tools)
        let bg_manager = Arc::new(arawn_engine::BackgroundTaskManager::new());

        registry.register(Box::new(arawn_engine::ShellTool::with_network_tools(
            config.sandbox.network_tools.clone(),
        ).with_background_manager(Arc::clone(&bg_manager))));
        registry.register(Box::new(arawn_engine::FileReadTool));
        registry.register(Box::new(arawn_engine::FileWriteTool));
        registry.register(Box::new(arawn_engine::FileEditTool));
        registry.register(Box::new(arawn_engine::GlobTool));
        registry.register(Box::new(arawn_engine::GrepTool));
        registry.register(Box::new(arawn_engine::WebFetchTool::new()));
        registry.register(Box::new(arawn_engine::WebSearchTool));
        registry.register(Box::new(arawn_engine::AskUserTool));
        let agents_dir = std::path::PathBuf::from(&data_dir).join("agents");
        let agent_defs = arawn_engine::agent_defs::get_all_agents(Some(&agents_dir));
        registry.register(Box::new(arawn_engine::AgentTool::new(
            Arc::clone(&registry),
            agent_defs,
        ).with_background_manager(Arc::clone(&bg_manager))));
        let task_store = arawn_engine::SessionTaskStore::new();
        registry.register(Box::new(arawn_engine::SleepTool));
        registry.register(Box::new(arawn_engine::TaskCreateTool::new(
            task_store.clone(),
        )));
        registry.register(Box::new(arawn_engine::TaskUpdateTool::new(
            task_store.clone(),
        )));
        registry.register(Box::new(arawn_engine::TaskGetTool::new(task_store.clone())));
        registry.register(Box::new(arawn_engine::TaskListTool::new(task_store)));
        registry.register(Box::new(arawn_engine::TaskOutputTool::new(Arc::clone(&bg_manager))));
        registry.register(Box::new(arawn_engine::TaskStopTool::new(Arc::clone(&bg_manager))));

        // Plan mode tools
        let plan_state = Arc::new(arawn_engine::PlanModeState::new());
        registry.register(Box::new(arawn_engine::EnterPlanModeTool::new(Arc::clone(&plan_state))));
        registry.register(Box::new(arawn_engine::ExitPlanModeTool::new(Arc::clone(&plan_state))));

        // Load legacy .arawn_tool plugins
        let tools_dir = std::path::PathBuf::from(&data_dir).join("plugins/tools");
        let build_dir = std::path::PathBuf::from(&data_dir).join("plugins/build");
        let plugin_tools = arawn_engine::PluginLoader::load_tools(&tools_dir, &build_dir);
        for tool in plugin_tools {
            registry.register_plugin(tool);
        }

        // Load new-style plugins (Claude Code compatible)
        let plugins_root = std::path::PathBuf::from(&data_dir).join("plugins");
        let skill_registry = Arc::new(SkillRegistry::new());
        let plugin_runtime = PluginRuntime::new(plugins_root)
            .with_settings(std::path::PathBuf::from(&data_dir).join("settings.json"));
        let plugin_result = plugin_runtime.load_all(&registry, &skill_registry);

        // Register SkillTool with loaded skills
        registry.register(Box::new(SkillTool::new(Arc::clone(&skill_registry))));
        info!(tools = registry.len(), "tools registered for serve mode");

        // Spawn file watcher for hot-reloading legacy plugins
        let watcher = PluginWatcher::new(tools_dir, build_dir, Arc::clone(&registry));
        let _watcher_handle = watcher.spawn();

        // Spawn hot-reload watcher for new-style plugins
        let _plugin_watcher = plugin_runtime.watch(Arc::clone(&registry), Arc::clone(&skill_registry));

        // Connect to MCP servers (from arawn.toml + plugins)
        let mcp_config = arawn_mcp::load_mcp_config(&std::path::PathBuf::from(&data_dir).join("arawn.toml"));
        let mut mcp_manager = arawn_mcp::McpManager::new();
        if !mcp_config.servers.is_empty() {
            info!(servers = mcp_config.servers.len(), "connecting to config MCP servers");
            mcp_manager.connect_all(&mcp_config.servers, &registry).await;
        }

        // Connect plugin MCP servers
        if !plugin_result.mcp_servers.is_empty() {
            let plugin_mcp_configs: Vec<arawn_mcp::McpServerConfig> = plugin_result
                .mcp_servers
                .iter()
                .map(|s| arawn_mcp::McpServerConfig {
                    name: s.name.clone(),
                    command: s.command.clone(),
                    args: s.args.clone(),
                    env: s.env.clone(),
                    enabled: true,
                })
                .collect();
            info!(servers = plugin_mcp_configs.len(), "connecting to plugin MCP servers");
            mcp_manager.connect_all(&plugin_mcp_configs, &registry).await;
        }

        if mcp_manager.tool_count() > 0 {
            info!(
                tools = mcp_manager.tool_count(),
                servers = mcp_manager.connected_servers().len(),
                "MCP servers connected"
            );
        }

        let mut engine_config = build_engine_config(&config, &workstream, &data_dir);

        // Inject KB memories into the system prompt
        if let Some(ref mgr) = memory_manager {
            let kb_memories = arawn_memory::load_memories_for_injection(mgr, None, None);
            if !kb_memories.is_empty() {
                if let Some(ref mut ctx) = engine_config.prompt_context {
                    ctx.memories = kb_memories;
                    info!(count = ctx.memories.len(), "KB memories injected into prompt");
                }
            }
        }

        // Inject MCP server descriptions into the system prompt
        let mcp_prompt = mcp_manager.system_prompt();
        if !mcp_prompt.is_empty() {
            if let Some(ref mut ctx) = engine_config.prompt_context {
                ctx.plugin_prompts.push(mcp_prompt);
            }
        }

        // Load permission rules from config
        let config_path = std::path::PathBuf::from(&data_dir).join("arawn.toml");
        let permission_rules = arawn_engine::permissions::load_permissions_from_file(&config_path).into_rules();

        // Wrap MCP manager for sharing with config watcher
        let mcp_manager = Arc::new(tokio::sync::Mutex::new(mcp_manager));

        let mut service = arawn_bin::LocalService::new(
            store,
            std::path::PathBuf::from(&data_dir),
            llm,
            registry.clone(),
            engine_config,
        )
        .with_permission_rules(permission_rules)
        .with_skill_registry(Arc::clone(&skill_registry))
        .with_plugin_registry(Arc::clone(&plugin_runtime.registry))
        .with_plan_state(plan_state)
        .with_background_tasks(bg_manager);

        if let Some(ref mgr) = memory_manager {
            service = service.with_memory_manager(Arc::clone(mgr));
        }

        // Spawn config watcher for hot-reloading arawn.toml changes
        let _config_watcher = arawn_bin::config_watcher::ConfigWatcher::new(
            config_path,
            std::path::PathBuf::from(&data_dir),
            service.shared_permission_rules(),
            mcp_manager,
            registry,
        )
        .spawn();

        arawn_bin::ws_server::run_server(service, serve_port).await?;
        return Ok(());
    }

    // Handle TUI mode
    if tui_mode || (prompt_parts.is_empty() && session_id.is_none() && !list_sessions) {
        info!("launching TUI, connecting to {}", tui_url);
        arawn_tui::run_tui(&tui_url, &config.engine_llm().model)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        return Ok(());
    }

    // Need a prompt
    if prompt_parts.is_empty() && session_id.is_none() {
        eprintln!("Usage: arawn [command] [options] <prompt>");
        eprintln!();
        eprintln!("Commands:");
        eprintln!("  serve              Start WebSocket server (default port 3100)");
        eprintln!("  tui                Launch interactive TUI (connects to server)");
        eprintln!("  <prompt>           One-shot CLI mode");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  --session <uuid>   Resume an existing session");
        eprintln!("  --list-sessions    List all sessions");
        eprintln!("  --port <port>      Server port (default: 3100)");
        eprintln!("  --url <ws-url>     TUI server URL (default: ws://127.0.0.1:3100/ws)");
        eprintln!();
        eprintln!("Environment:");
        eprintln!("  GROQ_API_KEY       Groq API key (required)");
        eprintln!("  GROQ_MODEL         Model name (default: {DEFAULT_MODEL})");
        eprintln!("  ARAWN_DATA_DIR     Data directory (default: ~/.arawn)");
        std::process::exit(1);
    }

    let user_input = prompt_parts.join(" ");

    // CLI prompt mode: connect to the running server via WebSocket.
    // The server handles the engine, tools, persistence — we just send/receive.
    let server_url = format!("ws://127.0.0.1:{}/ws", config.server.port);
    run_cli_via_server(&server_url, &user_input, session_id).await
}

/// Run a CLI prompt by connecting to the running server via WebSocket.
async fn run_cli_via_server(
    url: &str,
    prompt: &str,
    session_id: Option<Uuid>,
) -> Result<()> {
    use arawn_tui::ws_client::{WsClient, EventUpdate, engine_event_to_update, parse_engine_event};
    use futures_util::StreamExt;
    use tokio_tungstenite::tungstenite::Message as WsMessage;

    let mut client = WsClient::connect(url).await.map_err(|e| {
        anyhow::anyhow!(
            "Could not connect to arawn server at {url}: {e}\n\
             Start the server first: arawn serve"
        )
    })?;

    // Create or resume session
    let session_uuid = match session_id {
        Some(id) => {
            eprintln!("Resuming session {id}");
            id
        }
        None => {
            let s = client.create_session(None).await.map_err(|e| {
                anyhow::anyhow!("Failed to create session: {e}")
            })?;
            eprintln!("Session: {}", s.id);
            s.id
        }
    };

    // Send the prompt
    if prompt.is_empty() {
        eprintln!("No prompt provided");
        std::process::exit(1);
    }

    let req_id = client
        .send_request(
            "send_message",
            serde_json::json!({
                "session_id": session_uuid.to_string(),
                "content": prompt,
            }),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Failed to send message: {e}"))?;

    eprintln!("Thinking...\n");

    // Stream events until Complete or Error
    let mut final_text = String::new();
    loop {
        let msg = client.read.next().await;
        match msg {
            Some(Ok(WsMessage::Text(text))) => {
                if let Some(event) = parse_engine_event(&text) {
                    match engine_event_to_update(event) {
                        EventUpdate::AppendStreamingText(text) => {
                            // Could print incrementally here for live streaming
                        }
                        EventUpdate::AddToolCall { name, .. } => {
                            eprintln!("  [{name}]");
                        }
                        EventUpdate::AddToolResult { is_error, .. } => {
                            if is_error {
                                eprintln!("  [error]");
                            }
                        }
                        EventUpdate::Complete(text) => {
                            final_text = text;
                            break;
                        }
                        EventUpdate::Error(message) => {
                            eprintln!("Error: {message}");
                            std::process::exit(1);
                        }
                        _ => {}
                    }
                }
                // Check for JSON-RPC error response
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(&text) {
                    if data.get("id").and_then(|v| v.as_u64()) == Some(req_id)
                        && data.get("error").is_some()
                    {
                        let err = data["error"]["message"]
                            .as_str()
                            .unwrap_or("unknown error");
                        eprintln!("Server error: {err}");
                        std::process::exit(1);
                    }
                }
            }
            Some(Ok(WsMessage::Close(_))) => {
                eprintln!("Server closed connection");
                std::process::exit(1);
            }
            Some(Err(e)) => {
                eprintln!("WebSocket error: {e}");
                std::process::exit(1);
            }
            None => {
                eprintln!("Connection lost");
                std::process::exit(1);
            }
            _ => {}
        }
    }

    println!("{final_text}");
    Ok(())
}

fn build_engine_config(
    config: &arawn_bin::ArawnConfig,
    workstream: &arawn_core::Workstream,
    data_dir: &str,
) -> QueryEngineConfig {
    let engine_llm = config.engine_llm();
    QueryEngineConfig {
        model: engine_llm.model.clone(),
        max_iterations: config.engine.max_iterations,
        system_prompt: String::new(),
        max_tokens: Some(engine_llm.max_tokens),
        model_limits: arawn_engine::ModelLimits::new(
            engine_llm.context_window,
            config.compactor.compaction_threshold,
        ),
        data_dir: Some(std::path::PathBuf::from(data_dir)),
        prompt_context: Some(arawn_engine::PromptContext {
            prompts_dir: Some(config.prompts_dir()),
            os: std::env::consts::OS.to_string(),
            shell: std::env::var("SHELL").unwrap_or_else(|_| "sh".into()),
            cwd: workstream.root_dir.clone(),
            workstream_name: workstream.name.clone(),
            workstream_root: workstream.root_dir.clone(),
            context_files: arawn_engine::find_context_files(
                &workstream.root_dir,
                &std::path::PathBuf::from(data_dir),
            ),
            memories: vec![],
            session_context: String::new(),
            plugin_prompts: vec![],
        }),
    }
}

fn dirs_path() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        std::env::var("HOME").ok().map(|h| format!("{h}/.arawn"))
    }
    #[cfg(not(target_os = "macos"))]
    {
        std::env::var("HOME").ok().map(|h| format!("{h}/.arawn"))
    }
}
