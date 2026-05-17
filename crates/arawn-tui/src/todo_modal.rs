//! Interactive overlay for the `/todo` slash command (I-0049 T-0314).
//!
//! Mirrors `ceremony_modal.rs`'s state-machine pattern. Listing of
//! open todos with done-toggle, inline create, and archive. The event
//! loop pulls outcomes off this state and routes them to the
//! `todos.*` RPC methods.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::theme;

/// Compact dto mirror of `arawn_storage::Todo` — we don't depend on
/// the storage crate from the TUI; values arrive deserialized from
/// the RPC payload.
#[derive(Debug, Clone, PartialEq)]
pub struct TodoRow {
    pub id: String,
    pub body: String,
    pub kind: String,
    pub workstream: Option<String>,
    pub done_at: Option<String>,
    pub due_at: Option<String>,
}

pub struct TodoModalState {
    pub todos: Vec<TodoRow>,
    pub focused_index: usize,
    /// When `Some`, an inline text prompt is collecting the body for
    /// a new todo. `a` opens it; Enter submits via `todos.create`;
    /// Esc cancels.
    pub add_input: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TodoOutcome {
    None,
    /// Toggle done state via `todos.done` (if open) or `todos.undo`
    /// (if already done).
    Toggle { id: String, mark_done: bool },
    /// Archive via `todos.archive`.
    Archive { id: String },
    /// Create a new user todo via `todos.create`.
    Add { body: String },
    /// Close the overlay.
    Close,
}

impl TodoModalState {
    pub fn new(todos: Vec<TodoRow>) -> Self {
        Self {
            todos,
            focused_index: 0,
            add_input: None,
            last_error: None,
        }
    }

    pub fn set_todos(&mut self, todos: Vec<TodoRow>) {
        self.todos = todos;
        if !self.todos.is_empty() && self.focused_index >= self.todos.len() {
            self.focused_index = self.todos.len() - 1;
        }
    }

    fn focus_prev(&mut self) {
        if self.focused_index > 0 {
            self.focused_index -= 1;
        }
    }

    fn focus_next(&mut self) {
        if self.focused_index + 1 < self.todos.len() {
            self.focused_index += 1;
        }
    }

