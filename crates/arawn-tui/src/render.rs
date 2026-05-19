use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};

use crate::app::{App, ChatRole, Focus, LayoutRegions, SidebarSection};
use crate::theme;

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
}

fn render_sidebar_tab(frame: &mut Frame, area: ratatui::layout::Rect) {
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(theme::BORDER_INACTIVE));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mid = inner.height / 2;
    let bg_style = Style::default().bg(theme::SIDEBAR_TAB_BG);
    let mut lines: Vec<Line> = Vec::new();
    for i in 0..inner.height {
        if i == mid {
            lines.push(Line::from(Span::styled(
                " ▸",
                Style::default()
                    .fg(theme::BORDER_ACTIVE)
                    .bg(theme::SIDEBAR_TAB_BG),
            )));
        } else {
            lines.push(Line::from(Span::styled("  ", bg_style)));
        }
    }
    let widget = Paragraph::new(lines).style(bg_style);
    frame.render_widget(widget, inner);
}

fn render_status_bar(app: &App, frame: &mut Frame, area: ratatui::layout::Rect) {
    let bar_style = Style::default()
        .fg(theme::STATUS_BAR_FG)
        .bg(theme::STATUS_BAR_BG);
    let dim = Style::default()
        .fg(theme::OVERLAY0)
        .bg(theme::STATUS_BAR_BG);
    let mut spans = Vec::new();

    // Model name
    let model = if app.model_name.is_empty() {
        "no model"
    } else {
        &app.model_name
    };
    spans.push(Span::styled(format!(" {model}"), bar_style));

    // Token usage
    let (inp, out) = app.token_usage;
    if inp > 0 || out > 0 {
        spans.push(Span::styled(" │ ", dim));
        spans.push(Span::styled(format_tokens(inp), bar_style));
        spans.push(Span::styled(" / ", dim));
        spans.push(Span::styled(format_tokens(out), bar_style));
    }

    // Workstream
    spans.push(Span::styled(" │ ", dim));
    let ws_name = app
        .current_workstream
        .as_ref()
        .map(|ws| ws.name.as_str())
        .unwrap_or("no workstream");
    spans.push(Span::styled(ws_name.to_string(), bar_style));

    // Permission mode (T-0347: renamed to /autonomy vocabulary —
    // ask | edits | full | plan).
    if app.permission_mode != "ask" {
        spans.push(Span::styled(" │ ", dim));
        let (label, color) = match app.permission_mode.as_str() {
            "full" => ("FULL", theme::ERROR),
            "edits" => ("EDITS", theme::YELLOW),
            "plan" => ("PLAN", theme::SAPPHIRE),
            _ => ("ASK", theme::TEXT),
        };
        spans.push(Span::styled(
            label.to_string(),
            Style::default()
                .fg(color)
                .bg(crate::theme::STATUS_BAR_BG)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ));
    }

    // Session ID (8 chars)
    if let Some(ref session) = app.current_session {
        spans.push(Span::styled(" │ ", dim));
        let id_short = &session.id.to_string()[..8];
        spans.push(Span::styled(id_short.to_string(), bar_style));
    }

    // State indicator
    if app.is_generating {
        let frame_char = SPINNER_FRAMES[app.spinner_frame as usize % SPINNER_FRAMES.len()];
        let state_text = if let Some(ref tool) = app.active_tool {
            format!(" {frame_char} Running {tool}...")
        } else {
            format!(" {frame_char} Thinking...")
        };
        spans.push(Span::styled(
            state_text,
            Style::default()
                .fg(theme::GENERATING)
                .bg(theme::STATUS_BAR_BG),
        ));
        // Elapsed time
        if let Some(started) = app.generation_started {
            let elapsed = started.elapsed().as_secs();
            if elapsed >= 2 {
                spans.push(Span::styled(
                    format!(" {elapsed}s"),
                    Style::default()
                        .fg(theme::OVERLAY0)
                        .bg(theme::STATUS_BAR_BG),
                ));
            }
        }
    } else {
        spans.push(Span::styled(
            " Ready",
            Style::default().fg(theme::SUCCESS).bg(theme::STATUS_BAR_BG),
        ));
    }

    let status =
        Paragraph::new(Line::from(spans)).style(Style::default().bg(crate::theme::STATUS_BAR_BG));
    frame.render_widget(status, area);
}

