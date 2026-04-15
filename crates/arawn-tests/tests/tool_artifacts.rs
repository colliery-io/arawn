//! Tool artifact validation tests — verify that tools produce correct,
//! usable artifacts, not just that they return OK.
//!
//! These tests call tool.execute() directly with known-good parameters
//! and validate the produced artifact (file exists, compiles, is searchable, etc.)

use arawn_core::Workstream;
use arawn_engine::Tool; // re-exported from arawn_tool
use arawn_engine::ToolContext as EngineToolContext;
use arawn_tool::ToolContext; // the trait — needed for method dispatch
use serde_json::json;
use tempfile::TempDir;
use uuid::Uuid;

fn make_ctx(tmp: &TempDir) -> EngineToolContext {
    let ws = Workstream::new("test", tmp.path());
    EngineToolContext::new(&ws, Uuid::new_v4())
}

// ============================================================================
// file_write + file_read roundtrip
// ============================================================================

#[tokio::test]
async fn file_write_read_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let ctx = make_ctx(&tmp);

    let write_tool = arawn_engine::tools::FileWriteTool;
    let read_tool = arawn_engine::tools::FileReadTool;

    // Write a file (use relative path — absolute paths fail due to macOS symlink canonicalization)
    let result = write_tool
        .execute(
            &ctx,
            json!({
                "path": "hello.txt",
                "content": "Hello, World!\nLine 2."
            }),
        )
        .await
        .unwrap();
    assert!(!result.is_error, "file_write should succeed: {}", result.content);

    // Read it back
    let result = read_tool
        .execute(
            &ctx,
            json!({
                "path": "hello.txt"
            }),
        )
        .await
        .unwrap();
    assert!(!result.is_error, "file_read should succeed: {}", result.content);
    assert!(
        result.content.contains("Hello, World!"),
        "read should return written content, got: {}",
        result.content
    );
    assert!(
        result.content.contains("Line 2"),
        "read should return all lines"
    );
}

// ============================================================================
// file_edit
// ============================================================================

#[tokio::test]
async fn file_edit_applies_correctly() {
    let tmp = TempDir::new().unwrap();
    let ctx = make_ctx(&tmp);

    let write_tool = arawn_engine::tools::FileWriteTool;
    let read_tool = arawn_engine::tools::FileReadTool;
    let edit_tool = arawn_engine::tools::FileEditTool;

    // Write initial file (relative paths)
    write_tool
        .execute(
            &ctx,
            json!({
                "path": "config.toml",
                "content": "[server]\nhost = \"localhost\"\nport = 3000\n"
            }),
        )
        .await
        .unwrap();

    // Read it first (file_edit requires the file was read)
    read_tool
        .execute(&ctx, json!({"path": "config.toml"}))
        .await
        .unwrap();

    // Edit: change port
    let result = edit_tool
        .execute(
            &ctx,
            json!({
                "path": "config.toml",
                "old_string": "port = 3000",
                "new_string": "port = 8080"
            }),
        )
        .await
        .unwrap();
    assert!(!result.is_error, "file_edit should succeed: {}", result.content);

    // Read back and verify
    let result = read_tool
        .execute(&ctx, json!({"path": "config.toml"}))
        .await
        .unwrap();
    assert!(
        result.content.contains("port = 8080"),
        "edit should have changed port, got: {}",
        result.content
    );
    assert!(
        !result.content.contains("port = 3000"),
        "old value should be gone"
    );
}

// ============================================================================
// shell
// ============================================================================

#[tokio::test]
async fn shell_captures_output() {
    let tmp = TempDir::new().unwrap();
    let ctx = make_ctx(&tmp);
    let shell = arawn_engine::tools::ShellTool::default();

    let result = shell
        .execute(&ctx, json!({"command": "echo 'hello from shell'"}))
        .await
        .unwrap();
    assert!(!result.is_error, "shell should succeed: {}", result.content);
    assert!(
        result.content.contains("hello from shell"),
        "should capture stdout, got: {}",
        result.content
    );
}

#[tokio::test]
async fn shell_captures_exit_code_on_failure() {
    let tmp = TempDir::new().unwrap();
    let ctx = make_ctx(&tmp);
    let shell = arawn_engine::tools::ShellTool::default();

    let result = shell
        .execute(&ctx, json!({"command": "exit 42"}))
        .await
        .unwrap();
    assert!(
        result.is_error || result.content.contains("42") || result.content.contains("exit"),
        "should indicate failure, got: {}",
        result.content
    );
}

