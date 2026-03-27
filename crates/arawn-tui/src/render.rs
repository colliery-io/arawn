//! Ratatui rendering — sidebar + chat panel + input box.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

use crate::app::{App, Focus, SidebarSection};

/// Render the entire UI into the given frame.
pub fn draw(f: &mut Frame, app: &App) {
    // Top-level horizontal split: sidebar | main
    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(f.area());

    // Enforce minimum sidebar width of 20 cols; if terminal is too narrow,
    // the percentage constraint will naturally shrink but we still render.
    draw_sidebar(f, app, h_chunks[0]);

    // Right panel: vertical split for chat + input + optional status
    let v_constraints = if app.status.is_some() {
        vec![
            Constraint::Min(1),
            Constraint::Length(3),
            Constraint::Length(1),
        ]
    } else {
        vec![Constraint::Min(1), Constraint::Length(3)]
    };

    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(v_constraints)
        .split(h_chunks[1]);

    draw_chat(f, app, v_chunks[0]);
    draw_input(f, app, v_chunks[1]);

    if let Some(ref status) = app.status {
        draw_status(f, status, v_chunks[2]);
    }
}

/// Render the sidebar with workstreams and sessions sections.
fn draw_sidebar(f: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focus == Focus::Sidebar;

    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Split the inner area: workstreams header + items, divider, sessions
    let ws_count = app.workstreams.len().max(1); // at least 1 row for "(none)"
    let ws_header_height = 1u16;
    let ws_items_height = ws_count as u16;

    let available = inner.height;

    // Workstreams section
    let ws_section_height = (ws_header_height + ws_items_height).min(available);
    let ws_area = Rect::new(inner.x, inner.y, inner.width, ws_section_height);
    draw_workstreams_section(f, app, ws_area);

    let remaining_y = inner.y + ws_section_height;
    let remaining_h = available.saturating_sub(ws_section_height);

    if remaining_h == 0 {
        return;
    }

    // Divider line
    let divider_area = Rect::new(inner.x, remaining_y, inner.width, 1.min(remaining_h));
    let divider_text = "\u{2500}".repeat(inner.width as usize);
    let divider = Paragraph::new(divider_text).style(Style::default().fg(Color::DarkGray));
    f.render_widget(divider, divider_area);

    let remaining_y = remaining_y + 1;
    let remaining_h = remaining_h.saturating_sub(1);

    if remaining_h == 0 {
        return;
    }

    // Sessions section
    let sessions_area = Rect::new(inner.x, remaining_y, inner.width, remaining_h);
    draw_sessions_section(f, app, sessions_area);
}

