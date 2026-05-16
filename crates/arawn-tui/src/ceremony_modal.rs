//! Interactive overlays for ceremony slash commands (T-0308).
//!
//! The standard `ModalState` (modal.rs) is single-select via a oneshot
//! channel — it can't express multi-key actions like "space=confirm,
//! d=reject, a=add". These two overlays drive the `/week` priority flow
//! and the `/retro` diary editor with their own pure state machines.
//! The event loop pulls outcomes off them and routes RPCs.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::theme;

/// One of the two interactive ceremony surfaces. App holds at most one
/// at a time.
pub enum CeremonyOverlay {
    Priority(PriorityModalState),
    Diary(DiaryEditorState),
}

// -----------------------------------------------------------------------------
// Priority modal (slice 1)
// -----------------------------------------------------------------------------

/// State for the `/week` priority confirm/reject/add overlay.
pub struct PriorityModalState {
    pub tablet_id: String,
    pub priorities: Vec<arawn_ceremonies::PriorityDto>,
    pub focused_index: usize,
    /// When `Some`, an inline text prompt is collecting the body for a
    /// new candidate priority. `a` opens it; Enter submits; Esc cancels.
    pub add_input: Option<String>,
    /// Last RPC error to render on the next frame. Cleared on next input.
    pub last_error: Option<String>,
}

/// What the event loop should do after `handle_key`.
#[derive(Debug, Clone, PartialEq)]
pub enum PriorityOutcome {
    /// No-op; redraw only.
    None,
    /// Confirm the item with this id (RPC `ceremonies.confirm_priority`).
    Confirm { item_id: String },
    /// Reject the item with this id (RPC `ceremonies.reject_priority`).
    Reject { item_id: String },
    /// Submit a new priority (RPC `ceremonies.add_priority`).
    Add { tablet_id: String, body: String },
    /// Dismiss the overlay.
    Close,
}

impl PriorityModalState {
    pub fn new(tablet_id: String, priorities: Vec<arawn_ceremonies::PriorityDto>) -> Self {
        Self {
            tablet_id,
            priorities,
            focused_index: 0,
            add_input: None,
            last_error: None,
        }
    }

    /// Replace the priority list (e.g. after an RPC mutation succeeds).
    /// Clamps `focused_index` to stay in bounds.
    pub fn set_priorities(&mut self, priorities: Vec<arawn_ceremonies::PriorityDto>) {
        self.priorities = priorities;
        if !self.priorities.is_empty() && self.focused_index >= self.priorities.len() {
            self.focused_index = self.priorities.len() - 1;
        }
    }

    pub fn focus_prev(&mut self) {
        if self.focused_index > 0 {
            self.focused_index -= 1;
        }
    }

    pub fn focus_next(&mut self) {
        if self.focused_index + 1 < self.priorities.len() {
            self.focused_index += 1;
        }
    }

    fn focused_item_id(&self) -> Option<String> {
        self.priorities.get(self.focused_index).map(|p| p.id.clone())
    }

    /// Drive the state machine from a key event. Returns the outcome
    /// the event loop should act on.
    pub fn handle_key(&mut self, key: KeyEvent) -> PriorityOutcome {
        // Inline text prompt has priority over global bindings.
        if let Some(ref mut buf) = self.add_input {
            match key.code {
                KeyCode::Esc => {
                    self.add_input = None;
                    return PriorityOutcome::None;
                }
                KeyCode::Enter => {
                    let body = buf.trim().to_string();
                    self.add_input = None;
                    if body.is_empty() {
                        return PriorityOutcome::None;
                    }
                    return PriorityOutcome::Add {
                        tablet_id: self.tablet_id.clone(),
                        body,
                    };
                }
                KeyCode::Backspace => {
                    buf.pop();
                    return PriorityOutcome::None;
                }
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    buf.push(c);
                    return PriorityOutcome::None;
                }
                _ => return PriorityOutcome::None,
            }
        }

        self.last_error = None;
        match key.code {
            KeyCode::Up => {
                self.focus_prev();
                PriorityOutcome::None
            }
            KeyCode::Down => {
                self.focus_next();
                PriorityOutcome::None
            }
            KeyCode::Char(' ') => match self.focused_item_id() {
                Some(id) => PriorityOutcome::Confirm { item_id: id },
                None => PriorityOutcome::None,
            },
            KeyCode::Char('d') => match self.focused_item_id() {
                Some(id) => PriorityOutcome::Reject { item_id: id },
                None => PriorityOutcome::None,
            },
            KeyCode::Char('a') => {
                self.add_input = Some(String::new());
                PriorityOutcome::None
            }
            KeyCode::Char('q') | KeyCode::Esc => PriorityOutcome::Close,
            _ => PriorityOutcome::None,
        }
    }
}

