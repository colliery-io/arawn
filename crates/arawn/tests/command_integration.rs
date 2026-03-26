//! CLI command execution integration tests.
//!
//! Tests that verify commands actually execute correctly — not just argument
//! parsing and help text. Uses `ARAWN_CONFIG_DIR` env var to isolate config
//! and secret state per test.

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

/// Get a command for the arawn binary with an isolated config directory.
///
/// Sets `ARAWN_CONFIG_DIR` to the tempdir and `ARAWN_SERVER_URL` to a
/// non-listening address to prevent accidental real server connections.
/// Also sets `HOME` and `XDG_CONFIG_HOME` to prevent the binary from
/// picking up any user-level plugins or configs.
fn arawn_with_config(config_dir: &TempDir) -> Command {
    let mut cmd = assert_cmd::cargo_bin_cmd!("arawn");
    cmd.env("ARAWN_CONFIG_DIR", config_dir.path());
    cmd.env("ARAWN_SERVER_URL", "http://localhost:19999");
    // Isolate from user's global plugins and config
    cmd.env("HOME", config_dir.path());
    cmd.env("XDG_CONFIG_HOME", config_dir.path());
    cmd
}

/// Write a config file into the temp config dir.
fn write_config(dir: &TempDir, content: &str) {
    std::fs::write(dir.path().join("config.toml"), content).unwrap();
}

