//! Rendering for provider selection and provider-specific setup.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Padding, Paragraph};
use ratatui::Frame;

use crate::app::state::App;
use crate::providers::registry;

const SPINNER_FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

fn title_block(title: &str) -> Paragraph<'static> {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .padding(Padding::horizontal(1));
    Paragraph::new(Line::from(Span::styled(
        title.to_string(),
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    )))
    .block(block)
}

/// Renders enabled and planned providers during first-run setup.
///
/// # Arguments
///
/// * `f` - Frame to draw into.
/// * `area` - Region to render the title and provider list into.
/// * `app` - Application state used for the current provider selection
///   index.
pub fn render_picker(f: &mut Frame, area: Rect, app: &App) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);
    f.render_widget(title_block("Add Provider"), rows[0]);

    let providers = registry::list_providers();
    let safe_index = if providers.is_empty() {
        0
    } else {
        app.provider_picker_index.min(providers.len() - 1)
    };

    let mut lines = vec![
        Line::from(""),
        Line::from("Select a source for pull requests:"),
        Line::from(""),
    ];
    for (i, provider) in providers.iter().enumerate() {
        let selected = i == safe_index;
        let prefix = if selected { "▶ " } else { "  " };
        let mut style = Style::default();
        if !provider.enabled {
            style = style.add_modifier(Modifier::DIM);
        } else if selected {
            style = style.fg(Color::Cyan).add_modifier(Modifier::BOLD);
        }
        lines.push(Line::from(Span::styled(
            format!("{prefix}{}", provider.name),
            style,
        )));
    }
    f.render_widget(Paragraph::new(lines), rows[1]);
}

/// Renders Azure DevOps organization detection and entry state.
///
/// # Arguments
///
/// * `f` - Frame to draw into.
/// * `area` - Region to render the title and setup form into.
/// * `app` - Application state used for detection/loading/error state and
///   the organization name typed so far.
pub fn render_setup(f: &mut Frame, area: Rect, app: &App) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);
    f.render_widget(title_block("Azure DevOps Setup"), rows[0]);

    let mut lines: Vec<Line> = vec![Line::from("")];
    if app.setup_detecting {
        let frame = SPINNER_FRAMES[app.spinner_frame % SPINNER_FRAMES.len()];
        lines.push(Line::from(vec![
            Span::styled(format!("{frame} "), Style::default().fg(Color::Cyan)),
            Span::raw("Detecting existing Azure DevOps configuration…"),
        ]));
    } else {
        lines.push(Line::from("Enter your Azure DevOps organization:"));
        lines.push(Line::from(Span::styled(
            "This is the name in dev.azure.com/your-org",
            Style::default().add_modifier(Modifier::DIM),
        )));
        if app.setup_detected_from_az && !app.setup_value.is_empty() {
            lines.push(Line::from(Span::styled(
                "(detected from az devops configure)",
                Style::default().add_modifier(Modifier::DIM),
            )));
        }
        lines.push(Line::from(""));
        let cursor = if app.setup_loading {
            " (connecting…)".to_string()
        } else {
            "█".to_string()
        };
        lines.push(Line::from(vec![
            Span::styled("> ", Style::default().fg(Color::Cyan)),
            Span::raw(app.setup_value.clone()),
            Span::styled(cursor, Style::default().add_modifier(Modifier::DIM)),
        ]));
    }

    if let Some(err) = &app.setup_error {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            err.clone(),
            Style::default().fg(Color::Red),
        )));
    }

    f.render_widget(Paragraph::new(lines), rows[1]);
}
