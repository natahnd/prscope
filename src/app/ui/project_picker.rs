use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Padding, Paragraph};
use ratatui::Frame;

use crate::app::state::App;

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let filtered = app.filtered_projects();
    let safe_index = if filtered.is_empty() {
        0
    } else {
        app.project_index.min(filtered.len() - 1)
    };

    let mut title_spans = vec![
        Span::styled(
            "Select Project",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(" — {}", app.display_context)),
    ];
    if app.projects_loading {
        title_spans.push(Span::styled(
            " (loading…)",
            Style::default().add_modifier(Modifier::DIM),
        ));
    }
    let title_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .padding(Padding::horizontal(1));

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);
    f.render_widget(Paragraph::new(Line::from(title_spans)).block(title_block), rows[0]);

    let body_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(rows[1]);

    let mut lines: Vec<Line> = Vec::new();
    if let Some(err) = &app.projects_error {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            err.clone(),
            Style::default().fg(Color::Red),
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("/ search: ", Style::default().add_modifier(Modifier::DIM)),
        Span::raw(if app.project_search.is_empty() {
            "type to filter projects".to_string()
        } else {
            app.project_search.clone()
        }),
    ]));
    lines.push(Line::from(""));
    if filtered.is_empty() {
        lines.push(Line::from(Span::styled(
            "No projects match.",
            Style::default().add_modifier(Modifier::DIM),
        )));
    } else {
        for (i, project) in filtered.iter().enumerate() {
            let selected = i == safe_index;
            let is_last = app.last_project.as_deref() == Some(project.name.as_str());
            let prefix = if selected { "▶ " } else { "  " };
            let suffix = if is_last { " (last used)" } else { "" };
            let mut style = Style::default();
            if selected {
                style = style.fg(Color::Cyan).add_modifier(Modifier::BOLD);
            }
            lines.push(Line::from(Span::styled(
                format!("{prefix}{}{suffix}", project.name),
                style,
            )));
        }
    }
    f.render_widget(Paragraph::new(lines), body_rows[0]);

    let hint_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(Color::Gray))
        .padding(Padding::horizontal(1));
    let hint_para = Paragraph::new(Line::from(Span::styled(
        "↑↓ navigate · Enter select · / search · r refresh · q quit",
        Style::default().add_modifier(Modifier::DIM),
    )))
    .block(hint_block);
    f.render_widget(hint_para, body_rows[1]);
}
