use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::plugin_loader::PluginLoader;
use crate::tool::ToolRegistry;

/// Watches the plugin tools directory for `.arawn_tool` file changes
/// and hot-reloads them into the tool registry.
pub struct PluginWatcher {
    tools_dir: PathBuf,
    build_dir: PathBuf,
    registry: Arc<ToolRegistry>,
}

impl PluginWatcher {
    pub fn new(tools_dir: PathBuf, build_dir: PathBuf, registry: Arc<ToolRegistry>) -> Self {
        Self {
            tools_dir,
            build_dir,
            registry,
        }
    }

    /// Spawn the file watcher as a background tokio task.
    /// Returns a `JoinHandle` — dropping it cancels the watcher.
    pub fn spawn(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            if let Err(e) = self.run().await {
                warn!(error = %e, "plugin watcher exited with error");
            }
        })
    }

    async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Create directory if it doesn't exist
        std::fs::create_dir_all(&self.tools_dir)?;

        let (tx, mut rx) = mpsc::channel::<notify::Result<Event>>(64);

        let mut watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.blocking_send(res);
            },
            notify::Config::default().with_poll_interval(Duration::from_secs(2)),
        )?;

        watcher.watch(&self.tools_dir, RecursiveMode::NonRecursive)?;
        info!(dir = ?self.tools_dir, "plugin watcher started");

        // Debounce: wait a bit after events stop before reloading
        loop {
            // Wait for the next event
            let event = match rx.recv().await {
                Some(Ok(event)) => event,
                Some(Err(e)) => {
                    warn!(error = %e, "file watch error");
                    continue;
                }
                None => break, // channel closed
            };

            if !Self::is_plugin_event(&event) {
                continue;
            }

            // Debounce: drain any remaining events for 500ms
            let deadline = tokio::time::Instant::now() + Duration::from_millis(500);
            loop {
                match tokio::time::timeout_at(deadline, rx.recv()).await {
                    Ok(Some(Ok(_))) => continue, // drain
                    Ok(Some(Err(_))) => continue,
                    _ => break,
                }
            }

            info!("plugin change detected, reloading tools");
            self.reload_plugins();
        }

        Ok(())
    }

    fn is_plugin_event(event: &Event) -> bool {
        matches!(
            event.kind,
            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
        ) && event
            .paths
            .iter()
            .any(|p| p.extension().is_some_and(|ext| ext == "arawn_tool"))
    }

    fn reload_plugins(&self) {
        // Snapshot current plugin tool names (non-builtin tools are from plugins)
        // We identify plugin tools by re-scanning archives and comparing.
        let new_tools = PluginLoader::load_tools(&self.tools_dir, &self.build_dir);

        // Collect names of newly loaded tools
        let new_names: Vec<String> = new_tools.iter().map(|t| t.name().to_string()).collect();

        // Unregister old plugin tools that are no longer present.
        // We track which tools came from plugins by keeping the known set.
        // For now: unregister tools with names matching old plugin loads,
        // then re-register the fresh set.
        let current_names = self.registry.plugin_tool_names();
        for name in &current_names {
            self.registry.unregister(name);
            info!(tool = %name, "unregistered stale plugin tool");
        }

        // Register fresh tools
        for tool in new_tools {
            let name = tool.name().to_string();
            self.registry.register_plugin(tool);
            info!(tool = %name, "registered plugin tool");
        }

        info!(
            plugins = new_names.len(),
            total = self.registry.len(),
            "plugin reload complete"
        );
    }
}
