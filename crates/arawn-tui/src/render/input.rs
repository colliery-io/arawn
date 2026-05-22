use ratatui::Frame;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::app::{App, Focus};
use crate::theme;
use super::truncate_to;

pub(super) fn render_input(app: &App, frame: &mut Frame, area: ratatui::layout::Rect) {
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
pub(super) fn render_autocomplete(
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

