use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use super::events::HookInput;
use super::runner::HookRunner;

/// Watches file paths and fires `FileChanged` hooks when changes are detected.
///
/// Intended for project files that hooks care about (settings, config, etc.).
/// Uses debouncing to avoid firing on rapid successive changes.
pub struct HookFileWatcher {
    paths: Vec<PathBuf>,
    hook_runner: Arc<HookRunner>,
}

impl HookFileWatcher {
    pub fn new(paths: Vec<PathBuf>, hook_runner: Arc<HookRunner>) -> Self {
        Self { paths, hook_runner }
    }

    /// Spawn the file watcher as a background tokio task.
    pub fn spawn(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            if let Err(e) = self.run().await {
                warn!(error = %e, "hook file watcher exited with error");
            }
        })
    }

    async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (tx, mut rx) = mpsc::channel::<notify::Result<Event>>(64);

        let mut watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.blocking_send(res);
            },
            notify::Config::default().with_poll_interval(Duration::from_secs(2)),
        )?;

        for path in &self.paths {
            if path.exists() {
                let mode = if path.is_dir() {
                    RecursiveMode::Recursive
                } else {
                    RecursiveMode::NonRecursive
                };
                // For files, watch the parent directory
                let watch_path = if path.is_file() {
                    path.parent().unwrap_or(path)
                } else {
                    path
                };
                if let Err(e) = watcher.watch(watch_path, mode) {
                    warn!(path = ?watch_path, error = %e, "failed to watch path");
                } else {
                    debug!(path = ?watch_path, "watching for changes");
                }
            } else {
                debug!(path = ?path, "skipping non-existent watch path");
            }
        }

        info!(paths = self.paths.len(), "hook file watcher started");

        loop {
            let event = match rx.recv().await {
                Some(Ok(event)) => event,
                Some(Err(e)) => {
                    warn!(error = %e, "file watch error");
                    continue;
                }
                None => break,
            };

            if !matches!(
                event.kind,
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
            ) {
                continue;
            }

            // Debounce: drain events for 300ms
            let deadline = tokio::time::Instant::now() + Duration::from_millis(300);
            let mut all_paths: Vec<PathBuf> = event.paths.clone();
            loop {
                match tokio::time::timeout_at(deadline, rx.recv()).await {
                    Ok(Some(Ok(evt))) => {
                        all_paths.extend(evt.paths);
                    }
                    Ok(Some(Err(_))) => continue,
                    _ => break,
                }
            }

            // Filter to only paths we're actually watching
            let relevant_paths: Vec<&PathBuf> = all_paths
                .iter()
                .filter(|p| {
                    self.paths.iter().any(|watch| {
                        p.starts_with(watch) || p.as_path() == watch.as_path()
                    })
                })
                .collect();

            if relevant_paths.is_empty() {
                continue;
            }

            // Fire FileChanged hook for each unique changed path
            let mut seen = std::collections::HashSet::new();
            for path in relevant_paths {
                if !seen.insert(path) {
                    continue;
                }

                let change_type = match event.kind {
                    EventKind::Create(_) => "created",
                    EventKind::Remove(_) => "removed",
                    _ => "modified",
                };

                debug!(path = ?path, change_type, "firing FileChanged hook");

                let hook_input = HookInput::FileChanged {
                    file_path: path.to_string_lossy().to_string(),
                    change_type: change_type.to_string(),
                };
                let _ = self.hook_runner.run(&hook_input).await;
            }
        }

        Ok(())
    }
}
