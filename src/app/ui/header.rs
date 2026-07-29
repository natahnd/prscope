//! Rendering for the pull request screen header and refresh status.

use std::time::Instant;

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Padding, Paragraph};
use ratatui::Frame;

use crate::app::state::App;

/// Renders provider context, project, PR count, and refresh state.
///
/// # Arguments
///
/// * `f` - Frame to draw into.
/// * `area` - Region to render the header (and, if a load error is active,
///   the extra error line) into.
/// * `app` - Application state used for the display context, project name,
///   PR count, loading/refresh state, and any load error.
pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let has_error = app.prs_error.is_some();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if has_error {
            vec![Constraint::Length(3), Constraint::Length(2)]
        } else {
            vec![Constraint::Length(3)]
        })
        .split(area);

    let count = app.displayed_prs().len();
    let project = app.selected_project.as_deref().unwrap_or("");
    let mut spans = vec![
        Span::styled(
            "PR Reviews",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(
            " — {} / {} — {} PR{}",
            app.display_context,
            project,
            count,
            if count == 1 { "" } else { "s" }
        )),
    ];
    if app.prs_loading {
        spans.push(Span::styled(
            " (refreshing…)",
            Style::default().add_modifier(Modifier::DIM),
        ));
    } else if let Some(refreshed_at) = app.refreshed_at {
        spans.push(Span::styled(
            format!(" — refreshed {} ago", format_age(refreshed_at)),
            Style::default().add_modifier(Modifier::DIM),
        ));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .padding(Padding::horizontal(1));
    f.render_widget(Paragraph::new(Line::from(spans)).block(block), rows[0]);

    if has_error {
        if let Some(err) = &app.prs_error {
            let error_rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Length(1)])
                .split(rows[1]);
            let para = Paragraph::new(Line::from(Span::styled(
                err.clone(),
                Style::default().fg(Color::Red),
            )));
            f.render_widget(para, error_rows[1]);
        }
    }
}

fn format_age(instant: Instant) -> String {
    let secs = instant.elapsed().as_secs();
    if secs < 60 {
        return format!("{secs}s");
    }
    let mins = secs / 60;
    if mins < 60 {
        return format!("{mins}m");
    }
    format!("{}h", mins / 60)
}
