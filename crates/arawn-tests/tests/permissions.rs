//! Integration tests: permission system wired into the QueryEngine.

use std::sync::Arc;

use arawn_core::Message;
use arawn_engine::permissions::{
    MockModalPrompt, PermissionChecker, PermissionMode, PermissionRule, RuleKind,
};
use arawn_engine::testing::TestHarness;
use arawn_engine::tools::{FileWriteTool, ShellTool, ThinkTool};
use arawn_llm::MockResponse;

// ── Helpers ──────────────────────────────────────────────────────────────────

fn assert_tool_result_is_error(msgs: &[Message], index: usize, substring: &str) {
    match &msgs[index] {
        Message::ToolResult {
            is_error, content, ..
        } => {
            assert!(is_error, "expected error ToolResult, got success: {content}");
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
            assert!(
                !is_error,
                "expected success ToolResult, got error: {content}"
            );
        }
        other => panic!("expected ToolResult at index {index}, got {other:?}"),
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn deny_rule_blocks_tool_call() {
    let checker = Arc::new(
        PermissionChecker::new(vec![PermissionRule::new(RuleKind::Deny, "shell")])
            .with_prompter(Box::new(MockModalPrompt::always(None))),
    );

    let harness = TestHarness::builder()
        .with_tool(Box::new(ShellTool::default()))
        .with_permission_checker(checker)
        .with_script(vec![
            MockResponse::tool_call("c1", "shell", r#"{"command":"echo hi"}"#),
            MockResponse::text("Tool was denied"),
        ])
        .build();

    let result = harness.run("run echo").await;
    assert_eq!(result.final_text(), "Tool was denied");
    assert_tool_result_is_error(result.session_messages(), 2, "Permission denied");
}

#[tokio::test]
async fn allow_rule_permits_tool_call() {
    let checker = Arc::new(
        PermissionChecker::new(vec![PermissionRule::new(RuleKind::Allow, "think")])
            .with_prompter(Box::new(MockModalPrompt::always(None))),
    );

    let harness = TestHarness::builder()
        .with_tool(Box::new(ThinkTool))
        .with_permission_checker(checker)
        .with_script(vec![
            MockResponse::tool_call("c1", "think", r#"{"thought":"planning"}"#),
            MockResponse::text("Done thinking"),
        ])
        .build();

    let result = harness.run("think about it").await;
    assert_eq!(result.final_text(), "Done thinking");
    assert_tool_result_ok(result.session_messages(), 2);
}

#[tokio::test]
async fn bypass_mode_allows_all_tools() {
    // No explicit rules — BypassPermissions mode should auto-allow everything
    let checker = Arc::new(
        PermissionChecker::new(vec![]).with_mode(PermissionMode::BypassPermissions),
    );

    let harness = TestHarness::builder()
        .with_tool(Box::new(ShellTool::default()))
        .with_permission_checker(checker)
        .with_script(vec![
            MockResponse::tool_call("c1", "shell", r#"{"command":"echo bypass"}"#),
            MockResponse::text("Shell ran fine"),
        ])
        .build();

    let result = harness.run("run shell").await;
    assert_eq!(result.final_text(), "Shell ran fine");
    assert_tool_result_ok(result.session_messages(), 2);
}

#[tokio::test]
async fn accept_edits_mode_allows_file_write_but_asks_shell() {
    // AcceptEdits: file_write auto-allowed, shell triggers Ask.
    // MockModalPrompt denies → shell should be denied.
    let checker = Arc::new(
        PermissionChecker::new(vec![])
            .with_mode(PermissionMode::AcceptEdits)
            .with_prompter(Box::new(MockModalPrompt::always(None))), // deny on ask
    );

    let harness = TestHarness::builder()
        .with_tool(Box::new(FileWriteTool))
        .with_tool(Box::new(ShellTool::default()))
        .with_permission_checker(checker)
        .with_script(vec![
            // Turn 1: file_write → allowed by AcceptEdits mode
            MockResponse::tool_call(
                "c1",
                "file_write",
                r#"{"path":"test.txt","content":"hello"}"#,
            ),
            // Turn 2: shell → Ask → denied by mock prompter
            MockResponse::tool_call("c2", "shell", r#"{"command":"echo hi"}"#),
            MockResponse::text("Done"),
        ])
        .build();

    let result = harness.run("write then shell").await;
    assert_eq!(result.final_text(), "Done");

    let msgs = result.session_messages();
    // msg 0: User, msg 1: Assistant (file_write), msg 2: ToolResult (file_write ok)
    assert_tool_result_ok(msgs, 2);
    // msg 3: Assistant (shell), msg 4: ToolResult (shell denied)
    assert_tool_result_is_error(msgs, 4, "Permission denied");
}

#[tokio::test]
async fn ask_rule_with_mock_allowing() {
    // Ask rule, mock prompter returns Allow Once (index 0)
    let checker = Arc::new(
        PermissionChecker::new(vec![PermissionRule::new(RuleKind::Ask, "shell")])
            .with_prompter(Box::new(MockModalPrompt::always(Some(0)))), // Allow Once
    );

    let harness = TestHarness::builder()
        .with_tool(Box::new(ShellTool::default()))
        .with_permission_checker(checker)
        .with_script(vec![
            MockResponse::tool_call("c1", "shell", r#"{"command":"echo allowed"}"#),
            MockResponse::text("Shell ran"),
        ])
        .build();

    let result = harness.run("run shell").await;
    assert_eq!(result.final_text(), "Shell ran");
    assert_tool_result_ok(result.session_messages(), 2);
}

#[tokio::test]
async fn ask_rule_with_mock_denying() {
    // Ask rule, mock prompter returns None (cancel = deny)
    let checker = Arc::new(
        PermissionChecker::new(vec![PermissionRule::new(RuleKind::Ask, "shell")])
            .with_prompter(Box::new(MockModalPrompt::always(None))), // Deny
    );

    let harness = TestHarness::builder()
        .with_tool(Box::new(ShellTool::default()))
        .with_permission_checker(checker)
        .with_script(vec![
            MockResponse::tool_call("c1", "shell", r#"{"command":"echo denied"}"#),
            MockResponse::text("Shell blocked"),
        ])
        .build();

    let result = harness.run("run shell").await;
    assert_eq!(result.final_text(), "Shell blocked");
    assert_tool_result_is_error(result.session_messages(), 2, "Permission denied");
}

#[tokio::test]
async fn session_grants_persist_across_turns() {
    // Ask rule, mock prompter returns "Allow Always" (index 1) on first call,
    // then returns Deny on subsequent calls. Session grant should bypass the prompt.
    let checker = Arc::new(
        PermissionChecker::new(vec![PermissionRule::new(RuleKind::Ask, "think")])
            .with_prompter(Box::new(MockModalPrompt::with_responses(
                vec![Some(1)], // First prompt: Allow Always
                None,          // Subsequent prompts: Deny (but should never be reached)
            ))),
    );

    // First call: Ask → Allow Always → grants session permission
    let harness1 = TestHarness::builder()
        .with_tool(Box::new(ThinkTool))
        .with_permission_checker(checker.clone())
        .with_script(vec![
            MockResponse::tool_call("c1", "think", r#"{"thought":"first"}"#),
            MockResponse::text("First ok"),
        ])
        .build();
    let result1 = harness1.run("think first").await;
    assert_eq!(result1.final_text(), "First ok");
    assert_tool_result_ok(result1.session_messages(), 2);

    // Second call: session grant should short-circuit — no prompt needed
    let harness2 = TestHarness::builder()
        .with_tool(Box::new(ThinkTool))
        .with_permission_checker(checker)
        .with_script(vec![
            MockResponse::tool_call("c2", "think", r#"{"thought":"second"}"#),
            MockResponse::text("Second ok"),
        ])
        .build();
    let result2 = harness2.run("think again").await;
    assert_eq!(result2.final_text(), "Second ok");
    assert_tool_result_ok(result2.session_messages(), 2);
}
