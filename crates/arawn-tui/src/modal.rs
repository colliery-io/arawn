//! Interactive modal — unified primitive for permission prompts and AskUser tool.
//!
//! Renders as a centered overlay with selectable options, arrow key navigation,
//! Enter to confirm, Escape to cancel. Used for permission requests, AskUser
//! questions, and any future tool that needs user input.

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

/// A single option in the modal.
#[derive(Debug, Clone)]
pub struct ModalOption {
    pub label: String,
    pub description: Option<String>,
}

impl ModalOption {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            description: None,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// Active modal state. When present on App, the modal is visible and captures all input.
pub struct ModalState {
    pub title: String,
    pub subtitle: Option<String>,
    pub options: Vec<ModalOption>,
    pub focused_index: usize,
    pub border_color: Color,
    /// Channel to send the selected index back to the caller.
    /// None = cancel (Escape). Some(index) = selected option.
    pub result_tx: Option<tokio::sync::oneshot::Sender<Option<usize>>>,
}

impl ModalState {
    pub fn new(
        title: impl Into<String>,
        options: Vec<ModalOption>,
        border_color: Color,
        result_tx: tokio::sync::oneshot::Sender<Option<usize>>,
    ) -> Self {
        Self {
            title: title.into(),
            subtitle: None,
            options,
            focused_index: 0,
            border_color,
            result_tx: Some(result_tx),
        }
    }

    pub fn with_subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Move focus up.
    pub fn focus_prev(&mut self) {
        if self.focused_index > 0 {
            self.focused_index -= 1;
        }
    }

    /// Move focus down.
    pub fn focus_next(&mut self) {
        if self.focused_index + 1 < self.options.len() {
            self.focused_index += 1;
        }
    }

    /// Confirm the focused option. Sends the index and consumes the channel.
    pub fn confirm(&mut self) {
        if let Some(tx) = self.result_tx.take() {
            let _ = tx.send(Some(self.focused_index));
        }
    }

    /// Cancel (Escape). Sends None and consumes the channel.
    pub fn cancel(&mut self) {
        if let Some(tx) = self.result_tx.take() {
            let _ = tx.send(None);
        }
    }
}

/// Render the modal as a centered overlay.
pub fn render_modal(modal: &ModalState, frame: &mut Frame) {
    let area = frame.area();

    // Calculate centered rect (60% width, height based on content)
    let content_lines = 2 // title + blank
        + modal.subtitle.as_ref().map(|_| 2).unwrap_or(0) // subtitle + blank
        + modal.options.len() * 2 // each option + spacing
        + 2; // footer hints + border padding
    let modal_height = (content_lines as u16 + 2).min(area.height.saturating_sub(4)); // +2 for borders
    let modal_width = (area.width * 60 / 100).max(30).min(area.width.saturating_sub(4));
    let modal_rect = centered_rect(modal_width, modal_height, area);

    // Clear the area behind the modal
    frame.render_widget(Clear, modal_rect);

    // Render the border block
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(modal.border_color))
        .title(Span::styled(
            format!(" {} ", modal.title),
            Style::default()
                .fg(modal.border_color)
                .add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(modal_rect);
    frame.render_widget(block, modal_rect);

    // Build content lines
    let mut lines: Vec<Line> = Vec::new();

    // Subtitle
    if let Some(ref subtitle) = modal.subtitle {
        lines.push(Line::from(Span::styled(
            subtitle.clone(),
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(""));
    }

    // Options
    for (i, opt) in modal.options.iter().enumerate() {
        let is_focused = i == modal.focused_index;
        let indicator = if is_focused { "▸ " } else { "  " };
        let label_style = if is_focused {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
                .bg(Color::Rgb(50, 50, 65))
        } else {
            Style::default().fg(Color::Rgb(180, 180, 195))
        };

        lines.push(Line::from(vec![
            Span::styled(
                indicator,
                if is_focused {
                    Style::default().fg(modal.border_color)
                } else {
                    Style::default()
                },
            ),
            Span::styled(opt.label.clone(), label_style),
        ]));

        if let Some(ref desc) = opt.description {
            lines.push(Line::from(Span::styled(
                format!("    {desc}"),
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    // Footer
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " ↑↓ navigate  Enter confirm  Esc cancel",
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC),
    )));

    let content = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(content, inner);
}

/// Calculate a centered rectangle within an area.
fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_modal() -> ModalState {
        let (tx, _rx) = tokio::sync::oneshot::channel();
        ModalState::new(
            "Test Modal",
            vec![
                ModalOption::new("Option A"),
                ModalOption::new("Option B").with_description("Description for B"),
                ModalOption::new("Option C"),
            ],
            Color::Yellow,
            tx,
        )
    }

    #[test]
    fn navigation() {
        let mut modal = make_modal();
        assert_eq!(modal.focused_index, 0);

        modal.focus_next();
        assert_eq!(modal.focused_index, 1);

        modal.focus_next();
        assert_eq!(modal.focused_index, 2);

        // Clamp at end
        modal.focus_next();
        assert_eq!(modal.focused_index, 2);

        modal.focus_prev();
        assert_eq!(modal.focused_index, 1);

        modal.focus_prev();
        assert_eq!(modal.focused_index, 0);

        // Clamp at start
        modal.focus_prev();
        assert_eq!(modal.focused_index, 0);
    }

    #[test]
    fn confirm_sends_index() {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let mut modal = ModalState::new(
            "Test",
            vec![ModalOption::new("A"), ModalOption::new("B")],
            Color::Yellow,
            tx,
        );
        modal.focus_next(); // focus B (index 1)
        modal.confirm();

        assert_eq!(rx.blocking_recv().unwrap(), Some(1));
    }

    #[test]
    fn cancel_sends_none() {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let mut modal = ModalState::new(
            "Test",
            vec![ModalOption::new("A")],
            Color::Yellow,
            tx,
        );
        modal.cancel();

        assert_eq!(rx.blocking_recv().unwrap(), None);
    }

    #[test]
    fn confirm_only_sends_once() {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let mut modal = ModalState::new(
            "Test",
            vec![ModalOption::new("A")],
            Color::Yellow,
            tx,
        );
        modal.confirm();
        modal.confirm(); // second call is a no-op

        assert_eq!(rx.blocking_recv().unwrap(), Some(0));
    }

    #[test]
    fn centered_rect_calculation() {
        let area = Rect::new(0, 0, 80, 24);
        let r = centered_rect(40, 10, area);
        assert_eq!(r.x, 20);
        assert_eq!(r.y, 7);
        assert_eq!(r.width, 40);
        assert_eq!(r.height, 10);
    }
}
