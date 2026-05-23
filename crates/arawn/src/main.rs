use std::sync::Arc;

use anyhow::Result;
use tracing::{debug, error, info, warn};
use tracing_subscriber::{EnvFilter, Layer, fmt, layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

/// Adapter from `arawn_embed::Embedder` to the trait
/// `arawn_projections::Embedder` expects. Lets the embed pass run
/// against whatever backend arawn-embed is configured for.
struct EmbedderBridge {
    inner: Arc<dyn arawn_embed::Embedder>,
}

impl arawn_projections::Embedder for EmbedderBridge {
    fn embed_batch<'a>(
        &'a self,
        texts: &'a [&'a str],
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<Vec<f32>>, String>> + Send + 'a>,
    > {
        let inner = Arc::clone(&self.inner);
        let texts = texts.to_vec();
        Box::pin(async move { inner.embed_batch(&texts).await.map_err(|e| e.to_string()) })
    }
}
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
    #[command(
        name = "arawn",
        about = "Personal agentic assistant — watch, check, summarize, and nudge across your tools.",
        version
    )]
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
        /// Run diagnostic checks against the local install
        Doctor {
            /// Emit machine-readable JSON instead of human-readable text
            #[arg(long)]
            json: bool,
        },
        /// Show token usage rollups recorded by the local LLM tracker
        Usage {
            /// Window: day | week | month | all
            #[arg(long, default_value = "week")]
            period: String,
            /// Filter to a specific model name
            #[arg(long)]
            model: Option<String>,
            /// Group rollups by `call_site` tag as well
            #[arg(long)]
            by_site: bool,
            /// Emit JSON instead of human-readable text
            #[arg(long)]
            json: bool,
        },
    }

    let cli = Cli::parse();

    // Handle plugin subcommand immediately (exits process)
    if let Some(Command::Plugin { args: plugin_args }) = &cli.command {
        let base = cli
            .data_dir
            .as_deref()
            .map(String::from)
            .or_else(arawn_bin::startup::dirs_path)
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

    // Handle doctor subcommand immediately (exits process).
    // Doctor must run before the heavy startup path so a broken config
    // does not panic on the way to actually reporting "config broken".
    if let Some(Command::Doctor { json }) = &cli.command {
        let base = cli
            .data_dir
            .as_deref()
            .map(String::from)
            .or_else(arawn_bin::startup::dirs_path)
            .unwrap_or_else(|| ".arawn".into());
        let data_dir = std::path::PathBuf::from(base);
        let report = arawn_bin::doctor::run(&data_dir).await;
        if *json {
            println!("{}", report.render_json());
        } else {
            print!("{}", report.render_human());
        }
        std::process::exit(report.exit_code());
    }

    // Handle usage subcommand immediately (exits process). Reads
    // the on-disk token-usage log without spinning up the full
    // engine.
    if let Some(Command::Usage {
        period,
        model,
        by_site,
        json,
    }) = &cli.command
    {
        let base = cli
            .data_dir
            .as_deref()
            .map(String::from)
            .or_else(arawn_bin::startup::dirs_path)
            .unwrap_or_else(|| ".arawn".into());
        let data_dir = std::path::PathBuf::from(base);
        let period = match period.to_ascii_lowercase().as_str() {
            "day" => arawn_llm::usage::UsagePeriod::Day,
            "week" => arawn_llm::usage::UsagePeriod::Week,
            "month" => arawn_llm::usage::UsagePeriod::Month,
            "all" => arawn_llm::usage::UsagePeriod::All,
            other => {
                eprintln!("unknown --period {other}; expected day|week|month|all");
                std::process::exit(2);
            }
        };
        let tracker = arawn_llm::usage::UsageTracker::open(&data_dir);
        let summary = tracker.summary(period, model.as_deref(), *by_site);
        if *json {
            println!("{}", serde_json::to_string_pretty(&summary).unwrap());
        } else {
            print!("{}", arawn_llm::usage::render_usage_human(&summary));
        }
        std::process::exit(0);
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
        .unwrap_or_else(|| arawn_bin::startup::dirs_path().unwrap_or_else(|| ".arawn".into()));
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
                    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
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
    let _scratch_dir = std::path::PathBuf::from(&data_dir).join("workstreams/scratch");
    let workstream = match store.find_workstream_by_name("scratch")? {
        Some(ws) => {
            debug!("reusing existing scratch workstream");
            ws
        }
        None => {
            // Scratch is reserved; create via the dedicated path.
            let ws = store.ensure_scratch_workstream()?;
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

        // Install the process-wide token usage tracker *before* the
        // pool builds — the pool wraps every entry in a
        // `UsageTrackingClient` decorator that calls into the
        // tracker. Installation is idempotent.
        arawn_llm::usage::install(Arc::new(arawn_llm::usage::UsageTracker::open(
            std::path::Path::new(&data_dir),
        )));

        // Build the LLM client pool — fail-fast: any misconfigured `[llm.*]`
        // entry surfaces here, not mid-session.
        let llm_pool = Arc::new(arawn_bin::LlmClientPool::from_config(
            &config,
            arawn_bin::startup::build_llm_client,
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
                        Ok(()) => {
                            info!(name = %name, provider = %provider, model = %model, "LLM warmup OK")
                        }
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
        let embedder: Option<Arc<dyn arawn_embed::Embedder>> = match arawn_embed::create_embedder(
            &embed_config,
        ) {
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
        arawn_bin::startup::register_default_tools(
            &registry,
            &config,
            &data_dir,
            Arc::clone(&bg_manager),
            Arc::clone(&plan_state),
        );

        // Active-workstream shim shared between workstream slash
        // commands and the memory router. T-0250 routes memory tools
        // through this primitive so `/workstream switch` redirects
        // memory_store / memory_search to the new workstream's KB
        // on subsequent calls.
        let active_workstream = arawn_engine::SessionWorkstream::scratch();

        // Workstream memory router — hoisted to outer scope so the
        // per-workstream extractor (T-0251) can resolve KBs through
        // the same cache as the memory tools.
        let workstream_router: Option<Arc<arawn_engine::WorkstreamMemoryRouter>> =
            if memory_manager.is_some() {
                Some(Arc::new(arawn_engine::WorkstreamMemoryRouter::new(
                    std::path::PathBuf::from(&data_dir),
                    Some(embed_config.dimensions),
                    embedder.clone(),
                    active_workstream.clone(),
                )))
            } else {
                None
            };

        // Register memory tools (if memory system is available).
        // Use the routed handle so the active workstream determines
        // which KB the tools read/write.
        if let Some(ref router) = workstream_router {
            registry.register(Box::new(arawn_engine::MemoryStoreTool::new(
                Arc::clone(router),
                embedder.clone(),
            )));
            registry.register(Box::new(arawn_engine::MemorySearchTool::new(
                Arc::clone(router),
                embedder.clone(),
            )));
            registry.register(Box::new(arawn_engine::SignalSearchTool::new(
                Arc::clone(router),
                embedder.clone(),
            )));
            registry.register(Box::new(arawn_engine::SignalQueryTool::new(Arc::clone(
                router,
            ))));
            registry.register(Box::new(arawn_engine::SignalTimelineTool::new(Arc::clone(
                router,
            ))));
            info!("memory + signal tools registered (workstream-routed)");
        }

        // Shared projection store — feed_search, embed pass, feed
        // dispatch, and the extractor all read/write the same file.
        // Opening once and sharing the Arc keeps the connection pool
        // tight and lets us thread the same handle into the
        // ExtractorRunner that the backfill hook needs.
        let projections_db_path = std::path::PathBuf::from(&data_dir).join("projections.db");
        let projections: Option<Arc<arawn_projections::ProjectionStore>> =
            match arawn_projections::ProjectionStore::open(&projections_db_path) {
                Ok(store) => Some(Arc::new(store)),
                Err(e) => {
                    warn!(
                        error = %e,
                        path = %projections_db_path.display(),
                        "projection store unavailable — feed_search / extractor degraded"
                    );
                    None
                }
            };

        // feed_search tool — read-only over the projections db.
        if let Some(ref proj) = projections {
            registry.register(Box::new(arawn_engine::FeedSearchTool::new(
                Arc::clone(proj),
                embedder.clone(),
            )));
            info!("feed_search tool registered");
        }

        // Embed pass: walks projection rows whose embedding is NULL
        // and fills them in via the configured embedder. Runs every
        // 5 minutes on a tokio task; soft-fails if either the
        // projections db or the embedder is unavailable.
        if let Some(emb) = embedder.clone() {
            let projections_db_path = std::path::PathBuf::from(&data_dir).join("projections.db");
            if let Ok(store) = arawn_projections::ProjectionStore::open(&projections_db_path) {
                let store = Arc::new(store);
                let bridge =
                    Arc::new(EmbedderBridge { inner: emb }) as Arc<dyn arawn_projections::Embedder>;
                tokio::spawn(async move {
                    let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
                    // First tick fires immediately — kick off an embed
                    // pass at startup so backfill rows from prior runs
                    // get covered without waiting 5 min.
                    loop {
                        interval.tick().await;
                        match arawn_projections::run_embed_pass(&store, bridge.as_ref(), 32, 512)
                            .await
                        {
                            Ok(out) if out.embedded > 0 || out.skipped_empty > 0 => {
                                info!(
                                    embedded = out.embedded,
                                    skipped = out.skipped_empty,
                                    errors = out.errors,
                                    "projection embed pass"
                                );
                            }
                            Ok(_) => debug!("projection embed pass: nothing pending"),
                            Err(e) => warn!(error = %e, "projection embed pass failed"),
                        }
                    }
                });
                info!("projection embed pass scheduled (every 5 min)");
            }
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
        let mcp_manager = arawn_bin::startup::connect_mcp_servers(&data_dir, &plugin_result, &registry).await;

        let mut engine_config = arawn_bin::startup::build_engine_config(&config, &workstream, &data_dir);

        // Inject KB memories into the system prompt
        if let Some(ref mgr) = memory_manager {
            let kb_memories = arawn_memory::load_memories_for_injection(mgr, None, None);
            if !kb_memories.is_empty()
                && let Some(ref mut ctx) = engine_config.prompt_context
            {
                ctx.memories = kb_memories;
                info!(
                    count = ctx.memories.len(),
                    "KB memories injected into prompt"
                );
            }
        }

        // Inject MCP server descriptions into the system prompt
        let mcp_prompt = mcp_manager.system_prompt();
        if !mcp_prompt.is_empty()
            && let Some(ref mut ctx) = engine_config.prompt_context
        {
            ctx.plugin_prompts.push(mcp_prompt);
        }

        // Load permission rules + starting autonomy (T-0347) from config.
        let config_path = std::path::PathBuf::from(&data_dir).join("arawn.toml");
        let permissions_cfg =
            arawn_engine::permissions::load_permissions_from_file(&config_path);
        let permission_starting_mode = permissions_cfg
            .autonomy
            .unwrap_or(arawn_engine::permissions::PermissionMode::Ask);
        let permission_rules = permissions_cfg.into_rules();

        // Wrap MCP manager for sharing with config watcher
        let mcp_manager = Arc::new(tokio::sync::Mutex::new(mcp_manager));

        // I-0056 T-A: load hook config from user settings + project
        // settings and build a shared HookRunner. Fire sites in T-B/C/D
        // will exercise it; for now the runner is just attached so every
        // QueryEngine built by the service inherits it.
        let hook_runner = arawn_bin::startup::load_and_build_hook_runner(
            std::path::Path::new(&data_dir),
            &workstream.root_dir,
        );

        let mut service = arawn_bin::LocalService::new(
            store,
            std::path::PathBuf::from(&data_dir),
            Arc::clone(&llm_pool),
            registry.clone(),
            engine_config,
        )
        .with_permission_rules(permission_rules)
        .with_permission_mode(permission_starting_mode)
        .with_skill_registry(Arc::clone(&skill_registry))
        .with_plugin_registry(Arc::clone(&plugin_runtime.registry))
        .with_plan_state(plan_state)
        .with_background_tasks(bg_manager)
        .with_active_workstream(active_workstream.clone())
        .with_hook_runner(hook_runner);

        if let Some(ref mgr) = memory_manager {
            service = service.with_memory_manager(Arc::clone(mgr));
        }

        // Build the per-workstream extractor up-front so the
        // /workstream bind hook can spawn a backfill on first
        // binding. Requires both the projection store and the memory
        // router. StubChain for now; T-0254's tests swap in CotChain.
        let extractor_runner: Option<Arc<arawn_extractor::ExtractorRunner>> =
            match (projections.as_ref(), workstream_router.as_ref()) {
                (Some(proj), Some(router)) => {
                    let router_clone = Arc::clone(router);
                    let memory_resolver: arawn_extractor::runner::MemoryResolver =
                        Arc::new(move |name: &str| {
                            router_clone.for_workstream(name).map_err(|e| {
                                arawn_extractor::ExtractionError::Memory(e.to_string())
                            })
                        });
                    let chain: Arc<dyn arawn_extractor::ExtractionChain> =
                        Arc::new(arawn_extractor::StubChain);
                    Some(Arc::new(arawn_extractor::ExtractorRunner::new(
                        service.shared_store(),
                        Arc::clone(proj),
                        memory_resolver,
                        chain,
                    )))
                }
                _ => None,
            };

        // Steward — T-0256 scaffolding. Walks every active workstream
        // on a coarse cadence; identity subroutine only until
        // T-0257/T-0258 land the real ones. Spawned only when the
        // workstream router is available (steward writes per-workstream
        // journals into each KB).
        if let Some(ref router) = workstream_router {
            let router_clone = Arc::clone(router);
            let mem_resolver: arawn_steward::runner::MemoryResolver =
                Arc::new(move |name: &str| {
                    router_clone
                        .for_workstream(name)
                        .map_err(|e| arawn_steward::StewardError::Memory(e.to_string()))
                });
            // Reshelve uses the engine LLM by default. Cursor factory
            // opens a fresh CursorStore per workstream against the
            // same data dir.
            let data_dir_clone = std::path::PathBuf::from(&data_dir);
            let cursor_factory: Arc<
                dyn Fn(&str) -> Result<arawn_steward::CursorStore, arawn_steward::StewardError>
                    + Send
                    + Sync,
            > = Arc::new(move |name: &str| arawn_steward::CursorStore::open(&data_dir_clone, name));
            // Steward subroutines are focused summarisation/extraction
            // tasks — `hint:medium` is the right tier. Resolved via
            // `[routing.hints]` config — pure model-name lookup, no
            // runtime dispatch policy.
            let (reshelve_client, reshelve_model) =
                llm_pool.resolve_hint(&arawn_llm::ModelHint::Medium.as_hint());
            let reshelve = Arc::new(arawn_steward::ReshelveSubroutine::new(
                reshelve_client,
                reshelve_model,
                Arc::clone(&cursor_factory),
            ));
            let (map_client, map_model) =
                llm_pool.resolve_hint(&arawn_llm::ModelHint::Medium.as_hint());
            let map_sub = Arc::new(arawn_steward::MapSubroutine::new(
                map_client,
                map_model,
                Arc::clone(&cursor_factory),
            ));
            // Door-watch needs cross-workstream visibility: pass it the
            // shared Store + the same memory resolver the runner uses.
            let dw_resolver: arawn_steward::runner::MemoryResolver = {
                let router_clone = Arc::clone(router);
                Arc::new(move |name: &str| {
                    router_clone
                        .for_workstream(name)
                        .map_err(|e| arawn_steward::StewardError::Memory(e.to_string()))
                })
            };
            let (dw_client, dw_model) =
                llm_pool.resolve_hint(&arawn_llm::ModelHint::Medium.as_hint());
            let doorwatch = Arc::new(arawn_steward::DoorWatchSubroutine::new(
                dw_client,
                dw_model,
                Arc::clone(&cursor_factory),
                service.shared_store(),
                dw_resolver,
            ));
            // T-0265: tag-promoter subroutine (Suggest stage of ADR-0004).
            // Counts `tags_discovered` frequencies and proposes promotion
            // of recurring tags into the workstream's declared ontology.
            // Pure-stats subroutine — no LLM client needed.
            let tag_promoter = Arc::new(arawn_steward::TagPromoterSubroutine::default());
            let subs: Vec<Arc<dyn arawn_steward::StewardSubroutine>> =
                vec![reshelve, map_sub, doorwatch, tag_promoter];
            let steward_runner = Arc::new(arawn_steward::StewardRunner::new(
                service.shared_store(),
                std::path::PathBuf::from(&data_dir),
                mem_resolver,
                subs,
            ));
            // Coarse default cadence (1 hour) for dev — Phase 5's
            // test-harness work tunes this once real subroutines land.
            let interval_secs: u64 = 60 * 60;
            tokio::spawn(async move {
                let mut tick = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
                // First tick fires immediately; the identity subroutine
                // is a noop so this is safe at boot.
                loop {
                    tick.tick().await;
                    match steward_runner.run_pass_for_all().await {
                        Ok(s) if s.actions_journaled > 0 || s.errors > 0 => info!(
                            workstreams = s.workstreams_visited,
                            actions = s.actions_journaled,
                            errors = s.errors,
                            "steward pass"
                        ),
                        Ok(_) => debug!("steward pass: nothing to do"),
                        Err(e) => warn!(error = %e, "steward pass failed"),
                    }
                }
            });
            info!("steward scheduled (every 1h, reshelve + map + doorwatch)");
        }

        // Register workstream tools (need the shared store from the service).
        // The active-workstream shim is shared across the switch/show/list/delete
        // tools AND the memory router so they observe the same session-level state.
        // Idempotently materialize the scratch workstream so first-boot users
        // land in a valid scope.
        if let Err(e) = service
            .shared_store()
            .lock()
            .unwrap()
            .ensure_scratch_workstream()
        {
            warn!(error = %e, "failed to ensure scratch workstream");
        }
        registry.register(Box::new(arawn_engine::WorkstreamCreateTool::new(
            service.shared_store(),
        )));
        registry.register(Box::new(
            arawn_engine::WorkstreamListTool::new(service.shared_store())
                .with_active(active_workstream.clone()),
        ));
        registry.register(Box::new(arawn_engine::WorkstreamSwitchTool::new(
            service.shared_store(),
            active_workstream.clone(),
        )));
        registry.register(Box::new(arawn_engine::WorkstreamShowTool::new(
            service.shared_store(),
            active_workstream.clone(),
        )));
        registry.register(Box::new(arawn_engine::WorkstreamDescribeTool::new(
            service.shared_store(),
        )));

        // Generic todo tools (I-0049 T-0313) — always available,
        // not ceremony-gated. The agent uses these for chat-driven
        // todos ("remind me to ...") and the TUI `/todo` command
        // also routes through them.
        {
            let s = service.shared_store();
            let ev = Some(service.todo_event_sender());
            registry.register(Box::new(arawn_engine::TodoCreateTool::new(
                s.clone(),
                ev.clone(),
            )));
            registry.register(Box::new(arawn_engine::TodoListTool::new(
                s.clone(),
                ev.clone(),
            )));
            registry.register(Box::new(arawn_engine::TodoGetTool::new(
                s.clone(),
                ev.clone(),
            )));
            registry.register(Box::new(arawn_engine::TodoDoneTool::new(
                s.clone(),
                ev.clone(),
            )));
            registry.register(Box::new(arawn_engine::TodoUndoTool::new(
                s.clone(),
                ev.clone(),
            )));
            registry.register(Box::new(arawn_engine::TodoPatchTool::new(
                s.clone(),
                ev.clone(),
            )));
            registry.register(Box::new(arawn_engine::TodoArchiveTool::new(
                s.clone(),
                ev.clone(),
            )));
            registry.register(Box::new(arawn_engine::TodoSearchTool::new(s, ev)));
        }

        // T-0264: LLM-backed initial-ontology proposer for `/workstream-create`.
        registry.register(Box::new(arawn_engine::WorkstreamProposeOntologyTool::new(
            llm_pool.engine(),
            llm_pool.engine_config().model.clone(),
        )));
        // I-0050 T-0327 — late-bound github integration cell so the
        // bind hook can run the org-expand step. github_integration_for_feeds
        // is resolved further down (after the integration config block);
        // this cell is populated then. None means org-expand is a no-op.
        let github_for_bind_hook: Arc<
            std::sync::RwLock<Option<Arc<arawn_integrations::github::GithubIntegration>>>,
        > = Arc::new(std::sync::RwLock::new(None));

        // T-0329 — late-bound FeedRuntime cell so bind / unbind hooks
        // can register / unregister cron schedules without waiting for
        // a process restart. Populated after `arawn_feeds::start`
        // returns. None means hot register/unregister is a no-op.
        let feed_runtime_for_hooks: Arc<
            std::sync::RwLock<Option<Arc<arawn_feeds::FeedRuntime>>>,
        > = Arc::new(std::sync::RwLock::new(None));

        {
            let mut bind_tool = arawn_engine::WorkstreamBindTool::new(service.shared_store());
            if let Some(ref runner) = extractor_runner {
                // BindBackfillHook impl: on `/workstream bind`, look up
                // the feed_id in the feed store, map template →
                // projection feed_types, and spawn the extractor
                // backfill. Soft-fails when the feed isn't in the
                // store (e.g. unknown id) so the bind itself still
                // succeeds.
                struct ExtractorBindHook {
                    runner: Arc<arawn_extractor::ExtractorRunner>,
                    store: Arc<std::sync::Mutex<arawn_storage::Store>>,
                    github: Arc<
                        std::sync::RwLock<
                            Option<Arc<arawn_integrations::github::GithubIntegration>>,
                        >,
                    >,
                    feed_runtime: Arc<
                        std::sync::RwLock<Option<Arc<arawn_feeds::FeedRuntime>>>,
                    >,
                }
                impl arawn_engine::BindBackfillHook for ExtractorBindHook {
                    fn on_bind(&self, workstream_name: &str, feed_id: &str) {
                        // I-0045 T-0322 — github scope-bindings
                        // (github:repo:* / github:org:*) are synthetic
                        // ids; short-circuit to the three github feed
                        // types so existing projection rows get walked
                        // through the chain.
                        if arawn_engine::tools::workstream::is_github_scope_binding(feed_id) {
                            // I-0050 — backfill walks the user-scoped
                            // feeds (morning-brief signal) AND the
                            // four repo-mirror tables (commits/issues/
                            // prs/comments) the new template writes.
                            let feed_types = vec![
                                "github_notifications".to_string(),
                                "github_issues_and_prs".to_string(),
                                "github_review_queue".to_string(),
                                "github_repo_commits".to_string(),
                                "github_repo_issues".to_string(),
                                "github_repo_prs".to_string(),
                                "github_issue_or_pr_comments".to_string(),
                            ];
                            Arc::clone(&self.runner)
                                .spawn_backfill(workstream_name.to_string(), feed_types);
                            // I-0050 T-0327 — for org binds, kick off
                            // a list_org_repos expansion that registers
                            // one github-repo:owner/name feed per repo.
                            match arawn_engine::tools::workstream::parse_github_scope(feed_id) {
                                Some(arawn_engine::tools::workstream::GithubScope::Org {
                                    owner,
                                }) => {
                                    let gh = self.github.read().unwrap().clone();
                                    if let Some(gh) = gh {
                                        let store = Arc::clone(&self.store);
                                        let frt = self.feed_runtime.read().unwrap().clone();
                                        let ws = workstream_name.to_string();
                                        tokio::spawn(async move {
                                            arawn_bin::startup::expand_github_org(gh, store, frt, ws, owner).await;
                                        });
                                    } else {
                                        debug!(
                                            owner = %owner,
                                            "bind hook: github integration not wired; \
                                             skipping org expand. Re-bind after restart."
                                        );
                                    }
                                }
                                Some(arawn_engine::tools::workstream::GithubScope::Repo {
                                    owner,
                                    name,
                                }) => {
                                    // T-0329 — hot-register the cron
                                    // schedule for the newly-inserted
                                    // github-repo:owner/name feed so it
                                    // starts polling without a restart.
                                    let frt = self.feed_runtime.read().unwrap().clone();
                                    if let Some(frt) = frt {
                                        let store = Arc::clone(&self.store);
                                        let feed_id_full =
                                            format!("github-repo:{owner}/{name}");
                                        tokio::spawn(async move {
                                            arawn_bin::startup::register_one_feed(frt, store, &feed_id_full).await;
                                        });
                                    }
                                }
                                None => {}
                            }
                            return;
                        }
                        // Reach into the feeds table via the shared
                        // storage Database to resolve template → feed_types.
                        let template = {
                            let store = self.store.lock().unwrap();
                            let feed_store = arawn_feeds::FeedStore::new(store.database().conn());
                            match feed_store.get(feed_id) {
                                Ok(Some(rec)) => rec.template,
                                _ => {
                                    debug!(
                                        feed_id = %feed_id,
                                        "bind hook: feed not found; skipping backfill"
                                    );
                                    return;
                                }
                            }
                        };
                        let feed_types = arawn_feeds::projection_feed_types_for(&template);
                        if feed_types.is_empty() {
                            debug!(
                                template = %template,
                                "bind hook: template has no projection mapping"
                            );
                            return;
                        }
                        Arc::clone(&self.runner)
                            .spawn_backfill(workstream_name.to_string(), feed_types);
                    }
                }
                let hook: Arc<dyn arawn_engine::BindBackfillHook> = Arc::new(ExtractorBindHook {
                    runner: Arc::clone(runner),
                    store: service.shared_store(),
                    github: Arc::clone(&github_for_bind_hook),
                    feed_runtime: Arc::clone(&feed_runtime_for_hooks),
                });
                bind_tool = bind_tool.with_backfill_hook(hook);
            }
            registry.register(Box::new(bind_tool));
        }
        {
            let mut unbind_tool =
                arawn_engine::WorkstreamUnbindTool::new(service.shared_store());
            // T-0329 — drop the live cron schedule for each feed_id
            // the unbind removed from the feeds table.
            struct FeedRuntimeUnbindHook {
                feed_runtime:
                    Arc<std::sync::RwLock<Option<Arc<arawn_feeds::FeedRuntime>>>>,
            }
            impl arawn_engine::UnbindHook for FeedRuntimeUnbindHook {
                fn on_unbind(&self, removed_feed_ids: &[String]) {
                    let Some(frt) = self.feed_runtime.read().unwrap().clone() else {
                        return;
                    };
                    for id in removed_feed_ids.iter().cloned() {
                        let frt = Arc::clone(&frt);
                        tokio::spawn(async move {
                            if let Err(e) = frt.unregister_cron(&id).await {
                                warn!(feed_id = %id, error = %e,
                                      "unregister_cron failed");
                            }
                        });
                    }
                }
            }
            let hook: Arc<dyn arawn_engine::UnbindHook> =
                Arc::new(FeedRuntimeUnbindHook {
                    feed_runtime: Arc::clone(&feed_runtime_for_hooks),
                });
            unbind_tool = unbind_tool.with_unbind_hook(hook);
            registry.register(Box::new(unbind_tool));
        }
        registry.register(Box::new(arawn_engine::WorkstreamDeleteTool::new(
            service.shared_store(),
            active_workstream.clone(),
        )));
        // Steward surface — journal / refine / rollback. Routed
        // through the existing workstream memory router so default-to-
        // active behavior matches the rest of the workstream tools.
        if let Some(ref router) = workstream_router {
            registry.register(Box::new(arawn_engine::WorkstreamJournalTool::new(
                std::path::PathBuf::from(&data_dir),
                Arc::clone(router),
            )));
            registry.register(Box::new(arawn_engine::WorkstreamRefineTool::new(
                std::path::PathBuf::from(&data_dir),
                Arc::clone(router),
            )));
            registry.register(Box::new(arawn_engine::WorkstreamRollbackTool::new(
                std::path::PathBuf::from(&data_dir),
                Arc::clone(router),
            )));
            registry.register(Box::new(arawn_engine::WorkstreamApplyTool::new(
                std::path::PathBuf::from(&data_dir),
                Arc::clone(router),
            )));
            registry.register(Box::new(arawn_engine::WorkstreamDustTool::new(
                std::path::PathBuf::from(&data_dir),
                Arc::clone(router),
                llm_pool.engine(),
                llm_pool.engine_config().model.clone(),
            )));
            // T-0266: manual ontology CRUD outside the propose/accept cycle.
            registry.register(Box::new(arawn_engine::WorkstreamTagTool::new(
                std::path::PathBuf::from(&data_dir),
                Arc::clone(router),
            )));
        }
        // workstream_promote needs the router so it can reach into
        // arbitrary workstream KBs (not just the active one).
        if memory_manager.is_some() {
            let promote_router = Arc::new(arawn_engine::WorkstreamMemoryRouter::new(
                std::path::PathBuf::from(&data_dir),
                Some(embed_config.dimensions),
                embedder.clone(),
                active_workstream.clone(),
            ));
            registry.register(Box::new(arawn_engine::WorkstreamPromoteTool::new(
                service.shared_store(),
                promote_router,
            )));
        }

        // OAuth integrations (Gmail, Calendar, Drive, Atlassian, GitHub, Slack).
        // See `startup::integrations`.
        let integrations_for_feeds = arawn_bin::startup::integrations::wire_integrations(
            &config,
            &data_dir,
            &mut service,
            &registry,
            &github_for_bind_hook,
        );
        // Field-by-field destructuring removed — `integrations_for_feeds` is
        // passed by reference into `startup::feeds::wire_continual_feeds`.

        // Start workflow engine (cloacina DefaultRunner — background services start on construction)
        let workflow_config =
            arawn_workflow::runner::WorkflowRunnerConfig::new(std::path::Path::new(&data_dir));
        let workflows_dir = std::path::PathBuf::from(&data_dir).join("workflows");
        let shared_runner: arawn_workflow::SharedWorkflowRunner =
            Arc::new(tokio::sync::RwLock::new(None));

        let mut workflow_runner_handle: Option<Arc<arawn_workflow::WorkflowRunner>> = None;
        match arawn_workflow::WorkflowRunner::new(workflow_config).await {
            Ok(runner) => {
                info!("workflow runner started");
                let arc = Arc::new(runner);
                *shared_runner.write().await = Some(Arc::clone(&arc));
                workflow_runner_handle = Some(arc);
            }
            Err(e) => {
                warn!(error = %e, "workflow runner unavailable — continuing without workflows");
            }
        }

        // Register workflow tools (before config watcher takes registry ownership)
        arawn_bin::startup::register_workflow_tools(&registry, workflows_dir, Arc::clone(&shared_runner));

        // Continual data feeds (I-0039). See `startup::feeds`.
        arawn_bin::startup::feeds::wire_continual_feeds(
            workflow_runner_handle.as_ref(),
            &data_dir,
            &integrations_for_feeds,
            projections.as_ref(),
            extractor_runner.as_ref(),
            &mut service,
            &feed_runtime_for_hooks,
        ).await;

        // Ceremony engine (I-0043 + I-0041). See `startup::ceremonies`.
        arawn_bin::startup::ceremonies::wire_ceremony_engine(
            &config,
            workflow_runner_handle.as_ref(),
            &data_dir,
            &llm_pool,
            projections.as_ref(),
            &registry,
            &mut service,
        ).await;

        // Forward TodoEvents onto the notice broadcast so the TUI and
        // any other notice subscriber can react. Mirrors the ceremony
        // event forwarder pattern (T-0308).
        {
            let notice_tx_todo = service.notice_sender();
            let mut todo_rx = service.subscribe_todo_events();
            tokio::spawn(async move {
                loop {
                    match todo_rx.recv().await {
                        Ok(ev) => {
                            let message =
                                serde_json::to_string(&ev).unwrap_or_else(|_| "{}".to_string());
                            let notice = arawn_service::ServerNotice {
                                level: "info".into(),
                                category: "todo_event".into(),
                                message,
                                timestamp: chrono::Utc::now().to_rfc3339(),
                            };
                            let _ = notice_tx_todo.send(notice);
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(_) => break,
                    }
                }
            });
        }

        // Wire watchers into the broadcast so reload outcomes reach the TUI.
        let notice_tx_plugin = service.notice_sender();
        let _plugin_watcher = plugin_runtime.watch(
            Arc::clone(&skill_registry),
            Some(Arc::new(move |is_error: bool, msg: String| {
                let notice = arawn_service::ServerNotice {
                    level: if is_error {
                        "error".into()
                    } else {
                        "info".into()
                    },
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
                level: if is_error {
                    "error".into()
                } else {
                    "info".into()
                },
                category: "config_reload".into(),
                message: msg,
                timestamp: chrono::Utc::now().to_rfc3339(),
            };
            let _ = notice_tx_config.send(notice);
        }))
        .spawn();

        arawn_bin::ws_server::run_server(service, &config.server.host, serve_port).await?;

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
    arawn_bin::startup::run_cli_via_server(&server_url, &user_input, session_id).await
}







// T-0362: `render_usage_human` moved to
// `arawn_llm::usage::render_usage_human` so the TUI `/usage`
// slash command and the `arawn usage` CLI share a renderer.



