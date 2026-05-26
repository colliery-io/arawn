use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};

use crate::app::{App, Focus, LayoutRegions};

const SPINNER_FRAMES: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

/// I-0035 Phase 3 (T-0356) — fixed width for the right-pane
/// dashboard.
const DASHBOARD_WIDTH: u16 = 28;

/// Minimum terminal width to render the three-pane layout. Below
/// this we fall back to the existing two-pane layout (no dashboard).
/// Chosen so the chat pane keeps ~40 cells of breathing room even
/// after the sidebar (~24 cells at 20% of 120) + dashboard (28)
/// consume their share.
const MIN_FOR_THREE_PANE: u16 = 100;

/// Render function. Draws to Frame and updates app.layout for mouse hit-testing.

mod chat;
mod dashboard;
mod input;
mod overlays;
mod sidebar;
mod status_bar;

use chat::{render_chat, render_separator};
use dashboard::render_dashboard_pane;
use input::{render_autocomplete, render_input};
use overlays::{render_oauth_heartbeat, render_toast_bar};
use sidebar::{render_sidebar, render_sidebar_tab};
use status_bar::render_status_bar;

pub fn render(app: &mut App, frame: &mut Frame) {
    let area = frame.area();

    // Auto-clear an OAuth heartbeat that's been waiting longer than the
    // server-side 5-minute timeout — if no `[integration]` notice ever
    // arrived (network drop, cancelled in browser), the line should fade.
    if let Some((_, started)) = &app.oauth_in_flight
        && started.elapsed() >= std::time::Duration::from_secs(300)
    {
        app.oauth_in_flight = None;
    }

    let oauth_row = if app.oauth_in_flight.is_some() { 1 } else { 0 };
    // I-0035 Phase 4 T-A: drop expired toasts lazily before peeking
    // the front. The toast row only renders when a non-expired toast
    // exists at the head of the queue.
    crate::toast::drop_expired(&mut app.toast_queue, std::time::Instant::now());
    let toast_row = if app.toast_queue.front().is_some() {
        1
    } else {
        0
    };
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),            // chat area
            Constraint::Length(1),         // thin separator
            Constraint::Length(1),         // input (single line, borderless)
            Constraint::Length(toast_row), // toast (0 or 1 row)
            Constraint::Length(oauth_row), // OAuth heartbeat (0 or 1 row)
            Constraint::Length(1),         // status bar (bottom)
        ])
        .split(area);

    // I-0035 Phase 3: dashboard pane renders to the right of chat
    // when the terminal is wide enough. Width budget = sidebar
    // (3 or ~20%) + chat (min ~40) + dashboard (28).
    let show_dashboard = vertical[0].width >= MIN_FOR_THREE_PANE;
    let dashboard_width = if show_dashboard {
        DASHBOARD_WIDTH
    } else {
        0
    };

    if app.focus == Focus::Sidebar {
        let mut constraints = vec![Constraint::Percentage(20), Constraint::Min(1)];
        if show_dashboard {
            constraints.push(Constraint::Length(dashboard_width));
        }
        let middle = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(constraints)
            .split(vertical[0]);

        render_sidebar(app, frame, middle[0]);
        render_chat(app, frame, middle[1]);
        let dashboard_rect = if show_dashboard {
            render_dashboard_pane(app, frame, middle[2]);
            Some(middle[2])
        } else {
            None
        };

        app.layout = LayoutRegions {
            sidebar: Some(middle[0]),
            sidebar_ws: Some(middle[0]),
            sidebar_tab: None,
            chat: middle[1],
            input: vertical[2],
            dashboard: dashboard_rect,
        };
    } else {
        let mut constraints = vec![Constraint::Length(3), Constraint::Min(1)];
        if show_dashboard {
            constraints.push(Constraint::Length(dashboard_width));
        }
        let middle = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(constraints)
            .split(vertical[0]);

        render_sidebar_tab(frame, middle[0]);
        render_chat(app, frame, middle[1]);
        let dashboard_rect = if show_dashboard {
            render_dashboard_pane(app, frame, middle[2]);
            Some(middle[2])
        } else {
            None
        };

        app.layout = LayoutRegions {
            sidebar: None,
            sidebar_ws: None,
            sidebar_tab: Some(middle[0]),
            chat: middle[1],
            input: vertical[2],
            dashboard: dashboard_rect,
        };
    }

    render_separator(frame, vertical[1]);

    render_input(app, frame, vertical[2]);
    if toast_row == 1 {
        render_toast_bar(app, frame, vertical[3]);
    }
    if oauth_row == 1 {
        render_oauth_heartbeat(app, frame, vertical[4]);
    }
    render_status_bar(app, frame, vertical[5]);

    // Autocomplete dropdown (renders above the input line)
    if let Some(ref ac) = app.autocomplete {
        render_autocomplete(ac, frame, vertical[2]);
    }

    // Modal overlay (renders on top of everything)
    if let Some(ref modal) = app.active_modal {
        crate::modal::render_modal(modal, frame);
    }
    // Ceremony overlays (priority list / diary editor) render above
    // standard modals. Only one can be active at a time.
    if let Some(ref overlay) = app.ceremony_overlay {
        crate::ceremony_modal::render_overlay(overlay, frame);
    }
    if let Some(ref overlay) = app.todo_overlay {
        crate::todo_modal::render_todo_modal(overlay, frame);
    }
    if let Some(ref overlay) = app.watch_overlay {
        crate::watch_modal::render_watch_modal(overlay, frame);
    }
}





















