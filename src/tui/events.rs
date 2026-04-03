use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

/// Poll for a key event with timeout
pub fn poll_key(timeout: Duration) -> Option<KeyEvent> {
    if event::poll(timeout).ok()? {
        if let Event::Key(key) = event::read().ok()? {
            if key.kind == KeyEventKind::Press {
                return Some(key);
            }
        }
    }
    None
}

/// Check if this is Ctrl+C
pub fn is_quit(key: &KeyEvent) -> bool {
    key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL)
}

/// Check if this is the escape key
pub fn is_escape(key: &KeyEvent) -> bool {
    key.code == KeyCode::Esc
}
