use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Padding, Paragraph};
use ratatui::Frame;

use crate::app::right_panel_lines::{
    count_comment_sections, get_visible_right_panel_lines, line_style_for_kind,
    right_panel_content_width, ContentLine, LineKind, CHROME_ROWS,
};
use crate::app::state::{App, CommentStatusFilter, PanelFocus, RightPanelMode};
use crate::app::text_selection::{normalize_selection, row_highlight_range};
use crate::app::update::current_right_panel_lines;

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let focused = app.panel_focus == PanelFocus::Detail;
    let displayed = app.displayed_prs();
    let safe_index = app.safe_selected_index(displayed.len());
    let Some(pr) = displayed.get(safe_index).copied() else {
        render_placeholder(f, area);
        return;
    };

    let border_color = if focused { Color::Cyan } else { Color::Gray };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(border_color))
        .padding(Padding::horizontal(1));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let content_width = right_panel_content_width(area.width);
    let title_text = truncate_chars(&format!("#{} {}", pr.id, pr.title), content_width);
    let meta_text = truncate_chars(
        &format!(
            "{} · draft: {} · conflicts: {} · status: {}",
            pr.target_branch,
            if pr.is_draft { "yes" } else { "no" },
            if pr.merge_status == "conflicts" { "yes" } else { "none" },
            pr.status
        ),
        content_width,
    );

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(inner);

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            title_text,
            Style::default().add_modifier(Modifier::BOLD),
        ))),
        rows[0],
    );
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            meta_text,
            Style::default().add_modifier(Modifier::DIM),
        ))),
        rows[1],
    );

    let lines = current_right_panel_lines(app);
    let content_height = (area.height as i64 - CHROME_ROWS as i64).max(1) as usize;
    let current_scroll = if app.right_mode == RightPanelMode::Description {
        app.description_scroll
    } else {
        app.comments_scroll
    };
    let visible = get_visible_right_panel_lines(&lines, current_scroll, content_height);

    let sections_count = count_comment_sections(&app.detail_state.threads, app.comment_status_filter);
    let tab_line = build_tab_line(
        app,
        sections_count,
        content_width,
        visible.max_scroll,
        visible.clamped_scroll,
        lines.len(),
        content_height,
    );
    f.render_widget(Paragraph::new(tab_line), rows[3]);

    let selection = app.right_selection.map(normalize_selection);
    let mut content_lines: Vec<Line> = Vec::new();
    for (i, line) in visible.visible.iter().enumerate() {
        let absolute_row = visible.clamped_scroll + i;
        let highlight = row_highlight_range(absolute_row, selection.as_ref());
        content_lines.push(render_content_line(line, highlight, content_width));
    }
    f.render_widget(Paragraph::new(content_lines), rows[4]);
}

fn render_placeholder(f: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(Color::Gray))
        .padding(Padding::horizontal(1));
    let inner = block.inner(area);
    f.render_widget(block, area);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "Select a pull request to see details.",
            Style::default().add_modifier(Modifier::DIM),
        ))),
        inner,
    );
}

