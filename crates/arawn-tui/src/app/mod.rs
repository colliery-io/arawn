mod actions;
mod autocomplete;
mod events;
mod history;

use ratatui::layout::Rect;

use arawn_service::{SessionInfo, WorkstreamInfo};

use crate::command::{
    AutocompleteState, CommandRegistry, CommandResult,
};

/// Tracks the screen regions of each panel from the last render.
/// Used for mouse hit-testing.
#[derive(Debug, Clone, Default)]
pub struct LayoutRegions {
    pub sidebar: Option<Rect>,
    pub chat: Rect,
    pub input: Rect,
    /// Sidebar workstreams section (for click-to-select).
    pub sidebar_ws: Option<Rect>,
    /// Thin sidebar tab strip (visible when sidebar is hidden).
    pub sidebar_tab: Option<Rect>,
    /// I-0035 Phase 3 dashboard pane (brief + action items). `Some`
    /// when the terminal is wide enough for the three-pane layout;
    /// `None` on narrow terminals or when the layout otherwise
    /// collapses to two panes.
    pub dashboard: Option<Rect>,
}

/// Which panel has focus.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Focus {
    /// Main view: chat + input unified. Typing goes to input, Up/Down scroll chat.
    Main,
    Sidebar,
}

/// Which sidebar section is active.
///
/// I-0035 Phase 3 (T-0356) removed the `Sessions` variant — the
/// sidebar is now Workstreams-only. Sessions are accessible via the
/// `/session list` slash command. The enum is retained as a
/// single-variant placeholder so the focus/render code can still
/// dispatch on it (and to give future sidebar sections an obvious
/// place to hook in without re-introducing the variant).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SidebarSection {
    Workstreams,
}

/// A message displayed in the chat area.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    /// When this message was created (for elapsed time display on tool calls).
    pub created_at: std::time::Instant,
    /// Cached rendered lines for assistant markdown. Populated on first render.
    rendered_cache: Option<Vec<ratatui::text::Line<'static>>>,
    /// Width used for the cached render (invalidated on resize).
    cached_width: usize,
}

