/// All possible user actions in the TUI.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    TypeChar(char),
    Backspace,
    Delete,
    CursorLeft,
    CursorRight,
    CursorHome,
    CursorEnd,
    Submit,
    Tab,
    Quit,
    ScrollUp,
    ScrollDown,
    ScrollPageUp,
    ScrollPageDown,
    SidebarUp,
    SidebarDown,
    SidebarSelect,
    NewSession,
    Cancel,
    /// Click in a panel area to focus it.
    ClickFocus(crate::app::Focus),
    /// Click a specific sidebar item (section-relative index).
    ClickSidebarItem(usize),
    /// Click in the input area at a specific column (for cursor placement).
    ClickInput(u16),
    /// Toggle expand/collapse on a tool result message by index.
    ToggleToolResult(usize),
    /// Toggle expand/collapse on all tool results at once.
    ToggleAllToolResults,
    /// Modal: move focus up.
    ModalUp,
    /// Modal: move focus down.
    ModalDown,
    /// Modal: confirm selection.
    ModalConfirm,
    /// Modal: cancel.
    ModalCancel,
    /// Modal: directly pick option N (number-key shortcut). Idx is
    /// 0-based; `1` key produces `ModalSelectIndex(0)`. If the index
    /// is out of range for the current modal, the action is a no-op.
    ModalSelectIndex(usize),
    /// Autocomplete: move to next suggestion.
    AutocompleteNext,
    /// Autocomplete: move to previous suggestion.
    AutocompletePrev,
    /// Autocomplete: accept selected suggestion.
    AutocompleteAccept,
    /// Autocomplete: dismiss dropdown.
    AutocompleteDismiss,
    /// Esc pressed when nothing transient is active (no modal, no
    /// autocomplete, not generating). Used for double-Esc detection that
    /// opens the history modal.
    EscapeIdle,
    /// Recall a prompt from input history into the input buffer (Up arrow
    /// in main focus, when input is empty or already in history mode).
    HistoryRecallPrev,
    /// Move forward through history (Down arrow in main focus while in
    /// history mode); restores the saved draft past the most recent entry.
    HistoryRecallNext,
    /// Replace the input buffer with `history[index]`, exiting any
    /// history-modal selection.
    HistoryRecallAt(usize),
}