/// Format a token count for display: 1234 → "1.2k", 12345 → "12.3k", 500 → "500"
fn format_tokens(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn render_sidebar(app: &App, frame: &mut Frame, area: ratatui::layout::Rect) {
    let border_color = if app.focus == Focus::Sidebar {
        theme::BORDER_ACTIVE
    } else {
        theme::BORDER_INACTIVE
    };

    // I-0035 Phase 3 T-A: sidebar is Workstreams-only. The previous
    // Sessions sub-section was removed; sessions are accessible via
    // `/session list`.
    let ws_items: Vec<ListItem> = app
        .workstreams
        .iter()
        .enumerate()
        .map(|(i, ws)| {
            let prefix = if app.focus == Focus::Sidebar
                && app.sidebar_section == SidebarSection::Workstreams
                && i == app.sidebar_ws_index
            {
                "▸ "
            } else {
                "  "
            };
            let style = if Some(&ws.id) == app.current_workstream.as_ref().map(|w| &w.id) {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(format!("{prefix}{}", ws.name)).style(style)
        })
        .collect();

    let ws_block = Block::default()
        .title(" Workstreams ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));
    let ws_list = List::new(ws_items).block(ws_block);
    frame.render_widget(ws_list, area);
}

/// I-0035 Phase 3 dashboard pane. Renders the bordered ` Today `
/// container and splits the inner region into:
/// - Top: compact brief summary (T-0357 — `render_dashboard_brief`).
/// - Bottom: action items list (T-0358 — placeholder until that task
///   lands; for now shows a one-line hint).
fn render_dashboard_pane(app: &App, frame: &mut Frame, area: ratatui::layout::Rect) {
    let border_color = theme::BORDER_INACTIVE;
    let block = Block::default()
        .title(" Today ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Vertical split: brief section gets up to BRIEF_HEIGHT rows;
    // action items section takes the rest. Falls back to a single
    // section when the pane is too short to be useful.
    const BRIEF_HEIGHT: u16 = 12;
    if inner.height >= BRIEF_HEIGHT + 4 {
        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(BRIEF_HEIGHT), Constraint::Min(2)])
            .split(inner);
        render_dashboard_brief(app, frame, split[0]);
        render_dashboard_actions(app, frame, split[1]);
    } else {
        render_dashboard_brief(app, frame, inner);
    }
}

/// I-0035 Phase 3 T-B (T-0357) — compact brief summary in the
/// top portion of the dashboard pane. Shows date header + up to
/// `MAX_CAL` calendar bullets, with an inline `⚠ conflict` marker
/// when two events overlap.
fn render_dashboard_brief(app: &App, frame: &mut Frame, area: ratatui::layout::Rect) {
    if area.width < 6 || area.height < 1 {
        return;
    }
    let strong = Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD);
    let muted = Style::default().fg(theme::SUBTEXT0);
    let warn = Style::default().fg(theme::YELLOW);

    let mut lines: Vec<Line<'static>> = Vec::new();
    lines.push(Line::from(Span::styled("Brief".to_string(), strong)));

    let Some(view) = app.daily_view.as_ref() else {
        lines.push(Line::from(Span::styled(
            "(no brief yet — run /day to generate)".to_string(),
            muted,
        )));
        let para = Paragraph::new(lines);
        frame.render_widget(para, area);
        return;
    };

    // Date line: tablet's period_key (`YYYY-MM-DD`) + weekday from
    // chrono if parseable, else just the period_key.
    let date_line = format_brief_date_line(&view.tablet.period_key);
    lines.push(Line::from(Span::styled(date_line, muted)));
    lines.push(Line::from(""));

    let calendar_items: Vec<&arawn_ceremonies::service::ItemDto> =
        view.items.iter().filter(|i| i.section_key == "calendar").collect();
    if calendar_items.is_empty() {
        lines.push(Line::from(Span::styled(
            "(no calendar events today)".to_string(),
            muted,
        )));
    } else {
        const MAX_CAL: usize = 4;
        // Inner text budget = area.width - 2 (bullet "• ").
        let text_budget = (area.width as usize).saturating_sub(2);
        for item in calendar_items.iter().take(MAX_CAL) {
            let row = format_calendar_row(item, text_budget);
            lines.push(Line::from(vec![
                Span::raw("• "),
                Span::raw(row),
            ]));
        }
        if calendar_items.len() > MAX_CAL {
            lines.push(Line::from(Span::styled(
                format!("… +{} more", calendar_items.len() - MAX_CAL),
                muted,
            )));
        }
        if let Some(conflict_line) = detect_conflict(&calendar_items) {
            lines.push(Line::from(Span::styled(conflict_line, warn)));
        }
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, area);
}

/// I-0035 Phase 3 T-C (T-0358) — read-only action items list in
/// the bottom portion of the dashboard pane. Renders the daily
/// tablet's `attention` section as checkbox rows, with a
/// `─ Carried over ─` separator + carried-over rolling todos
/// below when both are present. Truncates with `…` to fit pane
/// width; overflows with `… +K more` footer.
fn render_dashboard_actions(app: &App, frame: &mut Frame, area: ratatui::layout::Rect) {
    if area.width < 6 || area.height < 1 {
        return;
    }
    let strong = Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD);
    let muted = Style::default().fg(theme::SUBTEXT0);

    let mut lines: Vec<Line<'static>> = Vec::new();
    lines.push(Line::from(Span::styled("Action items".to_string(), strong)));

    let Some(view) = app.daily_view.as_ref() else {
        lines.push(Line::from(Span::styled("(no action items)".to_string(), muted)));
        frame.render_widget(Paragraph::new(lines), area);
        return;
    };

    let attention: Vec<&arawn_ceremonies::service::ItemDto> = view
        .items
        .iter()
        .filter(|i| i.section_key == "attention")
        .collect();
    let carried: Vec<&arawn_ceremonies::service::ItemDto> = view
        .items
        .iter()
        .filter(|i| i.section_key == "todos")
        .collect();

    if attention.is_empty() && carried.is_empty() {
        lines.push(Line::from(Span::styled("(no action items)".to_string(), muted)));
        frame.render_widget(Paragraph::new(lines), area);
        return;
    }

    // Text budget: width - 2 (checkbox glyph + padding).
    let text_budget = (area.width as usize).saturating_sub(2);
    // Row budget: total area height - 1 (header). When we have
    // attention items AND carried-over todos, reserve 1 row for
    // the separator.
    let used_so_far = 1usize; // header
    let has_separator = !attention.is_empty() && !carried.is_empty();
    let separator_cost = if has_separator { 1 } else { 0 };
    let total_rows = area.height as usize;
    let body_budget = total_rows
        .saturating_sub(used_so_far)
        .saturating_sub(separator_cost);

    // Allocate budget across the two sections. Attention items get
    // priority — they're the most actionable. Carried-over get
    // whatever's left after attention's rows + overflow footer.
    let attention_alloc = if carried.is_empty() {
        body_budget
    } else {
        // Leave at least 2 rows for the carried section (1 item +
        // optional overflow footer). Squeeze attention to fit.
        body_budget.saturating_sub(2).max(1)
    };

    push_action_rows(&mut lines, &attention, attention_alloc, text_budget, muted);

    if has_separator {
        lines.push(Line::from(Span::styled(
            "─ Carried over ─".to_string(),
            muted,
        )));
        let carried_alloc = total_rows
            .saturating_sub(lines.len())
            .max(1);
        push_action_rows(&mut lines, &carried, carried_alloc, text_budget, muted);
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, area);
}

