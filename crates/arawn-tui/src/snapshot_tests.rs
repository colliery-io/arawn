#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::{TimeZone, Utc};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use uuid::Uuid;

    use crate::app::{App, ChatMessage, ChatRole, Focus, SidebarSection};
    use crate::render::render;
    use crate::snapshot::{buffer_to_snapshot, buffer_to_styled_snapshot};

    use arawn_service::{LensInfo, SessionInfo};

    fn make_terminal(w: u16, h: u16) -> Terminal<TestBackend> {
        Terminal::new(TestBackend::new(w, h)).unwrap()
    }

    fn draw(app: &mut App, terminal: &mut Terminal<TestBackend>) -> String {
        terminal.draw(|f| render(app, f)).unwrap();
        buffer_to_snapshot(terminal)
    }

    fn draw_styled(app: &mut App, terminal: &mut Terminal<TestBackend>) -> String {
        terminal.draw(|f| render(app, f)).unwrap();
        buffer_to_styled_snapshot(terminal)
    }

    // --- Empty states ---

    #[test]
    fn snapshot_empty_app() {
        let mut app = App::new();
        let mut terminal = make_terminal(100, 30);
        let snap = draw(&mut app, &mut terminal);
        insta::assert_snapshot!(snap);
    }

    // --- Chat with messages ---

    #[test]
    fn snapshot_chat_with_conversation() {
        let mut app = App::new();
        app.messages = vec![
            ChatMessage::new(ChatRole::User, "What files are in the project?"),
            ChatMessage::new(
                ChatRole::ToolCall {
                    name: "shell".into(),
                },
                "Calling shell...",
            ),
            ChatMessage::new(
                ChatRole::ToolResult {
                    name: "shell".into(),
                    is_error: false,
                },
                "Cargo.toml\ncrates/\nplugins/",
            ),
            ChatMessage::new(
                ChatRole::Assistant,
                "The project has a Cargo.toml, crates/ and plugins/ directories.",
            ),
        ];

        let mut terminal = make_terminal(100, 30);
        let snap = draw(&mut app, &mut terminal);
        insta::assert_snapshot!(snap);
    }

    // --- Streaming state ---

    #[test]
    fn snapshot_streaming_response() {
        let mut app = App::new();
        app.is_generating = true;
        app.streaming_text = "Here are the crates in this".into();
        app.messages = vec![ChatMessage::new(ChatRole::User, "List the crates")];

        let mut terminal = make_terminal(100, 30);
        let snap = draw(&mut app, &mut terminal);
        insta::assert_snapshot!(snap);
    }

    // --- Sidebar populated ---

    #[test]
    fn snapshot_sidebar_with_lenses() {
        let mut app = App::new();
        app.focus = Focus::Sidebar;
        app.lenses = vec![
            LensInfo {
                id: Uuid::nil(),
                name: "scratch".into(),
                root_dir: PathBuf::from("/tmp"),
                created_at: Utc.with_ymd_and_hms(2025, 1, 15, 12, 0, 0).unwrap(),
            },
            LensInfo {
                id: Uuid::nil(),
                name: "home".into(),
                root_dir: PathBuf::from("/tmp"),
                created_at: Utc.with_ymd_and_hms(2025, 1, 15, 12, 0, 0).unwrap(),
            },
        ];
        app.sessions = vec![SessionInfo {
            id: Uuid::nil(),
            lens_id: None,
            created_at: Utc.with_ymd_and_hms(2025, 1, 15, 12, 0, 0).unwrap(),
        }];
        app.current_lens = app.lenses.first().cloned();
        app.current_session = app.sessions.first().cloned();

        let mut terminal = make_terminal(100, 30);
        let snap = draw(&mut app, &mut terminal);
        insta::assert_snapshot!(snap);
    }

    // --- Focus states ---

    #[test]
    fn snapshot_focus_main() {
        let mut app = App::new();
        app.focus = Focus::Main;
        app.input_buffer = "hello world".into();
        app.cursor_pos = 11;

        let mut terminal = make_terminal(100, 30);
        let snap = draw(&mut app, &mut terminal);
        insta::assert_snapshot!(snap);
    }

    #[test]
    fn snapshot_focus_sidebar() {
        let mut app = App::new();
        app.focus = Focus::Sidebar;
        app.lenses = vec![LensInfo {
            id: Uuid::nil(),
            name: "scratch".into(),
            root_dir: PathBuf::from("/tmp"),
            created_at: Utc.with_ymd_and_hms(2025, 1, 15, 12, 0, 0).unwrap(),
        }];

        let mut terminal = make_terminal(100, 30);
        let snap = draw(&mut app, &mut terminal);
        insta::assert_snapshot!(snap);
    }

    #[test]
    fn snapshot_focus_main_with_messages() {
        let mut app = App::new();
        app.focus = Focus::Main;
        app.messages = vec![ChatMessage::new(ChatRole::Assistant, "Some response text.")];

        let mut terminal = make_terminal(100, 30);
        let snap = draw(&mut app, &mut terminal);
        insta::assert_snapshot!(snap);
    }

    // --- Input states ---

    #[test]
    fn snapshot_input_placeholder() {
        let mut app = App::new(); // empty buffer
        let mut terminal = make_terminal(100, 30);
        let snap = draw(&mut app, &mut terminal);
        insta::assert_snapshot!(snap);
    }

    #[test]
    fn snapshot_input_generating() {
        let mut app = App::new();
        app.is_generating = true;

        let mut terminal = make_terminal(100, 30);
        let snap = draw(&mut app, &mut terminal);
        insta::assert_snapshot!(snap);
    }

    // --- Error message ---

    #[test]
    fn snapshot_idle_hero() {
        // Regression guard for I-0036 Phase 5: empty chat shows the
        // centered wordmark + key-binding hints, not a blank surface.
        let mut app = App::new();
        app.messages.clear();
        app.is_generating = false;
        app.streaming_text.clear();
        // Pre-onboarding state: no brief cached, fall through to
        // the hero (T-0331 welcome).
        app.brief_markdown = None;

        let mut terminal = make_terminal(100, 24);
        let snap = draw(&mut app, &mut terminal);
        insta::assert_snapshot!(snap);
    }

    // I-0035 Phase 3 T-A: three-pane layout renders the dashboard
    // skeleton on the right when the terminal is wide enough.
    #[test]
    fn snapshot_layout_three_pane() {
        let mut app = App::new();
        app.messages.clear();
        app.is_generating = false;
        app.streaming_text.clear();
        app.brief_markdown = None;

        // 120 cols is comfortably above MIN_FOR_THREE_PANE (100).
        let mut terminal = make_terminal(120, 24);
        let snap = draw(&mut app, &mut terminal);
        insta::assert_snapshot!(snap);
    }

    // I-0035 Phase 3 T-B: dashboard brief section renders today's
    // calendar items.
    #[test]
    fn snapshot_dashboard_brief_with_calendar() {
        let mut app = App::new();
        app.messages.clear();
        app.is_generating = false;
        app.streaming_text.clear();
        app.daily_view = Some(arawn_ceremonies::DailyView {
            tablet: arawn_ceremonies::service::TabletDto {
                id: "daily-2026-05-19".into(),
                kind: "daily".into(),
                period_key: "2026-05-19".into(),
                generated_at: "2026-05-19T07:00:00Z".into(),
                status: "open".into(),
                lenses_scanned: serde_json::json!([]),
                priorities_confirmed_at: None,
                recovered: false,
            },
            items: vec![
                arawn_ceremonies::service::ItemDto {
                    id: "item-1".into(),
                    tablet_id: "daily-2026-05-19".into(),
                    section_key: "calendar".into(),
                    ordinal: 0,
                    kind: "freeform".into(),
                    body: serde_json::json!({
                        "text": "standup — platform",
                        "start_ts": "2026-05-19T09:00:00Z",
                        "end_ts": "2026-05-19T09:15:00Z",
                    }),
                    citation_id: None,
                    done_at: None,
                    created_at: "2026-05-19T07:00:00Z".into(),
                },
                arawn_ceremonies::service::ItemDto {
                    id: "item-2".into(),
                    tablet_id: "daily-2026-05-19".into(),
                    section_key: "calendar".into(),
                    ordinal: 1,
                    kind: "freeform".into(),
                    body: serde_json::json!({
                        "text": "1:1 with Jamie",
                        "start_ts": "2026-05-19T13:00:00Z",
                        "end_ts": "2026-05-19T13:30:00Z",
                    }),
                    citation_id: None,
                    done_at: None,
                    created_at: "2026-05-19T07:00:00Z".into(),
                },
            ],
        });

        let mut terminal = make_terminal(120, 24);
        let snap = draw(&mut app, &mut terminal);
        insta::assert_snapshot!(snap);
    }

    // I-0035 Phase 3 T-C: dashboard action items section with
    // three attention items + one carried-over todo.
    #[test]
    fn snapshot_dashboard_actions_with_items() {
        let mut app = App::new();
        app.messages.clear();
        app.is_generating = false;
        app.streaming_text.clear();
        let mk_item = |id: &str, section: &str, ordinal: i32, text: &str| {
            arawn_ceremonies::service::ItemDto {
                id: id.into(),
                tablet_id: "daily-2026-05-19".into(),
                section_key: section.into(),
                ordinal,
                kind: "freeform".into(),
                body: serde_json::json!({"text": text}),
                citation_id: None,
                done_at: None,
                created_at: "2026-05-19T07:00:00Z".into(),
            }
        };
        app.daily_view = Some(arawn_ceremonies::DailyView {
            tablet: arawn_ceremonies::service::TabletDto {
                id: "daily-2026-05-19".into(),
                kind: "daily".into(),
                period_key: "2026-05-19".into(),
                generated_at: "2026-05-19T07:00:00Z".into(),
                status: "open".into(),
                lenses_scanned: serde_json::json!([]),
                priorities_confirmed_at: None,
                recovered: false,
            },
            items: vec![
                mk_item("attn-1", "attention", 0, "Reply Alice RFC-0042"),
                mk_item("attn-2", "attention", 1, "Review PR #482"),
                mk_item("attn-3", "attention", 2, "ENG-712 stale 10d"),
                mk_item("todo-1", "todos", 0, "Carried: dust falcon"),
            ],
        });

        let mut terminal = make_terminal(120, 24);
        let snap = draw(&mut app, &mut terminal);
        insta::assert_snapshot!(snap);
    }

    // I-0035 Phase 4 T-A: toast row appears above the status bar.
    #[test]
    fn snapshot_toast_visible() {
        let mut app = App::new();
        app.messages.clear();
        app.is_generating = false;
        app.streaming_text.clear();
        app.post_toast(
            "Brief updated — daily tablet for 2026-05-19",
            crate::toast::ToastLevel::Info,
        );

        let mut terminal = make_terminal(100, 16);
        let snap = draw(&mut app, &mut terminal);
        insta::assert_snapshot!(snap);
    }

    // I-0035 Phase 3 T-A: below MIN_FOR_THREE_PANE the dashboard
    // pane is dropped and the chat area reclaims the right column.
    #[test]
    fn snapshot_layout_narrow_fallback() {
        let mut app = App::new();
        app.messages.clear();
        app.is_generating = false;
        app.streaming_text.clear();
        app.brief_markdown = None;

        // 80 cols < MIN_FOR_THREE_PANE.
        let mut terminal = make_terminal(80, 24);
        let snap = draw(&mut app, &mut terminal);
        insta::assert_snapshot!(snap);
    }

    // I-0035 Phase 2 (T-0354): with a cached brief, empty chat
    // renders the brief markdown instead of the welcome hero.
    #[test]
    fn snapshot_idle_hero_with_brief() {
        let mut app = App::new();
        app.messages.clear();
        app.is_generating = false;
        app.streaming_text.clear();
        app.brief_markdown = Some(
            "# Brief — 2026-05-19\n\n\
             ## Today\n\n\
             ### Today's calendar\n\n\
             - 09:00 standup\n\n\
             ### Carried-over todos\n\n\
             _(no todos rolled over from yesterday)_\n\n\
             ## This week\n\n\
             ### Priorities\n\n\
             - [x] Ship I-0035 Phase 2\n"
                .into(),
        );

        let mut terminal = make_terminal(100, 24);
        let snap = draw(&mut app, &mut terminal);
        insta::assert_snapshot!(snap);
    }

    // Empty markdown is treated as "no brief" → falls through to hero.
    #[test]
    fn snapshot_idle_hero_partial_brief_only_placeholders() {
        let mut app = App::new();
        app.messages.clear();
        app.is_generating = false;
        app.streaming_text.clear();
        // A brief where both sections are placeholders should still
        // render the brief (caller decided to cache it). Snapshot
        // captures that the placeholders render correctly.
        app.brief_markdown = Some(
            "# Brief — 2026-05-19\n\n\
             ## Today\n\n\
             - 09:00 standup\n\n\
             ## This week\n\n\
             _(no weekly tablet — Monday's ceremony will produce one)_\n"
                .into(),
        );

        let mut terminal = make_terminal(100, 24);
        let snap = draw(&mut app, &mut terminal);
        insta::assert_snapshot!(snap);
    }

    #[test]
    fn snapshot_unicode_chrome_alignment() {
        // Regression guard for I-0036 Phase 4: tool calls and result
        // previews containing wide-cell unicode (CJK, emoji) must stay
        // aligned. Display-width measurement, not byte / char count.
        let mut app = App::new();
        app.messages = vec![
            ChatMessage::new(ChatRole::User, "search for 日本語 docs"),
            ChatMessage::new(
                ChatRole::ToolCall {
                    name: "🔥shell".into(),
                },
                "grep -r 日本語 docs/",
            ),
            ChatMessage::new(
                ChatRole::ToolResult {
                    name: "🔥shell".into(),
                    is_error: false,
                },
                "docs/日本語/intro.md\ndocs/日本語/usage.md",
            ),
        ];

        let mut terminal = make_terminal(100, 24);
        let snap = draw(&mut app, &mut terminal);
        insta::assert_snapshot!(snap);
    }

    #[test]
    fn snapshot_speaker_gutters() {
        // Regression guard for I-0036 Phase 3: each speaker has a
        // distinct gutter signal (user `❯`, assistant `│`, tool `⏵`,
        // system `│ system:`).
        let mut app = App::new();
        app.messages = vec![
            ChatMessage::new(ChatRole::User, "what's up"),
            ChatMessage::new(
                ChatRole::Assistant,
                "Here's a quick answer with some prose.",
            ),
            ChatMessage::new(
                ChatRole::ToolCall {
                    name: "shell".into(),
                },
                "ls -la",
            ),
            ChatMessage::new(
                ChatRole::ToolResult {
                    name: "shell".into(),
                    is_error: false,
                },
                "file.rs\nfile2.rs",
            ),
            ChatMessage::new(ChatRole::System, "Permission required: shell"),
        ];

        let mut terminal = make_terminal(100, 30);
        let snap = draw(&mut app, &mut terminal);
        insta::assert_snapshot!(snap);
    }

    #[test]
    fn snapshot_ten_tool_calls_collapsed() {
        // Regression guard for I-0036 Phase 2: 10 tool-call/result pairs
        // should render as compact single lines, not 10 boxed cards.
        let mut app = App::new();
        let mut messages = vec![ChatMessage::new(ChatRole::User, "do many things")];
        for i in 0..10 {
            messages.push(ChatMessage::new(
                ChatRole::ToolCall {
                    name: "shell".into(),
                },
                format!("ls -la /tmp/dir-{i}"),
            ));
            messages.push(ChatMessage::new(
                ChatRole::ToolResult {
                    name: "shell".into(),
                    is_error: false,
                },
                format!("file-{i}.txt\nfile-{i}.log"),
            ));
        }
        app.messages = messages;

        let mut terminal = make_terminal(100, 40);
        let snap = draw(&mut app, &mut terminal);
        insta::assert_snapshot!(snap);
    }

    #[test]
    fn snapshot_error_in_chat() {
        let mut app = App::new();
        app.messages = vec![
            ChatMessage::new(ChatRole::User, "do something"),
            ChatMessage::new(ChatRole::System, "Error: API rate limited"),
        ];

        let mut terminal = make_terminal(100, 30);
        let snap = draw(&mut app, &mut terminal);
        insta::assert_snapshot!(snap);
    }

    // --- Styled snapshots (capture colors + modifiers) ---

    #[test]
    fn styled_snapshot_conversation() {
        let mut app = App::new();
        app.messages = vec![
            ChatMessage::new(ChatRole::User, "hello"),
            ChatMessage::new(ChatRole::Assistant, "world"),
            ChatMessage::new(
                ChatRole::ToolCall {
                    name: "shell".into(),
                },
                "Calling shell...",
            ),
            ChatMessage::new(
                ChatRole::ToolResult {
                    name: "shell".into(),
                    is_error: true,
                },
                "permission denied",
            ),
        ];

        let mut terminal = make_terminal(80, 20);
        let snap = draw_styled(&mut app, &mut terminal);
        insta::assert_snapshot!(snap);
    }

    #[test]
    fn styled_snapshot_focus_borders() {
        // Main focused — prompt visible, typing in input
        let mut app = App::new();
        app.focus = Focus::Main;
        app.input_buffer = "typing here".into();

        let mut terminal = make_terminal(80, 20);
        let snap = draw_styled(&mut app, &mut terminal);
        insta::assert_snapshot!(snap);
    }

    #[test]
    fn styled_snapshot_sidebar_focused() {
        // Sidebar focused — cyan border
        let mut app = App::new();
        app.focus = Focus::Sidebar;

        let mut terminal = make_terminal(80, 20);
        let snap = draw_styled(&mut app, &mut terminal);
        insta::assert_snapshot!(snap);
    }

    #[test]
    fn snapshot_rich_markdown() {
        let mut app = App::new();
        app.messages = vec![
            ChatMessage::new(ChatRole::User, "How do I fix this?"),
            ChatMessage::new(
                ChatRole::Assistant,
                r#"## Analysis

Here's what I found:

- **File A** has `3 errors` in the parser
- **File B** is clean
- The issue is in the `handle_input` function

```rust
fn fix() {
    let x = parse(input);
    validate(&x);
}
```

Try running `cargo test` to verify the fix. You can also check the [docs](https://example.com) for more info.

1. First, update the parser
2. Then run the tests
3. Finally, check the output"#,
            ),
        ];

        let mut terminal = make_terminal(80, 40);
        let snap = draw(&mut app, &mut terminal);
        insta::assert_snapshot!(snap);
    }

    #[test]
    fn styled_snapshot_rich_markdown() {
        let mut app = App::new();
        app.messages = vec![
            ChatMessage::new(ChatRole::User, "Explain this code"),
            ChatMessage::new(
                ChatRole::Assistant,
                r#"# Overview

This function does **two things**:

1. Parses the *input* into tokens
2. Validates each token against the schema

```python
def process(data):
    tokens = parse(data)
    return validate(tokens)
```

> Note: This is O(n) complexity.

The `parse` function handles edge cases like empty strings and unicode."#,
            ),
        ];

        let mut terminal = make_terminal(80, 35);
        let snap = draw_styled(&mut app, &mut terminal);
        insta::assert_snapshot!(snap);
    }

    #[test]
    fn styled_snapshot_generating_state() {
        let mut app = App::new();
        app.is_generating = true;
        app.streaming_text = "partial".into();
        app.messages = vec![ChatMessage::new(ChatRole::User, "question")];

        let mut terminal = make_terminal(80, 20);
        let snap = draw_styled(&mut app, &mut terminal);
        insta::assert_snapshot!(snap);
    }
}