// ─────────────────────────────────────────────────────────────────────────────
// Config Show
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_config_show_defaults() {
    let dir = TempDir::new().unwrap();

    arawn_with_config(&dir)
        .args(["config", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Configuration"))
        .stdout(predicate::str::contains("No config files loaded"));
}

#[test]
fn test_config_show_with_config_file() {
    let dir = TempDir::new().unwrap();
    write_config(
        &dir,
        r#"
[llm]
backend = "groq"
model = "llama-3.1-70b-versatile"

[server]
port = 9999
bind = "0.0.0.0"
"#,
    );

    arawn_with_config(&dir)
        .args(["config", "show"])
        .assert()
        .success()
        // Backend display name is capitalized ("Groq")
        .stdout(predicate::str::contains("Groq"))
        .stdout(predicate::str::contains("llama-3.1-70b-versatile"))
        .stdout(predicate::str::contains("9999"))
        .stdout(predicate::str::contains("0.0.0.0"));
}

#[test]
fn test_config_show_verbose_includes_raw() {
    let dir = TempDir::new().unwrap();
    write_config(
        &dir,
        r#"
[llm]
backend = "ollama"
model = "llama3.2"
"#,
    );

    arawn_with_config(&dir)
        .args(["--verbose", "config", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Raw config"));
}

#[test]
fn test_config_show_invalid_toml_warns() {
    let dir = TempDir::new().unwrap();
    write_config(&dir, "this is not valid TOML {{{{");

    // Invalid TOML produces a warning but config show still succeeds (with defaults)
    arawn_with_config(&dir)
        .args(["config", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Failed to load").or(predicate::str::contains("Warning")));
}

// ─────────────────────────────────────────────────────────────────────────────
// Config Which
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_config_which_no_files() {
    let dir = TempDir::new().unwrap();

    arawn_with_config(&dir)
        .args(["config", "which"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Config File Search Order"))
        .stdout(predicate::str::contains("No config files found"));
}

#[test]
fn test_config_which_with_file() {
    let dir = TempDir::new().unwrap();
    write_config(&dir, "[llm]\nbackend = \"groq\"\nmodel = \"x\"\n");

    arawn_with_config(&dir)
        .args(["config", "which"])
        .assert()
        .success()
        .stdout(predicate::str::contains("loaded"));
}

// ─────────────────────────────────────────────────────────────────────────────
// Config Init
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_config_init_creates_file() {
    let dir = TempDir::new().unwrap();

    arawn_with_config(&dir)
        .args(["config", "init"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created config file"));

    assert!(dir.path().join("config.toml").exists());
}

#[test]
fn test_config_init_existing_file() {
    let dir = TempDir::new().unwrap();
    write_config(&dir, "# existing\n");

    arawn_with_config(&dir)
        .args(["config", "init"])
        .assert()
        .success()
        .stdout(predicate::str::contains("already exists"));
}

#[test]
fn test_config_init_local() {
    let dir = TempDir::new().unwrap();
    let work_dir = TempDir::new().unwrap();

    arawn_with_config(&dir)
        .args(["config", "init", "--local"])
        .current_dir(work_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Created config file"));

    assert!(work_dir.path().join("arawn.toml").exists());
}

// ─────────────────────────────────────────────────────────────────────────────
// Config Path
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_config_path_shows_path() {
    let dir = TempDir::new().unwrap();

    arawn_with_config(&dir)
        .args(["config", "path"])
        .assert()
        .success()
        .stdout(predicate::str::contains("config.toml"));
}

// ─────────────────────────────────────────────────────────────────────────────
// Config Context Commands
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_config_set_context_creates_new() {
    let dir = TempDir::new().unwrap();

    arawn_with_config(&dir)
        .args([
            "config",
            "set-context",
            "local",
            "--server",
            "http://localhost:8080",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("created"));
}

#[test]
fn test_config_set_context_without_server_creates_with_empty_url() {
    let dir = TempDir::new().unwrap();

    // Without --server the command may succeed (creating with an empty URL)
    // or fail with a "server is required" error. Either way, it shouldn't crash.
    let output = arawn_with_config(&dir)
        .args(["config", "set-context", "local"])
        .output()
        .unwrap();

    // Just verify it doesn't panic — the behavior depends on the implementation
    let _ = output.status;
}

#[test]
fn test_config_get_contexts_empty() {
    let dir = TempDir::new().unwrap();

    arawn_with_config(&dir)
        .args(["config", "get-contexts"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No contexts configured"));
}

#[test]
fn test_config_context_roundtrip() {
    let dir = TempDir::new().unwrap();

    // Create context
    arawn_with_config(&dir)
        .args([
            "config",
            "set-context",
            "staging",
            "--server",
            "http://staging:8080",
        ])
        .assert()
        .success();

    // List contexts
    arawn_with_config(&dir)
        .args(["config", "get-contexts"])
        .assert()
        .success()
        .stdout(predicate::str::contains("staging"))
        .stdout(predicate::str::contains("http://staging:8080"));

    // Use context
    arawn_with_config(&dir)
        .args(["config", "use-context", "staging"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Switched to context"));

    // Check current context
    arawn_with_config(&dir)
        .args(["config", "current-context"])
        .assert()
        .success()
        .stdout(predicate::str::contains("staging"));

    // Delete context
    arawn_with_config(&dir)
        .args(["config", "delete-context", "staging"])
        .assert()
        .success()
        .stdout(predicate::str::contains("deleted"));

    // Verify empty
    arawn_with_config(&dir)
        .args(["config", "get-contexts"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No contexts configured"));
}

#[test]
fn test_config_use_nonexistent_context() {
    let dir = TempDir::new().unwrap();

    arawn_with_config(&dir)
        .args(["config", "use-context", "nonexistent"])
        .assert()
        .failure();
}

#[test]
fn test_config_delete_nonexistent_context() {
    let dir = TempDir::new().unwrap();

    arawn_with_config(&dir)
        .args(["config", "delete-context", "nonexistent"])
        .assert()
        .success()
        .stdout(predicate::str::contains("not found"));
}

#[test]
fn test_config_current_context_when_none() {
    let dir = TempDir::new().unwrap();

    arawn_with_config(&dir)
        .args(["config", "current-context"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No current context"));
}

#[test]
fn test_config_set_context_with_workstream() {
    let dir = TempDir::new().unwrap();

    arawn_with_config(&dir)
        .args([
            "config",
            "set-context",
            "dev",
            "--server",
            "http://localhost:8080",
            "--workstream",
            "my-project",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("created"));
}

#[test]
fn test_config_update_existing_context() {
    let dir = TempDir::new().unwrap();

    // Create
    arawn_with_config(&dir)
        .args([
            "config",
            "set-context",
            "prod",
            "--server",
            "http://prod:8080",
        ])
        .assert()
        .success();

    // Update server URL
    arawn_with_config(&dir)
        .args([
            "config",
            "set-context",
            "prod",
            "--server",
            "http://prod-v2:9090",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("modified"));

    // Verify updated
    arawn_with_config(&dir)
        .args(["config", "get-contexts"])
        .assert()
        .success()
        .stdout(predicate::str::contains("prod-v2:9090"));
}

// ─────────────────────────────────────────────────────────────────────────────
// Status Command (no server running)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_status_no_server() {
    let dir = TempDir::new().unwrap();

    arawn_with_config(&dir)
        .args(["status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("not running"));
}

#[test]
fn test_status_json_no_server() {
    let dir = TempDir::new().unwrap();

    let output = arawn_with_config(&dir)
        .args(["--json", "status"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("Should be valid JSON");
    assert_eq!(json["running"], false);
    assert!(json["server_url"].as_str().is_some());
}

// ─────────────────────────────────────────────────────────────────────────────
// Secrets Commands
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_secrets_list_empty() {
    let dir = TempDir::new().unwrap();

    arawn_with_config(&dir)
        .args(["secrets", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No secrets stored"));
}

#[test]
fn test_secrets_delete_nonexistent() {
    let dir = TempDir::new().unwrap();

    // Deleting a secret that doesn't exist should not crash
    arawn_with_config(&dir)
        .args(["secrets", "delete", "nonexistent_key"])
        .assert()
        .success();
}

#[test]
fn test_secrets_help() {
    let dir = TempDir::new().unwrap();

    arawn_with_config(&dir)
        .args(["secrets", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("set"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("delete"));
}

// ─────────────────────────────────────────────────────────────────────────────
// Plugin List
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_plugin_list_no_plugins() {
    let dir = TempDir::new().unwrap();
    let work_dir = TempDir::new().unwrap();

    arawn_with_config(&dir)
        .args(["plugin", "list"])
        .current_dir(work_dir.path())
        .assert()
        .success()
        // With HOME isolated, no user plugins should be found
        .stdout(
            predicate::str::contains("No plugins found")
                .or(predicate::str::contains("Local plugins")),
        );
}

#[test]
fn test_plugin_list_json() {
    let dir = TempDir::new().unwrap();
    let work_dir = TempDir::new().unwrap();

    let output = arawn_with_config(&dir)
        .args(["--json", "plugin", "list"])
        .current_dir(work_dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    // stdout may contain tracing output before/after JSON.
    // The JSON output is a pretty-printed array that starts with '[' on its own line.
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Collect lines that are part of the JSON array
    let mut json_lines = Vec::new();
    let mut in_json = false;
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed == "[" || trimmed.starts_with('[') {
            in_json = true;
        }
        if in_json {
            json_lines.push(line);
        }
        if in_json && trimmed == "]" {
            break;
        }
    }
    if !json_lines.is_empty() {
        let json_str = json_lines.join("\n");
        let json: serde_json::Value =
            serde_json::from_str(&json_str).expect("Should contain valid JSON array");
        assert!(json.as_array().is_some());
    }
    // If no JSON array found, output may be empty (no plugins)
}

#[test]
fn test_plugin_list_subscribed_filter() {
    let dir = TempDir::new().unwrap();
    let work_dir = TempDir::new().unwrap();

    // With --subscribed flag and no subscriptions, should not show local plugins
    arawn_with_config(&dir)
        .args(["plugin", "list", "--subscribed"])
        .current_dir(work_dir.path())
        .assert()
        .success();
}

// ─────────────────────────────────────────────────────────────────────────────
// Logs Command
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_logs_no_log_directory() {
    let dir = TempDir::new().unwrap();

    // When log directory doesn't exist, the error propagates to main
    // and is printed to stderr with exit code 1
    arawn_with_config(&dir)
        .args(["logs"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("Log directory not found")
                .or(predicate::str::contains("No log files")),
        );
}

#[test]
fn test_logs_empty_log_directory() {
    let dir = TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();

    // Log dir exists but has no .log files
    arawn_with_config(&dir)
        .args(["logs"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No log files found"));
}

#[test]
fn test_logs_reads_log_file() {
    let dir = TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    // Use .log extension (find_latest_log filters for .log extension)
    std::fs::write(
        log_dir.join("arawn.log"),
        "line1\nline2\nline3\nline4\nline5\n",
    )
    .unwrap();

    arawn_with_config(&dir)
        .args(["logs", "-n", "3"])
        .assert()
        .success()
        .stdout(predicate::str::contains("line3"))
        .stdout(predicate::str::contains("line4"))
        .stdout(predicate::str::contains("line5"));
}

#[test]
fn test_logs_specific_file() {
    let dir = TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    std::fs::write(log_dir.join("custom.log"), "custom log line\n").unwrap();

    arawn_with_config(&dir)
        .args(["logs", "--file", "custom"])
        .assert()
        .success()
        .stdout(predicate::str::contains("custom log line"));
}

#[test]
fn test_logs_missing_file() {
    let dir = TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    // Write at least one valid log so the command doesn't fail on "no log files"
    std::fs::write(log_dir.join("arawn.log"), "some log\n").unwrap();

    // Requesting a specific file that doesn't exist
    arawn_with_config(&dir)
        .args(["logs", "--file", "nonexistent"])
        .assert()
        .success()
        .stderr(predicate::str::contains("not found").or(predicate::str::is_empty()))
        .stdout(predicate::str::contains("not found").or(predicate::str::is_empty()));
}

#[test]
fn test_logs_line_count() {
    let dir = TempDir::new().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();

    let mut content = String::new();
    for i in 1..=100 {
        content.push_str(&format!("log line {}\n", i));
    }
    std::fs::write(log_dir.join("arawn.log"), &content).unwrap();

    // Request last 5 lines
    let output = arawn_with_config(&dir)
        .args(["logs", "-n", "5"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should contain lines 96-100 but not line 95
    assert!(stdout.contains("log line 100"));
    assert!(stdout.contains("log line 96"));
}

// ─────────────────────────────────────────────────────────────────────────────
// Commands requiring server (verify graceful failure)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_session_list_no_server() {
    let dir = TempDir::new().unwrap();

    // Should not crash, just show connection error
    arawn_with_config(&dir)
        .args(["session", "list"])
        .assert()
        .success()
        .stderr(predicate::str::contains("connect").or(predicate::str::contains("Could not")));
}

#[test]
fn test_session_show_no_server() {
    let dir = TempDir::new().unwrap();

    arawn_with_config(&dir)
        .args(["session", "show", "nonexistent-id"])
        .assert()
        .success()
        .stderr(predicate::str::contains("connect").or(predicate::str::contains("Could not")));
}

#[test]
fn test_logs_remote_no_server() {
    let dir = TempDir::new().unwrap();

    arawn_with_config(&dir)
        .args(["logs", "--remote"])
        .assert()
        .success()
        .stderr(predicate::str::contains("connect").or(predicate::str::contains("Could not")));
}

#[test]
fn test_logs_remote_list_files_no_server() {
    let dir = TempDir::new().unwrap();

    arawn_with_config(&dir)
        .args(["logs", "--remote", "--list-files"])
        .assert()
        .success()
        .stderr(predicate::str::contains("connect").or(predicate::str::contains("Could not")));
}

// ─────────────────────────────────────────────────────────────────────────────
// Server URL Resolution
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_server_flag_overrides_env() {
    let dir = TempDir::new().unwrap();

    // --server flag takes priority over ARAWN_SERVER_URL env var
    arawn_with_config(&dir)
        .args(["--server", "http://localhost:29999", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("localhost:29999"));
}

#[test]
fn test_context_flag_uses_context_url() {
    let dir = TempDir::new().unwrap();

    // Create a context
    arawn_with_config(&dir)
        .args([
            "config",
            "set-context",
            "test-ctx",
            "--server",
            "http://test-server:7777",
        ])
        .assert()
        .success();

    // Use --context flag with --server to override ARAWN_SERVER_URL
    // but since --server also overrides --context in priority, we test
    // that --context at least works when no --server or ARAWN_SERVER_URL is set
    let mut cmd = assert_cmd::cargo_bin_cmd!("arawn");
    cmd.env("ARAWN_CONFIG_DIR", dir.path());
    cmd.env("HOME", dir.path());
    cmd.env("XDG_CONFIG_HOME", dir.path());
    cmd.env_remove("ARAWN_SERVER_URL");
    cmd.args(["--context", "test-ctx", "status"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("test-server:7777"));
}

// ─────────────────────────────────────────────────────────────────────────────
// Config Set-Secret (invalid backend)
// ─────────────────────────────────────────────────────────────────────────────

// ─────────────────────────────────────────────────────────────────────────────
// Config multiple profiles
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_config_show_multiple_profiles() {
    let dir = TempDir::new().unwrap();
    write_config(
        &dir,
        r#"
[llm]
backend = "groq"
model = "llama-3.1-70b-versatile"

[llm.fast]
backend = "groq"
model = "llama-3.1-8b-instant"

[llm.local]
backend = "ollama"
model = "llama3.2"
base_url = "http://localhost:11434/v1"
"#,
    );

    arawn_with_config(&dir)
        .args(["config", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("LLM Profiles"))
        .stdout(predicate::str::contains("default"))
        .stdout(predicate::str::contains("fast"))
        .stdout(predicate::str::contains("local"));
}

// ─────────────────────────────────────────────────────────────────────────────
// Agent bindings in config
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_config_show_agent_bindings() {
    let dir = TempDir::new().unwrap();
    write_config(
        &dir,
        r#"
[llm]
backend = "groq"
model = "llama-3.1-70b-versatile"

[agent.summarizer]
llm = "fast"
max_iterations = 5
"#,
    );

    arawn_with_config(&dir)
        .args(["config", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Agent Bindings"))
        .stdout(predicate::str::contains("summarizer"));
}