/// Push action rows (checkbox + truncated body) into `lines`,
/// honoring a row budget. When `items.len() > budget`, render the
/// first `budget - 1` items plus a `… +K more` footer line.
fn push_action_rows(
    lines: &mut Vec<Line<'static>>,
    items: &[&arawn_ceremonies::service::ItemDto],
    budget: usize,
    text_budget: usize,
    muted_style: Style,
) {
    if budget == 0 || items.is_empty() {
        return;
    }
    if items.len() <= budget {
        for item in items {
            lines.push(Line::from(vec![
                Span::raw("☐ "),
                Span::raw(format_action_row(item, text_budget)),
            ]));
        }
    } else {
        let show = budget.saturating_sub(1).max(1);
        let hidden = items.len() - show;
        for item in items.iter().take(show) {
            lines.push(Line::from(vec![
                Span::raw("☐ "),
                Span::raw(format_action_row(item, text_budget)),
            ]));
        }
        lines.push(Line::from(Span::styled(
            format!("… +{hidden} more"),
            muted_style,
        )));
    }
}

fn format_action_row(item: &arawn_ceremonies::service::ItemDto, budget: usize) -> String {
    let title = item
        .body
        .get("text")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            item.body
                .get("summary")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .or_else(|| {
            item.body
                .get("subject")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| serde_json::to_string(&item.body).unwrap_or_default());
    crate::width::truncate_display(&title, budget)
}

/// Format the brief's date line: `YYYY-MM-DD · Wed` when the
/// period_key parses; otherwise the raw period_key.
fn format_brief_date_line(period_key: &str) -> String {
    use chrono::Datelike;
    chrono::NaiveDate::parse_from_str(period_key, "%Y-%m-%d")
        .map(|d| {
            let wd = match d.weekday() {
                chrono::Weekday::Mon => "Mon",
                chrono::Weekday::Tue => "Tue",
                chrono::Weekday::Wed => "Wed",
                chrono::Weekday::Thu => "Thu",
                chrono::Weekday::Fri => "Fri",
                chrono::Weekday::Sat => "Sat",
                chrono::Weekday::Sun => "Sun",
            };
            format!("{period_key} · {wd}")
        })
        .unwrap_or_else(|_| period_key.to_string())
}

/// Render one calendar row: `HH:MM <title>` when body has a
/// `start_ts`, else `<title>` falling back to body.text.
fn format_calendar_row(item: &arawn_ceremonies::service::ItemDto, budget: usize) -> String {
    let title = item
        .body
        .get("text")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            item.body
                .get("summary")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        });
    let time = item
        .body
        .get("start_ts")
        .and_then(|v| v.as_str())
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.format("%H:%M").to_string());
    let raw = match time {
        Some(t) => format!("{t} {title}"),
        None => title,
    };
    crate::width::truncate_display(&raw, budget)
}

