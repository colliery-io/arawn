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

    // Handle doctor subcommand immediately (exits process).
    // Doctor must run before the heavy startup path so a broken config
    // does not panic on the way to actually reporting "config broken".
    if let Some(Command::Doctor { json }) = &cli.command {
        let base = cli
            .data_dir
            .as_deref()
            .map(String::from)
            .or_else(dirs_path)
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
            .or_else(dirs_path)
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
        register_default_tools(
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
        let mcp_manager = connect_mcp_servers(&data_dir, &plugin_result, &registry).await;

        let mut engine_config = build_engine_config(&config, &workstream, &data_dir);

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
        .with_active_workstream(active_workstream.clone());

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
            // tasks — `hint:medium` is the right tier. Resolved through
            // the pool so future-T-0278 routing changes pick up here.
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
                                            expand_github_org(gh, store, frt, ws, owner).await;
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
                                            register_one_feed(frt, store, &feed_id_full).await;
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

        // Resolve OAuth credentials with precedence:
        //   env var → arawn.toml `[integrations.<service>]` → empty (skip).
        // This lets users persist creds in config without exporting env
        // vars on every shell, while keeping env-var override for ad-hoc
        // testing (different OAuth client per run, etc.).
        let resolve = |env_id: &str,
                       env_secret: &str,
                       cfg: &arawn_bin::config::IntegrationCredentials|
         -> Option<(String, String)> {
            let id = std::env::var(env_id)
                .ok()
                .filter(|s| !s.is_empty())
                .or_else(|| Some(cfg.client_id.clone()).filter(|s| !s.is_empty()))?;
            let secret = std::env::var(env_secret)
                .ok()
                .filter(|s| !s.is_empty())
                .or_else(|| Some(cfg.client_secret.clone()).filter(|s| !s.is_empty()))?;
            Some((id, secret))
        };

        // Register Gmail integration if creds are present (env or config).
        // Skipped silently otherwise — users without Gmail credentials still
        // get a working server. See docs/src/integrations/gmail.md.
        let gmail_creds = resolve(
            "ARAWN_GMAIL_CLIENT_ID",
            "ARAWN_GMAIL_CLIENT_SECRET",
            &config.integrations.gmail,
        )
        .or_else(|| {
            // Fall back to the shared Google credentials.
            resolve(
                "ARAWN_GOOGLE_CLIENT_ID",
                "ARAWN_GOOGLE_CLIENT_SECRET",
                &config.integrations.google,
            )
        });
        let gmail_integration_for_feeds: Option<Arc<arawn_integrations::gmail::GmailIntegration>>;
        if let Some((client_id, client_secret)) = gmail_creds {
            let gmail = Arc::new(arawn_integrations::gmail::GmailIntegration::new(
                std::path::PathBuf::from(&data_dir),
                client_id,
                client_secret,
            ));
            service.register_integration(
                Arc::clone(&gmail) as Arc<dyn arawn_integrations::Integration>
            );
            registry.register(Box::new(
                arawn_integrations::gmail::GmailInboxReadTool::new(Arc::clone(&gmail)),
            ));
            registry.register(Box::new(arawn_integrations::gmail::GmailSearchTool::new(
                Arc::clone(&gmail),
            )));
            registry.register(Box::new(
                arawn_integrations::gmail::GmailGetMessageTool::new(Arc::clone(&gmail)),
            ));
            registry.register(Box::new(arawn_integrations::gmail::GmailSendTool::new(
                Arc::clone(&gmail),
            )));
            registry.register(Box::new(arawn_integrations::gmail::GmailMarkReadTool::new(
                Arc::clone(&gmail),
            )));
            info!("Gmail integration registered (5 tools)");
            gmail_integration_for_feeds = Some(gmail);
        } else {
            gmail_integration_for_feeds = None;
            debug!(
                "Gmail integration skipped — set ARAWN_GMAIL_CLIENT_ID + \
                 ARAWN_GMAIL_CLIENT_SECRET (env) or [integrations.gmail] (config) \
                 to enable. See docs/src/integrations/gmail.md."
            );
        }

        // Register Google Calendar. Service-specific creds first; falls back
        // to the shared Google credentials so one OAuth project covers both.
        let gcal_creds = resolve(
            "ARAWN_GCAL_CLIENT_ID",
            "ARAWN_GCAL_CLIENT_SECRET",
            &config.integrations.calendar,
        )
        .or_else(|| {
            resolve(
                "ARAWN_GOOGLE_CLIENT_ID",
                "ARAWN_GOOGLE_CLIENT_SECRET",
                &config.integrations.google,
            )
        });
        let calendar_integration_for_feeds: Option<
            Arc<arawn_integrations::calendar::GoogleCalendarIntegration>,
        >;
        if let Some((client_id, client_secret)) = gcal_creds {
            let calendar = Arc::new(
                arawn_integrations::calendar::GoogleCalendarIntegration::new(
                    std::path::PathBuf::from(&data_dir),
                    client_id,
                    client_secret,
                ),
            );
            service.register_integration(
                Arc::clone(&calendar) as Arc<dyn arawn_integrations::Integration>
            );
            registry.register(Box::new(
                arawn_integrations::calendar::CalendarUpcomingTool::new(Arc::clone(&calendar)),
            ));
            registry.register(Box::new(
                arawn_integrations::calendar::CalendarCreateEventTool::new(Arc::clone(&calendar)),
            ));
            registry.register(Box::new(
                arawn_integrations::calendar::CalendarFindConflictsTool::new(Arc::clone(&calendar)),
            ));
            info!("Google Calendar integration registered (3 tools)");
            calendar_integration_for_feeds = Some(calendar);
        } else {
            calendar_integration_for_feeds = None;
            debug!(
                "Google Calendar integration skipped — set ARAWN_GCAL_CLIENT_ID + \
                 ARAWN_GCAL_CLIENT_SECRET (env) or [integrations.calendar] / \
                 [integrations.google] (config) to enable."
            );
        }

        // Register Google Drive. Same fallback chain as Calendar — service-specific
        // creds first, then the shared Google credentials.
        let drive_creds = resolve(
            "ARAWN_GDRIVE_CLIENT_ID",
            "ARAWN_GDRIVE_CLIENT_SECRET",
            &config.integrations.drive,
        )
        .or_else(|| {
            resolve(
                "ARAWN_GOOGLE_CLIENT_ID",
                "ARAWN_GOOGLE_CLIENT_SECRET",
                &config.integrations.google,
            )
        });
        let drive_integration_for_feeds: Option<
            Arc<arawn_integrations::drive::GoogleDriveIntegration>,
        >;
        if let Some((client_id, client_secret)) = drive_creds {
            let drive = Arc::new(arawn_integrations::drive::GoogleDriveIntegration::new(
                std::path::PathBuf::from(&data_dir),
                client_id,
                client_secret,
            ));
            service.register_integration(
                Arc::clone(&drive) as Arc<dyn arawn_integrations::Integration>
            );
            registry.register(Box::new(arawn_integrations::drive::DriveSearchTool::new(
                Arc::clone(&drive),
            )));
            registry.register(Box::new(arawn_integrations::drive::DriveListTool::new(
                Arc::clone(&drive),
            )));
            registry.register(Box::new(
                arawn_integrations::drive::DriveGetMetadataTool::new(Arc::clone(&drive)),
            ));
            registry.register(Box::new(arawn_integrations::drive::DriveReadTool::new(
                Arc::clone(&drive),
            )));
            registry.register(Box::new(arawn_integrations::drive::DriveUploadTool::new(
                Arc::clone(&drive),
            )));
            registry.register(Box::new(arawn_integrations::drive::DriveUpdateTool::new(
                Arc::clone(&drive),
            )));
            registry.register(Box::new(arawn_integrations::drive::DriveDeleteTool::new(
                Arc::clone(&drive),
            )));
            info!("Google Drive integration registered (7 tools)");
            drive_integration_for_feeds = Some(drive);
        } else {
            drive_integration_for_feeds = None;
            debug!(
                "Google Drive integration skipped — set ARAWN_GDRIVE_CLIENT_ID + \
                 ARAWN_GDRIVE_CLIENT_SECRET (env) or [integrations.drive] / \
                 [integrations.google] (config) to enable."
            );
        }

        // Register Atlassian (Jira + Confluence). One OAuth client, one
        // token; both tool families register together.
        let atlassian_integration_for_feeds: Option<
            Arc<arawn_integrations::atlassian::AtlassianIntegration>,
        >;
        if let Some((client_id, client_secret)) = resolve(
            "ARAWN_ATLASSIAN_CLIENT_ID",
            "ARAWN_ATLASSIAN_CLIENT_SECRET",
            &config.integrations.atlassian,
        ) {
            let atlassian = Arc::new(arawn_integrations::atlassian::AtlassianIntegration::new(
                std::path::PathBuf::from(&data_dir),
                client_id,
                client_secret,
            ));
            service.register_integration(
                Arc::clone(&atlassian) as Arc<dyn arawn_integrations::Integration>
            );
            registry.register(Box::new(
                arawn_integrations::atlassian::JiraSearchTool::new(Arc::clone(&atlassian)),
            ));
            registry.register(Box::new(
                arawn_integrations::atlassian::JiraGetIssueTool::new(Arc::clone(&atlassian)),
            ));
            registry.register(Box::new(
                arawn_integrations::atlassian::JiraCreateIssueTool::new(Arc::clone(&atlassian)),
            ));
            registry.register(Box::new(
                arawn_integrations::atlassian::JiraUpdateIssueTool::new(Arc::clone(&atlassian)),
            ));
            registry.register(Box::new(
                arawn_integrations::atlassian::JiraAddCommentTool::new(Arc::clone(&atlassian)),
            ));
            registry.register(Box::new(
                arawn_integrations::atlassian::JiraTransitionIssueTool::new(Arc::clone(&atlassian)),
            ));
            registry.register(Box::new(
                arawn_integrations::atlassian::ConfluenceSearchTool::new(Arc::clone(&atlassian)),
            ));
            registry.register(Box::new(
                arawn_integrations::atlassian::ConfluenceGetPageTool::new(Arc::clone(&atlassian)),
            ));
            registry.register(Box::new(
                arawn_integrations::atlassian::ConfluenceCreatePageTool::new(Arc::clone(
                    &atlassian,
                )),
            ));
            registry.register(Box::new(
                arawn_integrations::atlassian::ConfluenceUpdatePageTool::new(Arc::clone(
                    &atlassian,
                )),
            ));
            registry.register(Box::new(
                arawn_integrations::atlassian::ConfluenceListSpacesTool::new(Arc::clone(
                    &atlassian,
                )),
            ));
            info!("Atlassian integration registered (11 tools — 6 Jira, 5 Confluence)");
            // If the persisted token was minted by an older arawn
            // build that requested fewer scopes, surface that now —
            // confluence feeds will 401 with "scope does not match"
            // until the user re-runs /connect atlassian.
            if let Some(missing) = atlassian.missing_scopes() {
                warn!(
                    missing = ?missing,
                    "Atlassian token is missing scopes from the current build. \
                     Run `/disconnect atlassian` then `/connect atlassian` to \
                     mint a fresh token. Affected feeds (e.g. confluence/space-archive) \
                     will fail with 401 'scope does not match' until then."
                );
            }
            atlassian_integration_for_feeds = Some(atlassian);
        } else {
            atlassian_integration_for_feeds = None;
            debug!(
                "Atlassian integration skipped — set ARAWN_ATLASSIAN_CLIENT_ID + \
                 ARAWN_ATLASSIAN_CLIENT_SECRET (env) or [integrations.atlassian] (config) \
                 to enable."
            );
        }

        // Register GitHub (I-0045). Read-only v1 — the connect flow
        // captures an installation_id; tools/feed templates downstream
        // mint short-lived access tokens via the cached App config.
        let github_integration_for_feeds: Option<
            Arc<arawn_integrations::github::GithubIntegration>,
        >;
        let resolve_github = || -> Option<arawn_integrations::github::GithubAppConfig> {
            let cfg = &config.integrations.github;
            let app_id = std::env::var("ARAWN_GITHUB_APP_ID")
                .ok()
                .filter(|s| !s.is_empty())
                .or_else(|| Some(cfg.app_id.clone()).filter(|s| !s.is_empty()))?;
            let app_slug = std::env::var("ARAWN_GITHUB_APP_SLUG")
                .ok()
                .filter(|s| !s.is_empty())
                .or_else(|| Some(cfg.app_slug.clone()).filter(|s| !s.is_empty()))?;
            let private_key_pem = std::env::var("ARAWN_GITHUB_PRIVATE_KEY_PEM")
                .ok()
                .filter(|s| !s.is_empty())
                .or_else(|| {
                    let path = std::env::var("ARAWN_GITHUB_PRIVATE_KEY_PATH")
                        .ok()
                        .filter(|s| !s.is_empty())
                        .or_else(|| {
                            Some(cfg.private_key_path.clone()).filter(|s| !s.is_empty())
                        })?;
                    match std::fs::read_to_string(&path) {
                        Ok(pem) => Some(pem),
                        Err(e) => {
                            warn!(path = %path, error = %e,
                                "GitHub App private key path unreadable; skipping integration");
                            None
                        }
                    }
                })?;
            Some(arawn_integrations::github::GithubAppConfig {
                app_id,
                app_slug,
                private_key_pem,
            })
        };
        if let Some(app_cfg) = resolve_github() {
            let github = Arc::new(arawn_integrations::github::GithubIntegration::new(
                std::path::PathBuf::from(&data_dir),
                app_cfg,
            ));
            service.register_integration(
                Arc::clone(&github) as Arc<dyn arawn_integrations::Integration>,
            );
            info!("GitHub integration registered (read-only — no tools yet, feeds land in T-0319+)");
            // I-0050 T-0327 — wire the late-bound cell so the bind hook
            // can run list_org_repos expansion when github:org:owner
            // bindings land.
            *github_for_bind_hook.write().unwrap() = Some(Arc::clone(&github));
            github_integration_for_feeds = Some(github);
        } else {
            github_integration_for_feeds = None;
            debug!(
                "GitHub integration skipped — set ARAWN_GITHUB_APP_ID + \
                 ARAWN_GITHUB_APP_SLUG + ARAWN_GITHUB_PRIVATE_KEY_PATH (env) or \
                 [integrations.github] (config) to enable. See \
                 docs/src/integrations/github.md."
            );
        }
        // Register Slack. No sharing with Google — different OAuth ecosystem.
        let slack_integration_for_feeds: Option<Arc<arawn_integrations::slack::SlackIntegration>>;
        if let Some((client_id, client_secret)) = resolve(
            "ARAWN_SLACK_CLIENT_ID",
            "ARAWN_SLACK_CLIENT_SECRET",
            &config.integrations.slack,
        ) {
            let slack = Arc::new(arawn_integrations::slack::SlackIntegration::new(
                std::path::PathBuf::from(&data_dir),
                client_id,
                client_secret,
            ));
            service.register_integration(
                Arc::clone(&slack) as Arc<dyn arawn_integrations::Integration>
            );
            registry.register(Box::new(
                arawn_integrations::slack::SlackListChannelsTool::new(Arc::clone(&slack)),
            ));
            registry.register(Box::new(arawn_integrations::slack::SlackHistoryTool::new(
                Arc::clone(&slack),
            )));
            registry.register(Box::new(arawn_integrations::slack::SlackPostTool::new(
                Arc::clone(&slack),
            )));
            registry.register(Box::new(arawn_integrations::slack::SlackReactTool::new(
                Arc::clone(&slack),
            )));
            registry.register(Box::new(
                arawn_integrations::slack::SlackUsersListTool::new(Arc::clone(&slack)),
            ));
            registry.register(Box::new(arawn_integrations::slack::SlackOpenDmTool::new(
                Arc::clone(&slack),
            )));
            info!("Slack integration registered (6 tools)");
            slack_integration_for_feeds = Some(slack);
        } else {
            slack_integration_for_feeds = None;
            debug!(
                "Slack integration skipped — set ARAWN_SLACK_CLIENT_ID + \
                 ARAWN_SLACK_CLIENT_SECRET (env) or [integrations.slack] (config) \
                 to enable. See docs/src/integrations/slack.md."
            );
        }

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
        register_workflow_tools(&registry, workflows_dir, Arc::clone(&shared_runner));

        // Continual data feeds (I-0039). Registers per-feed cloacina
        // cron schedules that route through arawn-feeds' template
        // dispatcher. Skipped if the workflow runner failed to start —
        // feeds need cloacina to schedule them.
        if let Some(workflow_runner) = workflow_runner_handle.as_ref() {
            let feeds_db_path = std::path::PathBuf::from(&data_dir).join("arawn.db");
            match rusqlite::Connection::open(&feeds_db_path) {
                Ok(conn) => {
                    // arawn-feeds expects the schema to already be in
                    // place (V2 feeds migration is owned by
                    // arawn-storage and was applied when `Store::open`
                    // ran above).
                    let feeds_conn = Arc::new(tokio::sync::Mutex::new(conn));
                    let feeds_layout = Arc::new(arawn_feeds::DataLayout::new(&data_dir));
                    let feeds_registry = Arc::new(arawn_feeds::default_registry());

                    let mut clients = arawn_feeds::RealClients::new();
                    if let Some(slack) = slack_integration_for_feeds.as_ref() {
                        clients = clients.with_slack(Arc::clone(slack));
                    }
                    if let Some(cal) = calendar_integration_for_feeds.as_ref() {
                        clients = clients.with_calendar(Arc::clone(cal));
                    }
                    if let Some(gm) = gmail_integration_for_feeds.as_ref() {
                        clients = clients.with_gmail(Arc::clone(gm));
                    }
                    if let Some(dr) = drive_integration_for_feeds.as_ref() {
                        clients = clients.with_drive(Arc::clone(dr));
                    }
                    if let Some(at) = atlassian_integration_for_feeds.as_ref() {
                        clients = clients.with_atlassian(Arc::clone(at));
                    }
                    if let Some(gh) = github_integration_for_feeds.as_ref() {
                        clients = clients.with_github(Arc::clone(gh));
                    }
                    let clients: Arc<dyn arawn_feeds::FeedClients> = Arc::new(clients);

                    // Reuse the outer projection store + extractor runner
                    // built earlier so the bind hook, feed_search, embed
                    // pass, and feed dispatch all share one instance.
                    let feeds_projections = projections.clone();
                    let feeds_extractor = extractor_runner.clone();

                    match arawn_feeds::start(
                        workflow_runner.cloacina_runner(),
                        feeds_conn,
                        feeds_layout,
                        feeds_registry,
                        clients,
                        feeds_projections,
                        feeds_extractor,
                    )
                    .await
                    {
                        Ok(runtime) => {
                            // Hand the live runtime to the service so
                            // `/watch` and `/feeds` route through it.
                            let runtime = Arc::new(runtime);
                            service.set_feed_runtime(Arc::clone(&runtime));
                            // T-0329 — also expose the runtime to the
                            // bind/unbind hooks so they can hot-add or
                            // hot-remove cron schedules.
                            *feed_runtime_for_hooks.write().unwrap() =
                                Some(Arc::clone(&runtime));
                            info!("feed runtime started");
                        }
                        Err(e) => warn!(error = %e, "feed runtime failed to start"),
                    }
                }
                Err(e) => warn!(error = %e, db = %feeds_db_path.display(),
                    "feed runtime unavailable — could not open arawn.db"),
            }
        } else {
            debug!("feed runtime skipped — workflow runner not available");
        }

        // Ceremony engine (I-0043 + I-0041). The shared infra
        // (connection, plugin registry, dispatcher, service, runner)
        // is built when the cloacina workflow runner is available.
        // Per-plugin enablement is gated on `[ceremonies.<kind>]`
        // in arawn.toml: retro and daily each have their own
        // enabled-flag and override surface. Absent table or
        // missing fields → use the plugin's compiled-in defaults.
        let retro_cfg = config.ceremonies.get("retro");
        let retro_enabled = retro_cfg.is_none_or(arawn_bin::CeremonyConfig::is_enabled);
        let daily_cfg = config.ceremonies.get("daily");
        let daily_enabled = daily_cfg.is_none_or(arawn_bin::CeremonyConfig::is_enabled);
        let weekly_cfg = config.ceremonies.get("weekly");
        let weekly_enabled = weekly_cfg.is_none_or(arawn_bin::CeremonyConfig::is_enabled);

        if let Some(workflow_runner) = workflow_runner_handle.as_ref()
            && (retro_enabled || daily_enabled || weekly_enabled)
        {
            let cer_db_path = std::path::PathBuf::from(&data_dir).join("arawn.db");
            match rusqlite::Connection::open(&cer_db_path) {
                Ok(conn) => {
                    let conn_handle = arawn_ceremonies::ConnHandle::new(conn);
                    let plugin_reg = arawn_ceremonies::PluginRegistry::new();

                    // Retro plugin construction (gated). Defer the
                    // resolve_ceremony_tz fn definition below; inline
                    // the resolution to avoid forward-ref churn.
                    if retro_enabled {
                        let model_hint = retro_cfg
                            .and_then(|c| c.model.clone())
                            .unwrap_or_else(|| arawn_llm::ModelHint::Medium.as_hint());
                        let (retro_client, retro_model) = llm_pool.resolve_hint(&model_hint);
                        let retro_tz_raw = retro_cfg.and_then(|c| c.timezone.as_deref());
                        let retro_tz: chrono_tz::Tz = {
                            use std::str::FromStr;
                            match retro_tz_raw {
                                None => chrono_tz::UTC,
                                Some(s) => {
                                    let t = s.trim();
                                    if t.is_empty() || t.eq_ignore_ascii_case("local") {
                                        chrono_tz::UTC
                                    } else {
                                        chrono_tz::Tz::from_str(t).unwrap_or(chrono_tz::UTC)
                                    }
                                }
                            }
                        };
                        // Cadence resolution (T-0367):
                        //   1. DB row in ceremony_config (live tool writes)
                        //   2. [ceremonies.retro] cadence in arawn.toml
                        //   3. Weekly default.
                        let toml_cadence = retro_cfg
                            .and_then(|c| c.cadence.as_deref())
                            .and_then(arawn_ceremonies::RetroCadence::parse);
                        let (cadence, mut anchor) =
                            arawn_ceremonies::RetroCeremony::load_persisted_cadence(
                                &conn_handle,
                                toml_cadence,
                            );
                        // Biweekly needs an anchor. If absent (first
                        // boot on this cadence), initialise to "this
                        // Monday" and persist so the cycle is stable.
                        if cadence == arawn_ceremonies::RetroCadence::Biweekly && anchor.is_none() {
                            use chrono::Datelike;
                            let today = chrono::Utc::now().date_naive();
                            let weekday_offset =
                                today.weekday().num_days_from_monday() as i64;
                            let this_monday =
                                today - chrono::Duration::days(weekday_offset);
                            anchor = Some(this_monday);
                            if let Err(e) = arawn_ceremonies::RetroCeremony::save_cadence(
                                &conn_handle,
                                cadence,
                                anchor,
                            ) {
                                warn!(error = %e, "failed to persist initial retro cadence anchor");
                            }
                        }
                        let retro = arawn_ceremonies::RetroCeremony::new(retro_client, retro_model)
                            .with_detectors(arawn_ceremonies::retro_v1_catalog())
                            .with_timezone(retro_tz)
                            .with_cadence(cadence, anchor);
                        if let Err(e) = plugin_reg.register(Arc::new(retro)) {
                            warn!(error = %e, "ceremony retro plugin registration failed");
                        }
                    }

                    // Daily + weekly both depend on the projection
                    // store for calendar + attention sources. The
                    // attention source is timezone-agnostic and can
                    // be shared; the calendar source brackets day
                    // windows in the configured ceremony timezone, so
                    // daily and weekly each need their own instance
                    // when their `[ceremonies.<kind>].timezone` strings
                    // differ. We resolve the tz strings up-front and
                    // build per-plugin calendar handles below.
                    let daily_actually_enabled = daily_enabled && projections.is_some();
                    let weekly_actually_enabled = weekly_enabled && projections.is_some();

                    // Local helper: parse a CeremonyConfig.timezone
                    // into a chrono_tz::Tz, defaulting to UTC for
                    // "local"/missing values and warn-falling-back on
                    // unknown IANA zones. Kept inline because it's
                    // only used here.
                    fn resolve_ceremony_tz(kind: &str, raw: Option<&str>) -> chrono_tz::Tz {
                        use std::str::FromStr;
                        match raw {
                            None => chrono_tz::UTC,
                            Some(s) => {
                                let trimmed = s.trim();
                                if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("local") {
                                    tracing::debug!(
                                        kind,
                                        "ceremony timezone '{trimmed}' → UTC fallback"
                                    );
                                    chrono_tz::UTC
                                } else {
                                    chrono_tz::Tz::from_str(trimmed).unwrap_or_else(|_| {
                                        warn!(
                                            kind,
                                            raw = %trimmed,
                                            "unknown ceremony timezone — falling back to UTC"
                                        );
                                        chrono_tz::UTC
                                    })
                                }
                            }
                        }
                    }

                    let daily_tz =
                        resolve_ceremony_tz("daily", daily_cfg.and_then(|c| c.timezone.as_deref()));
                    let weekly_tz = resolve_ceremony_tz(
                        "weekly",
                        weekly_cfg.and_then(|c| c.timezone.as_deref()),
                    );

                    let attention_source: Option<Arc<dyn arawn_ceremonies::AttentionSource>> =
                        if (daily_actually_enabled || weekly_actually_enabled)
                            && let Some(projections) = projections.as_ref()
                        {
                            Some(Arc::new(arawn_engine::ProjectionsAttentionSource::new(
                                Arc::clone(projections),
                                service.shared_store(),
                            )))
                        } else {
                            None
                        };

                    let daily_calendar: Option<Arc<dyn arawn_ceremonies::CalendarSource>> =
                        if daily_actually_enabled && let Some(projections) = projections.as_ref() {
                            Some(Arc::new(
                                arawn_engine::ProjectionsCalendarSource::new(Arc::clone(
                                    projections,
                                ))
                                .with_tz(daily_tz),
                            ))
                        } else {
                            None
                        };
                    let weekly_calendar: Option<Arc<dyn arawn_ceremonies::CalendarSource>> =
                        if weekly_actually_enabled && let Some(projections) = projections.as_ref() {
                            Some(Arc::new(
                                arawn_engine::ProjectionsCalendarSource::new(Arc::clone(
                                    projections,
                                ))
                                .with_tz(weekly_tz),
                            ))
                        } else {
                            None
                        };

                    // Daily plugin construction (gated). Requires the
                    // projection store for calendar + attention
                    // sources; degrades to "daily disabled" if it
                    // isn't available.
                    if daily_actually_enabled
                        && let (Some(calendar), Some(attention)) =
                            (daily_calendar.as_ref(), attention_source.as_ref())
                    {
                        let model_hint = daily_cfg
                            .and_then(|c| c.model.clone())
                            .unwrap_or_else(|| arawn_llm::ModelHint::Medium.as_hint());
                        let (daily_client, daily_model) = llm_pool.resolve_hint(&model_hint);
                        let daily = arawn_ceremonies::DailyCeremony::new(
                            daily_client,
                            daily_model,
                            Arc::clone(calendar),
                            Arc::clone(attention),
                        )
                        .with_timezone(daily_tz);
                        if let Err(e) = plugin_reg.register(Arc::new(daily)) {
                            warn!(error = %e, "ceremony daily plugin registration failed");
                        }
                    } else if daily_enabled {
                        warn!(
                            "daily ceremony enabled in config but projection store unavailable — skipping"
                        );
                    }

                    // Weekly plugin construction (gated). Same
                    // projection-store dependency as daily.
                    if weekly_actually_enabled
                        && let (Some(calendar), Some(attention)) =
                            (weekly_calendar.as_ref(), attention_source.as_ref())
                    {
                        let model_hint = weekly_cfg
                            .and_then(|c| c.model.clone())
                            .unwrap_or_else(|| arawn_llm::ModelHint::Medium.as_hint());
                        let (weekly_client, weekly_model) = llm_pool.resolve_hint(&model_hint);
                        let weekly = arawn_ceremonies::WeeklyCeremony::new(
                            weekly_client,
                            weekly_model,
                            Arc::clone(calendar),
                            Arc::clone(attention),
                        )
                        .with_timezone(weekly_tz);
                        if let Err(e) = plugin_reg.register(Arc::new(weekly)) {
                            warn!(error = %e, "ceremony weekly plugin registration failed");
                        }
                    } else if weekly_enabled {
                        warn!(
                            "weekly ceremony enabled in config but projection store unavailable — skipping"
                        );
                    }

                    let (event_tx, mut event_rx) = arawn_ceremonies::event_channel();
                    // Forward ceremony events onto the existing notice
                    // broadcast so the TUI's read loop can react (see
                    // T-0308 slice 3). Drop-stale (lagged) errors are
                    // silently ignored — the receiver re-subscribes on
                    // the next event.
                    {
                        let notice_tx_cer = service.notice_sender();
                        tokio::spawn(async move {
                            loop {
                                match event_rx.recv().await {
                                    Ok(ev) => {
                                        // Side-by-side notices: the legacy
                                        // `ceremony_event` category carries the
                                        // raw JSON for existing TUI handlers
                                        // (priority modal refresh, etc.); the
                                        // I-0035 Phase 4 `briefing_ready`
                                        // category fires only on
                                        // `TabletGenerated` and triggers the
                                        // brief-cache refresh in the TUI.
                                        let message = serde_json::to_string(&ev)
                                            .unwrap_or_else(|_| "{}".to_string());
                                        let now = chrono::Utc::now().to_rfc3339();
                                        let _ = notice_tx_cer.send(
                                            arawn_service::ServerNotice {
                                                level: "info".into(),
                                                category: "ceremony_event".into(),
                                                message: message.clone(),
                                                timestamp: now.clone(),
                                            },
                                        );
                                        if let arawn_ceremonies::CeremonyEvent::TabletGenerated {
                                            kind,
                                            period_key,
                                            ..
                                        } = &ev
                                        {
                                            let _ = notice_tx_cer.send(
                                                arawn_service::ServerNotice {
                                                    level: "info".into(),
                                                    category: "briefing_ready".into(),
                                                    message: format!(
                                                        "Brief updated — {kind} tablet for {period_key}"
                                                    ),
                                                    timestamp: now,
                                                },
                                            );
                                        }
                                    }
                                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                                        continue;
                                    }
                                    Err(_) => break,
                                }
                            }
                        });
                    }
                    let dispatcher = Arc::new(
                        arawn_ceremonies::EngineDispatcher::new(
                            conn_handle.clone(),
                            plugin_reg.clone(),
                        )
                        .with_events(event_tx.clone()),
                    );
                    // Back-fill missed daily/weekly ceremonies (T-0366).
                    // Runs once on boot, *before* the cron loop attaches
                    // — prevents races with a cron tick that fires
                    // milliseconds later. retro is excluded by design;
                    // see ARAWN-I-0052.
                    let backfill_lookback = config.backfill.ceremony_lookback_days;
                    let backfill_registry = plugin_reg.clone();
                    let backfill_dispatcher: Arc<dyn arawn_ceremonies::CeremonyDispatcher> =
                        Arc::clone(&dispatcher)
                            as Arc<dyn arawn_ceremonies::CeremonyDispatcher>;
                    match arawn_ceremonies::backfill::run(
                        &backfill_registry,
                        backfill_dispatcher.as_ref(),
                        backfill_lookback,
                    )
                    .await
                    {
                        Ok(_report) => {}
                        Err(e) => {
                            warn!(error = %e, "ceremony back-fill failed — cron still attached");
                        }
                    }

                    let runner = arawn_ceremonies::CeremonyRunner::new(
                        plugin_reg,
                        workflow_runner.cloacina_runner(),
                        Arc::clone(&dispatcher) as Arc<dyn arawn_ceremonies::CeremonyDispatcher>,
                    );
                    let cer_service = Arc::new(
                        arawn_ceremonies::CeremonyService::new(
                            conn_handle.clone(),
                            Arc::clone(&dispatcher)
                                as Arc<dyn arawn_ceremonies::CeremonyDispatcher>,
                        )
                        .with_events(event_tx),
                    );

                    // Cron registration per enabled plugin. Each
                    // `[ceremonies.<kind>]` schedule override applied
                    // here; invalid expressions log a warn and fall
                    // back to plugin default rather than abort.
                    if retro_enabled {
                        let sched = retro_cfg.and_then(|c| {
                            c.schedule.as_ref().map(|expr| {
                                arawn_ceremonies::CronSchedule::new(
                                    expr.clone(),
                                    c.timezone.clone().unwrap_or_else(|| "Local".to_string()),
                                )
                            })
                        });
                        if let Err(e) = runner.register_one_with_schedule("retro", sched).await {
                            warn!(error = %e, "ceremony runner failed to register retro cron — manual runs still work");
                        }
                    }
                    if daily_actually_enabled {
                        let sched = daily_cfg.and_then(|c| {
                            c.schedule.as_ref().map(|expr| {
                                arawn_ceremonies::CronSchedule::new(
                                    expr.clone(),
                                    c.timezone.clone().unwrap_or_else(|| "Local".to_string()),
                                )
                            })
                        });
                        if let Err(e) = runner.register_one_with_schedule("daily", sched).await {
                            warn!(error = %e, "ceremony runner failed to register daily cron — manual runs still work");
                        }
                    }
                    if weekly_actually_enabled {
                        let sched = weekly_cfg.and_then(|c| {
                            c.schedule.as_ref().map(|expr| {
                                arawn_ceremonies::CronSchedule::new(
                                    expr.clone(),
                                    c.timezone.clone().unwrap_or_else(|| "Local".to_string()),
                                )
                            })
                        });
                        if let Err(e) = runner.register_one_with_schedule("weekly", sched).await {
                            warn!(error = %e, "ceremony runner failed to register weekly cron — manual runs still work");
                        }
                    }

                    // Sunday-night sweep — retro-only.
                    if retro_enabled {
                        let sweep_handle = conn_handle.clone();
                        tokio::spawn(async move {
                            let mut interval =
                                tokio::time::interval(std::time::Duration::from_secs(3600));
                            interval
                                .set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                            loop {
                                interval.tick().await;
                                match arawn_ceremonies::sweep_unreviewed_retros(&sweep_handle) {
                                    Ok(0) => {}
                                    Ok(n) => info!(transitioned = n, "retro sweep"),
                                    Err(e) => warn!(error = %e, "retro sweep failed"),
                                }
                            }
                        });
                    }

                    service.set_ceremony_service(Arc::clone(&cer_service));

                    // Retro agent tools (gated on retro_enabled).
                    if retro_enabled {
                        registry.register(Box::new(arawn_engine::RetroRunTool::new(Arc::clone(
                            &cer_service,
                        ))));
                        registry.register(Box::new(arawn_engine::RetroCurrentTool::new(
                            Arc::clone(&cer_service),
                        )));
                        registry.register(Box::new(arawn_engine::RetroListItemsTool::new(
                            Arc::clone(&cer_service),
                        )));
                        registry.register(Box::new(arawn_engine::RetroSaveDiaryTool::new(
                            Arc::clone(&cer_service),
                        )));
                        registry.register(Box::new(arawn_engine::RetroPatchItemTool::new(
                            Arc::clone(&cer_service),
                        )));
                        registry.register(Box::new(arawn_engine::RetroSetCadenceTool::new(
                            Arc::clone(&cer_service),
                        )));
                    }

                    // Daily agent tools (gated on daily_actually_enabled).
                    if daily_actually_enabled {
                        registry.register(Box::new(arawn_engine::DailyRunTool::new(Arc::clone(
                            &cer_service,
                        ))));
                        registry.register(Box::new(arawn_engine::DailyCurrentTool::new(
                            Arc::clone(&cer_service),
                        )));
                        registry.register(Box::new(arawn_engine::DailyListItemsTool::new(
                            Arc::clone(&cer_service),
                        )));
                        registry.register(Box::new(arawn_engine::DailyPatchItemTool::new(
                            Arc::clone(&cer_service),
                        )));
                        registry.register(Box::new(arawn_engine::DailyAddTodoTool::new(
                            Arc::clone(&cer_service),
                        )));
                    }

                    // Weekly agent tools (gated on weekly_actually_enabled).
                    if weekly_actually_enabled {
                        registry.register(Box::new(arawn_engine::WeeklyRunTool::new(Arc::clone(
                            &cer_service,
                        ))));
                        registry.register(Box::new(arawn_engine::WeeklyCurrentTool::new(
                            Arc::clone(&cer_service),
                        )));
                        registry.register(Box::new(arawn_engine::WeeklyListItemsTool::new(
                            Arc::clone(&cer_service),
                        )));
                        registry.register(Box::new(arawn_engine::WeeklyListPrioritiesTool::new(
                            Arc::clone(&cer_service),
                        )));
                        registry.register(Box::new(arawn_engine::WeeklyConfirmPriorityTool::new(
                            Arc::clone(&cer_service),
                        )));
                        registry.register(Box::new(arawn_engine::WeeklyRejectPriorityTool::new(
                            Arc::clone(&cer_service),
                        )));
                        registry.register(Box::new(arawn_engine::WeeklyAddPriorityTool::new(
                            Arc::clone(&cer_service),
                        )));
                    }

                    info!(
                        retro = retro_enabled,
                        daily = daily_actually_enabled,
                        weekly = weekly_actually_enabled,
                        "ceremony engine wired"
                    );
                }
                Err(e) => warn!(error = %e, db = %cer_db_path.display(),
                    "ceremony engine unavailable — could not open arawn.db"),
            }
        } else {
            debug!(
                "ceremony engine skipped — workflow runner not available or all ceremonies disabled"
            );
        }

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
    run_cli_via_server(&server_url, &user_input, session_id).await
}

