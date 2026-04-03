use ratatui::layout::{Constraint, Layout};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::tui::app::App;
use crate::tui::theme::Theme;

pub fn render(frame: &mut Frame, app: &App, idx: usize) {
    let area = frame.area();

    let entry = match app.entries.get(idx) {
        Some(e) => e,
        None => return,
    };

    let [main_area, status_area] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", entry.service))
        .style(Theme::normal());

    let inner = block.inner(main_area);
    frame.render_widget(block, main_area);

    let field_lines = build_detail_lines(app, idx);

    let detail = Paragraph::new(field_lines);
    frame.render_widget(detail, inner);

    // Status bar
    let mut spans = vec![
        Span::styled("[v]", Theme::key_hint()),
        Span::raw(if app.reveal_secret { " hide" } else { " reveal" }),
        Span::raw("  "),
        Span::styled("[c]", Theme::key_hint()),
        Span::raw(" copy"),
        Span::raw("  "),
        Span::styled("[Esc]", Theme::key_hint()),
        Span::raw(" back"),
    ];

    if let Some(ref clip) = app.clip_status {
        let remaining = clip.expires.duration_since(std::time::Instant::now());
        spans = vec![Span::styled(
            format!(" {} ({}s) ", clip.message, remaining.as_secs()),
            Theme::success(),
        )];
    }

    let bar = Paragraph::new(Line::from(spans)).style(Theme::dim());
    frame.render_widget(bar, status_area);
}

fn build_detail_lines<'a>(app: &'a App, idx: usize) -> Vec<Line<'a>> {
    let entry = &app.entries[idx];
    let mut lines = Vec::new();

    let add_field = |lines: &mut Vec<Line<'a>>, label: &'a str, value: String| {
        lines.push(Line::from(vec![
            Span::styled(format!("  {:<12}", label), Theme::header()),
            Span::raw(value),
        ]));
    };

    lines.push(Line::raw(""));

    add_field(&mut lines, "Service:", entry.service.clone());
    add_field(&mut lines, "Key:", entry.key.clone());
    add_field(&mut lines, "Backend:", entry.backend.to_string());

    if let Some(ref username) = entry.username {
        add_field(&mut lines, "Username:", username.clone());
    }
    if let Some(ref url) = entry.url {
        add_field(&mut lines, "URL:", url.clone());
    }
    if !entry.tags.is_empty() {
        add_field(&mut lines, "Tags:", entry.tags.join(", "));
    }

    add_field(
        &mut lines,
        "Created:",
        entry.created_at.format("%Y-%m-%d %H:%M").to_string(),
    );
    add_field(
        &mut lines,
        "Updated:",
        entry.updated_at.format("%Y-%m-%d %H:%M").to_string(),
    );

    let expires = entry
        .expires_at
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "never".into());
    add_field(&mut lines, "Expires:", expires);

    // Secret value
    lines.push(Line::raw(""));
    if app.reveal_secret {
        let secret_val = app
            .secret_cache
            .as_deref()
            .unwrap_or("<failed to load>");
        lines.push(Line::from(vec![
            Span::styled("  Value:      ", Theme::header()),
            Span::raw(secret_val.to_string()),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled("  Value:      ", Theme::header()),
            Span::styled("●●●●●●●●●●●●", Theme::password_mask()),
            Span::styled("  [v] reveal", Theme::dim()),
        ]));
    }

    if let Some(ref notes) = entry.notes {
        lines.push(Line::raw(""));
        add_field(&mut lines, "Notes:", notes.clone());
    }

    lines
}
