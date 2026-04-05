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
    /// Autocomplete: move to next suggestion.
    AutocompleteNext,
    /// Autocomplete: move to previous suggestion.
    AutocompletePrev,
    /// Autocomplete: accept selected suggestion.
    AutocompleteAccept,
    /// Autocomplete: dismiss dropdown.
    AutocompleteDismiss,
}