// -----------------------------------------------------------------------------
// Diary editor (slice 2)
// -----------------------------------------------------------------------------

/// State for the `/retro` diary editor. Minimal multi-line line buffer.
pub struct DiaryEditorState {
    pub tablet_id: String,
    pub body: String,
    /// Byte offset into `body`. Always on a char boundary.
    pub cursor: usize,
    /// True once the user pressed `e` to enter edit mode. Until then
    /// the overlay is read-only.
    pub editing: bool,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DiaryOutcome {
    None,
    Save { tablet_id: String, body: String },
    Close,
}

impl DiaryEditorState {
    pub fn new(tablet_id: String, body: String) -> Self {
        let cursor = body.len();
        Self {
            tablet_id,
            body,
            cursor,
            editing: false,
            last_error: None,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> DiaryOutcome {
        if !self.editing {
            match key.code {
                KeyCode::Char('e') => {
                    self.editing = true;
                    DiaryOutcome::None
                }
                KeyCode::Char('q') | KeyCode::Esc => DiaryOutcome::Close,
                _ => DiaryOutcome::None,
            }
        } else {
            // Edit mode: ctrl-s saves; esc cancels edit (drop changes is too
            // surprising — preserve the buffer but flip back to read-only).
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                if let KeyCode::Char('s') = key.code {
                    return DiaryOutcome::Save {
                        tablet_id: self.tablet_id.clone(),
                        body: self.body.clone(),
                    };
                }
            }
            match key.code {
                KeyCode::Esc => {
                    self.editing = false;
                    DiaryOutcome::None
                }
                KeyCode::Enter => {
                    self.insert_char('\n');
                    DiaryOutcome::None
                }
                KeyCode::Backspace => {
                    self.backspace();
                    DiaryOutcome::None
                }
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.insert_char(c);
                    DiaryOutcome::None
                }
                KeyCode::Left => {
                    if self.cursor > 0 {
                        let mut p = self.cursor - 1;
                        while p > 0 && !self.body.is_char_boundary(p) {
                            p -= 1;
                        }
                        self.cursor = p;
                    }
                    DiaryOutcome::None
                }
                KeyCode::Right => {
                    if self.cursor < self.body.len() {
                        let mut p = self.cursor + 1;
                        while p < self.body.len() && !self.body.is_char_boundary(p) {
                            p += 1;
                        }
                        self.cursor = p;
                    }
                    DiaryOutcome::None
                }
                _ => DiaryOutcome::None,
            }
        }
    }

