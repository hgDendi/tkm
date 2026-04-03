use ratatui::layout::{Constraint, Layout};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};
use ratatui::Frame;

use crate::core::token::TokenMeta;
use crate::tui::app::{App, Screen};
use crate::tui::theme::Theme;

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    let [main_area, status_area] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(area);

    // If searching, show search bar at top
    let (list_area, search_area) = if app.screen == Screen::Search {
        let [search, list] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Fill(1),
        ])
        .areas(main_area);
        (list, Some(search))
    } else {
        (main_area, None)
    };

    // Render search bar if in search mode
    if let Some(search_area) = search_area {
        let search_text = format!("/{}", app.search_query);
        let search = Paragraph::new(search_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Search ")
                    .style(Theme::title()),
            );
        frame.render_widget(search, search_area);
    }

    // Table header
    let header = Row::new(vec![
        Cell::from("SERVICE"),
        Cell::from("KEY"),
        Cell::from("BACKEND"),
        Cell::from("EXPIRES"),
        Cell::from("LABEL"),
    ])
    .style(Theme::header())
    .bottom_margin(0);

    // Build rows - collect data first to avoid borrow conflicts
    let visible: Vec<(usize, TokenMeta)> = app.visible_entries()
        .into_iter()
        .map(|(i, e)| (i, e.clone()))
        .collect();

    let visible_count = visible.len();

    let rows: Vec<Row> = visible
        .iter()
        .map(|(_, entry)| {
            let expires = entry
                .expires_at
                .map(|d| {
                    if entry.is_expired() {
                        format!("{} !", d.format("%Y-%m-%d"))
                    } else {
                        d.format("%Y-%m-%d").to_string()
                    }
                })
                .unwrap_or_else(|| "—".into());

            let expires_style = if entry.is_expired() {
                Theme::error()
            } else {
                Theme::normal()
            };

            Row::new(vec![
                Cell::from(entry.service.clone()),
                Cell::from(entry.key.clone()),
                Cell::from(entry.backend.to_string()),
                Cell::from(expires).style(expires_style),
                Cell::from(entry.label.clone().unwrap_or_default()),
            ])
        })
        .collect();

    let widths = [
        Constraint::Percentage(25),
        Constraint::Percentage(15),
        Constraint::Percentage(15),
        Constraint::Percentage(15),
        Constraint::Percentage(30),
    ];

    let title = if app.screen == Screen::Search {
        format!(" tkm — {} results ", visible_count)
    } else {
        format!(" tkm — {} tokens ", app.entries.len())
    };

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .style(Theme::normal()),
        )
        .row_highlight_style(Theme::selected())
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(table, list_area, &mut app.table_state);

    // Status bar
    render_status_bar(frame, app, status_area);
}

fn render_status_bar(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let mut spans = Vec::new();

    // Clip status takes priority
    if let Some(ref clip) = app.clip_status {
        let remaining = clip.expires.duration_since(std::time::Instant::now());
        spans.push(Span::styled(
            format!(" {} ({}s) ", clip.message, remaining.as_secs()),
            Theme::success(),
        ));
    } else if let Some(ref msg) = app.status_message {
        spans.push(Span::styled(format!(" {msg} "), Theme::warning()));
    } else {
        // Key hints
        let hints = match app.screen {
            Screen::Search => {
                vec![
                    ("Esc", "cancel"),
                    ("Enter", "confirm"),
                    ("↑↓", "navigate"),
                ]
            }
            _ => {
                vec![
                    ("a", "add"),
                    ("d", "delete"),
                    ("c", "copy"),
                    ("Enter", "detail"),
                    ("/", "search"),
                    ("q", "quit"),
                ]
            }
        };
        for (i, (key, desc)) in hints.iter().enumerate() {
            if i > 0 {
                spans.push(Span::raw(" "));
            }
            spans.push(Span::styled(format!("[{key}]"), Theme::key_hint()));
            spans.push(Span::raw(format!(" {desc}")));
        }
    }

    let bar = Paragraph::new(Line::from(spans)).style(Theme::dim());
    frame.render_widget(bar, area);
}