/// Pure conflict detection over calendar items. Looks at consecutive
/// items in tablet order; returns a one-line warning like
/// "⚠ conflict 14:00–15:00" for the first overlap found. Returns
/// None when no parseable timestamps overlap.
fn detect_conflict(items: &[&arawn_ceremonies::service::ItemDto]) -> Option<String> {
    let mut events: Vec<(chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)> = items
        .iter()
        .filter_map(|i| {
            let start = i
                .body
                .get("start_ts")
                .and_then(|v| v.as_str())
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())?
                .with_timezone(&chrono::Utc);
            let end = i
                .body
                .get("end_ts")
                .and_then(|v| v.as_str())
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc))
                // No end_ts → assume a 30-min default so we can still
                // detect overlap with a later event.
                .unwrap_or_else(|| start + chrono::Duration::minutes(30));
            Some((start, end))
        })
        .collect();
    events.sort_by_key(|e| e.0);
    for pair in events.windows(2) {
        if pair[1].0 < pair[0].1 {
            return Some(format!(
                "⚠ conflict {}–{}",
                pair[1].0.format("%H:%M"),
                pair[0].1.format("%H:%M")
            ));
        }
    }
    None
}

fn render_chat(app: &mut App, frame: &mut Frame, area: ratatui::layout::Rect) {
    // Empty chat, no streaming, not generating. Two cases:
    // - At least one ceremony tablet exists → render the cached brief
    //   markdown top-aligned in the chat area (I-0035 Phase 2). The
    //   brief shares the assistant-message render path so it inherits
    //   the same Catppuccin Mocha styling as everything else.
    // - No tablets yet → fall through to the T-0331 welcome hero
    //   (centered wordmark + key-binding hints).
    if app.messages.is_empty() && !app.is_generating && app.streaming_text.is_empty() {
        if app.should_show_brief_in_empty_chat() {
            render_empty_chat_brief(app, frame, area);
        } else {
            render_idle_hero(frame, area);
        }
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    let num_messages = app.messages.len();

    // Pre-compute per-message flags to avoid borrow conflicts in the loop.
    // For each tool call: does the next message have a result? Is it an error?
    let tool_call_flags: Vec<(bool, bool)> = (0..num_messages)
        .map(|i| {
            let next_is_result = i + 1 < num_messages
                && matches!(app.messages[i + 1].role, ChatRole::ToolResult { .. });
            let next_is_error = i + 1 < num_messages
                && matches!(
                    app.messages[i + 1].role,
                    ChatRole::ToolResult { is_error: true, .. }
                );
            (next_is_result, next_is_error)
        })
        .collect();

    let chat_width = area.width as usize;

    // Snapshot whether the last message is a completed-turn marker
    // candidate (assistant text or tool result). Captured before the
    // mutable iteration below so we can use it after the loop without
    // triggering a borrow conflict.
    let last_is_completed_turn = matches!(
        app.messages.last().map(|m| &m.role),
        Some(ChatRole::Assistant) | Some(ChatRole::ToolResult { .. })
    );

    for (msg_idx, msg) in app.messages.iter_mut().enumerate() {
        match &msg.role {
            ChatRole::User => {
                // Blank line before user messages (turn separator) unless first message
                if msg_idx > 0 {
                    lines.push(Line::from(""));
                }
                lines.push(Line::from(vec![
                    Span::styled(
                        "❯ ",
                        Style::default()
                            .fg(theme::USER)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(&msg.content),
                ]));
            }
            ChatRole::Assistant => {
                // Skip empty assistant messages (e.g., tool-use-only turns)
                if msg.content.trim().is_empty() {
                    continue;
                }
                lines.push(Line::from(""));
                let gutter_style = Style::default().fg(theme::CHROME);
                let content_width = chat_width.saturating_sub(4);
                for md_line in msg.rendered_lines(content_width) {
                    let mut prefixed = vec![Span::styled("│ ", gutter_style)];
                    prefixed.extend(md_line.spans.clone());
                    lines.push(Line::from(prefixed));
                }
            }
            ChatRole::ToolCall { name } => {
                let chrome = Style::default().fg(theme::CHROME);

                let (next_is_result, next_is_error) = tool_call_flags[msg_idx];
                let is_running = !next_is_result && app.is_generating;

                // Collapsed by default; expand when the user has toggled
                // either this call or its paired result, or when the
                // paired result is an error (errors always show body).
                let pair_toggled = app.expanded_tool_results.contains(&msg_idx)
                    || (next_is_result && app.expanded_tool_results.contains(&(msg_idx + 1)));
                let expand_card = next_is_error || pair_toggled;

                let summary_raw = compact_tool_summary(&msg.content);

                if !expand_card {
                    // Single-line collapsed render
                    let mut spans: Vec<Span<'static>> = Vec::with_capacity(8);
                    spans.push(Span::styled("  ⏵ ", chrome));
                    spans.push(Span::styled(
                        name.clone(),
                        Style::default()
                            .fg(theme::TOOL_NAME)
                            .add_modifier(Modifier::BOLD),
                    ));
                    if !summary_raw.is_empty() {
                        let trimmed = crate::width::truncate_display(&summary_raw, 60);
                        spans.push(Span::styled(" · ", chrome));
                        spans.push(Span::styled(
                            trimmed,
                            Style::default().fg(theme::TOOL_SUMMARY),
                        ));
                    }
                    if is_running {
                        let elapsed = msg.created_at.elapsed().as_secs_f64();
                        let frame_char =
                            SPINNER_FRAMES[app.spinner_frame as usize % SPINNER_FRAMES.len()];
                        spans.push(Span::styled(" · ", chrome));
                        spans.push(Span::styled(
                            format!("running {elapsed:.1}s "),
                            Style::default().fg(theme::OVERLAY0),
                        ));
                        spans.push(Span::styled(
                            frame_char.to_string(),
                            Style::default().fg(theme::GENERATING),
                        ));
                    } else if next_is_result {
                        let elapsed = msg.created_at.elapsed().as_secs_f64();
                        spans.push(Span::styled(" · ", chrome));
                        spans.push(Span::styled(
                            format!("{elapsed:.1}s"),
                            Style::default().fg(theme::OVERLAY0),
                        ));
                    }
                    lines.push(Line::from(spans));
                } else {
                    let icon = if is_running {
                        let frame_char =
                            SPINNER_FRAMES[app.spinner_frame as usize % SPINNER_FRAMES.len()];
                        Span::styled(
                            format!("{frame_char} "),
                            Style::default().fg(theme::GENERATING),
                        )
                    } else if next_is_error {
                        Span::styled("✗ ", Style::default().fg(theme::ERROR))
                    } else if next_is_result {
                        Span::styled("✓ ", Style::default().fg(theme::SUCCESS))
                    } else {
                        Span::styled("⏳ ", Style::default().fg(theme::GENERATING))
                    };

                    let tool_name = Span::styled(
                        name.clone(),
                        Style::default()
                            .fg(theme::TOOL_NAME)
                            .add_modifier(Modifier::BOLD),
                    );
                    let summary_span = if summary_raw.is_empty() {
                        Span::raw("")
                    } else {
                        Span::styled(
                            format!("  {summary_raw}"),
                            Style::default().fg(theme::TOOL_SUMMARY),
                        )
                    };

                    let elapsed_span = if is_running {
                        let elapsed = msg.created_at.elapsed().as_secs_f64();
                        Span::styled(
                            format!("  {elapsed:.1}s"),
                            Style::default().fg(theme::OVERLAY0),
                        )
                    } else {
                        Span::raw("")
                    };

                    use unicode_width::UnicodeWidthStr;
                    let box_width = chat_width.saturating_sub(2).min(82);
                    let header_spans = [&icon, &tool_name, &summary_span, &elapsed_span];
                    let header_display_width: usize = header_spans
                        .iter()
                        .map(|s| UnicodeWidthStr::width(s.content.as_ref()))
                        .sum();
                    let fill_len = box_width.saturating_sub(header_display_width + 4);
                    let fill = "─".repeat(fill_len);

                    lines.push(Line::from(vec![
                        Span::styled("  ┌ ", chrome),
                        icon,
                        tool_name,
                        summary_span,
                        elapsed_span,
                        Span::styled(format!(" {fill}┐"), chrome),
                    ]));
                }
            }
            ChatRole::ToolResult { name, is_error } => {
                let chrome = Style::default().fg(theme::CHROME);
                let is_expanded = app.expanded_tool_results.contains(&msg_idx)
                    || (msg_idx > 0 && app.expanded_tool_results.contains(&(msg_idx - 1)));

                if *is_error {
                    lines.push(Line::from(vec![
                        Span::styled("  │ ", chrome),
                        Span::styled("✗ ", Style::default().fg(theme::ERROR)),
                        Span::styled(
                            format!("{name} error"),
                            Style::default()
                                .fg(theme::ERROR)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]));
                    for err_line in msg.content.lines().take(10) {
                        lines.push(Line::from(vec![
                            Span::styled("  │  ", chrome),
                            Span::styled(err_line.to_string(), Style::default().fg(theme::ERROR)),
                        ]));
                    }
                    // Match the top border's box_width so top and bottom
                    // align even with unicode tool names. `└` + N dashes +
                    // `┘` consumes 2 corner chars, so dashes = box_width - 2.
                    let box_width = chat_width.saturating_sub(2).min(82);
                    let bottom_len = box_width.saturating_sub(2);
                    lines.push(Line::from(Span::styled(
                        format!("  └{}┘", "─".repeat(bottom_len)),
                        chrome,
                    )));
                } else if is_expanded {
                    let toggle_hint = Span::styled(
                        " (Ctrl+E to collapse)",
                        Style::default()
                            .fg(theme::CHROME)
                            .add_modifier(Modifier::ITALIC),
                    );
                    let result_text = Style::default().fg(theme::RESULT_TEXT);
                    let label_style = Style::default().fg(theme::RESULT_LABEL);
                    lines.push(Line::from(vec![
                        Span::styled("  │ ", chrome),
                        Span::styled("▾ ", label_style),
                        Span::styled(format!("{name} result"), label_style),
                        toggle_hint,
                    ]));
                    for result_line in msg.content.lines() {
                        lines.push(Line::from(vec![
                            Span::styled("  │  ", chrome),
                            Span::styled(result_line.to_string(), result_text),
                        ]));
                    }
                    // Match the top border's box_width so top and bottom
                    // align even with unicode tool names. `└` + N dashes +
                    // `┘` consumes 2 corner chars, so dashes = box_width - 2.
                    let box_width = chat_width.saturating_sub(2).min(82);
                    let bottom_len = box_width.saturating_sub(2);
                    lines.push(Line::from(Span::styled(
                        format!("  └{}┘", "─".repeat(bottom_len)),
                        chrome,
                    )));
                } else {
                    let total_lines = msg.content.lines().count();
                    let total_chars = msg.content.chars().count();
                    let max_preview_lines = 5;
                    // Per-line char budget: chat width minus the "  │  "
                    // gutter (5 chars) and a small safety margin, so the
                    // preview never overflows into wrap territory.
                    let line_budget = chat_width.saturating_sub(7).max(20);
                    let result_text = Style::default().fg(theme::RESULT_TEXT);
                    let label_style = Style::default().fg(theme::RESULT_LABEL);

                    // Two ways the preview can be incomplete: too many
                    // lines, OR any line is wider than `line_budget`
                    // (single-line JSON dumps used to leak through here).
                    let any_line_too_wide = msg
                        .content
                        .lines()
                        .any(|l| crate::width::display_width(l) > line_budget);
                    let truncated_lines = total_lines > max_preview_lines;
                    let truncated = truncated_lines || any_line_too_wide;

                    let toggle_hint = if truncated {
                        let label = if truncated_lines {
                            format!(" ({total_lines} lines — Ctrl+E to expand)")
                        } else {
                            format!(" ({total_chars} chars — Ctrl+E to expand)")
                        };
                        Span::styled(
                            label,
                            Style::default()
                                .fg(theme::CHROME)
                                .add_modifier(Modifier::ITALIC),
                        )
                    } else {
                        Span::raw("")
                    };
                    lines.push(Line::from(vec![
                        Span::styled("  ▸ ", label_style),
                        Span::styled(format!("{name} result"), label_style),
                        toggle_hint,
                    ]));
                    for result_line in msg.content.lines().take(max_preview_lines) {
                        let display = crate::width::truncate_display(result_line, line_budget);
                        lines.push(Line::from(vec![
                            Span::raw("    "),
                            Span::styled(display, result_text),
                        ]));
                    }
                    if truncated_lines {
                        lines.push(Line::from(vec![
                            Span::raw("    "),
                            Span::styled(
                                format!(
                                    "… {remaining} more",
                                    remaining = total_lines - max_preview_lines
                                ),
                                Style::default()
                                    .fg(theme::RESULT_HINT)
                                    .add_modifier(Modifier::ITALIC),
                            ),
                        ]));
                    }
                }
            }
            ChatRole::System => {
                let gutter_style = Style::default().fg(theme::SYSTEM);
                lines.push(Line::from(Span::styled(
                    "│ system:",
                    gutter_style.add_modifier(Modifier::ITALIC),
                )));
                let content_width = chat_width.saturating_sub(4);
                for md_line in msg.rendered_lines(content_width) {
                    let mut prefixed = vec![Span::styled("│ ", gutter_style)];
                    prefixed.extend(md_line.spans.clone());
                    lines.push(Line::from(prefixed));
                }
            }
        }
    }

    // Streaming text (in progress)
    if !app.streaming_text.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("│ ", Style::default().fg(theme::CHROME)),
            Span::raw(app.streaming_text.clone()),
            Span::styled("▌", Style::default().fg(theme::BORDER_ACTIVE)),
        ]));
    } else if app.is_generating {
        // Waiting for first token — show thinking indicator with elapsed time
        let frame_char = SPINNER_FRAMES[app.spinner_frame as usize % SPINNER_FRAMES.len()];
        let elapsed = app
            .generation_started
            .map(|t| format!(" {:.1}s", t.elapsed().as_secs_f64()))
            .unwrap_or_default();
        lines.push(Line::from(vec![
            Span::styled(format!("{frame_char} "), Style::default().fg(theme::BLUE)),
            Span::styled(
                format!("thinking...{elapsed}"),
                Style::default()
                    .fg(theme::OVERLAY0)
                    .add_modifier(Modifier::ITALIC),
            ),
        ]));
    } else if last_is_completed_turn {
        // End-of-response marker. Subtle dim horizontal rule beneath the
        // last completed turn so the user has a clear "agent is done,
        // your turn" signal — otherwise long output runs into the input
        // area with no visual delimiter.
        let rule_width = chat_width.saturating_sub(2).max(3);
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  {}", "─".repeat(rule_width)),
            Style::default().fg(theme::SEPARATOR),
        )));
    }

    // Pre-wrap to the chat area's exact width. We own the wrap so the
    // visual-line count is exact, and so the chat renderer can slice the
    // visible window directly — no `Paragraph::wrap` + estimated-scroll
    // mismatch (which used to clip the bottom of long messages).
    let content_width = (area.width as usize).max(1);
    let wrapped = crate::wrap::wrap_lines(lines, content_width);
    let visible_height = area.height as usize;
    let total = wrapped.len();
    let auto_scroll_pos = total.saturating_sub(visible_height);

    // Clamp scroll_offset to valid range — can't scroll past the top.
    if app.scroll_offset > auto_scroll_pos {
        app.scroll_offset = auto_scroll_pos;
    }
    let start = if app.scroll_offset == 0 {
        auto_scroll_pos
    } else {
        auto_scroll_pos.saturating_sub(app.scroll_offset)
    };
    let end = (start + visible_height).min(total);
    let view: Vec<Line<'static>> = wrapped[start..end].to_vec();

    // Flat blit — no Wrap, no scroll. The pre-wrapped slice is already the
    // exact visible content for this Rect.
    let chat = Paragraph::new(view);
    frame.render_widget(chat, area);
}

