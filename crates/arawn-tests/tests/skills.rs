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
            MockResponse::tool_call("c1", "Skill", r#"{"skill":"greet"}"#),
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
            MockResponse::tool_call("c1", "Skill", r#"{"skill":"deploy"}"#),
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
            MockResponse::tool_call("c1", "Skill", r#"{"skill":"nonexistent"}"#),
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

    assert_eq!(registry.all().len(), 2);
    let invocable = registry.user_invocable();
    assert_eq!(invocable.len(), 1);
    assert_eq!(invocable[0].name, "visible");
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
            MockResponse::tool_call("c1", "Skill", r#"{"skill":"my-plugin:format"}"#),
            MockResponse::text("Formatted"),
        ])
        .build();

    let result = harness.run("format code").await;
    assert_eq!(result.final_text(), "Formatted");
    assert_tool_result_ok_contains(result.session_messages(), 2, "Format code with the plugin formatter");
}
