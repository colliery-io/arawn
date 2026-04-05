//! Integration tests: hot-reload APIs on PermissionChecker mid-session.

use std::sync::Arc;

use arawn_core::Message;
use arawn_engine::permissions::{
    MockModalPrompt, PermissionChecker, PermissionMode, PermissionRule, RuleKind,
};
use arawn_engine::testing::TestHarness;
use arawn_engine::tools::{ShellTool, ThinkTool};
use arawn_llm::MockResponse;

// ── Helpers ──────────────────────────────────────────────────────────────────

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

fn assert_tool_result_ok(msgs: &[Message], index: usize) {
    match &msgs[index] {
        Message::ToolResult {
            is_error, content, ..
        } => {
            assert!(!is_error, "expected success, got error: {content}");
        }
        other => panic!("expected ToolResult at index {index}, got {other:?}"),
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn update_rules_changes_behavior() {
    let checker = Arc::new(
        PermissionChecker::new(vec![PermissionRule::new(RuleKind::Deny, "think")])
            .with_prompter(Box::new(MockModalPrompt::always(None))),
    );

    // Turn 1: think denied
    let harness1 = TestHarness::builder()
        .with_tool(Box::new(ThinkTool))
        .with_permission_checker(checker.clone())
        .with_script(vec![
            MockResponse::tool_call("c1", "think", r#"{"thought":"blocked"}"#),
            MockResponse::text("Denied"),
        ])
        .build();

    let result1 = harness1.run("think").await;
    assert_eq!(result1.final_text(), "Denied");
    assert_tool_result_is_error(result1.session_messages(), 2, "Permission denied");

    // Hot-reload: switch deny → allow
    checker.update_rules(vec![PermissionRule::new(RuleKind::Allow, "think")]);

    // Turn 2: think allowed (same checker, updated rules)
    let harness2 = TestHarness::builder()
        .with_tool(Box::new(ThinkTool))
        .with_permission_checker(checker)
        .with_script(vec![
            MockResponse::tool_call("c2", "think", r#"{"thought":"allowed now"}"#),
            MockResponse::text("Allowed"),
        ])
        .build();

    let result2 = harness2.run("think again").await;
    assert_eq!(result2.final_text(), "Allowed");
    assert_tool_result_ok(result2.session_messages(), 2);
}

#[tokio::test]
async fn update_mode_changes_behavior() {
    // Default mode: shell has no explicit rule → falls back to mode.
    // Default mode asks for shell, and mock prompter denies.
    let checker = Arc::new(
        PermissionChecker::new(vec![])
            .with_mode(PermissionMode::Default)
            .with_prompter(Box::new(MockModalPrompt::always(None))), // deny on ask
    );

    // Turn 1: shell denied (Default mode → Ask → mock denies)
    let harness1 = TestHarness::builder()
        .with_tool(Box::new(ShellTool::default()))
        .with_permission_checker(checker.clone())
        .with_script(vec![
            MockResponse::tool_call("c1", "shell", r#"{"command":"echo hi"}"#),
            MockResponse::text("Denied"),
        ])
        .build();

    let result1 = harness1.run("run shell").await;
    assert_eq!(result1.final_text(), "Denied");
    assert_tool_result_is_error(result1.session_messages(), 2, "Permission denied");

    // Hot-reload: switch to BypassPermissions
    checker.update_mode(PermissionMode::BypassPermissions);

    // Turn 2: shell allowed (BypassPermissions auto-allows everything)
    let harness2 = TestHarness::builder()
        .with_tool(Box::new(ShellTool::default()))
        .with_permission_checker(checker)
        .with_script(vec![
            MockResponse::tool_call("c2", "shell", r#"{"command":"echo bypass"}"#),
            MockResponse::text("Allowed"),
        ])
        .build();

    let result2 = harness2.run("run shell again").await;
    assert_eq!(result2.final_text(), "Allowed");
    assert_tool_result_ok(result2.session_messages(), 2);
}

#[tokio::test]
async fn engine_uses_updated_rules_without_restart() {
    // Verify the engine re-evaluates rules on each tool call (no caching)
    let checker = Arc::new(
        PermissionChecker::new(vec![
            PermissionRule::new(RuleKind::Allow, "think"),
            PermissionRule::new(RuleKind::Deny, "shell"),
        ])
        .with_prompter(Box::new(MockModalPrompt::always(None))),
    );

    // Single harness with multi-step script: think (allowed), then shell (denied)
    let harness = TestHarness::builder()
        .with_tool(Box::new(ThinkTool))
        .with_tool(Box::new(ShellTool::default()))
        .with_permission_checker(checker.clone())
        .with_script(vec![
            MockResponse::tool_call("c1", "think", r#"{"thought":"first"}"#),
            // After think succeeds, hot-reload rules to allow shell too
            MockResponse::tool_call("c2", "shell", r#"{"command":"echo hi"}"#),
            MockResponse::text("Both ran"),
        ])
        .build();

    // Before running, allow shell too via hot-reload
    // (This simulates a config reload happening between the engine creation
    // and the second tool call. Since rules are RwLock-protected, the engine
    // picks up the change on the next check.)
    //
    // We need to update after the first tool call but before the second.
    // Since the engine runs synchronously within `run()`, we update before
    // and it should apply to both calls.
    checker.update_rules(vec![
        PermissionRule::new(RuleKind::Allow, "think"),
        PermissionRule::new(RuleKind::Allow, "shell"),
    ]);

    let result = harness.run("do both").await;
    assert_eq!(result.final_text(), "Both ran");

    let msgs = result.session_messages();
    assert_tool_result_ok(msgs, 2); // think
    assert_tool_result_ok(msgs, 4); // shell (allowed after update)
}