#[allow(clippy::too_many_arguments)]
fn build_tab_line(
    app: &App,
    sections_count: usize,
    content_width: usize,
    max_scroll: usize,
    clamped_scroll: usize,
    total_lines: usize,
    content_height: usize,
) -> Line<'static> {
    let desc_active = app.right_mode == RightPanelMode::Description;
    let comments_active = app.right_mode == RightPanelMode::Comments;

    let desc_style = if desc_active {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    } else {
        Style::default().add_modifier(Modifier::DIM)
    };
    let comments_style = if comments_active {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    } else {
        Style::default().add_modifier(Modifier::DIM)
    };

    let comments_label = if sections_count > 0 {
        format!("Comments ({sections_count})")
    } else {
        "Comments".to_string()
    };

    let filter_suffix = if comments_active && app.comment_status_filter != CommentStatusFilter::All {
        format!(
            " · {}",
            if app.comment_status_filter == CommentStatusFilter::Active {
                "active only"
            } else {
                "resolved only"
            }
        )
    } else {
        String::new()
    };

    let mut spans = vec![
        Span::styled("Description", desc_style),
        Span::raw(" "),
        Span::styled(comments_label, comments_style),
    ];
    if !filter_suffix.is_empty() {
        spans.push(Span::styled(
            filter_suffix,
            Style::default().add_modifier(Modifier::DIM),
        ));
    }

    if max_scroll > 0 {
        let left_len: usize = spans.iter().map(|s| s.content.chars().count()).sum();
        let right_text = format!(
            "{}-{}/{}",
            clamped_scroll + 1,
            (clamped_scroll + content_height).min(total_lines),
            total_lines
        );
        let right_len = right_text.chars().count();
        let padding = content_width.saturating_sub(left_len + right_len).max(1);
        spans.push(Span::raw(" ".repeat(padding)));
        spans.push(Span::styled(
            right_text,
            Style::default().add_modifier(Modifier::DIM),
        ));
    }

    Line::from(spans)
}

fn display_spans(line: &ContentLine) -> Vec<(String, Style)> {
    if line.kind == LineKind::Markdown {
        return line.spans.clone();
    }
    let style = line_style_for_kind(line.kind, line.accent);
    let mut text = line.plain_text();
    if text.is_empty() {
        text = " ".to_string();
    }
    vec![(text, style)]
}

fn render_content_line(line: &ContentLine, highlight: Option<(usize, usize)>, content_width: usize) -> Line<'static> {
    if line.kind == LineKind::Empty {
        return Line::from(Span::styled(
            line.plain_text(),
            Style::default().add_modifier(Modifier::DIM),
        ));
    }
    let spans = display_spans(line);
    Line::from(spans_with_highlight(&spans, highlight, content_width))
}

/// Splits `spans` around the highlighted column range and overrides that
/// segment's style with the selection colors, mirroring `highlightedText` in
/// the original TS app. Full-row highlights get padded to `content_width` so
/// short lines in the middle of a multi-line selection still read as a
/// continuous highlighted block.
fn spans_with_highlight(
    spans: &[(String, Style)],
    highlight: Option<(usize, usize)>,
    content_width: usize,
) -> Vec<Span<'static>> {
    let Some((start, end)) = highlight else {
        return spans
            .iter()
            .map(|(t, s)| Span::styled(t.clone(), *s))
            .collect();
    };

    let mut chars: Vec<(char, Style)> = Vec::new();
    for (text, style) in spans {
        for c in text.chars() {
            chars.push((c, *style));
        }
    }

    let is_full_row = end >= content_width.saturating_sub(1) && start == 0;
    if is_full_row && chars.len() < content_width {
        chars.resize(content_width, (' ', Style::default()));
    }

    if chars.is_empty() {
        return vec![Span::raw(" ")];
    }

    let start_clamped = start.min(chars.len());
    let end_clamped = end.min(chars.len() - 1);
    if end_clamped < start_clamped {
        return collapse_runs(&chars);
    }

    let highlight_style = Style::default().bg(Color::Cyan).fg(Color::Black);
    for item in chars.iter_mut().take(end_clamped + 1).skip(start_clamped) {
        item.1 = highlight_style;
    }
    collapse_runs(&chars)
}

fn collapse_runs(chars: &[(char, Style)]) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut current_text = String::new();
    let mut current_style: Option<Style> = None;
    for (c, style) in chars {
        if current_style == Some(*style) {
            current_text.push(*c);
        } else {
            if let Some(s) = current_style.take() {
                spans.push(Span::styled(std::mem::take(&mut current_text), s));
            }
            current_style = Some(*style);
            current_text.push(*c);
        }
    }
    if let Some(s) = current_style {
        spans.push(Span::styled(current_text, s));
    }
    if spans.is_empty() {
        spans.push(Span::raw(" "));
    }
    spans
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
