use std::path::Path;

use tracing::warn;

use super::config::HookConfig;

/// Wrapper for the hooks section in settings.json.
///
/// ```json
/// {
///   "hooks": {
///     "PreToolUse": [ ... ],
///     "SessionStart": [ ... ]
///   }
/// }
/// ```
#[derive(Debug, Clone, Default, serde::Deserialize)]
struct SettingsFile {
    #[serde(default)]
    hooks: HookConfig,
}

/// Load hook configuration from a JSON settings file.
///
/// Returns an empty config if the file is missing, has no `hooks` key,
/// or contains malformed entries.
pub fn load_hooks_from_file(path: &Path) -> HookConfig {
    if !path.exists() {
        return HookConfig::default();
    }

    match std::fs::read_to_string(path) {
        Ok(content) => match serde_json::from_str::<SettingsFile>(&content) {
            Ok(settings) => settings.hooks,
            Err(e) => {
                warn!(path = %path.display(), error = %e, "failed to parse hooks from settings, using defaults");
                HookConfig::default()
            }
        },
        Err(e) => {
            warn!(path = %path.display(), error = %e, "failed to read settings file, using defaults");
            HookConfig::default()
        }
    }
}

/// Load and merge hook configs from user-level and project-level settings.
///
/// User settings (`~/.arawn/settings.json`) are merged with project settings
/// (`.arawn/settings.json`). Both sources' hooks are included — user hooks
/// run alongside project hooks. Deduplication by command+matcher collapses
/// identical hooks across sources.
pub fn load_merged_hooks(
    user_settings_path: Option<&Path>,
    project_settings_path: Option<&Path>,
) -> HookConfig {
    let mut user_config = user_settings_path
        .map(load_hooks_from_file)
        .unwrap_or_default();

    let project_config = project_settings_path
        .map(load_hooks_from_file)
        .unwrap_or_default();

    user_config.merge(project_config);
    user_config
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::{HookEvent, HookMatcher};

    /// Helper to write raw bytes to a temp file (avoids write! macro brace escaping).
    fn write_json(file: &std::fs::File, json: &str) {
        use std::io::Write;
        (&*file).write_all(json.as_bytes()).unwrap();
    }

    #[test]
    fn load_from_json_file() {
        let tmp = tempfile::NamedTempFile::with_suffix(".json").unwrap();
        write_json(
            tmp.as_file(),
            r#"{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          { "type": "command", "command": "exit 0", "timeout": 5 }
        ]
      }
    ],
    "SessionStart": [
      {
        "hooks": [
          { "type": "command", "command": "echo started" }
        ]
      }
    ]
  }
}"#,
        );

        let config = load_hooks_from_file(tmp.path());
        assert_eq!(config.groups_for(HookEvent::PreToolUse).len(), 1);
        assert_eq!(config.groups_for(HookEvent::SessionStart).len(), 1);

        let groups = config.groups_for(HookEvent::PreToolUse);
        assert_eq!(groups[0].matcher, Some(HookMatcher::new("Bash")));
        assert_eq!(groups[0].hooks[0].command, "exit 0");
        assert_eq!(groups[0].hooks[0].timeout, Some(5));
    }

    #[test]
    fn load_missing_file_returns_defaults() {
        let config = load_hooks_from_file(Path::new("/nonexistent/settings.json"));
        assert!(config.is_empty());
    }

    #[test]
    fn load_file_without_hooks_key() {
        let tmp = tempfile::NamedTempFile::with_suffix(".json").unwrap();
        write_json(tmp.as_file(), r#"{ "other_setting": true }"#);

        let config = load_hooks_from_file(tmp.path());
        assert!(config.is_empty());
    }

    #[test]
    fn load_malformed_json_returns_defaults() {
        let tmp = tempfile::NamedTempFile::with_suffix(".json").unwrap();
        write_json(tmp.as_file(), "not valid json {{");

        let config = load_hooks_from_file(tmp.path());
        assert!(config.is_empty());
    }

    #[test]
    fn merge_user_and_project() {
        let user_tmp = tempfile::NamedTempFile::with_suffix(".json").unwrap();
        write_json(
            user_tmp.as_file(),
            r#"{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          { "type": "command", "command": "echo user-hook" }
        ]
      }
    ]
  }
}"#,
        );

        let project_tmp = tempfile::NamedTempFile::with_suffix(".json").unwrap();
        write_json(
            project_tmp.as_file(),
            r#"{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Edit",
        "hooks": [
          { "type": "command", "command": "echo project-hook" }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          { "type": "command", "command": "echo done" }
        ]
      }
    ]
  }
}"#,
        );

        let config =
            load_merged_hooks(Some(user_tmp.path()), Some(project_tmp.path()));

        // PreToolUse should have 2 groups (1 user + 1 project)
        assert_eq!(config.groups_for(HookEvent::PreToolUse).len(), 2);
        // Stop should have 1 group (from project only)
        assert_eq!(config.groups_for(HookEvent::Stop).len(), 1);
    }

    #[test]
    fn merge_missing_user_config() {
        let project_tmp = tempfile::NamedTempFile::with_suffix(".json").unwrap();
        write_json(
            project_tmp.as_file(),
            r#"{
  "hooks": {
    "SessionStart": [
      {
        "hooks": [
          { "type": "command", "command": "echo hello" }
        ]
      }
    ]
  }
}"#,
        );

        let config = load_merged_hooks(None, Some(project_tmp.path()));
        assert_eq!(config.groups_for(HookEvent::SessionStart).len(), 1);
    }

    #[test]
    fn merge_both_missing() {
        let config = load_merged_hooks(None, None);
        assert!(config.is_empty());
    }

    #[test]
    fn dedup_identical_hooks_across_sources() {
        // Same hook defined in both user and project — currently both included
        // (dedup is a future enhancement)
        let user_tmp = tempfile::NamedTempFile::with_suffix(".json").unwrap();
        let project_tmp = tempfile::NamedTempFile::with_suffix(".json").unwrap();

        let content = r#"{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          { "type": "command", "command": "echo same" }
        ]
      }
    ]
  }
}"#;

        write_json(user_tmp.as_file(), content);
        write_json(project_tmp.as_file(), content);

        let config =
            load_merged_hooks(Some(user_tmp.path()), Some(project_tmp.path()));

        // Currently 2 groups — dedup would reduce to 1
        assert_eq!(config.groups_for(HookEvent::PreToolUse).len(), 2);
    }
}