impl ChatMessage {
    pub fn new(role: ChatRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
            created_at: std::time::Instant::now(),
            rendered_cache: None,
            cached_width: 0,
        }
    }

    /// Get or compute the cached markdown rendering for assistant messages.
    /// Width is used for table column sizing — cache invalidates on width change.
    pub fn rendered_lines(&mut self, width: usize) -> &[ratatui::text::Line<'static>] {
        // Invalidate cache if width changed (terminal resize)
        if self.rendered_cache.is_some() && self.cached_width != width {
            self.rendered_cache = None;
        }
        if self.rendered_cache.is_none() {
            self.rendered_cache = Some(crate::markdown::markdown_to_lines_with_width(
                &self.content,
                width,
            ));
            self.cached_width = width;
        }
        self.rendered_cache.as_ref().unwrap()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChatRole {
    User,
    Assistant,
    ToolCall { name: String },
    ToolResult { name: String, is_error: bool },
    System,
}

/// All mutable TUI state. Pure state machine — no I/O, no network.
pub struct App {
    pub focus: Focus,
    pub input_buffer: String,
    pub cursor_pos: usize,
    pub messages: Vec<ChatMessage>,
    pub workstreams: Vec<WorkstreamInfo>,
    pub sessions: Vec<SessionInfo>,
    pub current_workstream: Option<WorkstreamInfo>,
    pub current_session: Option<SessionInfo>,
    pub is_generating: bool,
    pub streaming_text: String,
    pub scroll_offset: usize,
    pub sidebar_section: SidebarSection,
    pub sidebar_ws_index: usize,
    pub sidebar_session_index: usize,
    pub should_quit: bool,
    pub dirty: bool,
    /// Wall-clock instant of the last `terminal.draw()` call. Used to cap
    /// render rate (see `MIN_FRAME_INTERVAL` in event_loop) so a flood of
    /// engine events doesn't melt the render path. Initialized to "long
    /// ago" so the first draw is never throttled.
    pub last_draw: std::time::Instant,
    /// Pending action that requires the event loop to send a WS message.
    pub pending_submit: Option<String>,
    /// True when the user pressed Esc during generation. The event loop
    /// reads this to send a `cancel` RPC to the server, then clears it.
    /// Without this, "cancel" would only flip `is_generating` locally
    /// while the model kept running and produced a duplicate Complete.
    pub pending_cancel: bool,
    /// Session-id-of-cancellation, set alongside `pending_cancel` to a
    /// turn marker. While Some, the WS event handler ignores incoming
    /// stream events (StreamingText / ToolCall / Complete) for that
    /// session — they're stale output from the cancelled turn. Cleared
    /// on next user submit.
    pub cancelled_session: Option<uuid::Uuid>,
    /// Set of message indices where tool results are expanded (show full content).
    pub expanded_tool_results: std::collections::HashSet<usize>,
    /// Current model name (for status bar display).
    pub model_name: String,
    /// Current permission mode label (fetched from server).
    pub permission_mode: String,
    /// Cumulative token usage: (input_tokens, output_tokens).
    pub token_usage: (u64, u64),
    /// When generation started (for elapsed time in status bar).
    pub generation_started: Option<std::time::Instant>,
    /// In-flight `/connect <svc>` OAuth flow: service name + start time.
    /// Renders a heartbeat line above the status bar so the user knows
    /// the app isn't frozen while the browser dance is in progress.
    pub oauth_in_flight: Option<(String, std::time::Instant)>,
    /// Active modal overlay (permission prompt, AskUser, etc.)
    pub active_modal: Option<crate::modal::ModalState>,
    /// Pending modal response to send back to server: (request_id, result_rx)
    pub pending_modal_response: Option<(String, tokio::sync::oneshot::Receiver<Option<usize>>)>,
    /// Spinner animation frame (0-9), ticked by the event loop.
    pub spinner_frame: u8,
    /// Name of the currently executing tool (set on ToolCallStart, cleared on ToolCallResult/Complete).
    pub active_tool: Option<String>,
    /// Panel regions from last render, for mouse hit-testing.
    pub layout: LayoutRegions,
    /// Slash command registry (built-in + cached skills).
    pub command_registry: CommandRegistry,
    /// Active autocomplete dropdown state (None = hidden).
    pub autocomplete: Option<AutocompleteState>,
    /// Pending command result that needs WS interaction (inventory query, skill invoke).
    pub pending_command: Option<CommandResult>,
    /// Submitted user prompts in chronological order. Drives Up/Down recall
    /// and the double-Esc history modal. Per-session, in-memory only.
    /// Each entry is `(text, is_chat)` — slash commands have `is_chat = false`
    /// so the branch modal can filter them out (only chat prompts correspond
    /// to session turns and are branchable).
    pub history: Vec<HistoryEntry>,
    /// Index into `history` while the user is browsing prior prompts via
    /// Up/Down. `None` = not browsing (Up/Down scrolls chat as before).
    /// `Some(i)` = currently showing `history[i]` in the input.
    pub history_cursor: Option<usize>,
    /// In-progress draft saved when the user enters history-browsing mode,
    /// restored when they exit (Down past the most recent entry).
    pub history_draft: String,
    /// Wall-clock instant of the most recent Esc press. Used to detect a
    /// double-Esc (within `DOUBLE_ESC_WINDOW`) → opens the history modal.
    pub last_esc_at: Option<std::time::Instant>,
    /// Active interactive ceremony overlay (priorities or diary) for the
    /// `/week` and `/retro` flows. Captures all key input while present;
    /// see `crate::ceremony_modal`. Distinct from `active_modal` because
    /// these surfaces need multi-key bindings the oneshot-driven modal
    /// can't express.
    pub ceremony_overlay: Option<crate::ceremony_modal::CeremonyOverlay>,
    /// Set when a `ceremony_event` ServerNotice arrives that touches the
    /// active overlay's tablet. The event loop drains this after each
    /// WS event batch and runs a refresh RPC. None = no refresh pending.
    pub pending_ceremony_refresh: bool,
    /// `/todo` modal (I-0049 T-0314). Independent of ceremony_overlay
    /// — the user can have only one open at a time.
    pub todo_overlay: Option<crate::todo_modal::TodoModalState>,
    /// Set when a `todo_event` ServerNotice arrives. Triggers a
    /// re-fetch of the open todo list and re-render.
    pub pending_todo_refresh: bool,
    /// `/watch` (no args) registration modal (ARAWN-I-0058). Mutually
    /// exclusive with the other overlays — one at a time.
    pub watch_overlay: Option<crate::watch_modal::WatchModalState>,
    /// I-0035 Phase 2 (T-0354): cached markdown for the empty-chat
    /// brief surface. Populated by the event loop on session
    /// start / switch via the same fetch path the `/brief` command
    /// uses. `Some(_)` with non-empty content → render the brief in
    /// the idle hero area; `None` (or empty) → fall through to the
    /// T-0331 welcome.
    pub brief_markdown: Option<String>,
    /// I-0035 Phase 3 T-B (T-0357): cached parsed `DailyView` for
    /// the dashboard's compact brief section. Populated alongside
    /// `brief_markdown` at session start. `None` matches the
    /// pre-onboarding / no-tablet state.
    pub daily_view: Option<arawn_ceremonies::DailyView>,
    /// I-0035 Phase 4 T-A (T-0359): queue of ephemeral 1-line
    /// toasts. Rendered above the status bar when non-empty.
    /// Each toast decays after its TTL; oldest is dropped first.
    pub toast_queue: std::collections::VecDeque<crate::toast::Toast>,
    /// I-0035 Phase 4 T-B (T-0360): set when a `briefing_ready`
    /// ServerNotice arrives. The event loop drains this and re-runs
    /// the brief fetch + cache the next tick.
    pub pending_brief_refresh: bool,
}

/// Window for double-Esc detection. Two Esc presses inside this opens
/// the history modal; longer than this is treated as two independent
/// Esc presses (the second one cancels whatever's transient).
pub const DOUBLE_ESC_WINDOW: std::time::Duration = std::time::Duration::from_millis(500);

/// One entry in the per-session input history.
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryEntry {
    pub text: String,
    /// True if this submission was a plain chat prompt that produced a
    /// session turn. False for slash commands (`/integrations`, `/clear`,
    /// etc.) — those are recallable via Up arrow but aren't branchable
    /// because they don't correspond to messages on the server.
    pub is_chat: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            focus: Focus::Main,
            input_buffer: String::new(),
            cursor_pos: 0,
            messages: Vec::new(),
            workstreams: Vec::new(),
            sessions: Vec::new(),
            current_workstream: None,
            current_session: None,
            is_generating: false,
            streaming_text: String::new(),
            scroll_offset: 0,
            sidebar_section: SidebarSection::Workstreams,
            sidebar_ws_index: 0,
            sidebar_session_index: 0,
            should_quit: false,
            dirty: true,
            last_draw: std::time::Instant::now() - std::time::Duration::from_secs(60),
            pending_submit: None,
            pending_cancel: false,
            cancelled_session: None,
            expanded_tool_results: std::collections::HashSet::new(),
            model_name: String::new(),
            permission_mode: "ask".into(),
            token_usage: (0, 0),
            active_modal: None,
            pending_modal_response: None,
            generation_started: None,
            oauth_in_flight: None,
            spinner_frame: 0,
            active_tool: None,
            layout: LayoutRegions::default(),
            command_registry: CommandRegistry::new(),
            autocomplete: None,
            pending_command: None,
            history: Vec::new(),
            history_cursor: None,
            history_draft: String::new(),
            last_esc_at: None,
            ceremony_overlay: None,
            pending_ceremony_refresh: false,
            todo_overlay: None,
            pending_todo_refresh: false,
            watch_overlay: None,
            brief_markdown: None,
            daily_view: None,
            toast_queue: std::collections::VecDeque::new(),
            pending_brief_refresh: false,
        }
    }














    fn prev_char_boundary(&self) -> usize {
        let mut pos = self.cursor_pos.saturating_sub(1);
        while pos > 0 && !self.input_buffer.is_char_boundary(pos) {
            pos -= 1;
        }
        pos
    }

    fn next_char_boundary(&self) -> usize {
        let mut pos = self.cursor_pos + 1;
        while pos < self.input_buffer.len() && !self.input_buffer.is_char_boundary(pos) {
            pos += 1;
        }
        pos
    }
}