// ============================================================================
// workflow_create — compilation validation
// ============================================================================

#[tokio::test]
#[ignore] // Slow (~30s compile time). Run with: cargo test --test tool_artifacts -- --ignored
async fn workflow_create_minimal_compiles() {
    let tmp = TempDir::new().unwrap();
    let ctx = make_ctx(&tmp);
    let workflows_dir = tmp.path().join("workflows");
    std::fs::create_dir_all(&workflows_dir).unwrap();

    let tool = arawn_workflow::WorkflowCreateTool::new(workflows_dir.clone());

    let result = tool
        .execute(
            &ctx,
            json!({
                "name": "test-hello",
                "description": "Minimal test workflow",
                "tasks": [
                    {
                        "id": "greet",
                        "body": "context.insert(\"msg\", serde_json::json!(\"hello\"))?;\nOk(())"
                    },
                    {
                        "id": "log",
                        "dependencies": ["greet"],
                        "body": "let _msg = context.get(\"msg\");\nOk(())"
                    }
                ]
            }),
        )
        .await
        .unwrap();

    assert!(
        !result.is_error,
        "workflow_create should compile successfully, got: {}",
        result.content
    );
    assert!(
        result.content.contains("installed") || result.content.contains("success") || result.content.contains("test-hello"),
        "should confirm installation, got: {}",
        result.content
    );

    // Verify the package was installed
    let installed = workflows_dir.join("test-hello");
    assert!(
        installed.exists(),
        "workflow package should exist at {:?}",
        installed
    );
    assert!(
        installed.join("package.toml").exists(),
        "package.toml should exist"
    );
}

#[tokio::test]
#[ignore] // Slow
async fn workflow_create_with_cron_compiles() {
    let tmp = TempDir::new().unwrap();
    let ctx = make_ctx(&tmp);
    let workflows_dir = tmp.path().join("workflows");
    std::fs::create_dir_all(&workflows_dir).unwrap();

    let tool = arawn_workflow::WorkflowCreateTool::new(workflows_dir);

    let result = tool
        .execute(
            &ctx,
            json!({
                "name": "daily-check",
                "description": "Daily health check",
                "tasks": [
                    {
                        "id": "check",
                        "body": "context.insert(\"status\", serde_json::json!(\"healthy\"))?;\nOk(())"
                    }
                ],
                "cron": "0 8 * * 1-5",
                "cron_timezone": "UTC"
            }),
        )
        .await
        .unwrap();

    assert!(
        !result.is_error,
        "workflow with cron should compile, got: {}",
        result.content
    );
}

// ============================================================================
// workflow_list and workflow_delete
// ============================================================================

#[tokio::test]
async fn workflow_list_shows_installed() {
    let tmp = TempDir::new().unwrap();
    let ctx = make_ctx(&tmp);
    let workflows_dir = tmp.path().join("workflows");

    // Create a fake installed workflow
    let pkg_dir = workflows_dir.join("my-workflow");
    std::fs::create_dir_all(&pkg_dir).unwrap();
    std::fs::write(
        pkg_dir.join("package.toml"),
        "[package]\nname = \"my-workflow\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();

    let tool = arawn_workflow::WorkflowListTool::new(workflows_dir);
    let result = tool.execute(&ctx, json!({})).await.unwrap();

    assert!(!result.is_error);
    assert!(
        result.content.contains("my-workflow"),
        "list should include installed workflow, got: {}",
        result.content
    );
}

#[tokio::test]
async fn workflow_delete_removes_installed() {
    let tmp = TempDir::new().unwrap();
    let ctx = make_ctx(&tmp);
    let workflows_dir = tmp.path().join("workflows");

    // Create a fake installed workflow
    let pkg_dir = workflows_dir.join("deleteme");
    std::fs::create_dir_all(&pkg_dir).unwrap();
    std::fs::write(pkg_dir.join("package.toml"), "[package]\nname = \"deleteme\"\n").unwrap();

    let tool = arawn_workflow::WorkflowDeleteTool::new(workflows_dir.clone());
    let result = tool
        .execute(&ctx, json!({"name": "deleteme"}))
        .await
        .unwrap();

    assert!(!result.is_error, "delete should succeed: {}", result.content);
    assert!(
        !workflows_dir.join("deleteme").exists(),
        "workflow dir should be removed"
    );
}
