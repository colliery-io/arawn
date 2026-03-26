//! Server assembly, token management, test data seeding, and log cleanup.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;

use arawn_agent_indexing::SessionIndexer;
use arawn_mcp::McpManager;
use arawn_memory::MemoryStore;
use arawn_server::{AppState, Server, ServerConfig};
use arawn_workstream::{WorkstreamConfig as WsConfig, WorkstreamManager};

use super::Context;

/// Phase 15: Assemble server config, AppState, workstreams, session cache, and compressor.
#[allow(clippy::too_many_arguments)]
pub(super) async fn assemble_server(
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

/// Load a persisted server token, or generate and save a new one.
pub(super) fn load_or_generate_server_token() -> Result<String> {
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

/// Seed the database with test workstreams and sessions for development.
pub(super) fn seed_test_data(manager: &WorkstreamManager, verbose: bool) {
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
pub(super) fn cleanup_old_logs(log_dir: &std::path::Path, max_age_days: u64, verbose: bool) {
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
