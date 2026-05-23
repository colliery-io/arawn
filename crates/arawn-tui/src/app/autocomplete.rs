

use crate::command::AutocompleteState;

use super::App;


impl App {
    /// Update autocomplete suggestions based on current input buffer.
    pub(super) fn update_autocomplete(&mut self) {
        let trimmed = self.input_buffer.trim_start();
        if trimmed.starts_with('/') && trimmed.len() > 1 {
            // Extract the command prefix (after /)
            let after_slash = &trimmed[1..];
            let prefix = after_slash.split_whitespace().next().unwrap_or(after_slash);

            let matches: Vec<_> = self
                .command_registry
                .matching(prefix)
                .into_iter()
                .cloned()
                .collect();

            if matches.is_empty() {
                self.autocomplete = None;
            } else {
                self.autocomplete = Some(AutocompleteState::new(matches));
            }
        } else if trimmed == "/" {
            // Show all commands when just "/" is typed
            let all: Vec<_> = self.command_registry.all().to_vec();
            self.autocomplete = Some(AutocompleteState::new(all));
        } else {
            self.autocomplete = None;
        }
    }

    /// Accept the currently selected autocomplete suggestion.
    pub(super) fn accept_autocomplete(&mut self) {
        if let Some(ref ac) = self.autocomplete
            && let Some(cmd) = ac.selected_command()
        {
            self.input_buffer = format!("/{}", cmd.name);
            self.cursor_pos = self.input_buffer.len();
        }
        self.autocomplete = None;
    }
}
