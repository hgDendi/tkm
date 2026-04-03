use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::core::token::BackendType;
use crate::tui::app::App;
use crate::tui::theme::Theme;

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let [main_area, status_area] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Add Token ")
        .style(Theme::normal());

    let inner = block.inner(main_area);
    frame.render_widget(block, main_area);

    let fields = [
        ("Service", &app.add_service, false),
        ("Key", &app.add_key, false),
        ("Value", &app.add_value, true), // masked
        ("Backend", &app.add_backend.to_string(), false),
        ("Label", &app.add_label, false),
    ];

    let mut lines = Vec::new();
    lines.push(Line::raw(""));

    for (i, (label, value, masked)) in fields.iter().enumerate() {
        let is_focused = i == app.add_field_idx;
        let indicator = if is_focused { "▶ " } else { "  " };

        let label_style = if is_focused {
            Theme::title()
        } else {
            Theme::header()
        };

        let display_value = if *masked && !value.is_empty() {
            "●".repeat(value.len())
        } else if i == 3 {
            // Backend field — show as toggle
            let backend_display = match app.add_backend {
                BackendType::Keychain => "[keychain] / file",
                BackendType::EncryptedFile => "keychain / [file]",
            };
            backend_display.to_string()
        } else {
            value.to_string()
        };

        let cursor = if is_focused && i != 3 { "▎" } else { "" };

        lines.push(Line::from(vec![
            Span::raw(indicator),
            Span::styled(format!("{:<10}", label), label_style),
            if *masked {
                Span::styled(display_value, Theme::password_mask())
            } else {
                Span::raw(display_value)
            },
            Span::styled(cursor, Style::new().add_modifier(Modifier::SLOW_BLINK)),
        ]));
        lines.push(Line::raw(""));
    }

    let content = Paragraph::new(lines);
    frame.render_widget(content, inner);

    // Status bar
    let mut hints = vec![
        Span::styled("[Tab]", Theme::key_hint()),
        Span::raw(" next field  "),
        Span::styled("[Ctrl+Enter]", Theme::key_hint()),
        Span::raw(" save  "),
        Span::styled("[Esc]", Theme::key_hint()),
        Span::raw(" cancel"),
    ];

    if let Some(ref msg) = app.status_message {
        hints = vec![Span::styled(format!(" {msg} "), Theme::warning())];
    }

    let bar = Paragraph::new(Line::from(hints)).style(Theme::dim());
    frame.render_widget(bar, status_area);
}
