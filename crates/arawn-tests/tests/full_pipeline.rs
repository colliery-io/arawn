//! Capstone integration test: all subsystems (permissions, hooks, skills, tools)
//! wired into the QueryEngine simultaneously.

use std::sync::Arc;

use arawn_core::Message;
use arawn_engine::hooks::{HookConfig, HookRunner};
use arawn_engine::permissions::{
    MockModalPrompt, PermissionChecker, PermissionRule, RuleKind,
};
use arawn_engine::skills::{SkillDefinition, SkillRegistry, SkillSource};
use arawn_engine::testing::TestHarness;
use arawn_engine::tools::{FileReadTool, ShellTool, ThinkTool};
use arawn_llm::MockResponse;
use tempfile::TempDir;

#[tokio::test]
async fn full_pipeline_all_subsystems_wired() {
    let tmp = TempDir::new().unwrap();

    // --- Permissions ---
    // allow think, deny shell, ask file_read (mock allows)
    let rules = vec![
        PermissionRule::new(RuleKind::Allow, "think"),
        PermissionRule::new(RuleKind::Deny, "shell"),
        PermissionRule::new(RuleKind::Ask, "file_read"),
    ];
    let checker = Arc::new(
        PermissionChecker::new(rules)
            .with_prompter(Box::new(MockModalPrompt::always(Some(0)))), // Allow Once
    );

    // --- Hooks ---
    // PreToolUse on file_read → allow (exit 0)
    // PostToolUse on think → side-effect marker file
    let marker = tmp.path().join("think_hook_fired");
    let touch_cmd = format!("touch {}", marker.display());
    let hook_config: HookConfig = serde_json::from_value(serde_json::json!({
        "PreToolUse": [{
            "matcher": "file_read",
            "hooks": [{"type": "command", "command": "exit 0"}]
        }],
        "PostToolUse": [{
            "matcher": "think",
            "hooks": [{"type": "command", "command": touch_cmd}]
        }]
    }))
    .unwrap();
    let runner = Arc::new(HookRunner::new(hook_config, tmp.path().to_path_buf()));

    // --- Skills ---
    let skill_registry = Arc::new(SkillRegistry::new());
    skill_registry.register(SkillDefinition {
        name: "greet".into(),
        description: "Greet the user".into(),
        prompt: "Say hello warmly and enthusiastically.".into(),
        argument_hint: None,
        allowed_tools: None,
        model: None,
        user_invocable: true,
        source: SkillSource::Project,
    });

    // --- Pre-populate a file for file_read ---
    let test_file = tmp.path().join("data.txt");
    std::fs::write(&test_file, "important data").unwrap();

    // --- Wire everything ---
    let harness = TestHarness::builder()
        .with_tool(Box::new(ThinkTool))
        .with_tool(Box::new(ShellTool::default()))
        .with_tool(Box::new(FileReadTool))
        .with_permission_checker(checker)
        .with_hook_runner(runner)
        .with_skill_registry(skill_registry)
        .with_workstream_file("data.txt", "important data")
        .with_script(vec![
            // Turn 1: think → allowed by permission rule, no PreToolUse hook match
            MockResponse::tool_call("c1", "think", r#"{"thought":"planning my approach"}"#),
            // Turn 2: shell → denied by permission rule
            MockResponse::tool_call("c2", "shell", r#"{"command":"echo hi"}"#),
            // Turn 3: Skill → greet skill invocation
            MockResponse::tool_call("c3", "Skill", r#"{"skill":"greet"}"#),
            // Turn 4: file_read → ask (mock allows), PreToolUse hook (exit 0 = allow)
            MockResponse::tool_call("c4", "file_read", r#"{"path":"data.txt"}"#),
            // Final text response
            MockResponse::text("All done — exercised all subsystems."),
        ])
        .build();

    let result = harness.run("Test the full pipeline").await;
    assert_eq!(result.final_text(), "All done — exercised all subsystems.");

    let msgs = result.session_messages();

    // --- Verify Turn 1: think (allowed, PostToolUse hook fires) ---
    // msg[0]: User, msg[1]: Assistant(think), msg[2]: ToolResult(think ok)
    match &msgs[2] {
        Message::ToolResult {
            is_error, content, ..
        } => {
            assert!(!is_error, "think should be allowed, got error: {content}");
            assert_eq!(content, "planning my approach");
        }
        other => panic!("expected ToolResult at 2, got {other:?}"),
    }

    // PostToolUse hook on think should have fired
    assert!(
        marker.exists(),
        "PostToolUse hook for think should have created marker file"
    );

    // --- Verify Turn 2: shell (denied by permission) ---
    // msg[3]: Assistant(shell), msg[4]: ToolResult(shell denied)
    match &msgs[4] {
        Message::ToolResult {
            is_error, content, ..
        } => {
            assert!(is_error, "shell should be denied");
            assert!(
                content.contains("Permission denied"),
                "expected permission denied, got: {content}"
            );
        }
        other => panic!("expected ToolResult at 4, got {other:?}"),
    }

    // --- Verify Turn 3: Skill invocation ---
    // msg[5]: Assistant(Skill), msg[6]: ToolResult(skill prompt)
    match &msgs[6] {
        Message::ToolResult {
            is_error, content, ..
        } => {
            assert!(!is_error, "skill should succeed, got error: {content}");
            assert!(
                content.contains("Say hello warmly"),
                "expected skill prompt, got: {content}"
            );
        }
        other => panic!("expected ToolResult at 6, got {other:?}"),
    }

    // --- Verify Turn 4: file_read (ask→allow, hook→allow) ---
    // msg[7]: Assistant(file_read), msg[8]: ToolResult(file content)
    match &msgs[8] {
        Message::ToolResult {
            is_error, content, ..
        } => {
            assert!(!is_error, "file_read should succeed, got error: {content}");
            assert!(
                content.contains("important data"),
                "expected file content, got: {content}"
            );
        }
        other => panic!("expected ToolResult at 8, got {other:?}"),
    }

    // --- Verify message count ---
    // User(0) + 4 * [Assistant + ToolResult] + final Assistant = 10
    assert_eq!(
        msgs.len(),
        10,
        "expected 10 messages (1 user + 4 tool rounds + 1 final), got {}",
        msgs.len()
    );
}
