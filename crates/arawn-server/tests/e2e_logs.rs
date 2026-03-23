//! E2E tests for the logs endpoints.
//!
//! These tests exercise GET /api/v1/logs and GET /api/v1/logs/files
//! by creating temporary log directories and setting ARAWN_CONFIG_DIR.

mod common;

use anyhow::Result;
use serial_test::serial;
use tempfile::TempDir;

use arawn_test_utils::server::TestServerBuilder;

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Create a temp dir that mimics the arawn config structure with log files,
/// set ARAWN_CONFIG_DIR, and return the TempDir handle (must be kept alive).
fn setup_log_dir(files: &[(&str, &str)]) -> TempDir {
    let tmp = TempDir::new().expect("Failed to create temp dir");
    let logs_dir = tmp.path().join("logs");
    std::fs::create_dir_all(&logs_dir).expect("Failed to create logs dir");

    for (name, content) in files {
        std::fs::write(logs_dir.join(name), content).expect("Failed to write log file");
    }

    // Safety: E2E tests run with --test-threads=1
    unsafe {
        std::env::set_var("ARAWN_CONFIG_DIR", tmp.path());
    }

    tmp
}

/// Clear the env var after a test.
fn teardown_log_dir() {
    unsafe {
        std::env::remove_var("ARAWN_CONFIG_DIR");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: List log files
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn scenario_list_log_files() -> Result<()> {
    let _tmp = setup_log_dir(&[
        ("2026-03-09.log", "line one\nline two\n"),
        ("2026-03-08.log", "old line\n"),
    ]);

    let server = TestServerBuilder::new().build().await?;

    let resp = server.get("/api/v1/logs/files").send().await?;
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await?;
    let files = body["files"].as_array().expect("files should be array");

    assert_eq!(files.len(), 2, "Should list 2 log files");

    // Files should be sorted by name descending (most recent first)
    let names: Vec<&str> = files.iter().filter_map(|f| f["name"].as_str()).collect();
    assert_eq!(names[0], "2026-03-09.log");
    assert_eq!(names[1], "2026-03-08.log");

    // Each file should have a size
    for file in files {
        assert!(file["size"].as_u64().is_some(), "File should have size");
        assert!(
            file["size"].as_u64().unwrap() > 0,
            "File size should be > 0"
        );
    }

    teardown_log_dir();
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: List log files when no logs exist
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn scenario_list_log_files_empty() -> Result<()> {
    let _tmp = setup_log_dir(&[]);

    let server = TestServerBuilder::new().build().await?;

    let resp = server.get("/api/v1/logs/files").send().await?;
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await?;
    let files = body["files"].as_array().expect("files should be array");
    assert!(files.is_empty(), "Should have no log files");

    teardown_log_dir();
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: List log files ignores non-.log files
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn scenario_list_log_files_filters_non_log() -> Result<()> {
    let _tmp = setup_log_dir(&[
        ("server.log", "log content\n"),
        ("config.toml", "not a log\n"),
        ("notes.txt", "also not a log\n"),
    ]);

    let server = TestServerBuilder::new().build().await?;

    let resp = server.get("/api/v1/logs/files").send().await?;
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await?;
    let files = body["files"].as_array().expect("files should be array");
    assert_eq!(files.len(), 1, "Should only list .log files");
    assert_eq!(files[0]["name"].as_str(), Some("server.log"));

    teardown_log_dir();
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Get latest log entries (no file param)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn scenario_get_logs_latest() -> Result<()> {
    let _tmp = setup_log_dir(&[
        (
            "2026-03-09.log",
            "today line 1\ntoday line 2\ntoday line 3\n",
        ),
        ("2026-03-08.log", "yesterday line 1\n"),
    ]);

    let server = TestServerBuilder::new().build().await?;

    let resp = server.get("/api/v1/logs").send().await?;
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await?;
    assert!(body["file"].as_str().is_some(), "Should return filename");
    assert!(body["count"].as_u64().unwrap() > 0, "Should have entries");

    let entries = body["entries"].as_array().expect("entries should be array");
    assert!(!entries.is_empty(), "Should have log entries");

    // Each entry should have a line field
    for entry in entries {
        assert!(entry["line"].as_str().is_some(), "Entry should have line");
    }

    teardown_log_dir();
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Get logs with specific file parameter
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn scenario_get_logs_specific_file() -> Result<()> {
    let _tmp = setup_log_dir(&[
        ("2026-03-09.log", "today\n"),
        ("2026-03-08.log", "yesterday line 1\nyesterday line 2\n"),
    ]);

    let server = TestServerBuilder::new().build().await?;

    // Request specific file by name (without .log extension)
    let resp = server.get("/api/v1/logs?file=2026-03-08").send().await?;
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["file"].as_str(), Some("2026-03-08.log"));
    assert_eq!(body["count"].as_u64(), Some(2));

    let entries = body["entries"].as_array().unwrap();
    assert_eq!(entries[0]["line"].as_str(), Some("yesterday line 1"));
    assert_eq!(entries[1]["line"].as_str(), Some("yesterday line 2"));

    teardown_log_dir();
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Get logs with lines limit
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn scenario_get_logs_with_lines_limit() -> Result<()> {
    // Create a file with 10 lines
    let content: String = (1..=10).map(|i| format!("line {}\n", i)).collect();
    let _tmp = setup_log_dir(&[("server.log", &content)]);

    let server = TestServerBuilder::new().build().await?;

    // Request only last 3 lines
    let resp = server.get("/api/v1/logs?lines=3").send().await?;
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["count"].as_u64(), Some(3));

    let entries = body["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 3);
    // Should be the last 3 lines (tail behavior)
    assert_eq!(entries[0]["line"].as_str(), Some("line 8"));
    assert_eq!(entries[1]["line"].as_str(), Some("line 9"));
    assert_eq!(entries[2]["line"].as_str(), Some("line 10"));

    teardown_log_dir();
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Get logs for nonexistent file returns 404
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn scenario_get_logs_file_not_found() -> Result<()> {
    let _tmp = setup_log_dir(&[("server.log", "content\n")]);

    let server = TestServerBuilder::new().build().await?;

    let resp = server.get("/api/v1/logs?file=nonexistent").send().await?;
    assert_eq!(
        resp.status().as_u16(),
        404,
        "Should return 404 for nonexistent log file"
    );

    teardown_log_dir();
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Get logs when no log files exist returns 404
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn scenario_get_logs_no_files_returns_404() -> Result<()> {
    let _tmp = setup_log_dir(&[]);

    let server = TestServerBuilder::new().build().await?;

    let resp = server.get("/api/v1/logs").send().await?;
    assert_eq!(
        resp.status().as_u16(),
        404,
        "Should return 404 when no log files exist"
    );

    teardown_log_dir();
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Get logs when log directory doesn't exist returns 404
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn scenario_get_logs_no_directory_returns_404() -> Result<()> {
    // Set ARAWN_CONFIG_DIR to a path that exists but has no logs subdir
    let tmp = TempDir::new()?;
    unsafe {
        std::env::set_var("ARAWN_CONFIG_DIR", tmp.path());
    }
    // Do NOT create the logs subdirectory

    let server = TestServerBuilder::new().build().await?;

    let resp = server.get("/api/v1/logs").send().await?;
    assert_eq!(
        resp.status().as_u16(),
        404,
        "Should return 404 when log directory doesn't exist"
    );

    teardown_log_dir();
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: List log files when directory doesn't exist returns empty
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn scenario_list_log_files_no_directory_returns_empty() -> Result<()> {
    let tmp = TempDir::new()?;
    unsafe {
        std::env::set_var("ARAWN_CONFIG_DIR", tmp.path());
    }

    let server = TestServerBuilder::new().build().await?;

    let resp = server.get("/api/v1/logs/files").send().await?;
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await?;
    let files = body["files"].as_array().unwrap();
    assert!(
        files.is_empty(),
        "Should return empty list when no logs dir"
    );

    teardown_log_dir();
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Lines limit is capped at 1000
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn scenario_get_logs_lines_capped_at_1000() -> Result<()> {
    // Create a file with 1500 lines
    let content: String = (1..=1500).map(|i| format!("line {}\n", i)).collect();
    let _tmp = setup_log_dir(&[("big.log", &content)]);

    let server = TestServerBuilder::new().build().await?;

    // Request 9999 lines — should be capped to 1000
    let resp = server
        .get("/api/v1/logs?lines=9999&file=big")
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(
        body["count"].as_u64(),
        Some(1000),
        "Lines should be capped at 1000"
    );

    teardown_log_dir();
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Get logs with file parameter using full filename
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
#[serial]
async fn scenario_get_logs_full_filename() -> Result<()> {
    let _tmp = setup_log_dir(&[("server.log", "full name test\n")]);

    let server = TestServerBuilder::new().build().await?;

    // Use the full filename with extension
    let resp = server.get("/api/v1/logs?file=server.log").send().await?;
    assert_eq!(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["file"].as_str(), Some("server.log"));
    assert_eq!(body["count"].as_u64(), Some(1));

    teardown_log_dir();
    Ok(())
}
