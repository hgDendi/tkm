use ratatui::layout::{Alignment, Constraint, Layout};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::tui::app::{App, ConfirmAction};
use crate::tui::theme::Theme;

pub fn render(frame: &mut Frame, app: &App, action: &ConfirmAction) {
    let area = frame.area();

    let [_, center_v, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(7),
        Constraint::Fill(1),
    ])
    .areas(area);

    let [_, center_h, _] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(50),
        Constraint::Fill(1),
    ])
    .areas(center_v);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Confirm ")
        .title_alignment(Alignment::Center)
        .style(Theme::warning());

    let inner = block.inner(center_h);
    frame.render_widget(block, center_h);

    let message = match action {
        ConfirmAction::Delete(idx) => {
            if let Some(entry) = app.entries.get(*idx) {
                format!(
                    "Delete {}:{} from {}?",
                    entry.service, entry.key, entry.backend
                )
            } else {
                "Delete this entry?".to_string()
            }
        }
    };

    let lines = vec![
        Line::raw(""),
        Line::raw(format!("  {message}")),
        Line::raw(""),
        Line::styled("  [y] Yes  [n] No", Theme::key_hint()),
    ];

    let content = Paragraph::new(lines);
    frame.render_widget(content, inner);
}