    fn insert_char(&mut self, c: char) {
        self.body.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let mut p = self.cursor - 1;
        while p > 0 && !self.body.is_char_boundary(p) {
            p -= 1;
        }
        self.body.drain(p..self.cursor);
        self.cursor = p;
    }
}

// -----------------------------------------------------------------------------
// Rendering
// -----------------------------------------------------------------------------

pub fn render_overlay(overlay: &CeremonyOverlay, frame: &mut Frame) {
    match overlay {
        CeremonyOverlay::Priority(state) => render_priority_modal(state, frame),
        CeremonyOverlay::Diary(state) => render_diary_editor(state, frame),
    }
}

fn render_priority_modal(state: &PriorityModalState, frame: &mut Frame) {
    let area = frame.area();
    let modal_width = (area.width * 70 / 100).max(40).min(area.width.saturating_sub(4));
    let modal_height = area.height.saturating_sub(4).min(24).max(10);
    let rect = centered_rect(modal_width, modal_height, area);

    frame.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta))
        .title(Span::styled(
            " Weekly priorities ",
            Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    let mut lines: Vec<Line> = Vec::new();
    if state.priorities.is_empty() {
        lines.push(Line::from(Span::styled(
            "No priorities on this tablet yet — press `a` to add one.",
            Style::default().fg(theme::SUBTEXT0),
        )));
    } else {
        for (i, p) in state.priorities.iter().enumerate() {
            let glyph = if p.confirmed_at.is_some() { "[x]" } else { "[ ]" };
            let is_focused = i == state.focused_index;
            let body = p
                .body
                .as_str()
                .map(|s| s.to_string())
                .unwrap_or_else(|| p.body.to_string());
            let style = if is_focused {
                Style::default()
                    .fg(theme::TEXT)
                    .bg(theme::SURFACE0)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::SUBTEXT1)
            };
            let indicator = if is_focused { "▸ " } else { "  " };
            lines.push(Line::from(vec![
                Span::styled(indicator, Style::default().fg(Color::Magenta)),
                Span::styled(format!("{glyph} {body}"), style),
            ]));
            if !p.rationale.is_empty() {
                lines.push(Line::from(Span::styled(
                    format!("      {}", p.rationale),
                    Style::default().fg(theme::OVERLAY1),
                )));
            }
        }
    }

    lines.push(Line::from(""));
    if let Some(ref buf) = state.add_input {
        lines.push(Line::from(Span::styled(
            "Add priority — type body, Enter to submit, Esc to cancel:",
            Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            format!("> {buf}_"),
            Style::default().fg(theme::TEXT),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            " space=confirm  d=reject  a=add  q=quit",
            Style::default().fg(theme::OVERLAY1).add_modifier(Modifier::ITALIC),
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

fn render_diary_editor(state: &DiaryEditorState, frame: &mut Frame) {
    let area = frame.area();
    let modal_width = (area.width * 80 / 100).max(50).min(area.width.saturating_sub(4));
    let modal_height = area.height.saturating_sub(4).min(24).max(10);
    let rect = centered_rect(modal_width, modal_height, area);

    frame.render_widget(Clear, rect);
    let title = if state.editing { " Diary (editing) " } else { " Diary " };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(
            title,
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    let mut lines: Vec<Line> = Vec::new();
    if state.body.is_empty() && !state.editing {
        lines.push(Line::from(Span::styled(
            "No diary entry yet — press `e` to write one.",
            Style::default().fg(theme::SUBTEXT0),
        )));
    } else {
        for line in state.body.split('\n') {
            lines.push(Line::from(Span::raw(line.to_string())));
        }
        if state.editing {
            // Cursor marker on the final line (good-enough hint without
            // computing actual screen position).
            lines.push(Line::from(Span::styled(
                "_",
                Style::default().fg(theme::TEXT).add_modifier(Modifier::SLOW_BLINK),
            )));
        }
    }

    lines.push(Line::from(""));
    let footer = if state.editing {
        " ctrl-s=save  esc=stop editing"
    } else {
        " e=edit  q=quit"
    };
    lines.push(Line::from(Span::styled(
        footer,
        Style::default().fg(theme::OVERLAY1).add_modifier(Modifier::ITALIC),
    )));
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
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}

#[cfg(test)]
mod tests {
    use super::*;
    use arawn_ceremonies::PriorityDto;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }
    fn ctrl(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    fn make_priority(id: &str, body: &str, confirmed: bool) -> PriorityDto {
        PriorityDto {
            id: id.into(),
            tablet_id: "t1".into(),
            body: serde_json::Value::String(body.into()),
            rationale: "because reasons".into(),
            citation_id: None,
            confirmed_at: if confirmed { Some("2026-01-01T00:00:00Z".into()) } else { None },
            done_at: None,
            ordinal: 0,
            source: if confirmed { "confirmed".into() } else { "candidate".into() },
        }
    }

    fn make_state(n: usize) -> PriorityModalState {
        let ps = (0..n).map(|i| make_priority(&format!("i{i}"), "body", false)).collect();
        PriorityModalState::new("t1".into(), ps)
    }

    // ---- PriorityModalState ----

    #[test]
    fn priority_focus_nav_clamps() {
        let mut s = make_state(3);
        assert_eq!(s.focused_index, 0);
        s.focus_next();
        s.focus_next();
        s.focus_next(); // clamp
        assert_eq!(s.focused_index, 2);
        s.focus_prev();
        s.focus_prev();
        s.focus_prev(); // clamp
        assert_eq!(s.focused_index, 0);
    }

    #[test]
    fn priority_space_confirms_focused_item() {
        let mut s = make_state(2);
        s.focus_next();
        let out = s.handle_key(key(KeyCode::Char(' ')));
        assert_eq!(out, PriorityOutcome::Confirm { item_id: "i1".into() });
    }

    #[test]
    fn priority_d_rejects_focused_item() {
        let mut s = make_state(2);
        let out = s.handle_key(key(KeyCode::Char('d')));
        assert_eq!(out, PriorityOutcome::Reject { item_id: "i0".into() });
    }

    #[test]
    fn priority_a_opens_add_buffer_and_collects_input() {
        let mut s = make_state(1);
        s.handle_key(key(KeyCode::Char('a')));
        assert!(s.add_input.is_some());
        s.handle_key(key(KeyCode::Char('h')));
        s.handle_key(key(KeyCode::Char('i')));
        let out = s.handle_key(key(KeyCode::Enter));
        assert_eq!(out, PriorityOutcome::Add {
            tablet_id: "t1".into(),
            body: "hi".into(),
        });
        assert!(s.add_input.is_none());
    }

    #[test]
    fn priority_add_esc_cancels_without_emitting() {
        let mut s = make_state(1);
        s.handle_key(key(KeyCode::Char('a')));
        s.handle_key(key(KeyCode::Char('x')));
        let out = s.handle_key(key(KeyCode::Esc));
        assert_eq!(out, PriorityOutcome::None);
        assert!(s.add_input.is_none());
    }

    #[test]
    fn priority_add_empty_body_does_not_emit() {
        let mut s = make_state(1);
        s.handle_key(key(KeyCode::Char('a')));
        let out = s.handle_key(key(KeyCode::Enter));
        assert_eq!(out, PriorityOutcome::None);
    }

    #[test]
    fn priority_q_and_esc_close_overlay() {
        let mut s = make_state(1);
        assert_eq!(s.handle_key(key(KeyCode::Char('q'))), PriorityOutcome::Close);
        let mut s2 = make_state(1);
        assert_eq!(s2.handle_key(key(KeyCode::Esc)), PriorityOutcome::Close);
    }

    #[test]
    fn priority_set_priorities_clamps_focused_index() {
        let mut s = make_state(3);
        s.focused_index = 2;
        s.set_priorities(vec![make_priority("only", "x", false)]);
        assert_eq!(s.focused_index, 0);
    }

    // ---- DiaryEditorState ----

    #[test]
    fn diary_e_enters_edit_mode_and_q_closes() {
        let mut d = DiaryEditorState::new("t1".into(), String::new());
        assert!(!d.editing);
        d.handle_key(key(KeyCode::Char('e')));
        assert!(d.editing);
        let d2_out = DiaryEditorState::new("t1".into(), String::new())
            .handle_key(key(KeyCode::Char('q')));
        assert_eq!(d2_out, DiaryOutcome::Close);
    }

    #[test]
    fn diary_char_insert_and_newline_and_backspace() {
        let mut d = DiaryEditorState::new("t1".into(), String::new());
        d.editing = true;
        d.handle_key(key(KeyCode::Char('h')));
        d.handle_key(key(KeyCode::Char('i')));
        d.handle_key(key(KeyCode::Enter));
        d.handle_key(key(KeyCode::Char('!')));
        assert_eq!(d.body, "hi\n!");
        d.handle_key(key(KeyCode::Backspace));
        assert_eq!(d.body, "hi\n");
        d.handle_key(key(KeyCode::Backspace));
        assert_eq!(d.body, "hi");
    }

    #[test]
    fn diary_ctrl_s_emits_save_with_body() {
        let mut d = DiaryEditorState::new("t1".into(), "draft".into());
        d.editing = true;
        let out = d.handle_key(ctrl('s'));
        assert_eq!(out, DiaryOutcome::Save {
            tablet_id: "t1".into(),
            body: "draft".into(),
        });
    }

    #[test]
    fn diary_esc_in_edit_mode_leaves_edit_but_keeps_body() {
        let mut d = DiaryEditorState::new("t1".into(), "kept".into());
        d.editing = true;
        d.handle_key(key(KeyCode::Esc));
        assert!(!d.editing);
        assert_eq!(d.body, "kept");
    }

    #[test]
    fn diary_cursor_left_right_navigates() {
        let mut d = DiaryEditorState::new("t1".into(), "ab".into());
        d.editing = true;
        assert_eq!(d.cursor, 2);
        d.handle_key(key(KeyCode::Left));
        assert_eq!(d.cursor, 1);
        d.handle_key(key(KeyCode::Char('X')));
        assert_eq!(d.body, "aXb");
        assert_eq!(d.cursor, 2);
    }
}
