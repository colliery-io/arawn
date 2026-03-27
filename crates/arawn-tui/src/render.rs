//! Ratatui rendering — chat panel + input box.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::app::App;

/// Render the entire UI into the given frame.
pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if app.status.is_some() {
            vec![
                Constraint::Min(1),
                Constraint::Length(3),
                Constraint::Length(1),
            ]
        } else {
            vec![Constraint::Min(1), Constraint::Length(3)]
        })
        .split(f.area());

    draw_chat(f, app, chunks[0]);
    draw_input(f, app, chunks[1]);

    if let Some(ref status) = app.status {
        draw_status(f, status, chunks[2]);
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
    let input_text = format!("> {}", app.input);
    let input = Paragraph::new(input_text.as_str())
        .block(Block::default().borders(Borders::ALL).title("Input"))
        .style(Style::default().fg(Color::White));

    f.render_widget(input, area);

    // Place cursor after the prompt + cursor_pos
    let cursor_x = area.x + 1 + 2 + app.cursor_pos as u16; // border + "> " + pos
    let cursor_y = area.y + 1; // border
    f.set_cursor_position((cursor_x, cursor_y));
}

/// Render the status line at the bottom.
fn draw_status(f: &mut Frame, status: &str, area: Rect) {
    let status_line = Paragraph::new(status).style(Style::default().fg(Color::DarkGray));
    f.render_widget(status_line, area);
}
