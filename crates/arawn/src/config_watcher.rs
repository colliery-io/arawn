//! Config watcher — watches arawn.toml and live-updates permissions, MCP servers, etc.
//!
//! Spawned as a background task in serve mode. Uses notify for file watching
//! with debouncing.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::{Mutex, mpsc};
use tracing::{info, warn};

use arawn_engine::PermissionRule;
use arawn_engine::tool::ToolRegistry;
use arawn_mcp::McpManager;

use crate::config::ArawnConfig;

/// Watches config files and dispatches live updates to running subsystems.
pub struct ConfigWatcher {
    config_path: PathBuf,
    data_dir: PathBuf,
    permission_rules: Arc<std::sync::RwLock<Vec<PermissionRule>>>,
    mcp_manager: Arc<Mutex<McpManager>>,
    tool_registry: Arc<ToolRegistry>,
    /// Optional callback fired after each reload completes (success or failure)
    /// with `(is_error, message)`. Wired by the binary to broadcast a
    /// `ServerNotice` so the TUI can surface the outcome.
    notify: Option<Arc<dyn Fn(bool, String) + Send + Sync>>,
}

impl ConfigWatcher {
    pub fn new(
        config_path: PathBuf,
        data_dir: PathBuf,
        permission_rules: Arc<std::sync::RwLock<Vec<PermissionRule>>>,
        mcp_manager: Arc<Mutex<McpManager>>,
        tool_registry: Arc<ToolRegistry>,
    ) -> Self {
        Self {
            config_path,
            data_dir,
            permission_rules,
            mcp_manager,
            tool_registry,
            notify: None,
        }
    }

    /// Attach a notify callback fired after each reload completes.
    pub fn with_notify(mut self, notify: Arc<dyn Fn(bool, String) + Send + Sync>) -> Self {
        self.notify = Some(notify);
        self
    }

    /// Spawn the file watcher as a background tokio task.
    pub fn spawn(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            if let Err(e) = self.run().await {
                warn!(error = %e, "config watcher exited with error");
            }
        })
    }

    async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (tx, mut rx) = mpsc::channel::<notify::Result<notify::Event>>(64);

        let mut watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.blocking_send(res);
            },
            notify::Config::default().with_poll_interval(Duration::from_secs(2)),
        )?;

        // Watch the config file's parent directory
        if let Some(parent) = self.config_path.parent() {
            watcher.watch(parent, RecursiveMode::NonRecursive)?;
        }

        info!(path = ?self.config_path, "config watcher started");

        loop {
            let event = match rx.recv().await {
                Some(Ok(event)) => event,
                Some(Err(e)) => {
                    warn!(error = %e, "config watch error");
                    continue;
                }
                None => break,
            };

            if !matches!(
                event.kind,
                EventKind::Create(_) | EventKind::Modify(_)
            ) {
                continue;
            }

            // Check if the event is for our config file
            let is_our_file = event
                .paths
                .iter()
                .any(|p: &std::path::PathBuf| p.ends_with("arawn.toml"));

            if !is_our_file {
                continue;
            }

            // Debounce: drain events for 500ms
            let deadline = tokio::time::Instant::now() + Duration::from_millis(500);
            loop {
                match tokio::time::timeout_at(deadline, rx.recv()).await {
                    Ok(Some(Ok(_))) => continue,
                    Ok(Some(Err(_))) => continue,
                    _ => break,
                }
            }

            info!("arawn.toml changed, hot-reloading config");
            self.reload().await;
        }

        Ok(())
    }

    async fn reload(&self) {
        let config = ArawnConfig::load(&self.data_dir);

        // Reload permissions
        let new_rules = arawn_engine::permissions::load_permissions_from_file(
            &self.config_path,
        )
        .into_rules();
        {
            let mut rules = self.permission_rules.write().unwrap();
            *rules = new_rules;
        }
        info!("permission rules reloaded");

        // Reload MCP servers
        let mcp_config =
            arawn_mcp::load_mcp_config(&self.config_path);
        {
            let mut manager = self.mcp_manager.lock().await;
            manager
                .sync_servers(&mcp_config.servers, &self.tool_registry)
                .await;
        }

        info!(
            model = %config.engine_llm().model,
            max_iterations = config.engine.max_iterations,
            "config hot-reload complete"
        );

        if let Some(ref n) = self.notify {
            n(
                false,
                format!(
                    "config reloaded: model={} max_iterations={}",
                    config.engine_llm().model,
                    config.engine.max_iterations
                ),
            );
        }
    }
}