fn render_separator(frame: &mut Frame, area: ratatui::layout::Rect) {
    let line = "─".repeat(area.width as usize);
    let sep = Paragraph::new(line).style(Style::default().fg(theme::SEPARATOR));
    frame.render_widget(sep, area);
}

fn render_input(app: &App, frame: &mut Frame, area: ratatui::layout::Rect) {
    let prompt = "> ";
    let prompt_style = Style::default()
        .fg(theme::INPUT_PROMPT)
        .add_modifier(Modifier::BOLD);

    if app.is_generating {
        let line = Line::from(vec![
            Span::styled(prompt, Style::default().fg(theme::PLACEHOLDER)),
            Span::styled("Generating...", Style::default().fg(theme::PLACEHOLDER)),
        ]);
        frame.render_widget(Paragraph::new(line), area);
    } else if app.input_buffer.is_empty() {
        let line = Line::from(vec![
            Span::styled(prompt, prompt_style),
            Span::styled(
                "Type your message...",
                Style::default().fg(theme::PLACEHOLDER),
            ),
        ]);
        frame.render_widget(Paragraph::new(line), area);
    } else {
        // Horizontal scroll: keep the cursor (measured in display cells)
        // visible within the available width.
        let prompt_len = prompt.len();
        let available = (area.width as usize).saturating_sub(prompt_len);
        let buf = &app.input_buffer;
        let cursor_byte = app.cursor_pos.min(buf.len());
        let cursor_cells = crate::width::display_width(&buf[..cursor_byte]);

        // Walk forward from the start until we drop enough leading cells
        // that the cursor lands within the visible window.
        let mut scroll_byte = 0usize;
        let mut scroll_cells = 0usize;
        if cursor_cells >= available {
            let target_drop = cursor_cells - available + 1;
            let mut acc = 0usize;
            for (idx, ch) in buf.char_indices() {
                if acc >= target_drop {
                    scroll_byte = idx;
                    scroll_cells = acc;
                    break;
                }
                acc += crate::width::display_width(&ch.to_string());
            }
        }

        let visible = &buf[scroll_byte..];
        let line = Line::from(vec![Span::styled(prompt, prompt_style), Span::raw(visible)]);
        frame.render_widget(Paragraph::new(line), area);

        if app.focus == Focus::Main {
            let cursor_x = area.x + prompt_len as u16 + (cursor_cells - scroll_cells) as u16;
            frame.set_cursor_position((cursor_x, area.y));
        }
        return;
    }

    if app.focus == Focus::Main && !app.is_generating {
        let cursor_byte = app.cursor_pos.min(app.input_buffer.len());
        let cursor_cells = crate::width::display_width(&app.input_buffer[..cursor_byte]);
        let x = area.x + prompt.len() as u16 + cursor_cells as u16;
        let y = area.y;
        frame.set_cursor_position((x, y));
    }
}

