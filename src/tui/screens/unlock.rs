use ratatui::layout::{Alignment, Constraint, Layout};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::tui::app::App;
use crate::tui::theme::Theme;

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Center the unlock box
    let [_, center_v, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(9),
        Constraint::Fill(1),
    ])
    .areas(area);

    let [_, center_h, _] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(50),
        Constraint::Fill(1),
    ])
    .areas(center_v);

    let box_area = center_h;

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" tkm ")
        .title_alignment(Alignment::Center)
        .style(Theme::normal());

    let inner = block.inner(box_area);
    frame.render_widget(block, box_area);

    let [_, title_area, _, pw_area, _, err_area, hint_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(inner);

    // Title
    let title = Paragraph::new("Enter master password:")
        .alignment(Alignment::Center)
        .style(Theme::title());
    frame.render_widget(title, title_area);

    // Password dots
    let dots = "●".repeat(app.password_input.len());
    let pw_display = Paragraph::new(dots)
        .alignment(Alignment::Center)
        .style(Theme::password_mask());
    frame.render_widget(pw_display, pw_area);

    // Error message
    if let Some(ref err) = app.unlock_error {
        let err_text = Paragraph::new(err.as_str())
            .alignment(Alignment::Center)
            .style(Theme::error());
        frame.render_widget(err_text, err_area);
    }

    // Hints
    let hints = Paragraph::new("[Enter] Unlock  [Esc] Quit")
        .alignment(Alignment::Center)
        .style(Theme::dim());
    frame.render_widget(hints, hint_area);
}