/// Run a CLI prompt by connecting to the running server via WebSocket.
async fn run_cli_via_server(url: &str, prompt: &str, session_id: Option<Uuid>) -> Result<()> {
    use arawn_tui::ws_client::{EventUpdate, WsClient, engine_event_to_update, parse_engine_event};

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
            let s = client
                .create_session(None)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to create session: {e}"))?;
            eprintln!("Session: {}", s.id);
            s.id
        }
    };

    // Send the prompt
    if prompt.is_empty() {
        eprintln!("No prompt provided");
        std::process::exit(1);
    }

    // request_response awaits the JSON-RPC ack via the dedicated reader
    // task — it can fail synchronously if the server rejected the request.
    let ack = client
        .request_response(
            "send_message",
            serde_json::json!({
                "session_id": session_uuid.to_string(),
                "content": prompt,
            }),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Failed to send message: {e}"))?;
    if let Some(err) = ack.get("error") {
        let msg = err
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown error");
        eprintln!("Server error: {msg}");
        std::process::exit(1);
    }

    eprintln!("Thinking...\n");

    let mut events = client
        .events_take()
        .ok_or_else(|| anyhow::anyhow!("ws events channel already taken"))?;

    // Stream events until Complete or Error
    let final_text = 'stream: loop {
        let ev = events.recv().await;
        match ev {
            Some(arawn_tui::ws_client::WsEvent::Text(text)) => {
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
            }
            Some(arawn_tui::ws_client::WsEvent::Closed) => {
                eprintln!("Server closed connection");
                std::process::exit(1);
            }
            Some(arawn_tui::ws_client::WsEvent::Error(e)) => {
                eprintln!("WebSocket error: {e}");
                std::process::exit(1);
            }
            None => {
                eprintln!("Connection lost");
                std::process::exit(1);
            }
        }
    };

    println!("{final_text}");
    Ok(())
}

