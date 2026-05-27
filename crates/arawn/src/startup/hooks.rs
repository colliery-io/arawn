//! Hook config startup loader (I-0056 T-A).
//!
//! Reads `~/.arawn/settings.json` (user level, sourced from `data_dir`)
//! and merges with `<lens_root>/.arawn/settings.json` (project
//! level). Constructs a [`HookRunner`] with `cwd = lens_root` so
//! hook subprocesses see project files relative to their own repo.
//!
//! This task only handles **wire-up**. The actual `.fire_hook(...)`
//! call sites live in T-B/C/D (ARAWN-T-0413/0414/0415).

use std::path::Path;
use std::sync::Arc;

use arawn_engine::hooks::{HookRunner, load_merged_hooks};
use tracing::info;

/// Load merged hook config and build a [`HookRunner`] with the
/// lens root as the subprocess cwd. Returns `Arc<HookRunner>`
/// so the runner can be shared between `LocalService` and every
/// `QueryEngine` instance the service spawns.
///
/// Missing settings files are silently treated as "no hooks" — the
/// returned runner has an empty config and is a no-op until events
/// fire against it.
pub fn load_and_build_hook_runner(data_dir: &Path, lens_root: &Path) -> Arc<HookRunner> {
    let user_path = data_dir.join("settings.json");
    let project_path = lens_root.join(".arawn").join("settings.json");

    let user_present = user_path.exists();
    let project_present = project_path.exists();

    let config = load_merged_hooks(Some(user_path.as_path()), Some(project_path.as_path()));

    let total: usize = config.events.values().map(|groups| groups.len()).sum();
    info!(
        user_settings_present = user_present,
        project_settings_present = project_present,
        total_hook_groups = total,
        cwd = %lens_root.display(),
        "hooks loaded"
    );

    Arc::new(HookRunner::new(config, lens_root.to_path_buf()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_settings(path: &Path, body: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, body).unwrap();
    }

    /// A minimal valid settings.json with one PreToolUse hook.
    fn minimal_settings(label: &str) -> String {
        format!(
            r#"{{
              "hooks": {{
                "PreToolUse": [
                  {{
                    "matcher": "{label}",
                    "hooks": [
                      {{
                        "type": "command",
                        "command": "echo {label}"
                      }}
                    ]
                  }}
                ]
              }}
            }}"#
        )
    }

    #[test]
    fn returns_empty_runner_when_no_settings_files_exist() {
        let data = TempDir::new().unwrap();
        let ws = TempDir::new().unwrap();
        let runner = load_and_build_hook_runner(data.path(), ws.path());
        // The runner is constructed but its config carries no hooks.
        // We can't introspect HookRunner directly, but we can assert
        // the call doesn't panic and returns a usable Arc.
        let _ = Arc::clone(&runner);
    }

    #[test]
    fn loads_user_only_settings() {
        let data = TempDir::new().unwrap();
        let ws = TempDir::new().unwrap();
        write_settings(
            &data.path().join("settings.json"),
            &minimal_settings("user"),
        );
        let runner = load_and_build_hook_runner(data.path(), ws.path());
        let _ = Arc::clone(&runner);
    }

    #[test]
    fn loads_project_only_settings() {
        let data = TempDir::new().unwrap();
        let ws = TempDir::new().unwrap();
        write_settings(
            &ws.path().join(".arawn").join("settings.json"),
            &minimal_settings("project"),
        );
        let runner = load_and_build_hook_runner(data.path(), ws.path());
        let _ = Arc::clone(&runner);
    }

    #[test]
    fn merges_user_and_project_settings() {
        let data = TempDir::new().unwrap();
        let ws = TempDir::new().unwrap();
        write_settings(
            &data.path().join("settings.json"),
            &minimal_settings("user"),
        );
        write_settings(
            &ws.path().join(".arawn").join("settings.json"),
            &minimal_settings("project"),
        );
        let runner = load_and_build_hook_runner(data.path(), ws.path());
        let _ = Arc::clone(&runner);
    }
}
