//! Ratatui rendering — sidebar + chat panel + input box.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

use crate::app::{App, Focus};

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

/// Render the workstream sidebar.
fn draw_sidebar(f: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focus == Focus::Sidebar;

    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(Span::styled(
            "Workstreams",
            Style::default().add_modifier(Modifier::BOLD),
        ));

    if app.workstreams.is_empty() {
        let empty = Paragraph::new("  (none)")
            .block(block)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(empty, area);
        return;
    }

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

            let style = if i == app.selected_workstream {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            ListItem::new(Line::from(Span::styled(label, style)))
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);
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
