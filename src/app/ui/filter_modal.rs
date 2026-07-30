use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Padding, Paragraph};
use ratatui::Frame;

use crate::app::filter_modal::{
    FilterModalState, FilterSection, APPROVED_BY_OPTIONS, AUTHOR_OPTIONS, COMMENT_OPTIONS,
    DRAFT_OPTIONS, REVIEW_STATUS_OPTIONS, STATUS_OPTIONS,
};
use crate::app::state::App;

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let Some(state) = app.filter_modal.as_ref() else {
        return;
    };
    let repo_count = app.repositories.len();
    let content_rows = 1
        + [
            4,
            1 + repo_count,
            3,
            2,
            REVIEW_STATUS_OPTIONS.len(),
            APPROVED_BY_OPTIONS.len(),
            AUTHOR_OPTIONS.len(),
            2,
        ]
        .into_iter()
        .max()
        .unwrap_or(4);
    let total_height = (2 + 1 + 1 + content_rows) as u16;
    let height = total_height.min(area.height);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(height), Constraint::Min(0)])
        .split(area);
    let modal_area = rows[0];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Yellow))
        .padding(Padding::horizontal(1));
    let inner = block.inner(modal_area);
    f.render_widget(block, modal_area);

    let inner_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Min(0)])
        .split(inner);

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "Filters",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ))),
        inner_rows[0],
    );

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(12),
            Constraint::Length(1),
            Constraint::Length(18),
            Constraint::Length(1),
            Constraint::Length(11),
            Constraint::Length(1),
            Constraint::Length(20),
            Constraint::Length(1),
            Constraint::Length(16),
            Constraint::Length(1),
            Constraint::Length(12),
            Constraint::Length(1),
            Constraint::Length(10),
            Constraint::Length(1),
            Constraint::Min(8),
        ])
        .split(inner_rows[2]);

    render_column(
        f,
        cols[0],
        "Status",
        state.section == FilterSection::Status,
        STATUS_OPTIONS.iter().map(|s| s.to_string()).collect(),
        state.status_index,
    );

    let repo_options: Vec<String> = std::iter::once("All repos".to_string())
        .chain(app.repositories.iter().map(|r| r.name.clone()))
        .collect();
    render_column(
        f,
        cols[2],
        "Repository",
        state.section == FilterSection::Repo,
        repo_options,
        state.repo_index,
    );

    render_column(
        f,
        cols[4],
        "Drafts",
        state.section == FilterSection::Draft,
        DRAFT_OPTIONS.iter().map(|(_, l)| l.to_string()).collect(),
        state.draft_index,
    );

    render_column(
        f,
        cols[6],
        "Comments",
        state.section == FilterSection::Comments,
        COMMENT_OPTIONS.iter().map(|(_, l)| l.to_string()).collect(),
        state.comment_index,
    );

    render_column(
        f,
        cols[8],
        "Review",
        state.section == FilterSection::Review,
        REVIEW_STATUS_OPTIONS
            .iter()
            .map(|v| v.label().to_string())
            .collect(),
        state.review_status_index,
    );

    render_column(
        f,
        cols[10],
        "Approved by",
        state.section == FilterSection::ApprovedBy,
        APPROVED_BY_OPTIONS
            .iter()
            .map(|v| v.label().to_string())
            .collect(),
        state.approved_by_index,
    );

    render_column(
        f,
        cols[12],
        "Author",
        state.section == FilterSection::Author,
        AUTHOR_OPTIONS
            .iter()
            .map(|v| v.label().to_string())
            .collect(),
        state.author_filter_index,
    );

    render_people_column(f, cols[14], state);
}

fn render_column(
    f: &mut Frame,
    area: Rect,
    header: &str,
    active: bool,
    options: Vec<String>,
    selected: usize,
) {
    let header_style = if active {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let mut lines = vec![Line::from(Span::styled(header.to_string(), header_style))];
    for (i, opt) in options.iter().enumerate() {
        let is_selected = i == selected;
        let prefix = if is_selected { "▶ " } else { "  " };
        let style = if is_selected {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        };
        lines.push(Line::from(Span::styled(format!("{prefix}{opt}"), style)));
    }
    f.render_widget(Paragraph::new(lines), area);
}

fn render_people_column(f: &mut Frame, area: Rect, state: &FilterModalState) {
    let active = state.section == FilterSection::Toggles;
    let header_style = if active {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let is_selected = active;
    let prefix = if is_selected { "▶ " } else { "  " };
    let style = if is_selected {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };
    let checkbox = if state.reviewer_me { "[x]" } else { "[ ]" };
    let lines = vec![
        Line::from(Span::styled("People", header_style)),
        Line::from(Span::styled(
            format!("{prefix}{checkbox} Reviewer: me"),
            style,
        )),
    ];
    f.render_widget(Paragraph::new(lines), area);
}
