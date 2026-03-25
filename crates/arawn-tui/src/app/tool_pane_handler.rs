//! Tool pane key handling and external editor support.

use crossterm::event::{KeyCode, KeyModifiers};

use super::App;

impl App {
    /// Handle keyboard events when the tool pane is focused.
    pub(crate) fn handle_tool_pane_key(&mut self, key: crossterm::event::KeyEvent) {
        let has_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        match key.code {
            KeyCode::Esc => {
                self.show_tool_pane = false;
                self.selected_tool_index = None;
                self.focus.return_to_input();
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if !self.tools.is_empty() {
                    let current = self.selected_tool_index.unwrap_or(0);
                    self.selected_tool_index = Some(current.saturating_sub(1));
                    self.tool_scroll = 0;
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if !self.tools.is_empty() {
                    let current = self.selected_tool_index.unwrap_or(0);
                    let max_idx = self.tools.len().saturating_sub(1);
                    self.selected_tool_index = Some((current + 1).min(max_idx));
                    self.tool_scroll = 0;
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.tool_scroll = self.tool_scroll.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.tool_scroll = self.tool_scroll.saturating_add(1);
            }
            KeyCode::PageUp => {
                self.tool_scroll = self.tool_scroll.saturating_sub(10);
            }
            KeyCode::PageDown => {
                self.tool_scroll = self.tool_scroll.saturating_add(10);
            }
            KeyCode::Home => {
                if has_ctrl {
                    if !self.tools.is_empty() {
                        self.selected_tool_index = Some(0);
                        self.tool_scroll = 0;
                    }
                } else {
                    self.tool_scroll = 0;
                }
            }
            KeyCode::End => {
                if has_ctrl {
                    if !self.tools.is_empty() {
                        self.selected_tool_index = Some(self.tools.len() - 1);
                        self.tool_scroll = 0;
                    }
                } else {
                    self.tool_scroll = usize::MAX;
                }
            }
            KeyCode::Char('o') if has_ctrl => {
                self.open_tool_in_editor();
            }
            _ => {}
        }
    }

    /// Open the selected tool's output in an external pager.
    pub(crate) fn open_tool_in_editor(&mut self) {
        let Some(idx) = self.selected_tool_index else {
            self.status_message = Some("No tool selected".to_string());
            return;
        };
        let Some(tool) = self.tools.get(idx) else {
            self.status_message = Some("Tool not found".to_string());
            return;
        };

        if tool.output.is_empty() {
            self.status_message = Some("Tool has no output".to_string());
            return;
        }

        let pager = std::env::var("PAGER")
            .or_else(|_| std::env::var("EDITOR"))
            .unwrap_or_else(|_| "less".to_string());

        let output = tool.output.clone();
        let tool_name = tool.name.clone();

        if let Err(e) = self.run_pager(&pager, &output) {
            self.status_message = Some(format!("Failed to open pager: {}", e));
        } else {
            self.status_message = Some(format!("Viewed {} output in {}", tool_name, pager));
        }
    }

    /// Run a pager with the given content, suspending and restoring the TUI.
    pub(crate) fn run_pager(&self, pager: &str, content: &str) -> std::io::Result<()> {
        use crossterm::{
            execute,
            terminal::{
                EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
            },
        };
        use std::io::Write;

        let mut tmp = tempfile::NamedTempFile::new()?;
        tmp.write_all(content.as_bytes())?;
        tmp.flush()?;

        disable_raw_mode()?;
        execute!(std::io::stdout(), LeaveAlternateScreen)?;

        let status = std::process::Command::new(pager).arg(tmp.path()).status();

        execute!(std::io::stdout(), EnterAlternateScreen)?;
        enable_raw_mode()?;

        match status {
            Ok(exit) if exit.success() => Ok(()),
            Ok(exit) => Err(std::io::Error::other(format!(
                "Pager exited with status: {}",
                exit
            ))),
            Err(e) => Err(e),
        }
    }
}