    fn focused(&self) -> Option<&TodoRow> {
        self.todos.get(self.focused_index)
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> TodoOutcome {
        if let Some(ref mut buf) = self.add_input {
            match key.code {
                KeyCode::Esc => {
                    self.add_input = None;
                    return TodoOutcome::None;
                }
                KeyCode::Enter => {
                    let body = buf.trim().to_string();
                    self.add_input = None;
                    if body.is_empty() {
                        return TodoOutcome::None;
                    }
                    return TodoOutcome::Add { body };
                }
                KeyCode::Backspace => {
                    buf.pop();
                    return TodoOutcome::None;
                }
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    buf.push(c);
                    return TodoOutcome::None;
                }
                _ => return TodoOutcome::None,
            }
        }

        self.last_error = None;
        match key.code {
            KeyCode::Up => {
                self.focus_prev();
                TodoOutcome::None
            }
            KeyCode::Down => {
                self.focus_next();
                TodoOutcome::None
            }
            KeyCode::Char(' ') => match self.focused() {
                Some(t) => TodoOutcome::Toggle {
                    id: t.id.clone(),
                    mark_done: t.done_at.is_none(),
                },
                None => TodoOutcome::None,
            },
            KeyCode::Char('a') => {
                self.add_input = Some(String::new());
                TodoOutcome::None
            }
            KeyCode::Char('d') => match self.focused() {
                Some(t) => TodoOutcome::Archive { id: t.id.clone() },
                None => TodoOutcome::None,
            },
            KeyCode::Char('q') | KeyCode::Esc => TodoOutcome::Close,
            _ => TodoOutcome::None,
        }
    }
}

pub fn render_todo_modal(state: &TodoModalState, frame: &mut Frame) {
    let area = frame.area();
    let modal_width = (area.width * 70 / 100)
        .max(40)
        .min(area.width.saturating_sub(4));
    let modal_height = area.height.saturating_sub(4).clamp(10, 24);
    let rect = centered_rect(modal_width, modal_height, area);

    frame.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(Span::styled(
            " Todos ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    let mut lines: Vec<Line> = Vec::new();
    if state.todos.is_empty() && state.add_input.is_none() {
        lines.push(Line::from(Span::styled(
            "No open todos — press `a` to add one.",
            Style::default().fg(theme::SUBTEXT0),
        )));
    } else {
        for (i, t) in state.todos.iter().enumerate() {
            let glyph = if t.done_at.is_some() { "[x]" } else { "[ ]" };
            let is_focused = i == state.focused_index;
            let style = if is_focused {
                Style::default()
                    .fg(theme::TEXT)
                    .bg(theme::SURFACE0)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::SUBTEXT1)
            };
            let indicator = if is_focused { "▸ " } else { "  " };
            let kind_tag = format!("[{}]", t.kind);
            let workstream_tag = t
                .workstream
                .as_deref()
                .map(|w| format!(" @{w}"))
                .unwrap_or_default();
            let due_tag = t
                .due_at
                .as_deref()
                .map(|d| format!(" (due {d})"))
                .unwrap_or_default();
            lines.push(Line::from(vec![
                Span::styled(indicator, Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!("{glyph} {} ", t.body),
                    style,
                ),
                Span::styled(kind_tag, Style::default().fg(theme::OVERLAY1)),
                Span::styled(workstream_tag, Style::default().fg(theme::OVERLAY1)),
                Span::styled(due_tag, Style::default().fg(theme::OVERLAY1)),
            ]));
        }
    }

    lines.push(Line::from(""));
    if let Some(ref buf) = state.add_input {
        lines.push(Line::from(Span::styled(
            "Add todo — type body, Enter to submit, Esc to cancel:",
            Style::default()
                .fg(theme::TEXT)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            format!("> {buf}_"),
            Style::default().fg(theme::TEXT),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            " space=toggle  a=add  d=archive  q=quit",
            Style::default()
                .fg(theme::OVERLAY1)
                .add_modifier(Modifier::ITALIC),
        )));
    }
    if let Some(ref err) = state.last_error {
        lines.push(Line::from(Span::styled(
            format!("error: {err}"),
            Style::default().fg(Color::Red),
        )));
    }

    let para = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(para, inner);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width,
        height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn todo(id: &str, body: &str, done: bool) -> TodoRow {
        TodoRow {
            id: id.into(),
            body: body.into(),
            kind: "user".into(),
            workstream: None,
            done_at: if done {
                Some("2026-05-17T08:00:00Z".into())
            } else {
                None
            },
            due_at: None,
        }
    }

    #[test]
    fn space_toggles_done_on_open_row() {
        let mut state = TodoModalState::new(vec![todo("a", "alpha", false)]);
        let outcome = state.handle_key(key(KeyCode::Char(' ')));
        assert_eq!(
            outcome,
            TodoOutcome::Toggle {
                id: "a".into(),
                mark_done: true
            }
        );
    }

    #[test]
    fn space_toggles_undo_on_done_row() {
        let mut state = TodoModalState::new(vec![todo("a", "alpha", true)]);
        let outcome = state.handle_key(key(KeyCode::Char(' ')));
        assert_eq!(
            outcome,
            TodoOutcome::Toggle {
                id: "a".into(),
                mark_done: false
            }
        );
    }

    #[test]
    fn down_advances_focus() {
        let mut state =
            TodoModalState::new(vec![todo("a", "a", false), todo("b", "b", false)]);
        state.handle_key(key(KeyCode::Down));
        assert_eq!(state.focused_index, 1);
        // No further advance past the end.
        state.handle_key(key(KeyCode::Down));
        assert_eq!(state.focused_index, 1);
    }

    #[test]
    fn a_opens_add_then_enter_submits() {
        let mut state = TodoModalState::new(vec![]);
        assert!(matches!(
            state.handle_key(key(KeyCode::Char('a'))),
            TodoOutcome::None
        ));
        assert!(state.add_input.is_some());
        state.handle_key(key(KeyCode::Char('h')));
        state.handle_key(key(KeyCode::Char('i')));
        let outcome = state.handle_key(key(KeyCode::Enter));
        assert_eq!(outcome, TodoOutcome::Add { body: "hi".into() });
        assert!(state.add_input.is_none());
    }

    #[test]
    fn esc_during_add_cancels_input() {
        let mut state = TodoModalState::new(vec![]);
        state.handle_key(key(KeyCode::Char('a')));
        state.handle_key(key(KeyCode::Char('x')));
        state.handle_key(key(KeyCode::Esc));
        assert!(state.add_input.is_none());
    }

    #[test]
    fn d_archives_focused_row() {
        let mut state = TodoModalState::new(vec![todo("a", "alpha", false)]);
        let outcome = state.handle_key(key(KeyCode::Char('d')));
        assert_eq!(outcome, TodoOutcome::Archive { id: "a".into() });
    }

    #[test]
    fn q_and_esc_close_overlay() {
        let mut state = TodoModalState::new(vec![]);
        assert_eq!(state.handle_key(key(KeyCode::Char('q'))), TodoOutcome::Close);
        assert_eq!(state.handle_key(key(KeyCode::Esc)), TodoOutcome::Close);
    }

    #[test]
    fn set_todos_clamps_focused_index() {
        let mut state =
            TodoModalState::new(vec![todo("a", "a", false), todo("b", "b", false)]);
        state.focused_index = 1;
        state.set_todos(vec![todo("c", "c", false)]);
        assert_eq!(state.focused_index, 0);
    }
}