/// Truncate a string to fit within a display width, adding "…" if needed.
pub(super) fn truncate_to(s: &str, max_cells: usize) -> String {
    crate::width::truncate_display(s, max_cells)
}

/// Extract a compact summary from tool call content for inline display.
pub(super) fn compact_tool_summary(content: &str) -> String {
    if content.is_empty() {
        return String::new();
    }
    truncate_for_display(content, 60)
}

pub(super) fn truncate_for_display(s: &str, max: usize) -> String {
    // Display-cell width — covers byte-boundary panic, char-count drift
    // for CJK, and zero-width / 2-width edge cases in one shot.
    crate::width::truncate_display(s, max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::dashboard::{detect_conflict, format_brief_date_line, render_dashboard_actions, render_dashboard_brief};
    use crate::app::{App, ChatMessage, ChatRole};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[test]
    fn truncate_for_display_handles_utf8_at_boundary() {
        // 🔥 is 4 bytes wide and 2 display cells. Byte-based slicing
        // panicked at sub-char indices; char-count over-counted because
        // emoji are 2 cells. Display-width is the correct measure.
        let s = "🔥🔥🔥hello";
        let _ = truncate_for_display(s, 1);
        let _ = truncate_for_display(s, 2);
        // Budget 7 cells: 3×🔥 (6 cells) + ellipsis (1) = 7. Fits exactly.
        let out = truncate_for_display(s, 7);
        assert_eq!(out, "🔥🔥🔥…");
    }

    #[test]
    fn truncate_for_display_passes_through_short_strings() {
        assert_eq!(truncate_for_display("hi", 10), "hi");
    }

    // ─── I-0035 Phase 3 T-B (T-0357) — dashboard brief ────────────────────

    fn cal_item(text: &str, start: &str, end: &str) -> arawn_ceremonies::service::ItemDto {
        arawn_ceremonies::service::ItemDto {
            id: format!("item-{start}"),
            tablet_id: "daily-2026-05-19".into(),
            section_key: "calendar".into(),
            ordinal: 0,
            kind: "freeform".into(),
            body: serde_json::json!({
                "text": text,
                "start_ts": start,
                "end_ts": end,
            }),
            citation_id: None,
            done_at: None,
            created_at: start.into(),
        }
    }

    fn daily_view_with_items(
        items: Vec<arawn_ceremonies::service::ItemDto>,
    ) -> arawn_ceremonies::DailyView {
        arawn_ceremonies::DailyView {
            tablet: arawn_ceremonies::service::TabletDto {
                id: "daily-2026-05-19".into(),
                kind: "daily".into(),
                period_key: "2026-05-19".into(),
                generated_at: "2026-05-19T07:00:00Z".into(),
                status: "open".into(),
                workstreams_scanned: serde_json::json!([]),
                priorities_confirmed_at: None,
            recovered: false,
            },
            items,
        }
    }

    #[test]
    fn dashboard_brief_renders_today_with_calendar() {
        let mut app = App::new();
        app.daily_view = Some(daily_view_with_items(vec![
            cal_item("09:00 standup", "2026-05-19T09:00:00Z", "2026-05-19T09:15:00Z"),
            cal_item("13:00 1:1 Jamie", "2026-05-19T13:00:00Z", "2026-05-19T13:30:00Z"),
        ]));
        let mut terminal = Terminal::new(TestBackend::new(28, 12)).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_dashboard_brief(&app, frame, area);
            })
            .unwrap();
        let mut hit_09 = false;
        let mut hit_13 = false;
        let mut hit_brief = false;
        for row in 0..terminal.backend().buffer().area.height {
            let line = buffer_to_string(&terminal, row);
            if line.contains("Brief") {
                hit_brief = true;
            }
            if line.contains("09:00") {
                hit_09 = true;
            }
            if line.contains("13:00") {
                hit_13 = true;
            }
        }
        assert!(hit_brief, "Brief header missing");
        assert!(hit_09, "09:00 standup row missing");
        assert!(hit_13, "13:00 1:1 row missing");
    }

    #[test]
    fn dashboard_brief_flags_conflict() {
        // Two events overlapping 14:00-15:00.
        let items = vec![
            cal_item("14:00 review", "2026-05-19T14:00:00Z", "2026-05-19T15:00:00Z"),
            cal_item("14:30 design", "2026-05-19T14:30:00Z", "2026-05-19T15:30:00Z"),
        ];
        let refs: Vec<&_> = items.iter().collect();
        let line = detect_conflict(&refs).expect("conflict detected");
        assert!(line.contains("conflict"), "{line}");
        assert!(line.contains("14:30"), "{line}");
        assert!(line.contains("15:00"), "{line}");
    }

    #[test]
    fn dashboard_brief_no_conflict_when_separated() {
        let items = vec![
            cal_item("09:00 standup", "2026-05-19T09:00:00Z", "2026-05-19T09:15:00Z"),
            cal_item("13:00 1:1", "2026-05-19T13:00:00Z", "2026-05-19T13:30:00Z"),
        ];
        let refs: Vec<&_> = items.iter().collect();
        assert!(detect_conflict(&refs).is_none());
    }

    #[test]
    fn dashboard_brief_empty_state_no_tablet() {
        let mut app = App::new();
        app.daily_view = None;
        let mut terminal = Terminal::new(TestBackend::new(28, 6)).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_dashboard_brief(&app, frame, area);
            })
            .unwrap();
        let mut hit_placeholder = false;
        for row in 0..terminal.backend().buffer().area.height {
            let line = buffer_to_string(&terminal, row);
            if line.contains("no brief yet") {
                hit_placeholder = true;
            }
        }
        assert!(hit_placeholder, "expected the 'no brief yet' placeholder");
    }

    #[test]
    fn dashboard_brief_empty_state_no_calendar_items() {
        let mut app = App::new();
        app.daily_view = Some(daily_view_with_items(vec![]));
        let mut terminal = Terminal::new(TestBackend::new(28, 6)).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_dashboard_brief(&app, frame, area);
            })
            .unwrap();
        let mut hit_empty = false;
        for row in 0..terminal.backend().buffer().area.height {
            let line = buffer_to_string(&terminal, row);
            if line.contains("no calendar events") {
                hit_empty = true;
            }
        }
        assert!(hit_empty, "expected the 'no calendar events' placeholder");
    }

    #[test]
    fn format_brief_date_line_includes_weekday() {
        // 2026-05-19 is a Tuesday.
        assert_eq!(format_brief_date_line("2026-05-19"), "2026-05-19 · Tue");
    }

    #[test]
    fn format_brief_date_line_fallback_on_garbage() {
        assert_eq!(format_brief_date_line("not-a-date"), "not-a-date");
    }

    // ─── I-0035 Phase 3 T-C (T-0358) — dashboard actions ─────────────────

    fn attn_item(
        ordinal: i32,
        text: &str,
    ) -> arawn_ceremonies::service::ItemDto {
        arawn_ceremonies::service::ItemDto {
            id: format!("attn-{ordinal}"),
            tablet_id: "daily-2026-05-19".into(),
            section_key: "attention".into(),
            ordinal,
            kind: "freeform".into(),
            body: serde_json::json!({"text": text}),
            citation_id: None,
            done_at: None,
            created_at: "2026-05-19T07:00:00Z".into(),
        }
    }

    fn todo_item(
        ordinal: i32,
        text: &str,
    ) -> arawn_ceremonies::service::ItemDto {
        arawn_ceremonies::service::ItemDto {
            id: format!("todo-{ordinal}"),
            tablet_id: "daily-2026-05-19".into(),
            section_key: "todos".into(),
            ordinal,
            kind: "freeform".into(),
            body: serde_json::json!({"text": text}),
            citation_id: None,
            done_at: None,
            created_at: "2026-05-19T07:00:00Z".into(),
        }
    }

    fn daily_view_for_actions(
        items: Vec<arawn_ceremonies::service::ItemDto>,
    ) -> arawn_ceremonies::DailyView {
        arawn_ceremonies::DailyView {
            tablet: arawn_ceremonies::service::TabletDto {
                id: "daily-2026-05-19".into(),
                kind: "daily".into(),
                period_key: "2026-05-19".into(),
                generated_at: "2026-05-19T07:00:00Z".into(),
                status: "open".into(),
                workstreams_scanned: serde_json::json!([]),
                priorities_confirmed_at: None,
            recovered: false,
            },
            items,
        }
    }

    fn draw_actions(app: &App, w: u16, h: u16) -> Terminal<TestBackend> {
        let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_dashboard_actions(app, frame, area);
            })
            .unwrap();
        terminal
    }

    fn buffer_contains(terminal: &Terminal<TestBackend>, needle: &str) -> bool {
        for row in 0..terminal.backend().buffer().area.height {
            if buffer_to_string(terminal, row).contains(needle) {
                return true;
            }
        }
        false
    }

    #[test]
    fn dashboard_actions_renders_attention_items() {
        let mut app = App::new();
        app.daily_view = Some(daily_view_for_actions(vec![
            attn_item(0, "Reply Alice RFC-0042"),
            attn_item(1, "Review PR #482"),
            attn_item(2, "ENG-712 stale 10d"),
        ]));
        let terminal = draw_actions(&app, 28, 8);
        assert!(buffer_contains(&terminal, "Action items"));
        assert!(buffer_contains(&terminal, "Reply Alice"));
        assert!(buffer_contains(&terminal, "Review PR #482"));
        assert!(buffer_contains(&terminal, "ENG-712 stale 10d"));
    }

    #[test]
    fn dashboard_actions_empty_state_no_tablet() {
        let mut app = App::new();
        app.daily_view = None;
        let terminal = draw_actions(&app, 28, 4);
        assert!(buffer_contains(&terminal, "Action items"));
        assert!(buffer_contains(&terminal, "(no action items)"));
    }

    #[test]
    fn dashboard_actions_empty_state_no_attention_items() {
        let mut app = App::new();
        app.daily_view = Some(daily_view_for_actions(vec![]));
        let terminal = draw_actions(&app, 28, 4);
        assert!(buffer_contains(&terminal, "(no action items)"));
    }

    #[test]
    fn dashboard_actions_truncates_long_titles() {
        let long = "This is a very long attention item that will definitely overflow the dashboard width";
        let mut app = App::new();
        app.daily_view = Some(daily_view_for_actions(vec![attn_item(0, long)]));
        let terminal = draw_actions(&app, 28, 4);
        // Truncation marker present somewhere on the row.
        let mut hit_ellipsis = false;
        for row in 0..terminal.backend().buffer().area.height {
            let line = buffer_to_string(&terminal, row);
            if line.contains('…') && line.contains("This is") {
                hit_ellipsis = true;
            }
        }
        assert!(hit_ellipsis, "expected truncation ellipsis on long title");
    }

    #[test]
    fn dashboard_actions_overflow_footer() {
        let mut items = vec![];
        for i in 0..10 {
            items.push(attn_item(i, &format!("item {i}")));
        }
        let mut app = App::new();
        app.daily_view = Some(daily_view_for_actions(items));
        // Only 4 visible rows: header + 2 items + overflow line (one row
        // is reserved for the budget calculation).
        let terminal = draw_actions(&app, 28, 4);
        assert!(buffer_contains(&terminal, "+"));
        assert!(buffer_contains(&terminal, "more"));
    }

    // ─── I-0035 Phase 4 T-A (T-0359) — toast renderer ────────────────────

    #[test]
    fn post_toast_enqueues_message() {
        let mut app = App::new();
        assert!(app.toast_queue.is_empty());
        app.post_toast("hello", crate::toast::ToastLevel::Info);
        assert_eq!(app.toast_queue.len(), 1);
        assert_eq!(app.toast_queue.front().unwrap().message, "hello");
        assert!(app.dirty, "post_toast should mark app dirty");
    }

    #[test]
    fn expired_toast_is_dropped_on_render() {
        use std::time::{Duration, Instant};
        let mut app = App::new();
        let mut t = crate::toast::Toast::new("old", crate::toast::ToastLevel::Info);
        t.posted_at = Instant::now() - Duration::from_secs(10);
        t.ttl = Duration::from_secs(5);
        app.toast_queue.push_back(t);
        let mut terminal = Terminal::new(TestBackend::new(40, 10)).unwrap();
        terminal
            .draw(|frame| {
                super::render(&mut app, frame);
            })
            .unwrap();
        assert!(
            app.toast_queue.is_empty(),
            "expired toast should be dropped during render"
        );
    }

    #[test]
    fn toast_truncates_long_message() {
        let long = "This is a very long toast message that will overflow the toast row width";
        let mut app = App::new();
        app.post_toast(long, crate::toast::ToastLevel::Info);
        let mut terminal = Terminal::new(TestBackend::new(40, 10)).unwrap();
        terminal
            .draw(|frame| {
                super::render(&mut app, frame);
            })
            .unwrap();
        let mut hit_ellipsis = false;
        for row in 0..terminal.backend().buffer().area.height {
            let line = buffer_to_string(&terminal, row);
            if line.contains('…') && line.contains("This is") {
                hit_ellipsis = true;
            }
        }
        assert!(hit_ellipsis, "expected toast truncation ellipsis");
    }

    #[test]
    fn dashboard_actions_carried_over_separator() {
        let mut app = App::new();
        app.daily_view = Some(daily_view_for_actions(vec![
            attn_item(0, "Reply Alice"),
            todo_item(0, "Carried todo"),
        ]));
        let terminal = draw_actions(&app, 28, 8);
        assert!(buffer_contains(&terminal, "Reply Alice"));
        assert!(buffer_contains(&terminal, "Carried over"));
        assert!(buffer_contains(&terminal, "Carried todo"));
    }

    fn buffer_to_string(terminal: &Terminal<TestBackend>, row: u16) -> String {
        let width = terminal.backend().buffer().area.width;
        (0..width)
            .map(|x| {
                terminal
                    .backend()
                    .buffer()
                    .cell((x, row))
                    .unwrap()
                    .symbol()
                    .chars()
                    .next()
                    .unwrap_or(' ')
            })
            .collect()
    }

    #[test]
    fn render_empty_app_has_status_bar() {
        let mut app = App::new();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let h = terminal.backend().buffer().area.height;
        let status_row = buffer_to_string(&terminal, h - 1);
        assert!(status_row.contains("no model") || status_row.contains("no workstream"));
    }

    #[test]
    fn render_with_messages_shows_content() {
        let mut app = App::new();
        app.messages
            .push(ChatMessage::new(ChatRole::User, "hello world"));
        app.messages
            .push(ChatMessage::new(ChatRole::Assistant, "hi there"));

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        // Check that message content appears somewhere in the buffer
        let buf = terminal.backend().buffer();
        let full_buffer: String = (0..buf.area.height)
            .flat_map(|y| (0..buf.area.width).map(move |x| (x, y)))
            .map(|(x, y)| {
                buf.cell((x, y))
                    .unwrap()
                    .symbol()
                    .chars()
                    .next()
                    .unwrap_or(' ')
            })
            .collect();
        assert!(full_buffer.contains("hello world"));
        assert!(full_buffer.contains("hi there"));
    }

    #[test]
    fn render_with_input_text() {
        let mut app = App::new();
        app.input_buffer = "test input".into();
        app.cursor_pos = 10;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        // Input bar is second from bottom (above status bar)
        let input_row = buffer_to_string(&terminal, 22);
        assert!(
            input_row.contains("test input"),
            "expected 'test input' in bottom row, got: {input_row}"
        );
    }

    #[test]
    fn render_streaming_shows_cursor() {
        let mut app = App::new();
        app.is_generating = true;
        app.streaming_text = "partial response".into();

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let buf = terminal.backend().buffer();
        let full_buffer: String = (0..buf.area.height)
            .flat_map(|y| (0..buf.area.width).map(move |x| (x, y)))
            .map(|(x, y)| {
                buf.cell((x, y))
                    .unwrap()
                    .symbol()
                    .chars()
                    .next()
                    .unwrap_or(' ')
            })
            .collect();
        assert!(full_buffer.contains("partial response"));
        assert!(full_buffer.contains("▌")); // thin cursor indicator (Phase 5)
    }

    #[test]
    fn render_small_terminal() {
        let mut app = App::new();
        let backend = TestBackend::new(40, 12);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();
    }

    #[test]
    fn render_large_terminal() {
        let mut app = App::new();
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();
    }

    // --- Helpers for region-specific assertions ---

    /// Extract text from a rectangular region of the buffer.
    fn region_text(terminal: &Terminal<TestBackend>, x: u16, y: u16, w: u16, h: u16) -> String {
        let buf = terminal.backend().buffer();
        let mut text = String::new();
        for row in y..y + h {
            for col in x..x + w {
                if let Some(cell) = buf.cell((col, row)) {
                    text.push_str(cell.symbol());
                }
            }
            text.push('\n');
        }
        text
    }

    /// Extract the chat area text. When sidebar is visible (focus == Sidebar),
    /// chat is right 80%; otherwise chat is full width.
    fn chat_region_for(terminal: &Terminal<TestBackend>, sidebar_visible: bool) -> String {
        let buf = terminal.backend().buffer();
        let w = buf.area.width;
        let h = buf.area.height;
        let (chat_x, chat_w) = if sidebar_visible {
            let sidebar_w = w / 5;
            (sidebar_w, w - sidebar_w)
        } else {
            (0, w)
        };
        let chat_y = 0;
        let chat_h = h.saturating_sub(3); // exclude separator (1) + input (1) + status bar (1)
        region_text(terminal, chat_x, chat_y, chat_w, chat_h)
    }

    /// Convenience: chat region for default app (sidebar hidden).
    fn chat_region(terminal: &Terminal<TestBackend>) -> String {
        chat_region_for(terminal, false)
    }

    /// Extract the sidebar text (left 20%, rows 1..height-3).
    /// Only meaningful when sidebar is visible (focus == Sidebar).
    fn sidebar_region(terminal: &Terminal<TestBackend>) -> String {
        let buf = terminal.backend().buffer();
        let w = buf.area.width;
        let h = buf.area.height;
        let sidebar_w = w / 5;
        let sidebar_y = 0;
        let sidebar_h = h.saturating_sub(3);
        region_text(terminal, 0, sidebar_y, sidebar_w, sidebar_h)
    }

    /// Extract the input bar text (second from bottom row).
    fn input_region(terminal: &Terminal<TestBackend>) -> String {
        let buf = terminal.backend().buffer();
        let w = buf.area.width;
        let h = buf.area.height;
        region_text(terminal, 0, h - 2, w, 1)
    }

    // --- Targeted component rendering tests ---

    #[test]
    fn chat_renders_user_message_with_prefix() {
        let mut app = App::new();
        app.messages
            .push(ChatMessage::new(ChatRole::User, "What files exist?"));

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let chat = chat_region(&terminal);
        assert!(
            chat.contains("❯") && chat.contains("What files exist?"),
            "chat should show '❯ What files exist?', got:\n{chat}"
        );
    }

    #[test]
    fn chat_renders_assistant_message_with_prefix() {
        let mut app = App::new();
        app.messages
            .push(ChatMessage::new(ChatRole::Assistant, "Here are the files."));

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let chat = chat_region(&terminal);
        assert!(
            chat.contains("Here are the files"),
            "chat should show assistant text, got:\n{chat}"
        );
    }

    #[test]
    fn chat_renders_tool_call_with_icon() {
        let mut app = App::new();
        app.messages.push(ChatMessage::new(
            ChatRole::ToolCall {
                name: "shell".into(),
            },
            "ls -la",
        ));
        // Add a result so the tool call shows ✓ (completed)
        app.messages.push(ChatMessage::new(
            ChatRole::ToolResult {
                name: "shell".into(),
                is_error: false,
            },
            "file1.rs",
        ));

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let chat = chat_region(&terminal);
        assert!(
            chat.contains("shell"),
            "chat should show tool name 'shell', got:\n{chat}"
        );
        assert!(
            chat.contains("⏵"),
            "chat should show ⏵ marker for collapsed tool call, got:\n{chat}"
        );
    }

    #[test]
    fn chat_renders_tool_result_collapsed() {
        let mut app = App::new();
        app.messages.push(ChatMessage::new(
            ChatRole::ToolResult {
                name: "shell".into(),
                is_error: false,
            },
            "file1.rs\nfile2.rs\nfile3.rs",
        ));

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let chat = chat_region(&terminal);
        assert!(
            chat.contains("shell result"),
            "chat should show 'shell result', got:\n{chat}"
        );
        assert!(
            chat.contains("file1.rs"),
            "chat should show tool output, got:\n{chat}"
        );
        // Collapsed indicator
        assert!(
            chat.contains("▸"),
            "collapsed result should show ▸, got:\n{chat}"
        );
    }

    #[test]
    fn chat_renders_tool_error_result() {
        let mut app = App::new();
        app.messages.push(ChatMessage::new(
            ChatRole::ToolResult {
                name: "shell".into(),
                is_error: true,
            },
            "permission denied",
        ));

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let chat = chat_region(&terminal);
        assert!(
            chat.contains("✗") && chat.contains("shell") && chat.contains("error"),
            "chat should show '✗ shell error', got:\n{chat}"
        );
        assert!(
            chat.contains("permission denied"),
            "error content should be visible, got:\n{chat}"
        );
    }

    #[test]
    fn chat_renders_tool_result_truncated() {
        let mut app = App::new();
        let long_result = (0..20)
            .map(|i| format!("line_{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        app.messages.push(ChatMessage::new(
            ChatRole::ToolResult {
                name: "grep".into(),
                is_error: false,
            },
            long_result,
        ));

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let chat = chat_region(&terminal);
        // First 5 lines should be visible
        assert!(chat.contains("line_0"), "first line should be visible");
        assert!(chat.contains("line_4"), "5th line should be visible");
        // Should show truncation indicator
        assert!(
            chat.contains("more"),
            "should show truncation indicator, got:\n{chat}"
        );
    }

    #[test]
    fn chat_streaming_text_appears_in_chat_area() {
        let mut app = App::new();
        app.is_generating = true;
        app.streaming_text = "streaming response here".into();

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let chat = chat_region(&terminal);
        assert!(
            chat.contains("streaming response here"),
            "streaming text should be in chat area, got:\n{chat}"
        );
        assert!(
            chat.contains("▌"),
            "streaming text should show thin cursor, got:\n{chat}"
        );
    }

    #[test]
    fn sidebar_renders_workstream_names() {
        use arawn_service::WorkstreamInfo;
        use chrono::Utc;
        use std::path::PathBuf;
        use uuid::Uuid;

        let mut app = App::new();
        app.focus = Focus::Sidebar;
        app.workstreams = vec![
            WorkstreamInfo {
                id: Uuid::new_v4(),
                name: "scratch".into(),
                root_dir: PathBuf::from("/tmp"),
                created_at: Utc::now(),
            },
            WorkstreamInfo {
                id: Uuid::new_v4(),
                name: "myproject".into(),
                root_dir: PathBuf::from("/tmp"),
                created_at: Utc::now(),
            },
        ];

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let sidebar = sidebar_region(&terminal);
        assert!(
            sidebar.contains("scratch"),
            "sidebar should show 'scratch', got:\n{sidebar}"
        );
        assert!(
            sidebar.contains("myproject"),
            "sidebar should show 'myproject', got:\n{sidebar}"
        );
    }

    #[test]
    fn sidebar_does_not_leak_into_chat() {
        use arawn_service::WorkstreamInfo;
        use chrono::Utc;
        use std::path::PathBuf;
        use uuid::Uuid;

        let mut app = App::new();
        app.focus = Focus::Sidebar;
        app.workstreams = vec![WorkstreamInfo {
            id: Uuid::new_v4(),
            name: "sb_data".into(),
            root_dir: PathBuf::from("/tmp"),
            created_at: Utc::now(),
        }];
        app.messages
            .push(ChatMessage::new(ChatRole::User, "ch_data"));

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let sidebar = sidebar_region(&terminal);
        let chat = chat_region_for(&terminal, true);

        assert!(sidebar.contains("sb_data"));
        assert!(
            !sidebar.contains("ch_data"),
            "chat content should not appear in sidebar"
        );
        assert!(chat.contains("ch_data"));
        assert!(
            !chat.contains("sb_data"),
            "sidebar content should not appear in chat"
        );
    }

    #[test]
    fn input_shows_placeholder_when_empty() {
        let mut app = App::new(); // empty buffer, focus on input
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let input = input_region(&terminal);
        assert!(
            input.contains("Type your message"),
            "empty input should show placeholder, got:\n{input}"
        );
    }

    #[test]
    fn input_shows_generating_when_active() {
        let mut app = App::new();
        app.is_generating = true;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let input = input_region(&terminal);
        assert!(
            input.contains("Generating"),
            "input should show 'Generating...' during generation, got:\n{input}"
        );
    }

    #[test]
    fn status_bar_shows_generating_indicator() {
        let mut app = App::new();
        app.is_generating = true;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let h = terminal.backend().buffer().area.height;
        let status = buffer_to_string(&terminal, h - 1);
        assert!(
            status.contains("Thinking"),
            "status bar should show generating indicator, got:\n{status}"
        );
    }

    #[test]
    fn status_bar_shows_workstream_name() {
        use arawn_service::WorkstreamInfo;
        use chrono::Utc;
        use std::path::PathBuf;
        use uuid::Uuid;

        let mut app = App::new();
        app.current_workstream = Some(WorkstreamInfo {
            id: Uuid::new_v4(),
            name: "Home Maintenance".into(),
            root_dir: PathBuf::from("/tmp"),
            created_at: Utc::now(),
        });

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let h = terminal.backend().buffer().area.height;
        let status = buffer_to_string(&terminal, h - 1);
        assert!(
            status.contains("Home Maintenance"),
            "status bar should show workstream name, got:\n{status}"
        );
    }

    #[test]
    fn messages_do_not_appear_in_input_area() {
        let mut app = App::new();
        app.messages
            .push(ChatMessage::new(ChatRole::User, "unique_chat_text_xyz"));
        app.input_buffer = "unique_input_text_abc".into();

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let input = input_region(&terminal);
        let chat = chat_region(&terminal);

        assert!(input.contains("unique_input_text_abc"));
        assert!(
            !input.contains("unique_chat_text_xyz"),
            "chat content should not leak into input"
        );
        assert!(chat.contains("unique_chat_text_xyz"));
        assert!(
            !chat.contains("unique_input_text_abc"),
            "input content should not leak into chat"
        );
    }

    // --- Scroll tests ---

    #[test]
    fn chat_auto_scrolls_to_bottom_with_many_messages() {
        let mut app = App::new();

        // Add enough messages to overflow a 20-row chat area
        // Each message = 2 lines (content + blank separator)
        for i in 0..30 {
            app.messages
                .push(ChatMessage::new(ChatRole::User, format!("MSG_NUM_{i:03}")));
        }

        // scroll_offset = 0 means "show bottom"
        assert_eq!(app.scroll_offset, 0);

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let chat = chat_region(&terminal);

        // Bottom messages should be visible
        assert!(
            chat.contains("MSG_NUM_029"),
            "last message should be visible (auto-scroll to bottom), got:\n{chat}"
        );

        // Top messages should NOT be visible (scrolled off the top)
        assert!(
            !chat.contains("MSG_NUM_000"),
            "first message should be scrolled off the top, got:\n{chat}"
        );
    }

    #[test]
    fn chat_scroll_up_reveals_older_messages() {
        let mut app = App::new();

        for i in 0..30 {
            app.messages
                .push(ChatMessage::new(ChatRole::User, format!("MSG_NUM_{i:03}")));
        }

        // Scroll up a lot — should reveal earlier messages
        app.scroll_offset = 50; // scroll way up

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let chat = chat_region(&terminal);

        // First message should now be visible
        assert!(
            chat.contains("MSG_NUM_000"),
            "first message should be visible after scrolling up, got:\n{chat}"
        );

        // Last message should NOT be visible (scrolled below)
        assert!(
            !chat.contains("MSG_NUM_029"),
            "last message should be scrolled off bottom after scrolling up, got:\n{chat}"
        );
    }

    #[test]
    fn chat_few_messages_all_visible() {
        let mut app = App::new();
        app.messages
            .push(ChatMessage::new(ChatRole::User, "ONLY_MSG"));

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let chat = chat_region(&terminal);
        assert!(
            chat.contains("ONLY_MSG"),
            "single message should be visible, got:\n{chat}"
        );
    }

    #[test]
    fn last_message_visible_above_input() {
        // The REAL failure mode: scroll doesn't go far enough, so the last
        // message ends up on rows that the separator/input then paints over.
        // We can't detect "painted over" — so we read the FULL buffer and
        // check the marker appears ABOVE the separator row.
        let mut app = App::new();

        for i in 0..15 {
            app.messages
                .push(ChatMessage::new(ChatRole::User, format!("question_{i}")));
            app.messages.push(ChatMessage::new(
                ChatRole::Assistant,
                format!("answer_{i} with some extra text"),
            ));
        }
        app.messages.push(ChatMessage::new(
            ChatRole::Assistant,
            "FINAL_VISIBLE_ANSWER",
        ));
        assert_eq!(app.scroll_offset, 0);

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let buf = terminal.backend().buffer();
        let h = buf.area.height;
        let w = buf.area.width;

        // Find which row contains the separator (───)
        let sep_row = (0..h).find(|&row| {
            let row_text: String = (0..w)
                .map(|x| buf.cell((x, row)).unwrap().symbol().to_string())
                .collect();
            row_text.contains("───")
        });
        let sep_row = sep_row.expect("should have a separator row");

        // Read all rows ABOVE the separator — this is what the user actually sees as chat
        let visible_chat: String = (0..sep_row)
            .map(|row| {
                let line: String = (0..w)
                    .map(|x| buf.cell((x, row)).unwrap().symbol().to_string())
                    .collect();
                line
            })
            .collect::<Vec<_>>()
            .join("\n");

        assert!(
            visible_chat.contains("FINAL_VISIBLE_ANSWER"),
            "last message must appear in rows ABOVE the separator (row {sep_row}), got:\n{visible_chat}"
        );
    }

    #[test]
    fn last_tool_result_visible_above_input() {
        let mut app = App::new();

        for i in 0..10 {
            app.messages
                .push(ChatMessage::new(ChatRole::User, format!("q_{i}")));
            app.messages
                .push(ChatMessage::new(ChatRole::Assistant, format!("a_{i}")));
        }
        app.messages.push(ChatMessage::new(
            ChatRole::ToolCall {
                name: "shell".into(),
            },
            "cargo test",
        ));
        app.messages.push(ChatMessage::new(
            ChatRole::ToolResult {
                name: "shell".into(),
                is_error: false,
            },
            "TOOL_OUTPUT_VISIBLE",
        ));
        app.messages
            .push(ChatMessage::new(ChatRole::Assistant, "AFTER_TOOL_VISIBLE"));

        let backend = TestBackend::new(80, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let buf = terminal.backend().buffer();
        let h = buf.area.height;
        let w = buf.area.width;

        // Find the thin separator row (between chat and input) — skip box-drawing
        // inside tool cards by looking for a full-width separator row.
        let sep_row = (0..h)
            .rev()
            .find(|&row| {
                let row_text: String = (0..w)
                    .map(|x| buf.cell((x, row)).unwrap().symbol().to_string())
                    .collect();
                // The separator is a full line of ─ with no other box chars (│┌└┐┘)
                row_text.contains("───")
                    && !row_text.contains('│')
                    && !row_text.contains('┌')
                    && !row_text.contains('└')
            })
            .expect("should have separator row");

        let visible_chat: String = (0..sep_row)
            .map(|row| {
                let line: String = (0..w)
                    .map(|x| buf.cell((x, row)).unwrap().symbol().to_string())
                    .collect();
                line
            })
            .collect::<Vec<_>>()
            .join("\n");

        assert!(
            visible_chat.contains("AFTER_TOOL_VISIBLE"),
            "message after tool result must appear above separator (row {sep_row}), got:\n{visible_chat}"
        );
    }
}
