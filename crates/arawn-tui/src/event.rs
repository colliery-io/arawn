use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::action::Action;
use crate::app::Focus;

/// Map a crossterm KeyEvent to an Action, given the current focus.
pub fn map_key_event(
    key: KeyEvent,
    focus: Focus,
    is_generating: bool,
    has_modal: bool,
    has_autocomplete: bool,
) -> Option<Action> {
    // Modal overlay captures all keys when active
    if has_modal {
        return map_modal_key(key);
    }

    // Autocomplete dropdown captures navigation keys when active
    if has_autocomplete {
        match key.code {
            KeyCode::Up => return Some(Action::AutocompletePrev),
            KeyCode::Down => return Some(Action::AutocompleteNext),
            KeyCode::Tab => return Some(Action::AutocompleteAccept),
            KeyCode::Esc => return Some(Action::AutocompleteDismiss),
            KeyCode::Enter => return Some(Action::Submit), // Submit the command
            // All other keys fall through to normal handling (typing continues to filter)
            _ => {}
        }
    }

    // Global shortcuts (work in any focus)
    // Ctrl+key shortcuts. Terminals may report these in two ways:
    // 1. Char('c') with CONTROL modifier (some terminals/crossterm versions)
    // 2. Raw control code Char('\x03') without CONTROL modifier (most terminals)
    // Handle both forms.
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return match key.code {
            KeyCode::Char('c') => Some(Action::Quit),
            KeyCode::Char('e') => Some(Action::ToggleAllToolResults),
            _ => None,
        };
    }
    match key.code {
        KeyCode::Char('\x05') => return Some(Action::ToggleAllToolResults), // Ctrl+E
        _ => {}
    }

    if key.code == KeyCode::Esc {
        return if is_generating {
            Some(Action::Cancel)
        } else {
            None
        };
    }

    if key.code == KeyCode::Tab {
        return Some(Action::Tab);
    }

    // Focus-specific mappings
    match focus {
        Focus::Main => map_main_key(key),
        Focus::Sidebar => map_sidebar_key(key),
    }
}

fn map_main_key(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Char(c) => Some(Action::TypeChar(c)),
        KeyCode::Backspace => Some(Action::Backspace),
        KeyCode::Delete => Some(Action::Delete),
        KeyCode::Left => Some(Action::CursorLeft),
        KeyCode::Right => Some(Action::CursorRight),
        KeyCode::Home => Some(Action::CursorHome),
        KeyCode::End => Some(Action::CursorEnd),
        KeyCode::Enter => Some(Action::Submit),
        KeyCode::Up => Some(Action::ScrollUp),
        KeyCode::Down => Some(Action::ScrollDown),
        KeyCode::PageUp => Some(Action::ScrollPageUp),
        KeyCode::PageDown => Some(Action::ScrollPageDown),
        _ => None,
    }
}

fn map_modal_key(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Up => Some(Action::ModalUp),
        KeyCode::Down => Some(Action::ModalDown),
        KeyCode::Enter => Some(Action::ModalConfirm),
        KeyCode::Esc => Some(Action::ModalCancel),
        // Number keys for direct selection (1-indexed)
        KeyCode::Char('1'..='9') => {
            // TODO: direct index selection. For now just map to confirm; the
            // app handler can set focus_index + confirm.
            Some(Action::ModalConfirm)
        }
        _ => None, // Swallow all other keys while modal is active
    }
}

fn map_sidebar_key(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Up => Some(Action::SidebarUp),
        KeyCode::Down => Some(Action::SidebarDown),
        KeyCode::Enter => Some(Action::SidebarSelect),
        KeyCode::Char('n') => Some(Action::NewSession),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    #[test]
    fn ctrl_c_quits_from_any_focus() {
        assert_eq!(
            map_key_event(ctrl('c'), Focus::Main, false, false, false),
            Some(Action::Quit)
        );
        assert_eq!(
            map_key_event(ctrl('c'), Focus::Sidebar, false, false, false),
            Some(Action::Quit)
        );
    }

    #[test]
    fn tab_toggles_from_any_focus() {
        assert_eq!(
            map_key_event(key(KeyCode::Tab), Focus::Main, false, false, false),
            Some(Action::Tab)
        );
        assert_eq!(
            map_key_event(key(KeyCode::Tab), Focus::Sidebar, false, false, false),
            Some(Action::Tab)
        );
    }

    #[test]
    fn esc_cancels_when_generating() {
        assert_eq!(
            map_key_event(key(KeyCode::Esc), Focus::Main, true, false, false),
            Some(Action::Cancel)
        );
        assert_eq!(map_key_event(key(KeyCode::Esc), Focus::Main, false, false, false), None);
    }

    #[test]
    fn main_focus_typing() {
        assert_eq!(
            map_key_event(key(KeyCode::Char('a')), Focus::Main, false, false, false),
            Some(Action::TypeChar('a'))
        );
        assert_eq!(
            map_key_event(key(KeyCode::Backspace), Focus::Main, false, false, false),
            Some(Action::Backspace)
        );
        assert_eq!(
            map_key_event(key(KeyCode::Enter), Focus::Main, false, false, false),
            Some(Action::Submit)
        );
    }

    #[test]
    fn main_focus_scrolling() {
        assert_eq!(
            map_key_event(key(KeyCode::Up), Focus::Main, false, false, false),
            Some(Action::ScrollUp)
        );
        assert_eq!(
            map_key_event(key(KeyCode::Down), Focus::Main, false, false, false),
            Some(Action::ScrollDown)
        );
        assert_eq!(
            map_key_event(key(KeyCode::PageUp), Focus::Main, false, false, false),
            Some(Action::ScrollPageUp)
        );
    }

    #[test]
    fn ctrl_e_toggles_tool_results() {
        // With CONTROL modifier
        assert_eq!(
            map_key_event(ctrl('e'), Focus::Main, false, false, false),
            Some(Action::ToggleAllToolResults)
        );
        // Raw control code (how most terminals send it)
        assert_eq!(
            map_key_event(key(KeyCode::Char('\x05')), Focus::Main, false, false, false),
            Some(Action::ToggleAllToolResults)
        );
    }

    #[test]
    fn sidebar_focus_navigation() {
        assert_eq!(
            map_key_event(key(KeyCode::Up), Focus::Sidebar, false, false, false),
            Some(Action::SidebarUp)
        );
        assert_eq!(
            map_key_event(key(KeyCode::Down), Focus::Sidebar, false, false, false),
            Some(Action::SidebarDown)
        );
        assert_eq!(
            map_key_event(key(KeyCode::Enter), Focus::Sidebar, false, false, false),
            Some(Action::SidebarSelect)
        );
        assert_eq!(
            map_key_event(key(KeyCode::Char('n')), Focus::Sidebar, false, false, false),
            Some(Action::NewSession)
        );
    }
}
