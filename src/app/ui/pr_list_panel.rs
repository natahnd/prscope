use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Padding, Paragraph};
use ratatui::Frame;

use crate::api::types::{NormalizedPullRequest, PrListLayout};
use crate::app::comment_activity::CommentActivity;
use crate::app::pr_key::pr_key_for;
use crate::app::pr_list_rows::{get_visible_pr_rows, VisualRow};
use crate::app::review_status::reviewer_count_label;
use crate::app::state::{App, PanelFocus};
use crate::app::text_width::{clip_line_to_width, display_width, truncate_to_width};
use crate::humanize::{format_compact_age, parse_date};

const META_INDENT: &str = "    ";
const META_SEP: &str = " · ";
const ACTIVE_COMMENTS_SUFFIX: &str = " · active comments";

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

    let inner_width = inner.width.max(1) as usize;
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
    lines.push(clip_line_to_width(
        Line::from(Span::styled(
            format!("Pull Requests{count_suffix}"),
            title_style,
        )),
        inner_width,
    ));

    if app.search_mode {
        lines.push(clip_line_to_width(
            Line::from(vec![
                Span::styled("/ ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    "search: ",
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::DIM),
                ),
                Span::styled(app.search_query.clone(), Style::default().fg(Color::Cyan)),
                Span::styled(
                    " (Enter apply · Esc clear)",
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::DIM),
                ),
            ]),
            inner_width,
        ));
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
    lines.push(clip_line_to_width(
        Line::from(vec![
            Span::styled("Filters: ", Style::default().add_modifier(Modifier::DIM)),
            Span::styled(filters_text, Style::default().add_modifier(Modifier::DIM)),
            Span::styled(" · Group: ", Style::default().add_modifier(Modifier::DIM)),
            Span::styled(
                app.list_layout.label(),
                Style::default().add_modifier(Modifier::DIM),
            ),
        ]),
        inner_width,
    ));

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
        for row in &visible.visible {
            lines.push(render_row(
                row,
                &displayed,
                safe_index,
                inner_width,
                app.list_layout,
                app,
            ));
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
            truncate_to_width(label, width),
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
            let title_width = width
                .saturating_sub(display_width(prefix))
                .saturating_sub(display_width(tag))
                .max(1);
            let title = truncate_to_width(&pr.title, title_width);
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
            clip_line_to_width(Line::from(spans), width)
        }
        VisualRow::Meta { pr_index, .. } => {
            let Some(pr) = displayed.get(*pr_index) else {
                return Line::from("");
            };
            build_meta_line(pr, layout, width, app)
        }
    }
}

fn build_meta_line(
    pr: &NormalizedPullRequest,
    layout: PrListLayout,
    width: usize,
    app: &App,
) -> Line<'static> {
    let has_active_comments =
        app.comment_activity.get(&pr_key_for(pr)) == Some(&CommentActivity::Active);
    let reviewer_label = reviewer_count_label(pr).unwrap_or_default();
    let reviewer_width = display_width(&reviewer_label);
    let comments_width = if has_active_comments {
        display_width(ACTIVE_COMMENTS_SUFFIX)
    } else {
        0
    };

    let age = format_compact_age(parse_date(&pr.created_at));
    let indent_width = display_width(META_INDENT);
    let sep_width = display_width(META_SEP);
    let age_width = display_width(&age);
    let base_budget = width.saturating_sub(comments_width + reviewer_width);

    let meta_text = match layout {
        PrListLayout::GroupByRepo => {
            let author_max = base_budget.saturating_sub(indent_width + sep_width + age_width);
            let author = truncate_to_width(&pr.author, author_max.max(1));
            format!("{META_INDENT}{author}{META_SEP}{age}")
        }
        PrListLayout::GroupByAuthor => {
            let repo_max = base_budget.saturating_sub(indent_width + sep_width + age_width);
            let repo = truncate_to_width(&pr.repo_name, repo_max.max(1));
            format!("{META_INDENT}{repo}{META_SEP}{age}")
        }
        PrListLayout::SortByAgeDesc | PrListLayout::SortByAgeAsc => {
            let separators = indent_width + sep_width + age_width + sep_width;
            let fields_budget = base_budget.saturating_sub(separators);
            let repo_max = fields_budget / 2;
            let author_max = fields_budget.saturating_sub(repo_max);
            let repo = truncate_to_width(&pr.repo_name, repo_max.max(1));
            let author = truncate_to_width(&pr.author, author_max.max(1));
            format!("{META_INDENT}{repo}{META_SEP}{author}{META_SEP}{age}")
        }
    };

    let mut spans = vec![Span::styled(
        meta_text,
        Style::default().add_modifier(Modifier::DIM),
    )];
    if !reviewer_label.is_empty() {
        spans.push(Span::styled(
            reviewer_label,
            Style::default().add_modifier(Modifier::DIM),
        ));
    }
    if has_active_comments {
        spans.push(Span::styled(
            ACTIVE_COMMENTS_SUFFIX,
            Style::default().fg(Color::Magenta),
        ));
    }
    clip_line_to_width(Line::from(spans), width)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::types::{AdoReviewer, PullRequestFilters};
    use crate::app::state::App;
    use chrono::Utc;

    fn sample_pr() -> NormalizedPullRequest {
        NormalizedPullRequest {
            id: 1,
            title: "Title".into(),
            status: "active".into(),
            is_draft: false,
            repo_name: "[Digital Product Platform]/Digital Product Platform".into(),
            repository_id: "r1".into(),
            author: "Nathan Davey".into(),
            is_author_current_user: true,
            reviewers: vec![AdoReviewer {
                display_name: "Nathan Davey".into(),
                vote: 0,
                is_required: false,
                is_current_user: true,
                is_team: false,
            }],
            target_branch: "main".into(),
            created_at: Utc::now().to_rfc3339(),
            url: "https://example.com".into(),
            merge_status: "none".into(),
            needs_my_review: true,
        }
    }

    #[test]
    fn meta_line_fits_narrow_width() {
        let pr = sample_pr();
        let app = App::new(None, PullRequestFilters::default());
        let line = build_meta_line(&pr, PrListLayout::SortByAgeDesc, 40, &app);
        assert!(line.width() <= 40);
    }

    #[test]
    fn meta_line_includes_reviewer_count() {
        let pr = sample_pr();
        let app = App::new(None, PullRequestFilters::default());
        let line = build_meta_line(&pr, PrListLayout::GroupByRepo, 80, &app);
        let text: String = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();
        assert!(text.contains("1 reviewer"));
    }
}
