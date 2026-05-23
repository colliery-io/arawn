


use super::{App, HistoryEntry};


impl App {
    /// Append `text` to input history, skipping empty input and deduping
    /// consecutive duplicates. Resets browse state — the next Up/Down
    /// starts fresh from the newest entry. `is_chat = false` flags
    /// slash-command entries so the branch modal can skip them.
    pub(super) fn record_input_history(&mut self, text: &str, is_chat: bool) {
        if !text.is_empty() && self.history.last().map(|e| e.text.as_str()) != Some(text) {
            self.history.push(HistoryEntry {
                text: text.to_string(),
                is_chat,
            });
        }
        self.history_cursor = None;
        self.history_draft.clear();
    }

    /// Move backward in input history. Saves the current draft on first
    /// entry into history mode so Down can restore it past the newest entry.
    pub(super) fn history_recall_prev(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let next_idx = match self.history_cursor {
            None => {
                self.history_draft = self.input_buffer.clone();
                self.history.len() - 1
            }
            Some(0) => return, // already at oldest
            Some(i) => i - 1,
        };
        self.history_cursor = Some(next_idx);
        self.input_buffer = self.history[next_idx].text.clone();
        self.cursor_pos = self.input_buffer.chars().count();
    }

    /// Move forward in input history. Past the newest entry, restores
    /// the saved draft and exits history mode.
    pub(super) fn history_recall_next(&mut self) {
        let Some(idx) = self.history_cursor else {
            return;
        };
        if idx + 1 < self.history.len() {
            let next = idx + 1;
            self.history_cursor = Some(next);
            self.input_buffer = self.history[next].text.clone();
            self.cursor_pos = self.input_buffer.chars().count();
        } else {
            // Past newest — restore draft and leave history mode.
            self.history_cursor = None;
            self.input_buffer = std::mem::take(&mut self.history_draft);
            self.cursor_pos = self.input_buffer.chars().count();
        }
    }

    /// Open a modal listing branchable history entries (chat prompts only,
    /// newest first). Selecting an entry triggers a session truncate to
    /// that point and loads the entry into the input buffer for editing.
    /// Slash commands are excluded — they don't correspond to a server-
    /// side message turn and aren't branchable.
    pub(super) fn open_history_modal(&mut self) {
        // Collect (history_index, chat_index, text) triples for chat
        // entries only. chat_index is the position in the chat-only
        // chronological list — what the truncate RPC takes as its
        // user_message_index.
        let chat_entries: Vec<(usize, usize, String)> = self
            .history
            .iter()
            .enumerate()
            .filter(|(_, e)| e.is_chat)
            .scan(0usize, |chat_idx, (history_idx, e)| {
                let triple = (history_idx, *chat_idx, e.text.clone());
                *chat_idx += 1;
                Some(triple)
            })
            .collect();

        if chat_entries.is_empty() {
            return;
        }

        // Newest first; truncate over-long entries for the modal label.
        // The description carries the history index AND chat index so the
        // event loop can read them back without recomputing.
        let options: Vec<crate::modal::ModalOption> = chat_entries
            .iter()
            .rev()
            .map(|(history_idx, chat_idx, text)| {
                let label = if text.chars().count() > 80 {
                    let head: String = text.chars().take(79).collect();
                    format!("{head}…")
                } else {
                    text.clone()
                };
                crate::modal::ModalOption::new(label)
                    .with_description(format!("h={history_idx} c={chat_idx}"))
            })
            .collect();

        let (tx, rx) = tokio::sync::oneshot::channel();
        let mut modal = crate::modal::ModalState::new(
            "Branch from a prior prompt — pick one to rewind to and edit",
            options,
            ratatui::style::Color::Cyan,
            tx,
        );
        modal = modal.with_subtitle(
            "The session will rewind to before this prompt; the prompt loads into your input for editing.",
        );
        self.active_modal = Some(modal);
        // The event loop handles modal close — recognizes this special
        // request_id, parses h=/c= from the selected option's description,
        // calls truncate RPC + loads the text.
        self.pending_modal_response = Some(("__history_branch__".into(), rx));
    }
}