/// Build the appropriate LLM client based on provider config.
fn build_llm_client(config: &arawn_bin::LlmConfig) -> Result<Arc<dyn arawn_llm::LlmClient>> {
    let resolved_key = arawn_bin::ArawnConfig::resolve_api_key(config);
    match config.provider.as_str() {
        "anthropic" => {
            let api_key = resolved_key.ok_or_else(|| {
                anyhow::anyhow!(
                    "Anthropic provider requires an API key — set `api_key` in [llm.<name>] or export {}",
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
                resolved_key,
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
        FileWriteTool, GlobTool, GrepTool, ShellTool, SleepTool, TaskGetTool, TaskListTool,
        TaskOutputTool, TaskStopTool, ThinkTool, WebFetchTool, WebSearchTool,
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

    registry.register(Box::new(SleepTool));
    registry.register(Box::new(TaskListTool::new(Arc::clone(&bg_manager))));
    registry.register(Box::new(TaskGetTool::new(Arc::clone(&bg_manager))));
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
        mcp_manager.connect_all(&mcp_config.servers, registry).await;
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
        mcp_manager.connect_all(&plugin_mcp_configs, registry).await;
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
        // Emit a hint instead of a concrete model name. The pool resolves
        // this at engine-construction time in `local_service`. T-0278 will
        // promote this to per-call resolution.
        model: arawn_llm::ModelHint::Heavy.as_hint(),
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
            // Overridden per-session in `LocalService` from the active
            // workstream's column; the template carries the boot workstream's
            // value so single-shot CLI flows pick the right persona too.
            identity_profile: workstream.identity_profile,
            // Filled in by LocalService per-query (it has access to the
            // integration registry); the template stays None.
            integration_capabilities: None,
        }),
        tool_timeout_secs: config.engine.tool_timeout_secs,
    }
}

// T-0362: `render_usage_human` moved to
// `arawn_llm::usage::render_usage_human` so the TUI `/usage`
// slash command and the `arawn usage` CLI share a renderer.

/// I-0050 T-0327 — list every repo under `owner` (via the github
/// integration's authenticated client) and register one
/// `github/repo-mirror` feed per repo against `workstream`.
/// Idempotent: existing feeds are skipped.
async fn expand_github_org(
    github: Arc<arawn_integrations::github::GithubIntegration>,
    store: Arc<std::sync::Mutex<arawn_storage::Store>>,
    feed_runtime: Option<Arc<arawn_feeds::FeedRuntime>>,
    workstream: String,
    owner: String,
) {
    use arawn_feeds::GithubFeedClient;
    let real_client = arawn_feeds::clients::RealGithubClient::new(github);
    let repos = match real_client.list_org_repos(&owner, 10).await {
        Ok(v) => v,
        Err(e) => {
            warn!(owner = %owner, error = %e, "org-expand: list_org_repos failed");
            return;
        }
    };
    info!(owner = %owner, count = repos.len(), workstream = %workstream,
          "org-expand: registering per-repo feeds");
    let now = chrono::Utc::now().to_rfc3339();
    let mut new_feed_ids: Vec<String> = Vec::new();
    {
        let store_guard = match store.lock() {
            Ok(g) => g,
            Err(_) => {
                warn!("org-expand: store mutex poisoned");
                return;
            }
        };
        let conn = store_guard.database().conn();
        for repo in repos {
            let name = repo
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("")
                .to_string();
            if name.is_empty() {
                continue;
            }
            let feed_id = format!("github-repo:{owner}/{name}");
            let params_json =
                serde_json::json!({"owner": &owner, "name": &name}).to_string();
            // Idempotent insert — skip if already exists.
            match conn.execute(
                "INSERT OR IGNORE INTO feeds \
                 (id, template, params, cadence, enabled, created_at, updated_at) \
                 VALUES (?1, 'github/repo-mirror', ?2, '*/30 * * * *', 1, ?3, ?3)",
                rusqlite::params![&feed_id, &params_json, &now],
            ) {
                Ok(rows) if rows > 0 => new_feed_ids.push(feed_id),
                Ok(_) => {} // already existed → no cron register
                Err(e) => warn!(feed_id = %feed_id, error = %e,
                                "org-expand: insert failed"),
            }
        }
    }
    // T-0329 — hot-register cron for each newly-inserted feed so the
    // org expand becomes live without a restart.
    if let Some(frt) = feed_runtime {
        for id in new_feed_ids {
            register_one_feed(Arc::clone(&frt), Arc::clone(&store), &id).await;
        }
    }
    let _ = workstream; // future: persist which workstream owns these
}

/// T-0329 — fetch a feed record by id and register its cron schedule
/// with the live runtime. Idempotent: cloacina's `register_one`
/// deletes any pre-existing schedule for the same workflow_name
/// before inserting a new one.
async fn register_one_feed(
    feed_runtime: Arc<arawn_feeds::FeedRuntime>,
    store: Arc<std::sync::Mutex<arawn_storage::Store>>,
    feed_id: &str,
) {
    let record = {
        let s = match store.lock() {
            Ok(g) => g,
            Err(_) => {
                warn!(feed_id = %feed_id, "register_one_feed: store mutex poisoned");
                return;
            }
        };
        let feed_store = arawn_feeds::FeedStore::new(s.database().conn());
        match feed_store.get(feed_id) {
            Ok(Some(r)) => r,
            Ok(None) => {
                debug!(feed_id = %feed_id,
                       "register_one_feed: row not found; skipping");
                return;
            }
            Err(e) => {
                warn!(feed_id = %feed_id, error = %e,
                      "register_one_feed: lookup failed");
                return;
            }
        }
    };
    if let Err(e) = feed_runtime.register_feed_runtime(&record).await {
        warn!(feed_id = %feed_id, error = %e,
              "register_one_feed: cron registration failed");
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
