//! Minimal keyboard handling - vim-like navigation

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub enum Action {
    Quit,
    MoveUp,
    MoveDown,
    PageUp,
    PageDown,
    SelectItem,
    ToggleView,
    Refresh,
    None,
}

/// Process keyboard input
#[inline]
pub fn handle_key(key: KeyEvent) -> Action {
    match key.code {
        // Quit
        KeyCode::Char('q') => Action::Quit,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,

        // Navigation
        KeyCode::Up | KeyCode::Char('k') => Action::MoveUp,
        KeyCode::Down | KeyCode::Char('j') => Action::MoveDown,
        KeyCode::PageUp | KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Action::PageUp
        }
        KeyCode::PageDown | KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Action::PageDown
        }

        // Selection
        KeyCode::Enter => Action::SelectItem,
        KeyCode::Tab => Action::ToggleView,

        // Refresh
        KeyCode::Char('r') => Action::Refresh,

        _ => Action::None,
    }
}