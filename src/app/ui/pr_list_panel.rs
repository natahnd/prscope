//! Rendering for grouped pull request rows, filters, and inline search.

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Padding, Paragraph};
use ratatui::Frame;

use crate::api::types::{NormalizedPullRequest, PrListLayout};
use crate::app::comment_activity::CommentActivity;
use crate::app::pr_key::pr_key_for;
use crate::app::pr_list_rows::{get_visible_pr_rows, VisualRow};
use crate::app::state::{App, PanelFocus};
use crate::humanize::{format_compact_age, parse_date};

/// Renders the filtered and grouped pull request viewport.
///
/// # Arguments
///
/// * `f` - Frame to draw into.
/// * `area` - Region to render the list panel (border, header lines, and
///   scrollable rows) into.
/// * `app` - Application state used for the filtered/sorted PR list,
///   selection, active filters, layout, and search state.
pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let focused = app.panel_focus == PanelFocus::List;
    let displayed = app.displayed_prs();
    let safe_index = app.safe_selected_index(displayed.len());

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(if focused { Color::Cyan } else { Color::Gray }))
        .padding(Padding::horizontal(1));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    let title_style = if focused {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().add_modifier(Modifier::BOLD)
    };
    let count_suffix = if displayed.is_empty() {
        String::new()
    } else {
        format!(" ({})", displayed.len())
    };
    lines.push(Line::from(Span::styled(
        format!("Pull Requests{count_suffix}"),
        title_style,
    )));

    if app.search_mode {
        lines.push(Line::from(vec![
            Span::styled("/ ", Style::default().fg(Color::Cyan)),
            Span::styled("search: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::DIM)),
            Span::styled(app.search_query.clone(), Style::default().fg(Color::Cyan)),
            Span::styled(
                " (Enter apply · Esc clear)",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::DIM),
            ),
        ]));
    }

    let filter_labels = app.filter_labels();
    let filters_text = if filter_labels.is_empty() {
        "none".to_string()
    } else {
        filter_labels
            .iter()
            .map(|f| format!("[{f}]"))
            .collect::<Vec<_>>()
            .join(" · ")
    };
    lines.push(Line::from(vec![
        Span::styled("Filters: ", Style::default().add_modifier(Modifier::DIM)),
        Span::styled(filters_text, Style::default().add_modifier(Modifier::DIM)),
        Span::styled(" · Group: ", Style::default().add_modifier(Modifier::DIM)),
        Span::styled(app.list_layout.label(), Style::default().add_modifier(Modifier::DIM)),
    ]));

    if displayed.is_empty() {
        lines.push(Line::from(Span::styled(
            "No pull requests match your filters.",
            Style::default().add_modifier(Modifier::DIM),
        )));
    } else {
        let visible = get_visible_pr_rows(
            &displayed,
            safe_index,
            inner.height as usize,
            app.search_mode,
            app.list_layout,
        );
        let inner_width = (inner.width as i64).max(1) as usize;
        for row in &visible.visible {
            lines.push(render_row(row, &displayed, safe_index, inner_width, app.list_layout, app));
        }
    }

    f.render_widget(Paragraph::new(lines), inner);
}

fn render_row<'a>(
    row: &VisualRow,
    displayed: &[&NormalizedPullRequest],
    selected_index: usize,
    width: usize,
    layout: PrListLayout,
    app: &App,
) -> Line<'a> {
    match row {
        VisualRow::Header { label, .. } => Line::from(Span::styled(
            truncate_chars(label, width),
            Style::default()
                .fg(Color::LightBlue)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )),
        VisualRow::Spacer { .. } => Line::from(""),
        VisualRow::Title { pr_index, .. } => {
            let Some(pr) = displayed.get(*pr_index) else {
                return Line::from("");
            };
            let selected = *pr_index == selected_index;
            let prefix = if selected { "▶ " } else { "  " };
            let tag = if pr.is_draft { " DRAFT" } else { "" };
            let inner_width = width.max(1);
            let title_width = inner_width
                .saturating_sub(prefix.chars().count())
                .saturating_sub(tag.chars().count())
                .max(1);
            let title = truncate_chars(&pr.title, title_width);
            let mut style = row_color(pr, selected);
            if selected {
                style = style.add_modifier(Modifier::BOLD);
            }
            let mut spans = vec![Span::styled(format!("{prefix}{title}"), style)];
            if !tag.is_empty() {
                spans.push(Span::styled(
                    tag,
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                ));
            }
            Line::from(spans)
        }
        VisualRow::Meta { pr_index, .. } => {
            let Some(pr) = displayed.get(*pr_index) else {
                return Line::from("");
            };
            let has_active_comments =
                app.comment_activity.get(&pr_key_for(pr)) == Some(&CommentActivity::Active);
            let age = format_compact_age(parse_date(&pr.created_at));
            let meta_text = match layout {
                PrListLayout::GroupByRepo => format!("    {} · {}", pr.author, age),
                PrListLayout::GroupByAuthor => format!("    {} · {}", pr.repo_name, age),
                PrListLayout::SortByAgeDesc | PrListLayout::SortByAgeAsc => {
                    format!("    {} · {} · {}", pr.repo_name, pr.author, age)
                }
            };
            let mut spans = vec![Span::styled(
                meta_text,
                Style::default().add_modifier(Modifier::DIM),
            )];
            if has_active_comments {
                spans.push(Span::styled(" · active comments", Style::default().fg(Color::Magenta)));
            }
            Line::from(spans)
        }
    }
}

fn row_color(pr: &NormalizedPullRequest, selected: bool) -> Style {
    if selected {
        return Style::default().fg(Color::Cyan);
    }
    if pr.merge_status == "conflicts" {
        return Style::default().fg(Color::Red);
    }
    if pr.needs_my_review {
        return Style::default().fg(Color::Yellow);
    }
    Style::default()
}

fn truncate_chars(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    let count = text.chars().count();
    if count <= max_width {
        return text.to_string();
    }
    if max_width == 1 {
        return "…".to_string();
    }
    let truncated: String = text.chars().take(max_width - 1).collect();
    format!("{truncated}…")
}