/// Format tool input args into a compact display string.
pub fn format_tool_input(tool_name: &str, input: &serde_json::Value) -> String {
    match tool_name {
        "shell" | "Bash" => input
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        "file_read" | "Read" | "FileRead" => input
            .get("path")
            .or_else(|| input.get("file_path"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        "file_write" | "Write" | "FileWrite" => input
            .get("path")
            .or_else(|| input.get("file_path"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        "file_edit" | "Edit" | "FileEdit" => input
            .get("path")
            .or_else(|| input.get("file_path"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        "grep" | "Grep" => input
            .get("pattern")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        "glob" | "Glob" => input
            .get("pattern")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        _ => {
            // Generic: show first string field value, truncated
            if let Some(obj) = input.as_object() {
                for (_k, v) in obj {
                    if let Some(s) = v.as_str() {
                        let truncated = if s.len() > 60 { &s[..60] } else { s };
                        return truncated.to_string();
                    }
                }
            }
            String::new()
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

/// T-0363 — pick the default `/export` path when the user invokes
/// `/export` with no arg:
/// `$HOME/.arawn/exports/<workstream>-<session-short>-<YYYYMMDD-HHMM>.md`.
/// Falls back to the current directory when `$HOME` is unset.
pub(super) fn default_export_path(app: &App) -> std::path::PathBuf {
    let home = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    let workstream = app
        .current_workstream
        .as_ref()
        .map(|w| w.name.clone())
        .unwrap_or_else(|| "scratch".to_string());
    let session_short = app
        .current_session
        .as_ref()
        .map(|s| s.id.to_string().chars().take(8).collect::<String>())
        .unwrap_or_else(|| "session".to_string());
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M");
    home.join(".arawn")
        .join("exports")
        .join(format!("{workstream}-{session_short}-{stamp}.md"))
}

/// T-0363 — expand a leading `~` in a path to `$HOME`. Doesn't
/// touch other tilde forms (`~user/...`); the goal is just to
/// accept `~/notes/foo.md` from the CLI naturally.
pub(super) fn shellexpand_tilde(input: &str) -> String {
    if let Some(rest) = input.strip_prefix("~/")
        && let Ok(home) = std::env::var("HOME")
    {
        return format!("{home}/{rest}");
    }
    if input == "~"
        && let Ok(home) = std::env::var("HOME")
    {
        return home;
    }
    input.to_string()
}

/// T-0363 — render the full transcript as a markdown document.
/// YAML frontmatter carries workstream, session id, timestamp, and
/// the message count; the body emits one `## <role>\n\n<content>`
/// block per message. Tool calls and tool results are skipped —
/// they're transient bookkeeping the user doesn't want preserved
/// in an export.
pub(super) fn render_conversation_markdown(app: &App) -> String {
    let workstream = app
        .current_workstream
        .as_ref()
        .map(|w| w.name.as_str())
        .unwrap_or("scratch");
    let session_id = app
        .current_session
        .as_ref()
        .map(|s| s.id.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let generated_at = chrono::Utc::now().to_rfc3339();
    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(&format!("workstream: {workstream}\n"));
    out.push_str(&format!("session_id: {session_id}\n"));
    out.push_str(&format!("generated_at: {generated_at}\n"));
    out.push_str(&format!("message_count: {}\n", app.messages.len()));
    out.push_str("---\n\n");
    out.push_str(&format!("# Conversation — {workstream}\n\n"));
    for m in &app.messages {
        let role = match &m.role {
            ChatRole::User => "User",
            ChatRole::Assistant => "Assistant",
            ChatRole::System => "System",
            // Tool calls / results are noise in an exported document.
            ChatRole::ToolCall { .. } | ChatRole::ToolResult { .. } => continue,
        };
        out.push_str(&format!("## {role}\n\n"));
        out.push_str(m.content.trim());
        out.push_str("\n\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::Action;

    #[test]
    fn type_chars_updates_buffer() {
        let mut app = App::new();
        app.handle_action(Action::TypeChar('h'));
        app.handle_action(Action::TypeChar('i'));
        assert_eq!(app.input_buffer, "hi");
        assert_eq!(app.cursor_pos, 2);
    }

    #[test]
    fn backspace_removes_char() {
        let mut app = App::new();
        app.handle_action(Action::TypeChar('a'));
        app.handle_action(Action::TypeChar('b'));
        app.handle_action(Action::Backspace);
        assert_eq!(app.input_buffer, "a");
        assert_eq!(app.cursor_pos, 1);
    }

    #[test]
    fn submit_moves_to_messages() {
        let mut app = App::new();
        app.handle_action(Action::TypeChar('h'));
        app.handle_action(Action::TypeChar('i'));
        app.handle_action(Action::Submit);

        assert_eq!(app.input_buffer, "");
        assert_eq!(app.cursor_pos, 0);
        assert!(app.is_generating);
        assert_eq!(app.messages.len(), 1);
        assert_eq!(app.messages[0].content, "hi");
        assert_eq!(app.pending_submit, Some("hi".into()));
    }

    #[test]
    fn submit_blocked_when_empty() {
        let mut app = App::new();
        let changed = app.handle_action(Action::Submit);
        assert!(!changed);
        assert!(app.messages.is_empty());
        assert!(!app.is_generating);
    }

    #[test]
    fn submit_blocked_while_generating() {
        let mut app = App::new();
        app.is_generating = true;
        app.handle_action(Action::TypeChar('x'));
        // TypeChar blocked during generation
        assert_eq!(app.input_buffer, "");
    }

    #[test]
    fn tab_toggles_focus() {
        let mut app = App::new();
        assert_eq!(app.focus, Focus::Main);
        app.handle_action(Action::Tab);
        assert_eq!(app.focus, Focus::Sidebar);
        app.handle_action(Action::Tab);
        assert_eq!(app.focus, Focus::Main);
    }

    #[test]
    fn scroll_updates_offset() {
        let mut app = App::new();
        app.handle_action(Action::ScrollUp);
        assert_eq!(app.scroll_offset, 1);
        app.handle_action(Action::ScrollUp);
        assert_eq!(app.scroll_offset, 2);
        app.handle_action(Action::ScrollDown);
        assert_eq!(app.scroll_offset, 1);
    }

    #[test]
    fn cancel_stops_generation() {
        let mut app = App::new();
        app.is_generating = true;
        app.streaming_text = "partial response".into();
        app.handle_action(Action::Cancel);
        assert!(!app.is_generating);
        assert!(app.streaming_text.is_empty());
        assert_eq!(app.messages.len(), 1);
        assert!(app.messages[0].content.contains("cancelled"));
    }

    #[test]
    fn quit_sets_flag() {
        let mut app = App::new();
        app.handle_action(Action::Quit);
        assert!(app.should_quit);
    }

    #[test]
    fn cursor_movement() {
        let mut app = App::new();
        app.handle_action(Action::TypeChar('a'));
        app.handle_action(Action::TypeChar('b'));
        app.handle_action(Action::TypeChar('c'));
        assert_eq!(app.cursor_pos, 3);

        app.handle_action(Action::CursorLeft);
        assert_eq!(app.cursor_pos, 2);

        app.handle_action(Action::CursorHome);
        assert_eq!(app.cursor_pos, 0);

        app.handle_action(Action::CursorEnd);
        assert_eq!(app.cursor_pos, 3);

        // Insert at middle
        app.handle_action(Action::CursorHome);
        app.handle_action(Action::CursorRight);
        app.handle_action(Action::TypeChar('X'));
        assert_eq!(app.input_buffer, "aXbc");
    }

    // --- Integration tests: full conversation flow via EventUpdate ---

    #[test]
    fn full_conversation_flow() {
        use crate::ws_client::EventUpdate;

        let mut app = App::new();

        // User types and submits
        app.handle_action(Action::TypeChar('h'));
        app.handle_action(Action::TypeChar('e'));
        app.handle_action(Action::TypeChar('l'));
        app.handle_action(Action::TypeChar('l'));
        app.handle_action(Action::TypeChar('o'));
        app.handle_action(Action::Submit);

        assert!(app.is_generating);
        assert_eq!(app.messages.len(), 1);
        assert_eq!(app.messages[0].role, ChatRole::User);

        // Streaming text arrives
        app.apply_engine_event(EventUpdate::AppendStreamingText("Hi ".into()));
        app.apply_engine_event(EventUpdate::AppendStreamingText("there!".into()));
        assert_eq!(app.streaming_text, "Hi there!");
        assert!(app.is_generating);

        // Complete
        app.apply_engine_event(EventUpdate::Complete("Hi there!".into()));
        assert!(!app.is_generating);
        assert!(app.streaming_text.is_empty());
        assert_eq!(app.messages.len(), 2);
        assert_eq!(app.messages[1].role, ChatRole::Assistant);
        assert_eq!(app.messages[1].content, "Hi there!");
    }

    #[test]
    fn tool_call_flow() {
        use crate::ws_client::EventUpdate;

        let mut app = App::new();
        app.is_generating = true;

        // Tool call start
        app.apply_engine_event(EventUpdate::AddToolCall {
            id: "c1".into(),
            name: "shell".into(),
            input: serde_json::json!({"command": "ls -la"}),
        });
        assert_eq!(app.messages.len(), 1);
        assert!(matches!(&app.messages[0].role, ChatRole::ToolCall { name } if name == "shell"));

        // Tool call result
        app.apply_engine_event(EventUpdate::AddToolResult {
            id: "c1".into(),
            content: "file1.rs\nfile2.rs".into(),
            is_error: false,
        });
        assert_eq!(app.messages.len(), 2);
        assert!(
            matches!(&app.messages[1].role, ChatRole::ToolResult { name, is_error } if name == "shell" && !is_error)
        );

        // Complete
        app.apply_engine_event(EventUpdate::Complete("Here are the files.".into()));
        assert!(!app.is_generating);
        assert_eq!(app.messages.len(), 3);
        assert_eq!(app.messages[2].content, "Here are the files.");
    }

    #[test]
    fn error_event_clears_generating() {
        use crate::ws_client::EventUpdate;

        let mut app = App::new();
        app.is_generating = true;
        app.streaming_text = "partial".into();

        app.apply_engine_event(EventUpdate::Error("API error".into()));

        assert!(!app.is_generating);
        assert!(app.streaming_text.is_empty());
        assert_eq!(app.messages.len(), 1);
        assert!(app.messages[0].content.contains("API error"));
        assert_eq!(app.messages[0].role, ChatRole::System);
    }

    #[test]
    fn sidebar_navigation() {
        use arawn_service::WorkstreamInfo;
        use chrono::Utc;
        use std::path::PathBuf;
        use uuid::Uuid;

        let mut app = App::new();
        app.focus = Focus::Sidebar;
        app.sidebar_section = SidebarSection::Workstreams;
        app.workstreams = vec![
            WorkstreamInfo {
                id: Uuid::new_v4(),
                name: "scratch".into(),
                root_dir: PathBuf::from("/tmp/a"),
                created_at: Utc::now(),
            },
            WorkstreamInfo {
                id: Uuid::new_v4(),
                name: "project".into(),
                root_dir: PathBuf::from("/tmp/b"),
                created_at: Utc::now(),
            },
        ];

        assert_eq!(app.sidebar_ws_index, 0);
        app.handle_action(Action::SidebarDown);
        assert_eq!(app.sidebar_ws_index, 1);
        app.handle_action(Action::SidebarDown);
        assert_eq!(app.sidebar_ws_index, 1); // clamped
        app.handle_action(Action::SidebarUp);
        assert_eq!(app.sidebar_ws_index, 0);
    }

    fn submit_via_input(app: &mut App, text: &str) {
        app.input_buffer = text.into();
        app.cursor_pos = text.chars().count();
        app.handle_action(Action::Submit);
        // Reset transient state the event loop would otherwise drive.
        app.pending_submit = None;
        app.is_generating = false;
    }

    fn history_text(app: &App) -> Vec<&str> {
        app.history.iter().map(|e| e.text.as_str()).collect()
    }

    #[test]
    fn history_records_submitted_prompts() {
        let mut app = App::new();
        submit_via_input(&mut app, "hello");
        submit_via_input(&mut app, "world");
        assert_eq!(history_text(&app), vec!["hello", "world"]);
        assert!(app.history.iter().all(|e| e.is_chat));
    }

    #[test]
    fn history_records_slash_commands_with_is_chat_false() {
        let mut app = App::new();
        submit_via_input(&mut app, "/integrations");
        submit_via_input(&mut app, "hello");
        submit_via_input(&mut app, "/clear");
        assert_eq!(history_text(&app), vec!["/integrations", "hello", "/clear"]);
        assert_eq!(
            app.history.iter().map(|e| e.is_chat).collect::<Vec<_>>(),
            vec![false, true, false]
        );
    }

    #[test]
    fn history_dedupes_consecutive_duplicates() {
        let mut app = App::new();
        submit_via_input(&mut app, "same");
        submit_via_input(&mut app, "same");
        submit_via_input(&mut app, "different");
        submit_via_input(&mut app, "same");
        assert_eq!(history_text(&app), vec!["same", "different", "same"]);
    }

    #[test]
    fn branch_modal_filters_out_slash_commands() {
        let mut app = App::new();
        submit_via_input(&mut app, "first chat");
        submit_via_input(&mut app, "/integrations");
        submit_via_input(&mut app, "second chat");
        submit_via_input(&mut app, "/clear");
        // Trigger double-Esc to open the branch modal.
        app.handle_action(Action::EscapeIdle);
        app.handle_action(Action::EscapeIdle);
        let modal = app.active_modal.as_ref().expect("modal should be open");
        // Only the two chat entries should appear, newest first.
        assert_eq!(modal.options.len(), 2);
        assert!(modal.options[0].label.contains("second chat"));
        assert!(modal.options[1].label.contains("first chat"));
    }

    #[test]
    fn branch_modal_skipped_when_no_chat_history() {
        let mut app = App::new();
        // Only slash commands — no chat prompts to branch from.
        submit_via_input(&mut app, "/integrations");
        submit_via_input(&mut app, "/clear");
        app.handle_action(Action::EscapeIdle);
        app.handle_action(Action::EscapeIdle);
        assert!(app.active_modal.is_none());
    }

    #[test]
    fn up_arrow_recalls_most_recent_when_input_empty() {
        let mut app = App::new();
        submit_via_input(&mut app, "first");
        submit_via_input(&mut app, "second");
        // Up should load "second"
        app.handle_action(Action::ScrollUp);
        assert_eq!(app.input_buffer, "second");
        assert_eq!(app.history_cursor, Some(1));
        // Up again loads "first"
        app.handle_action(Action::ScrollUp);
        assert_eq!(app.input_buffer, "first");
        // Up at oldest stays put
        app.handle_action(Action::ScrollUp);
        assert_eq!(app.input_buffer, "first");
        assert_eq!(app.history_cursor, Some(0));
    }

    #[test]
    fn down_arrow_restores_draft_past_newest() {
        let mut app = App::new();
        submit_via_input(&mut app, "old");
        // User starts typing then hits Up — draft saved
        app.input_buffer = "draft in progress".into();
        app.cursor_pos = app.input_buffer.chars().count();
        // Up arrow with non-empty input should NOT recall (falls through to scroll)
        app.handle_action(Action::ScrollUp);
        assert_eq!(app.input_buffer, "draft in progress");
        // Clear input so Up starts history mode
        app.input_buffer.clear();
        app.cursor_pos = 0;
        app.handle_action(Action::ScrollUp);
        assert_eq!(app.input_buffer, "old");
        // Down past newest restores empty draft (we cleared it)
        app.handle_action(Action::ScrollDown);
        assert_eq!(app.input_buffer, "");
        assert_eq!(app.history_cursor, None);
    }

    #[test]
    fn double_esc_within_window_opens_history_modal() {
        let mut app = App::new();
        submit_via_input(&mut app, "a");
        submit_via_input(&mut app, "b");
        // First Esc — no modal yet
        app.handle_action(Action::EscapeIdle);
        assert!(app.active_modal.is_none());
        assert!(app.last_esc_at.is_some());
        // Second Esc immediately — opens modal
        app.handle_action(Action::EscapeIdle);
        assert!(app.active_modal.is_some());
        assert!(app.last_esc_at.is_none());
    }

    #[test]
    fn double_esc_outside_window_does_not_open_modal() {
        let mut app = App::new();
        submit_via_input(&mut app, "a");
        app.handle_action(Action::EscapeIdle);
        // Pretend a long time passed
        app.last_esc_at = Some(std::time::Instant::now() - std::time::Duration::from_secs(2));
        app.handle_action(Action::EscapeIdle);
        assert!(app.active_modal.is_none());
    }

    #[test]
    fn history_recall_at_loads_entry_into_input() {
        let mut app = App::new();
        submit_via_input(&mut app, "alpha");
        submit_via_input(&mut app, "bravo");
        submit_via_input(&mut app, "charlie");
        app.handle_action(Action::HistoryRecallAt(1));
        assert_eq!(app.input_buffer, "bravo");
        assert_eq!(app.history_cursor, Some(1));
    }

    #[test]
    fn empty_history_modal_is_a_no_op() {
        let mut app = App::new();
        // Trigger double-Esc with no history — should not open a modal
        app.handle_action(Action::EscapeIdle);
        app.handle_action(Action::EscapeIdle);
        assert!(app.active_modal.is_none());
    }

    #[test]
    fn modal_select_index_picks_option_directly() {
        let mut app = App::new();
        // Open a modal with three options (we use the history modal path
        // since it's local-only and doesn't need a ws RPC).
        for label in &["alpha", "bravo", "charlie"] {
            submit_via_input(&mut app, label);
        }
        app.handle_action(Action::EscapeIdle);
        app.handle_action(Action::EscapeIdle);
        let modal = app.active_modal.as_ref().expect("modal should be open");
        assert_eq!(modal.options.len(), 3);
        assert_eq!(modal.focused_index, 0); // before the action

        // Direct-select option index 2 (third in the list — "alpha", since
        // the modal renders newest-first: bravo bravo charlie order is
        // reversed to charlie/bravo/alpha — so index 2 = "alpha").
        app.handle_action(Action::ModalSelectIndex(2));
        // Modal closes after selection; the prompt at history index 0
        // ("alpha") should now be loaded into the input buffer via the
        // event-loop side of the branch flow, but at the App-only level
        // we just assert the modal closed.
        assert!(app.active_modal.is_none());
    }

    #[test]
    fn cancel_marks_session_for_stale_event_drop() {
        let mut app = App::new();
        // Set up an in-progress generation on a session.
        let session = SessionInfo {
            id: uuid::Uuid::new_v4(),
            workstream_id: None,
            created_at: chrono::Utc::now(),
        };
        app.current_session = Some(session.clone());
        app.is_generating = true;
        app.streaming_text = "partial...".into();

        app.handle_action(Action::Cancel);

        assert!(!app.is_generating, "cancel must clear is_generating");
        assert!(app.pending_cancel, "cancel must request RPC dispatch");
        assert_eq!(
            app.cancelled_session,
            Some(session.id),
            "cancel must mark the session so the event loop drops stale stream events"
        );
        // The streaming buffer was flushed into a "(cancelled)" message.
        assert!(app.streaming_text.is_empty());
        assert!(matches!(
            app.messages.last().map(|m| &m.role),
            Some(ChatRole::Assistant)
        ));
    }

    #[test]
    fn next_submit_clears_cancelled_session_marker() {
        let mut app = App::new();
        let session = SessionInfo {
            id: uuid::Uuid::new_v4(),
            workstream_id: None,
            created_at: chrono::Utc::now(),
        };
        app.current_session = Some(session.clone());
        app.cancelled_session = Some(session.id);

        // Submit a fresh message — the cancelled marker should clear so
        // stream events for this NEW turn render normally.
        submit_via_input(&mut app, "fresh prompt");
        assert!(app.cancelled_session.is_none());
    }

    #[test]
    fn modal_select_out_of_range_is_no_op() {
        let mut app = App::new();
        submit_via_input(&mut app, "only one");
        app.handle_action(Action::EscapeIdle);
        app.handle_action(Action::EscapeIdle);
        assert!(app.active_modal.is_some());

        // Modal has 1 option; pressing `5` should not confirm anything.
        app.handle_action(Action::ModalSelectIndex(4));
        assert!(
            app.active_modal.is_some(),
            "out-of-range index must not close the modal"
        );
    }

    // T-0361 — `/copy` behavior tests.

    #[test]
    fn copy_last_response_posts_toast_with_assistant_text() {
        let mut app = App::new();
        app.messages
            .push(ChatMessage::new(ChatRole::User, "what's the postgres rfc?"));
        app.messages.push(ChatMessage::new(
            ChatRole::Assistant,
            "Alice wants multi-AZ async + 4h PITR.",
        ));
        app.handle_copy_last_response();
        assert_eq!(app.toast_queue.len(), 1);
        let toast = app.toast_queue.front().unwrap();
        assert!(toast.message.contains("Copied last response"));
        assert!(toast.message.contains("chars"));
        assert_eq!(toast.level, crate::toast::ToastLevel::Info);
    }

    #[test]
    fn copy_last_response_warns_when_no_assistant_messages() {
        let mut app = App::new();
        app.messages
            .push(ChatMessage::new(ChatRole::User, "hi"));
        app.handle_copy_last_response();
        assert_eq!(app.toast_queue.len(), 1);
        let toast = app.toast_queue.front().unwrap();
        assert!(toast.message.contains("No assistant response"));
        assert_eq!(toast.level, crate::toast::ToastLevel::Warn);
    }

    // T-0363 — `/export` behavior tests.

    #[test]
    fn export_warns_on_empty_transcript() {
        let mut app = App::new();
        app.handle_export_conversation(None);
        assert_eq!(app.toast_queue.len(), 1);
        let toast = app.toast_queue.front().unwrap();
        assert!(toast.message.contains("Nothing to export"));
        assert_eq!(toast.level, crate::toast::ToastLevel::Warn);
    }

    #[test]
    fn export_writes_markdown_to_explicit_path() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("subdir").join("conv.md");
        let mut app = App::new();
        app.messages
            .push(ChatMessage::new(ChatRole::User, "what's the postgres rfc?"));
        app.messages.push(ChatMessage::new(
            ChatRole::Assistant,
            "Alice wants multi-AZ async + 4h PITR.",
        ));
        app.handle_export_conversation(Some(target.display().to_string()));
        assert!(target.exists(), "export file should be created");
        let body = std::fs::read_to_string(&target).unwrap();
        assert!(body.starts_with("---\n"));
        assert!(body.contains("message_count: 2"));
        assert!(body.contains("## User"));
        assert!(body.contains("## Assistant"));
        assert!(body.contains("postgres rfc"));
        assert!(body.contains("multi-AZ async"));

        // Toast confirms with the absolute path.
        let toast = app.toast_queue.front().unwrap();
        assert!(toast.message.contains("Exported to"));
        assert!(toast.message.contains("conv.md"));
        assert_eq!(toast.level, crate::toast::ToastLevel::Info);
    }

    #[test]
    fn export_skips_tool_call_and_tool_result_rows() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("conv.md");
        let mut app = App::new();
        app.messages
            .push(ChatMessage::new(ChatRole::User, "list files"));
        app.messages.push(ChatMessage::new(
            ChatRole::ToolCall { name: "shell".into() },
            "ls -la",
        ));
        app.messages.push(ChatMessage::new(
            ChatRole::ToolResult {
                name: "shell".into(),
                is_error: false,
            },
            "Cargo.toml\nsrc",
        ));
        app.messages
            .push(ChatMessage::new(ChatRole::Assistant, "two files."));
        app.handle_export_conversation(Some(target.display().to_string()));
        let body = std::fs::read_to_string(&target).unwrap();
        assert!(!body.contains("## ToolCall"));
        assert!(!body.contains("## ToolResult"));
        assert!(!body.contains("ls -la"));
        assert!(body.contains("## User"));
        assert!(body.contains("## Assistant"));
    }

    #[test]
    fn shellexpand_tilde_expands_home() {
        // SAFETY: tests scope to this process; restore HOME after.
        let orig_home = std::env::var("HOME").ok();
        unsafe { std::env::set_var("HOME", "/h") };
        assert_eq!(shellexpand_tilde("~/x/y"), "/h/x/y");
        assert_eq!(shellexpand_tilde("~"), "/h");
        assert_eq!(shellexpand_tilde("/abs/path"), "/abs/path");
        assert_eq!(shellexpand_tilde("rel/path"), "rel/path");
        if let Some(h) = orig_home {
            unsafe { std::env::set_var("HOME", h) };
        } else {
            unsafe { std::env::remove_var("HOME") };
        }
    }

    #[test]
    fn copy_last_response_picks_most_recent_assistant_turn() {
        let mut app = App::new();
        app.messages
            .push(ChatMessage::new(ChatRole::Assistant, "first"));
        app.messages
            .push(ChatMessage::new(ChatRole::User, "follow-up"));
        app.messages
            .push(ChatMessage::new(ChatRole::Assistant, "second"));
        app.handle_copy_last_response();
        // Char count: "second" = 6 chars. Message includes the count.
        let toast = app.toast_queue.front().unwrap();
        assert!(toast.message.contains("(6 chars)"));
    }
}
