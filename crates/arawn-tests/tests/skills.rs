//! Integration tests: skill loading and invocation through the QueryEngine.

use std::sync::Arc;

use arawn_core::Message;
use arawn_engine::skills::{SkillDefinition, SkillRegistry, SkillSource, load_skills_dir};
use arawn_engine::testing::TestHarness;
use arawn_llm::MockResponse;
use tempfile::TempDir;

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

fn assert_tool_result_is_error(msgs: &[Message], index: usize, substring: &str) {
    match &msgs[index] {
        Message::ToolResult {
            is_error, content, ..
        } => {
            assert!(is_error, "expected error, got success: {content}");
            assert!(
                content.contains(substring),
                "expected '{substring}' in error, got: {content}"
            );
        }
        other => panic!("expected ToolResult at index {index}, got {other:?}"),
    }
}

fn make_skill(name: &str, prompt: &str, user_invocable: bool, source: SkillSource) -> SkillDefinition {
    SkillDefinition {
        name: name.into(),
        description: format!("Test skill: {name}"),
        prompt: prompt.into(),
        argument_hint: None,
        allowed_tools: None,
        model: None,
        user_invocable,
        source,
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn register_skill_in_memory_invoke_through_engine() {
    let skill_registry = Arc::new(SkillRegistry::new());
    skill_registry.register(make_skill(
        "greet",
        "Say hello to the user warmly.",
        true,
        SkillSource::Project,
    ));

    let harness = TestHarness::builder()
        .with_skill_registry(skill_registry)
        .with_script(vec![
            MockResponse::tool_call("c1", "skill", r#"{"skill":"greet"}"#),
            MockResponse::text("Hello there!"),
        ])
        .build();

    let result = harness.run("/greet").await;
    assert_eq!(result.final_text(), "Hello there!");
    // ToolResult should contain the skill prompt
    assert_tool_result_ok_contains(result.session_messages(), 2, "Say hello to the user warmly");
}

#[tokio::test]
async fn load_skill_from_markdown_file_and_invoke() {
    let tmp = TempDir::new().unwrap();
    let skill_dir = tmp.path().join("skills");
    std::fs::create_dir(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("deploy.md"),
        r#"---
description: "Deploy the application to production"
user_invocable: true
---

Check the current branch, run tests, and deploy.
"#,
    )
    .unwrap();

    let skills = load_skills_dir(&skill_dir, SkillSource::Project);
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "deploy");

    let skill_registry = Arc::new(SkillRegistry::new());
    for skill in skills {
        skill_registry.register(skill);
    }

    let harness = TestHarness::builder()
        .with_skill_registry(skill_registry)
        .with_script(vec![
            MockResponse::tool_call("c1", "skill", r#"{"skill":"deploy"}"#),
            MockResponse::text("Deploying now"),
        ])
        .build();

    let result = harness.run("/deploy").await;
    assert_eq!(result.final_text(), "Deploying now");
    assert_tool_result_ok_contains(result.session_messages(), 2, "run tests, and deploy");
}

#[tokio::test]
async fn skill_not_found_returns_error() {
    let skill_registry = Arc::new(SkillRegistry::new());
    skill_registry.register(make_skill("commit", "Commit changes", true, SkillSource::Project));

    let harness = TestHarness::builder()
        .with_skill_registry(skill_registry)
        .with_script(vec![
            MockResponse::tool_call("c1", "skill", r#"{"skill":"nonexistent"}"#),
            MockResponse::text("Skill not available"),
        ])
        .build();

    let result = harness.run("/nonexistent").await;
    assert_eq!(result.final_text(), "Skill not available");
    let msgs = result.session_messages();
    assert_tool_result_is_error(msgs, 2, "not found");
    // Should list available skills
    match &msgs[2] {
        Message::ToolResult { content, .. } => {
            assert!(content.contains("commit"), "should list available skills, got: {content}");
        }
        _ => unreachable!(),
    }
}

#[tokio::test]
async fn user_invocable_filtering() {
    let registry = SkillRegistry::new();
    registry.register(make_skill("visible", "Public skill", true, SkillSource::Project));
    registry.register(make_skill("hidden", "Internal skill", false, SkillSource::BuiltIn));

    // Registry has built-in skills plus the two we added
    let invocable = registry.user_invocable();
    assert!(invocable.iter().any(|s| s.name == "visible"));
    assert!(!invocable.iter().any(|s| s.name == "hidden"));
}

#[tokio::test]
async fn plugin_namespaced_skill_accessible() {
    let skill_registry = Arc::new(SkillRegistry::new());
    skill_registry.register(make_skill(
        "my-plugin:format",
        "Format code with the plugin formatter.",
        true,
        SkillSource::Plugin("my-plugin".into()),
    ));

    let harness = TestHarness::builder()
        .with_skill_registry(skill_registry)
        .with_script(vec![
            MockResponse::tool_call("c1", "skill", r#"{"skill":"my-plugin:format"}"#),
            MockResponse::text("Formatted"),
        ])
        .build();

    let result = harness.run("format code").await;
    assert_eq!(result.final_text(), "Formatted");
    assert_tool_result_ok_contains(result.session_messages(), 2, "Format code with the plugin formatter");
}

// ── Built-in skill loading ──────────────────────────────────────────────────

#[tokio::test]
async fn builtin_workflows_skill_loads_on_registry_creation() {
    let registry = SkillRegistry::new();
    let all = registry.all();

    let workflows = all.iter().find(|s| s.name == "workflows");
    assert!(
        workflows.is_some(),
        "built-in 'workflows' skill should load automatically. Got: {:?}",
        all.iter().map(|s| &s.name).collect::<Vec<_>>()
    );

    let ws = workflows.unwrap();
    assert!(ws.user_invocable, "workflows skill should be user-invocable");
    assert!(
        ws.description.contains("scheduled workflow"),
        "workflows description should mention scheduled workflows, got: {}",
        ws.description
    );
    assert!(
        ws.prompt.contains("workflow_create"),
        "workflows prompt should reference the workflow_create tool"
    );
}

// ── Skill listing in system prompt ──────────────────────────────────────────

#[tokio::test]
async fn format_skill_listing_includes_builtins() {
    let registry = SkillRegistry::new();
    let invocable = registry.user_invocable();
    let listing = arawn_engine::skills::format_skill_listing(&invocable, 10000, 250);

    assert!(
        listing.contains("workflows"),
        "skill listing should include built-in 'workflows' skill. Got:\n{listing}"
    );
    assert!(
        listing.contains("scheduled workflow"),
        "skill listing should include the workflows description"
    );
}

#[tokio::test]
async fn skill_listing_appears_in_assembled_system_prompt() {
    let registry = Arc::new(SkillRegistry::new());
    registry.register(make_skill(
        "commit",
        "Commit staged changes with a message",
        true,
        SkillSource::Project,
    ));

    // Build a skill listing as the engine does
    let skills = registry.user_invocable();
    let listing = arawn_engine::skills::format_skill_listing(&skills, 4000, 250);

    // Feed it into the system prompt builder as a plugin prompt (how the engine does it)
    let prompt = arawn_engine::SystemPromptBuilder::new()
        .load_static_sections(None)
        .environment("linux", "bash", std::path::Path::new("/tmp"), "test-model")
        .plugin_prompts(&[listing])
        .build();

    assert!(
        prompt.contains("workflows"),
        "assembled system prompt should contain the workflows skill"
    );
    assert!(
        prompt.contains("commit"),
        "assembled system prompt should contain the registered commit skill"
    );
}

// ── Skill description matching quality ──────────────────────────────────────

#[tokio::test]
async fn skill_descriptions_distinguish_different_use_cases() {
    let registry = SkillRegistry::new();
    registry.register(SkillDefinition {
        name: "commit".into(),
        description: "Use when the user wants to commit changes, create a git commit, or save their work to version control".into(),
        prompt: "Commit workflow".into(),
        argument_hint: None,
        allowed_tools: None,
        model: None,
        user_invocable: true,
        source: SkillSource::Project,
    });
    registry.register(SkillDefinition {
        name: "review-pr".into(),
        description: "Use when the user asks to review a pull request, check PR changes, or give feedback on a PR".into(),
        prompt: "PR review workflow".into(),
        argument_hint: None,
        allowed_tools: None,
        model: None,
        user_invocable: true,
        source: SkillSource::Project,
    });

    let listing = arawn_engine::skills::format_skill_listing(&registry.user_invocable(), 10000, 250);

    // Each skill's description should contain unique differentiating keywords
    assert!(listing.contains("commit changes"), "commit skill should mention committing, got:\n{listing}");
    assert!(listing.contains("review a pull request"), "review-pr skill should mention PR review");
    assert!(listing.contains("scheduled workflow"), "workflows skill should mention scheduling");

    // A model reading this listing should be able to match:
    // "create a daily pipeline" → workflows (scheduled)
    // "commit my changes" → commit (git)
    // "review this PR" → review-pr (PR)
    // Verify all three are present and distinct
    let lines: Vec<&str> = listing.lines().filter(|l| l.starts_with("- ")).collect();
    assert!(
        lines.len() >= 3,
        "should have at least 3 skill entries (workflows + commit + review-pr), got {}",
        lines.len()
    );
}

// ── Skill → tool chaining ───────────────────────────────────────────────────

#[tokio::test]
async fn skill_invocation_chains_into_domain_tool() {
    // Scenario: model calls skill("workflows") → gets workflow guide →
    // then calls workflow_create with a spec → gets success.
    // This tests the full chain that failed in UAT.

    let skill_registry = Arc::new(SkillRegistry::new());
    // Built-in workflows skill is already loaded

    let tmp = TempDir::new().unwrap();
    let workflows_dir = tmp.path().join("workflows");
    std::fs::create_dir_all(&workflows_dir).unwrap();

    let harness = TestHarness::builder()
        .with_skill_registry(skill_registry)
        .with_tool(Box::new(arawn_workflow::WorkflowCreateTool::new(workflows_dir.clone())))
        .with_script(vec![
            // Step 1: Model invokes the workflows skill to learn how to create workflows
            MockResponse::tool_call("c1", "skill", r#"{"skill": "workflows"}"#),
            // Step 2: Model uses workflow_create with a valid spec (informed by the skill guide)
            MockResponse::tool_call("c2", "workflow_create", r#"{
                "name": "daily-monitor",
                "description": "Daily GitHub monitoring",
                "tasks": [
                    {
                        "id": "fetch",
                        "body": "println!(\"fetching...\"); Ok(())"
                    }
                ],
                "cron": "0 8 * * 1-5"
            }"#),
            // Step 3: Model reports success
            MockResponse::text("I've created the daily-monitor workflow. It will run at 8 AM UTC on weekdays."),
        ])
        .build();

    let result = harness.run("Create a daily monitoring workflow that runs every weekday morning").await;

    // Verify the chain: skill → workflow_create → success
    let calls = result.tool_calls();
    assert!(
        calls.len() >= 2,
        "should have at least 2 tool calls (skill + workflow_create), got {}",
        calls.len()
    );
    assert_eq!(calls[0].0, "skill", "first call should be the skill tool");
    assert_eq!(calls[1].0, "workflow_create", "second call should be workflow_create");

    // The skill result should contain the workflow authoring guide
    assert_tool_result_ok_contains(
        result.session_messages(),
        2, // index of first ToolResult
        "workflow_create",
    );

    // Final text should confirm creation
    assert!(
        result.final_text().contains("daily-monitor"),
        "final response should reference the created workflow"
    );
}
