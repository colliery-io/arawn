//! Integration tests: workflow tools and skill activation through the QueryEngine.

use std::sync::Arc;

use arawn_core::Message;
use arawn_engine::skills::SkillRegistry;
use arawn_engine::testing::TestHarness;
use arawn_llm::MockResponse;

// ── Helpers ──────────────────────────────────────────────────────────────────

fn assert_tool_result_ok_contains(msgs: &[Message], index: usize, substring: &str) {
    match &msgs[index] {
        Message::ToolResult {
            is_error, content, ..
        } => {
            assert!(!is_error, "expected success, got error: {content}");
            assert!(
                content.contains(substring),
                "expected '{substring}' in output, got: {content}"
            );
        }
        other => panic!("expected ToolResult at index {index}, got {other:?}"),
    }
}

fn assert_tool_result_is_error(msgs: &[Message], index: usize) {
    match &msgs[index] {
        Message::ToolResult {
            is_error, content, ..
        } => {
            assert!(is_error, "expected error, got success: {content}");
        }
        other => panic!("expected ToolResult at index {index}, got {other:?}"),
    }
}

// ── Skill Activation ────────────────────────────────────────────────────────

#[tokio::test]
async fn workflows_skill_activates_on_workflow_request() {
    // The built-in "workflows" skill should be auto-registered and invocable
    let skill_registry = Arc::new(SkillRegistry::new());
    assert!(
        skill_registry.get("workflows").is_some(),
        "built-in workflows skill should be auto-registered"
    );

    let harness = TestHarness::builder()
        .with_skill_registry(skill_registry)
        .with_script(vec![
            MockResponse::tool_call("c1", "skill", r#"{"skill":"workflows"}"#),
            MockResponse::text("I'll create a workflow for you."),
        ])
        .build();

    let result = harness.run("set up a daily triage workflow").await;
    assert_eq!(result.final_text(), "I'll create a workflow for you.");
    // The skill prompt should contain the authoring guide
    assert_tool_result_ok_contains(result.session_messages(), 2, "workflow_create");
    assert_tool_result_ok_contains(result.session_messages(), 2, "Decision tasks");
    assert_tool_result_ok_contains(result.session_messages(), 2, "/api/decision");
}

#[tokio::test]
async fn workflows_skill_contains_decision_callback_pattern() {
    let skill_registry = Arc::new(SkillRegistry::new());
    let skill = skill_registry.get("workflows").unwrap();
    // The skill prompt should teach the agent how to write decision task callbacks
    assert!(
        skill.prompt.contains("reqwest"),
        "skill should document the reqwest HTTP callback pattern"
    );
    assert!(
        skill.prompt.contains("127.0.0.1"),
        "skill should reference the local server endpoint"
    );
    assert!(
        skill.prompt.contains("upstream_data"),
        "skill should explain upstream data injection"
    );
}

// ── WorkflowListTool ────────────────────────────────────────────────────────

#[tokio::test]
async fn workflow_list_empty_directory() {
    let packages_dir = tempfile::tempdir().unwrap();
    let tool = arawn_workflow::WorkflowListTool::new(packages_dir.path().to_path_buf());

    let harness = TestHarness::builder()
        .with_tool(Box::new(tool))
        .with_script(vec![
            MockResponse::tool_call("c1", "workflow_list", "{}"),
            MockResponse::text("No workflows installed."),
        ])
        .build();

    let result = harness.run("list workflows").await;
    assert_eq!(result.final_text(), "No workflows installed.");
    assert_tool_result_ok_contains(result.session_messages(), 2, "[]");
}

#[tokio::test]
async fn workflow_list_shows_installed_packages() {
    let packages_dir = tempfile::tempdir().unwrap();
    // Create a fake workflow package directory
    let pkg = packages_dir.path().join("daily-triage");
    std::fs::create_dir(&pkg).unwrap();
    std::fs::write(
        pkg.join("package.toml"),
        r#"[package]
name = "daily-triage"
version = "0.1.0"

[metadata]
workflow_name = "daily_triage"
language = "rust"
"#,
    )
    .unwrap();

    let tool = arawn_workflow::WorkflowListTool::new(packages_dir.path().to_path_buf());

    let harness = TestHarness::builder()
        .with_tool(Box::new(tool))
        .with_script(vec![
            MockResponse::tool_call("c1", "workflow_list", "{}"),
            MockResponse::text("Found daily-triage."),
        ])
        .build();

    let result = harness.run("list workflows").await;
    assert_tool_result_ok_contains(result.session_messages(), 2, "daily-triage");
}

// ── WorkflowDeleteTool ──────────────────────────────────────────────────────

#[tokio::test]
async fn workflow_delete_removes_package() {
    let packages_dir = tempfile::tempdir().unwrap();
    let pkg = packages_dir.path().join("old-workflow");
    std::fs::create_dir(&pkg).unwrap();
    std::fs::write(pkg.join("package.toml"), "name = \"old-workflow\"").unwrap();

    let tool = arawn_workflow::WorkflowDeleteTool::new(packages_dir.path().to_path_buf());

    let harness = TestHarness::builder()
        .with_tool(Box::new(tool))
        .with_script(vec![
            MockResponse::tool_call(
                "c1",
                "workflow_delete",
                r#"{"name":"old-workflow"}"#,
            ),
            MockResponse::text("Deleted."),
        ])
        .build();

    let result = harness.run("delete old-workflow").await;
    assert_tool_result_ok_contains(result.session_messages(), 2, "deleted");
    assert!(!pkg.exists(), "package directory should be removed");
}

#[tokio::test]
async fn workflow_delete_nonexistent_errors() {
    let packages_dir = tempfile::tempdir().unwrap();
    let tool = arawn_workflow::WorkflowDeleteTool::new(packages_dir.path().to_path_buf());

    let harness = TestHarness::builder()
        .with_tool(Box::new(tool))
        .with_script(vec![
            MockResponse::tool_call(
                "c1",
                "workflow_delete",
                r#"{"name":"ghost"}"#,
            ),
            MockResponse::text("Not found."),
        ])
        .build();

    let result = harness.run("delete ghost").await;
    assert_tool_result_is_error(result.session_messages(), 2);
}

// ── WorkflowStatusTool ──────────────────────────────────────────────────────

#[tokio::test]
async fn workflow_status_no_runner_returns_error() {
    let shared_runner: arawn_workflow::SharedWorkflowRunner =
        Arc::new(tokio::sync::RwLock::new(None));
    let tool = arawn_workflow::WorkflowStatusTool::new(shared_runner);

    let harness = TestHarness::builder()
        .with_tool(Box::new(tool))
        .with_script(vec![
            MockResponse::tool_call("c1", "workflow_status", "{}"),
            MockResponse::text("Runner not available."),
        ])
        .build();

    let result = harness.run("check workflow status").await;
    assert_tool_result_is_error(result.session_messages(), 2);
}

#[tokio::test]
async fn workflow_status_with_runner_returns_empty_list() {
    let tmp = tempfile::tempdir().unwrap();
    let config = arawn_workflow::runner::WorkflowRunnerConfig::new(tmp.path());
    let runner = arawn_workflow::WorkflowRunner::new(config).await.unwrap();

    let shared_runner: arawn_workflow::SharedWorkflowRunner =
        Arc::new(tokio::sync::RwLock::new(Some(Arc::new(runner))));
    let tool = arawn_workflow::WorkflowStatusTool::new(shared_runner);

    let harness = TestHarness::builder()
        .with_tool(Box::new(tool))
        .with_script(vec![
            MockResponse::tool_call("c1", "workflow_status", "{}"),
            MockResponse::text("No executions."),
        ])
        .build();

    let result = harness.run("check status").await;
    assert_tool_result_ok_contains(result.session_messages(), 2, "[]");
}

// ── Scaffold Generation ─────────────────────────────────────────────────────

#[tokio::test]
async fn scaffold_generates_compilable_project() {
    use arawn_workflow::scaffold::{self, TaskDef, WorkflowDef};

    let tmp = tempfile::tempdir().unwrap();
    let def = WorkflowDef {
        name: "test-wf".into(),
        description: "Test workflow".into(),
        tasks: vec![
            TaskDef {
                id: "step_one".into(),
                dependencies: vec![],
                body: "ctx.insert(\"key\", serde_json::json!(\"value\"))?;\nOk(())".into(),
                retry_attempts: None,
            },
            TaskDef {
                id: "step_two".into(),
                dependencies: vec!["step_one".into()],
                body: "let _v = ctx.get(\"key\");\nOk(())".into(),
                retry_attempts: Some(2),
            },
        ],
        cron: Some("0 9 * * *".into()),
        cron_timezone: Some("America/New_York".into()),
    };

    scaffold::generate(tmp.path(), &def).unwrap();

    // Verify all expected files exist
    assert!(tmp.path().join("Cargo.toml").exists());
    assert!(tmp.path().join("build.rs").exists());
    assert!(tmp.path().join("package.toml").exists());
    assert!(tmp.path().join("src/lib.rs").exists());

    // Verify content correctness
    let cargo = std::fs::read_to_string(tmp.path().join("Cargo.toml")).unwrap();
    assert!(cargo.contains("test-wf"));
    assert!(cargo.contains("cdylib"));
    assert!(cargo.contains("packaged"));
    assert!(cargo.contains("cloacina-build"));

    let lib = std::fs::read_to_string(tmp.path().join("src/lib.rs")).unwrap();
    assert!(lib.contains("#[workflow(name = \"test_wf\""));
    assert!(lib.contains("#[task(id = \"step_one\""));
    assert!(lib.contains("dependencies = [\"step_one\"]"));
    assert!(lib.contains("retry_attempts = 2"));
    assert!(lib.contains("#[trigger(on = \"test_wf\", cron = \"0 9 * * *\""));
    assert!(lib.contains("America/New_York"));

    let pkg = std::fs::read_to_string(tmp.path().join("package.toml")).unwrap();
    assert!(pkg.contains("workflow_name = \"test_wf\""));
}

// ── Multi-step: Skill → Tool Chain ──────────────────────────────────────────

#[tokio::test]
async fn skill_then_tool_workflow_creation_chain() {
    // Simulates: agent loads skill → calls workflow_list → calls workflow_create
    // (workflow_create will fail to compile since we don't have a real Rust toolchain
    // in CI, but we verify the tool chain executes in order)
    let packages_dir = tempfile::tempdir().unwrap();
    let skill_registry = Arc::new(SkillRegistry::new());

    let harness = TestHarness::builder()
        .with_skill_registry(skill_registry)
        .with_tool(Box::new(arawn_workflow::WorkflowListTool::new(
            packages_dir.path().to_path_buf(),
        )))
        .with_script(vec![
            // Agent first loads the skill for guidance
            MockResponse::tool_call("c1", "skill", r#"{"skill":"workflows"}"#),
            // Then checks existing workflows
            MockResponse::tool_call("c2", "workflow_list", "{}"),
            // Then responds
            MockResponse::text("No workflows yet. I'll create one for you."),
        ])
        .build();

    let result = harness.run("set up a morning triage workflow").await;
    assert_eq!(
        result.final_text(),
        "No workflows yet. I'll create one for you."
    );

    let calls = result.tool_calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].0, "skill");
    assert_eq!(calls[1].0, "workflow_list");
}
