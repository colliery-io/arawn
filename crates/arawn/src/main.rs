use std::sync::Arc;

use anyhow::Result;
use tracing::{debug, error, info, warn};
use tracing_subscriber::{EnvFilter, Layer, fmt, layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

use arawn_core::Workstream;
use arawn_engine::QueryEngineConfig;
use arawn_engine::SkillTool;
use arawn_engine::plugins::PluginRuntime;
use arawn_engine::skills::SkillRegistry;
use arawn_storage::Store;

const DEFAULT_MODEL: &str = "llama-3.3-70b-versatile";

/// Default file log filter: debug for arawn crates, warn for third-party.
const FILE_LOG_FILTER: &str = "warn,arawn=debug,arawn_bin=debug,arawn_tui=debug,arawn_engine=debug,arawn_llm=debug,arawn_storage=debug,arawn_core=debug,arawn_mcp=debug,arawn_memory=debug,arawn_service=debug,arawn_embed=debug";

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI args
    use clap::{Parser, Subcommand};

    #[derive(Parser)]
    #[command(name = "arawn", about = "LLM-powered coding assistant", version)]
    struct Cli {
        #[command(subcommand)]
        command: Option<Command>,

        /// Data directory (default: ~/.arawn, or ARAWN_DATA_DIR env var)
        #[arg(long, env = "ARAWN_DATA_DIR")]
        data_dir: Option<String>,

        /// Resume an existing session by UUID
        #[arg(long)]
        session: Option<Uuid>,

        /// List all sessions
        #[arg(long)]
        list_sessions: bool,

        /// Prompt text (when not using a subcommand)
        #[arg(trailing_var_arg = true)]
        prompt: Vec<String>,
    }

    #[derive(Subcommand)]
    enum Command {
        /// Start the WebSocket server
        Serve {
            /// Server port
            #[arg(long, default_value_t = 3100)]
            port: u16,
        },
        /// Launch the TUI client
        Tui {
            /// WebSocket server URL
            #[arg(long, default_value = "ws://127.0.0.1:3100/ws")]
            url: String,
        },
        /// Plugin management commands
        Plugin {
            /// Plugin subcommand arguments
            #[arg(trailing_var_arg = true)]
            args: Vec<String>,
        },
    }

    let cli = Cli::parse();

    // Handle plugin subcommand immediately (exits process)
    if let Some(Command::Plugin { args: plugin_args }) = &cli.command {
        let base = cli.data_dir.as_deref()
            .map(String::from)
            .or_else(dirs_path)
            .unwrap_or_else(|| ".arawn".into());
        let plugins_root = std::path::PathBuf::from(base).join("plugins");
        match arawn_bin::plugin_cmd::run_plugin_command(plugin_args, &plugins_root) {
            Ok(()) => std::process::exit(0),
            Err(e) => {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
    }

    let serve_mode = matches!(cli.command, Some(Command::Serve { .. }));
    let tui_mode = matches!(cli.command, Some(Command::Tui { .. }));
    let serve_port = match &cli.command {
        Some(Command::Serve { port }) => *port,
        _ => 3100,
    };
    let tui_url = match &cli.command {
        Some(Command::Tui { url }) => url.clone(),
        _ => "ws://127.0.0.1:3100/ws".to_string(),
    };
    let session_id = cli.session;
    let list_sessions = cli.list_sessions;
    let prompt_parts = cli.prompt;

    // Resolve data directory: --data-dir flag > ARAWN_DATA_DIR env > ~/.arawn
    let bootstrap_dir = cli
        .data_dir
        .unwrap_or_else(|| dirs_path().unwrap_or_else(|| ".arawn".into()));
    let config = arawn_bin::ArawnConfig::load(std::path::Path::new(&bootstrap_dir));
    let data_dir = config.data_dir().to_string_lossy().to_string();

    // Initialize logging — file-based for serve/tui, stderr-only for CLI
    let _log_guard: Option<tracing_appender::non_blocking::WorkerGuard>;
    {
        let log_dir = std::path::PathBuf::from(&data_dir).join("logs");
        let _ = std::fs::create_dir_all(&log_dir);
        let is_tui = tui_mode
            || (!serve_mode && !list_sessions && session_id.is_none() && prompt_parts.is_empty());

        if serve_mode {
            let file_appender = tracing_appender::rolling::daily(&log_dir, "server.log");
            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
            _log_guard = Some(guard);
            tracing_subscriber::registry()
                .with(
                    fmt::layer()
                        .with_writer(non_blocking)
                        .with_ansi(false)
                        .with_filter(EnvFilter::new(FILE_LOG_FILTER)),
                )
                .with(
                    fmt::layer()
                        .with_writer(std::io::stderr)
                        .with_target(false)
                        .with_filter(
                            EnvFilter::try_from_default_env()
                                .unwrap_or_else(|_| EnvFilter::new("info")),
                        ),
                )
                .init();
        } else if is_tui {
            let file_appender = tracing_appender::rolling::daily(&log_dir, "tui.log");
            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
            _log_guard = Some(guard);
            tracing_subscriber::registry()
                .with(
                    fmt::layer()
                        .with_writer(non_blocking)
                        .with_ansi(false)
                        .with_filter(EnvFilter::new(FILE_LOG_FILTER)),
                )
                .init();
        } else {
            _log_guard = None;
            tracing_subscriber::fmt()
                .with_env_filter(
                    EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| EnvFilter::new("info")),
                )
                .with_target(false)
                .with_writer(std::io::stderr)
                .init();
        }
    }

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
        // Install the rustls crypto provider as the process default. Required
        // before any integration constructs a hyper-rustls connector
        // (slack-morphism, the Google API hubs, etc.) — rustls 0.23 with
        // both ring and aws-lc-rs visible in the dep tree won't auto-pick.
        // Idempotent.
        arawn_integrations::install_default_crypto_provider();

        // Build the LLM client pool — fail-fast: any misconfigured `[llm.*]`
        // entry surfaces here, not mid-session.
        let llm_pool = Arc::new(arawn_bin::LlmClientPool::from_config(
            &config,
            build_llm_client,
        )?);
        info!(
            entries = llm_pool.len(),
            engine = llm_pool.engine_name(),
            compactor = llm_pool.compactor_name(),
            engine_model = %llm_pool.engine_config().model,
            compactor_model = %llm_pool.compactor_config().model,
            "LLM client pool ready"
        );

        // Eagerly warm up every configured LLM in the background. Server
        // startup proceeds immediately; warmup failures are logged but never
        // block startup. Lazy warmup on first `stream` call covers any
        // provider that comes back up after this initial probe fails.
        {
            let pool_for_warmup = Arc::clone(&llm_pool);
            tokio::spawn(async move {
                let results = pool_for_warmup.warmup_all().await;
                for (name, result) in results {
                    let model = pool_for_warmup
                        .config(&name)
                        .map(|c| c.model.as_str())
                        .unwrap_or("?");
                    let provider = pool_for_warmup
                        .config(&name)
                        .map(|c| c.provider.as_str())
                        .unwrap_or("?");
                    match result {
                        Ok(()) => info!(name = %name, provider = %provider, model = %model, "LLM warmup OK"),
                        Err(e) => error!(
                            name = %name,
                            provider = %provider,
                            model = %model,
                            error = %e,
                            "LLM warmup failed — model may be unavailable; lazy warmup will retry on first request"
                        ),
                    }
                }
            });
        }

        // Initialize embedding model
        let embed_config = arawn_embed::EmbeddingConfig::default();
        let embedder: Option<Arc<dyn arawn_embed::Embedder>> =
            match arawn_embed::create_embedder(&embed_config) {
                Ok(e) => {
                    info!(model = %embed_config.model, dims = embed_config.dimensions, "embedding model loaded");
                    Some(e)
                }
                Err(e) => {
                    warn!(error = %e, "embedding model unavailable — memory system will use FTS only");
                    None
                }
            };

        // Initialize memory system (two-tier KB) with optional embedder
        let ws_dir = arawn_storage::workstream_dir_name(&workstream.name, workstream.id);
        let memory_manager: Option<Arc<arawn_memory::MemoryManager>> =
            match arawn_memory::MemoryManager::open(
                std::path::Path::new(&data_dir),
                &ws_dir,
                Some(embed_config.dimensions),
            ) {
                Ok(mut mgr) => {
                    if let Some(ref emb) = embedder {
                        mgr = mgr.with_embedder(Arc::clone(emb));
                    }
                    info!("memory system initialized (global + workstream KB)");
                    Some(Arc::new(mgr))
                }
                Err(e) => {
                    warn!(error = %e, "memory system unavailable — continuing without memory");
                    None
                }
            };

        let registry = Arc::new(arawn_engine::ToolRegistry::new());
        let bg_manager = Arc::new(arawn_engine::BackgroundTaskManager::new());
        let plan_state = Arc::new(arawn_engine::PlanModeState::new());
        register_default_tools(
            &registry,
            &config,
            &data_dir,
            Arc::clone(&bg_manager),
            Arc::clone(&plan_state),
        );

        // Register memory tools (if memory system is available)
        if let Some(ref mgr) = memory_manager {
            registry.register(Box::new(arawn_engine::MemoryStoreTool::new(
                Arc::clone(mgr),
                embedder.clone(),
            )));
            registry.register(Box::new(arawn_engine::MemorySearchTool::new(
                Arc::clone(mgr),
                embedder.clone(),
            )));
            info!("memory tools registered (store + search)");
        }

        // Load new-style plugins (Claude Code compatible)
        let plugins_root = std::path::PathBuf::from(&data_dir).join("plugins");
        let skill_registry = Arc::new(SkillRegistry::new());
        let plugin_runtime = PluginRuntime::new(plugins_root)
            .with_settings(std::path::PathBuf::from(&data_dir).join("settings.json"));
        let plugin_result = plugin_runtime.load_all(&skill_registry);

        // Register SkillTool with loaded skills
        registry.register(Box::new(SkillTool::new(Arc::clone(&skill_registry))));
        info!(tools = registry.len(), "tools registered for serve mode");

        // Plugin hot-reload watcher is spawned later, after `service` is
        // constructed, so we can wire it into the broadcast channel and
        // surface reload outcomes in the TUI.

        // Connect MCP servers (config + plugins)
        let mcp_manager = connect_mcp_servers(&data_dir, &plugin_result, &registry).await;

        let mut engine_config = build_engine_config(&config, &workstream, &data_dir);

        // Inject KB memories into the system prompt
        if let Some(ref mgr) = memory_manager {
            let kb_memories = arawn_memory::load_memories_for_injection(mgr, None, None);
            if !kb_memories.is_empty()
                && let Some(ref mut ctx) = engine_config.prompt_context {
                    ctx.memories = kb_memories;
                    info!(count = ctx.memories.len(), "KB memories injected into prompt");
                }
        }

        // Inject MCP server descriptions into the system prompt
        let mcp_prompt = mcp_manager.system_prompt();
        if !mcp_prompt.is_empty()
            && let Some(ref mut ctx) = engine_config.prompt_context {
                ctx.plugin_prompts.push(mcp_prompt);
            }

        // Load permission rules from config
        let config_path = std::path::PathBuf::from(&data_dir).join("arawn.toml");
        let permission_rules = arawn_engine::permissions::load_permissions_from_file(&config_path).into_rules();

        // Wrap MCP manager for sharing with config watcher
        let mcp_manager = Arc::new(tokio::sync::Mutex::new(mcp_manager));

        let mut service = arawn_bin::LocalService::new(
            store,
            std::path::PathBuf::from(&data_dir),
            Arc::clone(&llm_pool),
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

        // Register workstream tools (need the shared store from the service)
        registry.register(Box::new(arawn_engine::WorkstreamCreateTool::new(service.shared_store())));
        registry.register(Box::new(arawn_engine::WorkstreamListTool::new(service.shared_store())));

        // Register Gmail integration if env vars are present. Skipped silently
        // otherwise — users without Gmail credentials still get a working
        // server. See docs/src/integrations/gmail.md for the Cloud Console
        // setup that produces these env vars.
        if let (Ok(client_id), Ok(client_secret)) = (
            std::env::var("ARAWN_GMAIL_CLIENT_ID"),
            std::env::var("ARAWN_GMAIL_CLIENT_SECRET"),
        ) {
            let gmail = Arc::new(arawn_integrations::gmail::GmailIntegration::new(
                std::path::PathBuf::from(&data_dir),
                client_id,
                client_secret,
            ));
            service.register_integration(Arc::clone(&gmail) as Arc<dyn arawn_integrations::Integration>);
            registry.register(Box::new(arawn_integrations::gmail::GmailInboxReadTool::new(Arc::clone(&gmail))));
            registry.register(Box::new(arawn_integrations::gmail::GmailSearchTool::new(Arc::clone(&gmail))));
            registry.register(Box::new(arawn_integrations::gmail::GmailGetMessageTool::new(Arc::clone(&gmail))));
            registry.register(Box::new(arawn_integrations::gmail::GmailSendTool::new(Arc::clone(&gmail))));
            registry.register(Box::new(arawn_integrations::gmail::GmailMarkReadTool::new(Arc::clone(&gmail))));
            info!("Gmail integration registered (5 tools)");
        } else {
            debug!(
                "Gmail integration skipped — set ARAWN_GMAIL_CLIENT_ID + \
                 ARAWN_GMAIL_CLIENT_SECRET to enable. See docs/src/integrations/gmail.md."
            );
        }

        // Register Google Calendar similarly. Reads ARAWN_GCAL_CLIENT_ID /
        // _SECRET first; falls back to ARAWN_GOOGLE_CLIENT_ID / _SECRET so a
        // user with one shared OAuth project for both Gmail and Calendar can
        // configure once. See docs/src/integrations/calendar.md.
        let gcal_client = std::env::var("ARAWN_GCAL_CLIENT_ID")
            .ok()
            .or_else(|| std::env::var("ARAWN_GOOGLE_CLIENT_ID").ok());
        let gcal_secret = std::env::var("ARAWN_GCAL_CLIENT_SECRET")
            .ok()
            .or_else(|| std::env::var("ARAWN_GOOGLE_CLIENT_SECRET").ok());
        if let (Some(client_id), Some(client_secret)) = (gcal_client, gcal_secret) {
            let calendar = Arc::new(arawn_integrations::calendar::GoogleCalendarIntegration::new(
                std::path::PathBuf::from(&data_dir),
                client_id,
                client_secret,
            ));
            service.register_integration(Arc::clone(&calendar) as Arc<dyn arawn_integrations::Integration>);
            registry.register(Box::new(arawn_integrations::calendar::CalendarUpcomingTool::new(Arc::clone(&calendar))));
            registry.register(Box::new(arawn_integrations::calendar::CalendarCreateEventTool::new(Arc::clone(&calendar))));
            registry.register(Box::new(arawn_integrations::calendar::CalendarFindConflictsTool::new(Arc::clone(&calendar))));
            info!("Google Calendar integration registered (3 tools)");
        } else {
            debug!(
                "Google Calendar integration skipped — set ARAWN_GCAL_CLIENT_ID + \
                 ARAWN_GCAL_CLIENT_SECRET (or share ARAWN_GOOGLE_CLIENT_ID / _SECRET with Gmail) \
                 to enable. See docs/src/integrations/calendar.md."
            );
        }

        // Register Slack. No env-var sharing with Google — Slack apps are
        // unrelated. See docs/src/integrations/slack.md.
        if let (Ok(client_id), Ok(client_secret)) = (
            std::env::var("ARAWN_SLACK_CLIENT_ID"),
            std::env::var("ARAWN_SLACK_CLIENT_SECRET"),
        ) {
            let slack = Arc::new(arawn_integrations::slack::SlackIntegration::new(
                std::path::PathBuf::from(&data_dir),
                client_id,
                client_secret,
            ));
            service.register_integration(Arc::clone(&slack) as Arc<dyn arawn_integrations::Integration>);
            registry.register(Box::new(arawn_integrations::slack::SlackListChannelsTool::new(Arc::clone(&slack))));
            registry.register(Box::new(arawn_integrations::slack::SlackHistoryTool::new(Arc::clone(&slack))));
            registry.register(Box::new(arawn_integrations::slack::SlackPostTool::new(Arc::clone(&slack))));
            registry.register(Box::new(arawn_integrations::slack::SlackReactTool::new(Arc::clone(&slack))));
            info!("Slack integration registered (4 tools)");
        } else {
            debug!(
                "Slack integration skipped — set ARAWN_SLACK_CLIENT_ID + \
                 ARAWN_SLACK_CLIENT_SECRET to enable. See docs/src/integrations/slack.md."
            );
        }

        // Start workflow engine (cloacina DefaultRunner — background services start on construction)
        let workflow_config = arawn_workflow::runner::WorkflowRunnerConfig::new(
            std::path::Path::new(&data_dir),
        );
        let workflows_dir = std::path::PathBuf::from(&data_dir).join("workflows");
        let shared_runner: arawn_workflow::SharedWorkflowRunner =
            Arc::new(tokio::sync::RwLock::new(None));

        match arawn_workflow::WorkflowRunner::new(workflow_config).await {
            Ok(runner) => {
                info!("workflow runner started");
                *shared_runner.write().await = Some(Arc::new(runner));
            }
            Err(e) => {
                warn!(error = %e, "workflow runner unavailable — continuing without workflows");
            }
        }

        // Register workflow tools (before config watcher takes registry ownership)
        register_workflow_tools(&registry, workflows_dir, Arc::clone(&shared_runner));

        // Wire watchers into the broadcast so reload outcomes reach the TUI.
        let notice_tx_plugin = service.notice_sender();
        let _plugin_watcher = plugin_runtime.watch(
            Arc::clone(&skill_registry),
            Some(Arc::new(move |is_error: bool, msg: String| {
                let notice = arawn_service::ServerNotice {
                    level: if is_error { "error".into() } else { "info".into() },
                    category: "plugin_reload".into(),
                    message: msg,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                };
                let _ = notice_tx_plugin.send(notice);
            })),
        );

        // Spawn config watcher for hot-reloading arawn.toml changes
        let notice_tx_config = service.notice_sender();
        let _config_watcher = arawn_bin::config_watcher::ConfigWatcher::new(
            config_path,
            std::path::PathBuf::from(&data_dir),
            service.shared_permission_rules(),
            mcp_manager,
            registry,
        )
        .with_notify(Arc::new(move |is_error: bool, msg: String| {
            let notice = arawn_service::ServerNotice {
                level: if is_error { "error".into() } else { "info".into() },
                category: "config_reload".into(),
                message: msg,
                timestamp: chrono::Utc::now().to_rfc3339(),
            };
            let _ = notice_tx_config.send(notice);
        }))
        .spawn();

        arawn_bin::ws_server::run_server(service, serve_port).await?;

        // Graceful shutdown of workflow runner
        if let Some(ref runner) = *shared_runner.read().await {
            runner.shutdown().await;
        }

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
    let final_text = 'stream: loop {
        let msg = client.read.next().await;
        match msg {
            Some(Ok(WsMessage::Text(text))) => {
                if let Some(event) = parse_engine_event(&text) {
                    match engine_event_to_update(event) {
                        EventUpdate::AddToolCall { name, .. } => {
                            eprintln!("  [{name}]");
                        }
                        EventUpdate::AddToolResult { is_error, .. } => {
                            if is_error {
                                eprintln!("  [error]");
                            }
                        }
                        EventUpdate::Complete(text) => {
                            break 'stream text;
                        }
                        EventUpdate::Error(message) => {
                            eprintln!("Error: {message}");
                            std::process::exit(1);
                        }
                        _ => {}
                    }
                }
                // Check for JSON-RPC error response
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(&text)
                    && data.get("id").and_then(|v| v.as_u64()) == Some(req_id)
                        && data.get("error").is_some()
                    {
                        let err = data["error"]["message"]
                            .as_str()
                            .unwrap_or("unknown error");
                        eprintln!("Server error: {err}");
                        std::process::exit(1);
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
    };

    println!("{final_text}");
    Ok(())
}

/// Build the appropriate LLM client based on provider config.
fn build_llm_client(
    config: &arawn_bin::LlmConfig,
) -> Result<Arc<dyn arawn_llm::LlmClient>> {
    match config.provider.as_str() {
        "anthropic" => {
            let api_key = std::env::var(&config.api_key_env).map_err(|_| {
                anyhow::anyhow!(
                    "{} environment variable not set (required for Anthropic provider)",
                    config.api_key_env
                )
            })?;
            Ok(Arc::new(arawn_llm::AnthropicClient::new(api_key)))
        }
        _ => {
            // All other providers use OpenAI-compatible client
            Ok(Arc::new(arawn_llm::OpenAICompatibleClient::from_config(
                &config.provider,
                config.base_url.as_deref(),
                &config.api_key_env,
            )?))
        }
    }
}

/// Register all default tools into the registry.
fn register_default_tools(
    registry: &Arc<arawn_engine::ToolRegistry>,
    config: &arawn_bin::ArawnConfig,
    data_dir: &str,
    bg_manager: Arc<arawn_engine::BackgroundTaskManager>,
    plan_state: Arc<arawn_engine::PlanModeState>,
) {
    use arawn_engine::{
        AgentTool, AskUserTool, EnterPlanModeTool, ExitPlanModeTool, FileEditTool, FileReadTool,
        FileWriteTool, GlobTool, GrepTool, SessionTaskStore, ShellTool, SleepTool, TaskCreateTool,
        TaskGetTool, TaskListTool, TaskOutputTool, TaskStopTool, TaskUpdateTool, ThinkTool,
        WebFetchTool, WebSearchTool,
    };

    registry.register(Box::new(ThinkTool));
    registry.register(Box::new(
        ShellTool::with_network_tools(config.sandbox.network_tools.clone())
            .with_background_manager(Arc::clone(&bg_manager)),
    ));
    registry.register(Box::new(FileReadTool));
    registry.register(Box::new(FileWriteTool));
    registry.register(Box::new(FileEditTool));
    registry.register(Box::new(GlobTool));
    registry.register(Box::new(GrepTool));
    registry.register(Box::new(WebFetchTool::new()));
    registry.register(Box::new(WebSearchTool));
    registry.register(Box::new(AskUserTool));

    let agents_dir = std::path::PathBuf::from(data_dir).join("agents");
    let agent_defs = arawn_engine::agent_defs::get_all_agents(Some(&agents_dir));
    registry.register(Box::new(
        AgentTool::new(Arc::clone(registry), agent_defs)
            .with_background_manager(Arc::clone(&bg_manager)),
    ));

    let task_store = SessionTaskStore::new();
    registry.register(Box::new(SleepTool));
    registry.register(Box::new(TaskCreateTool::new(task_store.clone())));
    registry.register(Box::new(TaskUpdateTool::new(task_store.clone())));
    registry.register(Box::new(TaskGetTool::new(task_store.clone())));
    registry.register(Box::new(TaskListTool::new(task_store)));
    registry.register(Box::new(TaskOutputTool::new(Arc::clone(&bg_manager))));
    registry.register(Box::new(TaskStopTool::new(Arc::clone(&bg_manager))));

    registry.register(Box::new(EnterPlanModeTool::new(Arc::clone(&plan_state))));
    registry.register(Box::new(ExitPlanModeTool::new(Arc::clone(&plan_state))));
}

/// Connect to MCP servers from config and plugins.
async fn connect_mcp_servers(
    data_dir: &str,
    plugin_result: &arawn_engine::plugins::PluginLoadResult,
    registry: &Arc<arawn_engine::ToolRegistry>,
) -> arawn_mcp::McpManager {
    let mcp_config =
        arawn_mcp::load_mcp_config(&std::path::PathBuf::from(data_dir).join("arawn.toml"));
    let mut mcp_manager = arawn_mcp::McpManager::new();
    if !mcp_config.servers.is_empty() {
        info!(
            servers = mcp_config.servers.len(),
            "connecting to config MCP servers"
        );
        mcp_manager
            .connect_all(&mcp_config.servers, registry)
            .await;
    }

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
        info!(
            servers = plugin_mcp_configs.len(),
            "connecting to plugin MCP servers"
        );
        mcp_manager
            .connect_all(&plugin_mcp_configs, registry)
            .await;
    }

    if mcp_manager.tool_count() > 0 {
        info!(
            tools = mcp_manager.tool_count(),
            servers = mcp_manager.connected_servers().len(),
            "MCP servers connected"
        );
    }

    mcp_manager
}

/// Register workflow management tools.
fn register_workflow_tools(
    registry: &Arc<arawn_engine::ToolRegistry>,
    workflows_dir: std::path::PathBuf,
    shared_runner: arawn_workflow::SharedWorkflowRunner,
) {
    registry.register(Box::new(arawn_workflow::WorkflowCreateTool::new(
        workflows_dir.clone(),
    )));
    registry.register(Box::new(arawn_workflow::WorkflowListTool::new(
        workflows_dir.clone(),
    )));
    registry.register(Box::new(arawn_workflow::WorkflowDeleteTool::new(
        workflows_dir,
    )));
    registry.register(Box::new(arawn_workflow::WorkflowStatusTool::new(
        shared_runner,
    )));
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
