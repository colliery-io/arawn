//! Log panel key handling.

use crossterm::event::{KeyCode, KeyModifiers};

use super::App;

impl App {
    /// Handle keyboard events when the log panel is focused.
    pub(crate) fn handle_logs_key(&mut self, key: crossterm::event::KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.show_logs = false;
                self.focus.return_to_input();
            }
            KeyCode::Up => {
                self.log_scroll = self.log_scroll.saturating_sub(1);
            }
            KeyCode::Down => {
                self.log_scroll = self.log_scroll.saturating_add(1);
            }
            KeyCode::PageUp => {
                self.log_scroll = self.log_scroll.saturating_sub(10);
            }
            KeyCode::PageDown => {
                self.log_scroll = self.log_scroll.saturating_add(10);
            }
            KeyCode::Home => {
                self.log_scroll = 0;
            }
            KeyCode::End => {
                self.log_scroll = usize::MAX;
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.log_buffer.clear();
                self.log_scroll = 0;
            }
            _ => {}
        }
    }
}
