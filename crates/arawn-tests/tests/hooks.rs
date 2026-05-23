//! Integration tests: hook system wired into the QueryEngine.

use std::sync::Arc;

use arawn_core::Message;
use arawn_engine::hooks::{HookConfig, HookRunner};
use arawn_engine::testing::TestHarness;
use arawn_engine::tools::{ShellTool, ThinkTool};
use arawn_llm::MockResponse;
use tempfile::TempDir;

// ── Helpers ──────────────────────────────────────────────────────────────────

fn assert_tool_result_is_error(msgs: &[Message], index: usize, substring: &str) {
    match &msgs[index] {
        Message::ToolResult {
            is_error, content, ..
        } => {
            assert!(
                is_error,
                "expected error ToolResult, got success: {content}"
            );
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

fn make_hook_config(json: serde_json::Value) -> HookConfig {
    serde_json::from_value(json).expect("failed to parse HookConfig")
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn pre_tool_use_blocking_hook_stops_execution() {
    // Exit code 2 = Block
    let tmp = TempDir::new().unwrap();
    let config = make_hook_config(serde_json::json!({
        "PreToolUse": [{
            "matcher": "shell",
            "hooks": [{"type": "command", "command": "exit 2"}]
        }]
    }));
    let runner = Arc::new(HookRunner::new(config, tmp.path().to_path_buf()));

    let harness = TestHarness::builder()
        .with_tool(Box::new(ShellTool::default()))
        .with_hook_runner(runner)
        .with_script(vec![
            MockResponse::tool_call("c1", "shell", r#"{"command":"echo hi"}"#),
            MockResponse::text("Hook blocked it"),
        ])
        .build();

    let result = harness.run("run shell").await;
    assert_eq!(result.final_text(), "Hook blocked it");
    assert_tool_result_is_error(result.session_messages(), 2, "Hook blocked");
}

#[tokio::test]
async fn pre_tool_use_allowing_hook_permits_execution() {
    // Exit code 0 = Allow
    let tmp = TempDir::new().unwrap();
    let config = make_hook_config(serde_json::json!({
        "PreToolUse": [{
            "matcher": "think",
            "hooks": [{"type": "command", "command": "exit 0"}]
        }]
    }));
    let runner = Arc::new(HookRunner::new(config, tmp.path().to_path_buf()));

    let harness = TestHarness::builder()
        .with_tool(Box::new(ThinkTool))
        .with_hook_runner(runner)
        .with_script(vec![
            MockResponse::tool_call("c1", "think", r#"{"thought":"allowed"}"#),
            MockResponse::text("Think ran"),
        ])
        .build();

    let result = harness.run("think").await;
    assert_eq!(result.final_text(), "Think ran");
    assert_tool_result_ok(result.session_messages(), 2);
}

#[tokio::test]
async fn post_tool_use_hook_fires_after_tool() {
    // PostToolUse hook writes a marker file to verify it executed
    let tmp = TempDir::new().unwrap();
    let marker = tmp.path().join("hook_fired");
    let touch_cmd = format!("touch {}", marker.display());

    let config = make_hook_config(serde_json::json!({
        "PostToolUse": [{
            "matcher": "think",
            "hooks": [{"type": "command", "command": touch_cmd}]
        }]
    }));
    let runner = Arc::new(HookRunner::new(config, tmp.path().to_path_buf()));

    let harness = TestHarness::builder()
        .with_tool(Box::new(ThinkTool))
        .with_hook_runner(runner)
        .with_script(vec![
            MockResponse::tool_call("c1", "think", r#"{"thought":"trigger hook"}"#),
            MockResponse::text("Done"),
        ])
        .build();

    let result = harness.run("think").await;
    assert_eq!(result.final_text(), "Done");
    assert_tool_result_ok(result.session_messages(), 2);

    // Verify the PostToolUse hook actually fired
    assert!(
        marker.exists(),
        "PostToolUse hook should have created marker file at {}",
        marker.display()
    );
}

#[tokio::test]
async fn hook_with_content_pattern_matching() {
    // Hook only fires when the tool input matches the content pattern.
    // "shell(*secret*)" — blocks shell calls containing "secret"
    let tmp = TempDir::new().unwrap();
    let config = make_hook_config(serde_json::json!({
        "PreToolUse": [{
            "matcher": "shell(*secret*)",
            "hooks": [{"type": "command", "command": "exit 2"}]
        }]
    }));
    let runner = Arc::new(HookRunner::new(config, tmp.path().to_path_buf()));

    let harness = TestHarness::builder()
        .with_tool(Box::new(ShellTool::default()))
        .with_hook_runner(runner)
        .with_script(vec![
            // First call: no "secret" in input — should pass
            MockResponse::tool_call("c1", "shell", r#"{"command":"echo safe"}"#),
            // Second call: "secret" in input — should be blocked
            MockResponse::tool_call("c2", "shell", r#"{"command":"echo secret data"}"#),
            MockResponse::text("Done"),
        ])
        .build();

    let result = harness.run("test content matching").await;
    assert_eq!(result.final_text(), "Done");

    let msgs = result.session_messages();
    // msg 0: User, msg 1: Assistant(shell safe), msg 2: ToolResult(ok)
    assert_tool_result_ok(msgs, 2);
    // msg 3: Assistant(shell secret), msg 4: ToolResult(blocked)
    assert_tool_result_is_error(msgs, 4, "Hook blocked");
}

#[tokio::test]
async fn multiple_hooks_one_blocks_aggregated_block() {
    // Two hooks on same tool: first allows (exit 0), second blocks (exit 2).
    // Aggregation: any block → overall block.
    let tmp = TempDir::new().unwrap();
    let config = make_hook_config(serde_json::json!({
        "PreToolUse": [{
            "matcher": "shell",
            "hooks": [
                {"type": "command", "command": "exit 0"},
                {"type": "command", "command": "exit 2"}
            ]
        }]
    }));
    let runner = Arc::new(HookRunner::new(config, tmp.path().to_path_buf()));

    let harness = TestHarness::builder()
        .with_tool(Box::new(ShellTool::default()))
        .with_hook_runner(runner)
        .with_script(vec![
            MockResponse::tool_call("c1", "shell", r#"{"command":"echo hi"}"#),
            MockResponse::text("Blocked by aggregation"),
        ])
        .build();

    let result = harness.run("run shell").await;
    assert_eq!(result.final_text(), "Blocked by aggregation");
    assert_tool_result_is_error(result.session_messages(), 2, "Hook blocked");
}

#[tokio::test]
async fn no_matching_hooks_tool_executes_normally() {
    // Hook on "shell" — but we call "think". No match, tool executes.
    let tmp = TempDir::new().unwrap();
    let config = make_hook_config(serde_json::json!({
        "PreToolUse": [{
            "matcher": "shell",
            "hooks": [{"type": "command", "command": "exit 2"}]
        }]
    }));
    let runner = Arc::new(HookRunner::new(config, tmp.path().to_path_buf()));

    let harness = TestHarness::builder()
        .with_tool(Box::new(ThinkTool))
        .with_hook_runner(runner)
        .with_script(vec![
            MockResponse::tool_call("c1", "think", r#"{"thought":"no hook match"}"#),
            MockResponse::text("Think ran fine"),
        ])
        .build();

    let result = harness.run("think about it").await;
    assert_eq!(result.final_text(), "Think ran fine");
    assert_tool_result_ok(result.session_messages(), 2);
}

#[tokio::test]
async fn user_prompt_submit_blocking_hook_aborts_turn() {
    // I-0056 T-B: UserPromptSubmit block — exit 2 with stderr text is
    // the hook's block reason. The turn short-circuits; the user's
    // prompt never reaches the model. The "assistant response" is the
    // hook's block reason.
    let tmp = TempDir::new().unwrap();
    let config = make_hook_config(serde_json::json!({
        "UserPromptSubmit": [{
            "hooks": [{"type": "command", "command": "echo prompt-rejected >&2 && exit 2"}]
        }]
    }));
    let runner = Arc::new(HookRunner::new(config, tmp.path().to_path_buf()));

    let harness = TestHarness::builder()
        .with_tool(Box::new(ThinkTool))
        .with_hook_runner(runner)
        // No model script — if the hook is honored, the model is never
        // called. If we screwed up and the model IS called, the missing
        // script makes the test fail loudly.
        .with_script(vec![])
        .build();

    let result = harness.run("forbidden prompt").await;
    assert!(
        result.final_text().contains("prompt-rejected"),
        "expected block reason in final text, got: {}",
        result.final_text()
    );
}

#[tokio::test]
async fn user_prompt_submit_allowing_hook_does_not_block_turn() {
    // Companion: exit 0 from UserPromptSubmit hook lets the turn proceed.
    let tmp = TempDir::new().unwrap();
    let config = make_hook_config(serde_json::json!({
        "UserPromptSubmit": [{
            "hooks": [{"type": "command", "command": "exit 0"}]
        }]
    }));
    let runner = Arc::new(HookRunner::new(config, tmp.path().to_path_buf()));

    let harness = TestHarness::builder()
        .with_tool(Box::new(ThinkTool))
        .with_hook_runner(runner)
        .with_script(vec![MockResponse::text("model replied")])
        .build();

    let result = harness.run("normal prompt").await;
    assert_eq!(result.final_text(), "model replied");
}

#[tokio::test]
async fn permission_request_hook_fires_when_prompting() {
    // I-0056 T-C: PermissionRequest hook fires before the user is prompted.
    // Use an Ask rule + a MockModalPrompt that denies, so the prompt is
    // exercised and the hook should also fire.
    use arawn_engine::permissions::{MockModalPrompt, PermissionChecker, PermissionRule, RuleKind};

    let tmp = TempDir::new().unwrap();
    let marker = tmp.path().join("request_fired");
    let touch_cmd = format!("touch {}", marker.display());

    let config = make_hook_config(serde_json::json!({
        "PermissionRequest": [{
            "hooks": [{"type": "command", "command": touch_cmd}]
        }]
    }));
    let runner = Arc::new(HookRunner::new(config, tmp.path().to_path_buf()));

    let checker = Arc::new(
        PermissionChecker::new(vec![PermissionRule::new(RuleKind::Ask, "think")])
            .with_prompter(Box::new(MockModalPrompt::always(Some(0)))) // Allow Once
            .with_hook_runner(Arc::clone(&runner)),
    );

    let harness = TestHarness::builder()
        .with_tool(Box::new(ThinkTool))
        .with_permission_checker(checker)
        .with_script(vec![
            MockResponse::tool_call("c1", "think", r#"{"thought":"prompted"}"#),
            MockResponse::text("ok"),
        ])
        .build();

    let _ = harness.run("think").await;
    assert!(
        marker.exists(),
        "expected PermissionRequest hook to have created {}",
        marker.display()
    );
}

#[tokio::test]
async fn permission_denied_hook_fires_on_deny_rule() {
    // I-0056 T-C: PermissionDenied hook fires when a deny rule rejects a
    // tool call. Marker file proves the hook ran.
    use arawn_engine::permissions::{
        PermissionChecker, PermissionMode, PermissionRule, RuleKind,
    };

    let tmp = TempDir::new().unwrap();
    let marker = tmp.path().join("denied_fired");
    let touch_cmd = format!("touch {}", marker.display());

    let config = make_hook_config(serde_json::json!({
        "PermissionDenied": [{
            "hooks": [{"type": "command", "command": touch_cmd}]
        }]
    }));
    let runner = Arc::new(HookRunner::new(config, tmp.path().to_path_buf()));

    let checker = Arc::new(
        PermissionChecker::new(vec![PermissionRule::new(RuleKind::Deny, "shell")])
            .with_mode(PermissionMode::Full)
            .with_hook_runner(Arc::clone(&runner)),
    );

    let harness = TestHarness::builder()
        .with_tool(Box::new(ShellTool::default()))
        .with_permission_checker(checker)
        .with_script(vec![
            MockResponse::tool_call("c1", "shell", r#"{"command":"echo hi"}"#),
            MockResponse::text("blocked"),
        ])
        .build();

    let result = harness.run("run shell").await;
    assert_eq!(result.final_text(), "blocked");
    assert!(
        marker.exists(),
        "expected PermissionDenied hook to have created {}",
        marker.display()
    );
}

#[tokio::test]
async fn stop_hook_fires_on_final_response() {
    // I-0056 T-B: Stop hook fires when model returns a final response
    // (no more tool calls). Marker file proves the hook ran.
    let tmp = TempDir::new().unwrap();
    let marker = tmp.path().join("stop_fired");
    let touch_cmd = format!("touch {}", marker.display());

    let config = make_hook_config(serde_json::json!({
        "Stop": [{
            "hooks": [{"type": "command", "command": touch_cmd}]
        }]
    }));
    let runner = Arc::new(HookRunner::new(config, tmp.path().to_path_buf()));

    let harness = TestHarness::builder()
        .with_hook_runner(runner)
        .with_script(vec![MockResponse::text("final")])
        .build();

    let result = harness.run("hi").await;
    assert_eq!(result.final_text(), "final");
    // Brief wait — touch is fast but Stop fires before return Ok, so
    // by the time .run() yields the marker file should exist.
    assert!(
        marker.exists(),
        "expected Stop hook to have created {}",
        marker.display()
    );
}