/// Render the autocomplete dropdown above the input line.
fn render_autocomplete(
    ac: &crate::command::AutocompleteState,
    frame: &mut Frame,
    input_area: ratatui::layout::Rect,
) {
    if ac.is_empty() {
        return;
    }

    let max_visible = 8.min(ac.suggestions.len());
    let dropdown_height = max_visible as u16 + 2; // +2 for border

    // Position: directly above the input line
    let dropdown_area = ratatui::layout::Rect {
        x: input_area.x + 2, // offset to align with text after "> "
        y: input_area.y.saturating_sub(dropdown_height),
        width: input_area.width.clamp(20, 50),
        height: dropdown_height,
    };

    // Clear the area
    frame.render_widget(Clear, dropdown_area);

    let items: Vec<Line> = ac
        .suggestions
        .iter()
        .take(max_visible)
        .enumerate()
        .map(|(i, cmd)| {
            let is_selected = i == ac.selected;
            let prefix = if is_selected { "▸ " } else { "  " };
            let name_style = if is_selected {
                Style::default()
                    .fg(theme::BORDER_ACTIVE)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::TEXT)
            };
            let desc_style = Style::default().fg(theme::OVERLAY1);

            Line::from(vec![
                Span::raw(prefix),
                Span::styled(format!("/{:<12}", cmd.name), name_style),
                Span::styled(
                    truncate_to(&cmd.description, dropdown_area.width as usize - 18),
                    desc_style,
                ),
            ])
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::BORDER_INACTIVE))
        .style(Style::default().bg(theme::SURFACE0));

    let paragraph = Paragraph::new(items).block(block);
    frame.render_widget(paragraph, dropdown_area);
}