/// Render the workstreams section within the sidebar.
fn draw_workstreams_section(f: &mut Frame, app: &App, area: Rect) {
    if area.height == 0 {
        return;
    }

    // Header
    let header_area = Rect::new(area.x, area.y, area.width, 1);
    let header = Paragraph::new(Span::styled(
        " Workstreams",
        Style::default()
            .add_modifier(Modifier::BOLD)
            .fg(Color::White),
    ));
    f.render_widget(header, header_area);

    let items_area = Rect::new(
        area.x,
        area.y + 1,
        area.width,
        area.height.saturating_sub(1),
    );
    if items_area.height == 0 {
        return;
    }

    if app.workstreams.is_empty() {
        let empty = Paragraph::new("  (none)").style(Style::default().fg(Color::DarkGray));
        f.render_widget(empty, items_area);
        return;
    }

    let is_ws_section =
        app.focus == Focus::Sidebar && app.sidebar_section == SidebarSection::Workstreams;

    let items: Vec<ListItem> = app
        .workstreams
        .iter()
        .enumerate()
        .map(|(i, ws)| {
            let label = if ws.is_scratch {
                format!(" {} (scratch)", ws.title)
            } else {
                format!(" {}", ws.title)
            };

            let style = if is_ws_section && i == app.selected_workstream {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else if Some(ws.id.as_str()) == app.workstream_id.as_deref() {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            ListItem::new(Line::from(Span::styled(label, style)))
        })
        .collect();

    let list = List::new(items);
    f.render_widget(list, items_area);
}

/// Render the sessions section within the sidebar.
fn draw_sessions_section(f: &mut Frame, app: &App, area: Rect) {
    if area.height == 0 {
        return;
    }

    // Header
    let header_area = Rect::new(area.x, area.y, area.width, 1);
    let header = Paragraph::new(Span::styled(
        " Sessions",
        Style::default()
            .add_modifier(Modifier::BOLD)
            .fg(Color::White),
    ));
    f.render_widget(header, header_area);

    let items_area = Rect::new(
        area.x,
        area.y + 1,
        area.width,
        area.height.saturating_sub(1),
    );
    if items_area.height == 0 {
        return;
    }

    let is_session_section =
        app.focus == Focus::Sidebar && app.sidebar_section == SidebarSection::Sessions;

    // Build items: "+ New Session" first, then actual sessions
    let mut items: Vec<ListItem> = Vec::new();

    // "+ New Session" entry (index 0)
    let new_session_style = if is_session_section && app.selected_session == 0 {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Green)
    };
    items.push(ListItem::new(Line::from(Span::styled(
        " + New Session",
        new_session_style,
    ))));

    // Actual sessions
    for (i, session) in app.sessions.iter().enumerate() {
        let display_idx = i + 1; // offset by 1 for "+ New Session"
        let label = format_session_label(&session.started_at);

        let style = if is_session_section && app.selected_session == display_idx {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else if app.session_id.as_deref() == Some(&session.id) {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        items.push(ListItem::new(Line::from(Span::styled(
            format!(" {}", label),
            style,
        ))));
    }

    let list = List::new(items);
    f.render_widget(list, items_area);
}

/// Format a session started_at timestamp for sidebar display.
fn format_session_label(started_at: &str) -> String {
    // Try to parse ISO 8601 and display a shorter form
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(started_at) {
        dt.format("%b %d %H:%M").to_string()
    } else {
        // Fallback: show the raw string, truncated
        started_at.chars().take(16).collect()
    }
}

/// Render the chat messages area.
fn draw_chat(f: &mut Frame, app: &App, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();

    for msg in &app.messages {
        let (prefix, style) = if msg.is_user {
            (
                "You: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            let prefix = if msg.streaming { "Arawn: " } else { "Arawn: " };
            (prefix, Style::default().fg(Color::Green))
        };

        // Split content into wrapped lines
        let content_lines: Vec<&str> = msg.content.split('\n').collect();
        for (i, line) in content_lines.iter().enumerate() {
            if i == 0 {
                lines.push(Line::from(vec![
                    Span::styled(prefix, style),
                    Span::raw(*line),
                ]));
            } else {
                // Continuation lines get indentation matching prefix width
                let indent = " ".repeat(prefix.len());
                lines.push(Line::from(vec![Span::raw(format!("{}{}", indent, line))]));
            }
        }

        // Blank line between messages
        lines.push(Line::from(""));
    }

    if app.waiting {
        lines.push(Line::from(Span::styled(
            "Arawn is thinking...",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::ITALIC),
        )));
    }

    // Calculate scroll: auto-scroll to bottom if enabled
    let visible_height = area.height.saturating_sub(2) as usize; // subtract borders
    let total_lines = lines.len();
    let scroll = if app.auto_scroll {
        total_lines.saturating_sub(visible_height)
    } else {
        app.chat_scroll
    };

    let chat = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Chat"))
        .wrap(Wrap { trim: false })
        .scroll((scroll as u16, 0));

    f.render_widget(chat, area);
}

/// Render the input box.
fn draw_input(f: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focus == Focus::Input;

    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let input_text = format!("> {}", app.input);
    let input = Paragraph::new(input_text.as_str())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title("Input"),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(input, area);

    // Only show cursor when input is focused
    if is_focused {
        // Place cursor after the prompt + cursor_pos
        let cursor_x = area.x + 1 + 2 + app.cursor_pos as u16; // border + "> " + pos
        let cursor_y = area.y + 1; // border
        f.set_cursor_position((cursor_x, cursor_y));
    }
}

/// Render the status line at the bottom.
fn draw_status(f: &mut Frame, status: &str, area: Rect) {
    let status_line = Paragraph::new(status).style(Style::default().fg(Color::DarkGray));
    f.render_widget(status_line, area);
}
