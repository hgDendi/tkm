use ratatui::style::{Color, Modifier, Style};

pub struct Theme;

impl Theme {
    pub fn title() -> Style {
        Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    }

    pub fn header() -> Style {
        Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    }

    pub fn selected() -> Style {
        Style::new().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
    }

    pub fn normal() -> Style {
        Style::new().fg(Color::White)
    }

    pub fn dim() -> Style {
        Style::new().fg(Color::DarkGray)
    }

    pub fn success() -> Style {
        Style::new().fg(Color::Green)
    }

    pub fn warning() -> Style {
        Style::new().fg(Color::Yellow)
    }

    pub fn error() -> Style {
        Style::new().fg(Color::Red)
    }

    pub fn key_hint() -> Style {
        Style::new().fg(Color::Cyan)
    }

    pub fn password_mask() -> Style {
        Style::new().fg(Color::Magenta)
    }
}