/// I-0035 Phase 4 T-A — render the toast at the head of the queue
/// (already filtered by `drop_expired` in the entry point). Single
/// styled line, truncated to fit the row width.
fn render_toast_bar(app: &App, frame: &mut Frame, area: ratatui::layout::Rect) {
    let Some(toast) = app.toast_queue.front() else {
        return;
    };
    let fg = match toast.level {
        crate::toast::ToastLevel::Info => theme::SUBTEXT0,
        crate::toast::ToastLevel::Warn => theme::YELLOW,
        crate::toast::ToastLevel::Error => theme::RED,
    };
    let style = Style::default().fg(fg).add_modifier(Modifier::BOLD);
    let budget = (area.width as usize).saturating_sub(2);
    let text = crate::width::truncate_display(&toast.message, budget);
    let line = Line::from(vec![Span::raw(" "), Span::styled(text, style)]);
    let para = Paragraph::new(vec![line]);
    frame.render_widget(para, area);
}

fn render_oauth_heartbeat(app: &App, frame: &mut Frame, area: ratatui::layout::Rect) {
    let Some((svc, started)) = &app.oauth_in_flight else {
        return;
    };
    let elapsed = started.elapsed().as_secs();
    // Pulse the bullet between yellow and overlay every other second
    // using the existing spinner_frame so no new tick state is needed.
    let pulse_on = (app.spinner_frame / 5).is_multiple_of(2);
    let bullet_color = if pulse_on {
        theme::GENERATING
    } else {
        theme::OVERLAY0
    };
    let dim = Style::default().fg(theme::OVERLAY1);
    let line = Line::from(vec![
        Span::raw(" "),
        Span::styled("•", Style::default().fg(bullet_color)),
        Span::styled(
            format!(" waiting for {svc} OAuth in browser… {elapsed}s · Esc to cancel"),
            dim,
        ),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

/// I-0035 Phase 2 (T-0354): render the cached brief markdown in the
/// empty-chat area. Inherits the same `markdown_to_lines_with_width`
/// pipeline as assistant messages so the brief reads visually
/// identically whether shown via `/brief` or auto-rendered here.
fn render_empty_chat_brief(app: &App, frame: &mut Frame, area: ratatui::layout::Rect) {
    let Some(md) = app.brief_markdown.as_deref() else {
        return;
    };
    // 2-cell left/right margin so the brief breathes inside the chat
    // pane and lines wrap at the right column.
    let content_width = area.width.saturating_sub(4) as usize;
    if content_width < 10 {
        return;
    }
    let lines = crate::markdown::markdown_to_lines_with_width(md, content_width);
    let inner = ratatui::layout::Rect::new(
        area.x + 2,
        area.y,
        area.width.saturating_sub(4),
        area.height,
    );
    let para = ratatui::widgets::Paragraph::new(lines)
        .alignment(ratatui::layout::Alignment::Left);
    frame.render_widget(para, inner);
}

fn render_idle_hero(frame: &mut Frame, area: ratatui::layout::Rect) {
    let chrome = Style::default().fg(theme::CHROME);
    let dim = Style::default().fg(theme::SUBTEXT0);
    let hint = Style::default().fg(theme::OVERLAY1);

    let hero_lines: Vec<Line> = vec![
        Line::from(Span::styled("╭─────────────╮", chrome)),
        Line::from(vec![
            Span::styled("│    ", chrome),
            Span::styled("arawn", dim.add_modifier(Modifier::BOLD)),
            Span::styled("    │", chrome),
        ]),
        Line::from(Span::styled("╰─────────────╯", chrome)),
        Line::from(""),
        Line::from(Span::styled(
            "Welcome — your personal agentic assistant.",
            dim,
        )),
        Line::from(Span::styled(
            "I watch, check, summarize, and nudge across your tools.",
            dim,
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Type / for commands · Tab to toggle sidebar",
            hint,
        )),
        Line::from(Span::styled("/connect <service> · ↑ recall", hint)),
    ];

    let hero_height = hero_lines.len() as u16;
    let hero_width = 56u16;
    if area.width < hero_width || area.height < hero_height {
        return;
    }
    let x = area.x + (area.width.saturating_sub(hero_width)) / 2;
    let y = area.y + (area.height.saturating_sub(hero_height)) / 2;
    let rect = ratatui::layout::Rect::new(x, y, hero_width, hero_height);
    let para = Paragraph::new(hero_lines).alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(para, rect);
}

/// Truncate a string to fit within a display width, adding "…" if needed.
fn truncate_to(s: &str, max_cells: usize) -> String {
    crate::width::truncate_display(s, max_cells)
}

/// Extract a compact summary from tool call content for inline display.
fn compact_tool_summary(content: &str) -> String {
    if content.is_empty() {
        return String::new();
    }
    truncate_for_display(content, 60)
}

fn truncate_for_display(s: &str, max: usize) -> String {
    // Display-cell width — covers byte-boundary panic, char-count drift
    // for CJK, and zero-width / 2-width edge cases in one shot.
    crate::width::truncate_display(s, max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{App, ChatMessage};
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
