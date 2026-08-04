use ratatui::style::{Color, Modifier, Style};

use crate::api::types::{DiffLineKind, FileDiff, NormalizedPullRequest, PrComment, PrThread};
use crate::humanize::{format_distance_to_now, parse_date};

use super::comment_status::is_resolved_status;
use super::detail_cache::PullRequestDetailState;
use super::markdown::{render_markdown_to_lines, StyledLine};
use super::review_status::{review_vote_symbol, reviewer_detail_name, reviewer_line_style};
use super::text_width::truncate_to_width;
use super::wrap_text::strip_html;

pub type RightPanelMode = super::state::RightPanelMode;
pub type CommentStatusFilter = super::state::CommentStatusFilter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    Header,
    Meta,
    Body,
    Markdown,
    Blank,
    Empty,
    DiffHunkHeader,
    DiffAdded,
    DiffRemoved,
    DiffContext,
    DiffComment,
}

#[derive(Debug, Clone)]
pub struct ContentLine {
    pub key: String,
    pub kind: LineKind,
    pub spans: Vec<(String, Style)>,
    pub accent: Option<Color>,
}

impl ContentLine {
    pub fn plain(key: impl Into<String>, kind: LineKind, text: impl Into<String>) -> Self {
        ContentLine {
            key: key.into(),
            kind,
            spans: vec![(text.into(), Style::default())],
            accent: None,
        }
    }

    fn markdown(key: impl Into<String>, line: StyledLine) -> Self {
        ContentLine {
            key: key.into(),
            kind: LineKind::Markdown,
            spans: line.0,
            accent: None,
        }
    }

    pub fn styled(key: impl Into<String>, kind: LineKind, text: impl Into<String>, style: Style) -> Self {
        ContentLine {
            key: key.into(),
            kind,
            spans: vec![(text.into(), style)],
            accent: None,
        }
    }

    pub fn plain_text(&self) -> String {
        self.spans.iter().map(|(t, _)| t.as_str()).collect()
    }
}

enum Section {
    General(Vec<PrComment>),
    File(PrThread),
}

/// border (top+bottom) + title + meta + tab-strip line
pub const CHROME_ROWS: usize = 5;
/// Rows before the scrollable content starts: everything in CHROME_ROWS except
/// the bottom border. Shared by rendering and mouse hit-testing/selection so
/// they can never drift out of sync.
pub const CONTENT_TOP_ROWS: usize = CHROME_ROWS - 1;

pub fn right_panel_content_width(panel_width: u16) -> usize {
    (panel_width as i64 - 4).max(10) as usize
}

pub struct BuildRightPanelLinesArgs<'a> {
    pub pull_request: Option<&'a NormalizedPullRequest>,
    pub detail_state: &'a PullRequestDetailState,
    pub mode: RightPanelMode,
    pub comment_status_filter: CommentStatusFilter,
    pub content_width: usize,
}

pub fn build_right_panel_lines(args: BuildRightPanelLinesArgs<'_>) -> Vec<ContentLine> {
    let BuildRightPanelLinesArgs {
        pull_request,
        detail_state,
        mode,
        comment_status_filter,
        content_width,
    } = args;

    if pull_request.is_none() {
        return vec![];
    }

    if detail_state.loading {
        let text = if mode == RightPanelMode::Description {
            "Loading description…"
        } else {
            "Loading comments…"
        };
        return vec![ContentLine::plain("loading", LineKind::Empty, text)];
    }

    if let Some(error) = &detail_state.error {
        return vec![ContentLine::plain("error", LineKind::Empty, error.clone())];
    }

    if mode == RightPanelMode::Description {
        let pr = pull_request.expect("pull request required for description mode");
        let mut lines = build_reviewer_lines(pr, content_width);
        lines.push(ContentLine::plain("reviewers-gap", LineKind::Blank, ""));

        let description = detail_state
            .detail
            .as_ref()
            .map(|d| d.description.trim().to_string())
            .unwrap_or_default();
        if description.is_empty() {
            lines.push(ContentLine::plain(
                "empty",
                LineKind::Empty,
                "No description provided.",
            ));
            return lines;
        }
        lines.extend(
            render_markdown_to_lines(&description, content_width)
                .into_iter()
                .enumerate()
                .map(|(i, line)| ContentLine::markdown(format!("d-{i}"), line)),
        );
        return lines;
    }

    let sections = build_sections(&detail_state.threads, comment_status_filter);
    if sections.is_empty() {
        let empty_text = if comment_status_filter == CommentStatusFilter::All {
            "No comments on this pull request.".to_string()
        } else {
            let label = if comment_status_filter == CommentStatusFilter::Resolved {
                "resolved"
            } else {
                "active"
            };
            format!("No {label} comments on this pull request.")
        };
        return vec![ContentLine::plain("empty", LineKind::Empty, empty_text)];
    }
    build_comment_lines(sections, content_width)
}

/// Number of comment "sections" (the General bucket plus one per file
/// thread) under the given filter — used for the `Comments (N)` tab label.
pub fn count_comment_sections(threads: &[PrThread], status_filter: CommentStatusFilter) -> usize {
    build_sections(threads, status_filter).len()
}

fn build_reviewer_lines(pr: &NormalizedPullRequest, content_width: usize) -> Vec<ContentLine> {
    let mut lines = vec![ContentLine::plain(
        "reviewers-h",
        LineKind::Header,
        "Reviewers:",
    )];

    if pr.is_draft {
        lines.push(ContentLine::styled(
            "reviewers-none",
            LineKind::Body,
            "  (draft — no reviewers)",
            Style::default().add_modifier(Modifier::DIM),
        ));
        return lines;
    }

    if pr.reviewers.is_empty() {
        lines.push(ContentLine::styled(
            "reviewers-none",
            LineKind::Body,
            "  none",
            Style::default().add_modifier(Modifier::DIM),
        ));
        return lines;
    }

    for (i, reviewer) in pr.reviewers.iter().enumerate() {
        let symbol = review_vote_symbol(reviewer.vote);
        let name = reviewer_detail_name(reviewer);
        let team_suffix = if reviewer.is_team { " (team)" } else { "" };
        let text = truncate_to_width(
            &format!("  - {symbol} {name}{team_suffix}"),
            content_width,
        );
        lines.push(ContentLine::styled(
            format!("reviewer-{i}"),
            LineKind::Body,
            text,
            reviewer_line_style(reviewer.vote, reviewer.is_current_user),
        ));
    }

    lines
}

fn build_sections(threads: &[PrThread], status_filter: CommentStatusFilter) -> Vec<Section> {
    let filtered: Vec<&PrThread> = threads
        .iter()
        .filter(|t| match status_filter {
            CommentStatusFilter::All => true,
            CommentStatusFilter::Resolved => is_resolved_status(&t.status),
            CommentStatusFilter::Active => !is_resolved_status(&t.status),
        })
        .collect();

    let general_comments: Vec<PrComment> = filtered
        .iter()
        .filter(|t| t.file_path.is_none())
        .flat_map(|t| t.comments.clone())
        .collect();
    let file_threads: Vec<&PrThread> = filtered.iter().filter(|t| t.file_path.is_some()).copied().collect();

    let mut sections = Vec::new();
    if !general_comments.is_empty() {
        sections.push(Section::General(general_comments));
    }
    for thread in file_threads {
        sections.push(Section::File(thread.clone()));
    }
    sections
}

fn build_comment_lines(sections: Vec<Section>, width: usize) -> Vec<ContentLine> {
    let mut lines: Vec<ContentLine> = Vec::new();

    for section in sections {
        match section {
            Section::General(comments) => {
                lines.push(ContentLine {
                    key: "general-header".to_string(),
                    kind: LineKind::Header,
                    spans: vec![("General".to_string(), Style::default())],
                    accent: Some(Color::Cyan),
                });
                for comment in &comments {
                    push_comment_lines(&mut lines, "general", comment, width);
                }
            }
            Section::File(thread) => {
                let header = format_thread_header(&thread);
                let accent = if is_resolved_status(&thread.status) {
                    Color::Green
                } else {
                    Color::Magenta
                };
                lines.push(ContentLine {
                    key: format!("t-{}-header", thread.id),
                    kind: LineKind::Header,
                    spans: vec![(header, Style::default())],
                    accent: Some(accent),
                });
                for comment in &thread.comments {
                    push_comment_lines(&mut lines, &format!("t-{}", thread.id), comment, width);
                }
            }
        }
    }

    while matches!(lines.last(), Some(l) if l.kind == LineKind::Blank) {
        lines.pop();
    }

    lines
}

fn push_comment_lines(lines: &mut Vec<ContentLine>, scope: &str, comment: &PrComment, width: usize) {
    let prefix = format!("{scope}-c-{}", comment.id);
    let meta = format!(
        "{} · {}",
        comment.author,
        format_distance_to_now(parse_date(&comment.published_at), true)
    );
    lines.push(ContentLine::plain(format!("{prefix}-meta"), LineKind::Meta, meta));
    for (i, line) in render_markdown_to_lines(&strip_html(&comment.content), width)
        .into_iter()
        .enumerate()
    {
        lines.push(ContentLine::markdown(format!("{prefix}-b-{i}"), line));
    }
    lines.push(ContentLine::plain(format!("{prefix}-blank"), LineKind::Blank, ""));
}

fn format_thread_header(thread: &PrThread) -> String {
    let base = match &thread.file_path {
        None => "Comment".to_string(),
        Some(path) => match thread.line {
            None => path.clone(),
            Some(line) => {
                let label = match thread.line_end {
                    Some(end) if end != line => format!("Line {line}-{end}"),
                    _ => format!("Line {line}"),
                };
                format!("{path}: {label}")
            }
        },
    };
    if is_resolved_status(&thread.status) {
        format!("{base} [{}]", thread.status)
    } else {
        base
    }
}

pub struct VisibleRightPanelLines<'a> {
    pub visible: &'a [ContentLine],
    pub clamped_scroll: usize,
    pub max_scroll: usize,
}

pub fn get_visible_right_panel_lines(
    lines: &[ContentLine],
    scroll_offset: usize,
    content_height: usize,
) -> VisibleRightPanelLines<'_> {
    let max_scroll = lines.len().saturating_sub(content_height);
    let clamped_scroll = scroll_offset.min(max_scroll);
    let end = (clamped_scroll + content_height).min(lines.len());
    VisibleRightPanelLines {
        visible: &lines[clamped_scroll..end],
        clamped_scroll,
        max_scroll,
    }
}

pub fn line_style_for_kind(kind: LineKind, accent: Option<Color>) -> Style {
    match kind {
        LineKind::Empty => Style::default().add_modifier(Modifier::DIM),
        LineKind::Header => Style::default()
            .fg(accent.unwrap_or(Color::Magenta))
            .add_modifier(Modifier::BOLD),
        LineKind::Meta => Style::default().add_modifier(Modifier::DIM),
        LineKind::Blank | LineKind::Body | LineKind::Markdown => Style::default(),
        LineKind::DiffHunkHeader => Style::default().fg(Color::Cyan).add_modifier(Modifier::DIM),
        LineKind::DiffAdded => Style::default().fg(Color::Green),
        LineKind::DiffRemoved => Style::default().fg(Color::Red),
        LineKind::DiffContext => Style::default(),
        LineKind::DiffComment => Style::default().fg(Color::Magenta),
    }
}

/// Builds the scrollable content lines for the diff pane of the Files tab:
/// hunk headers, +/-/context lines with old/new line-number gutters, and any
/// PR comments attached to a line (via `diffing::attach_comments`) rendered
/// indented immediately below it.
pub fn build_diff_lines_for_file(diff: &FileDiff, width: usize) -> Vec<ContentLine> {
    if diff.is_binary {
        return vec![ContentLine::plain(
            "binary",
            LineKind::Empty,
            "Binary file not shown.",
        )];
    }
    if diff.lines.is_empty() {
        return vec![ContentLine::plain(
            "empty-diff",
            LineKind::Empty,
            "No changes to display.",
        )];
    }

    // "{old:>4} {new:>4} {marker} " — must match the gutter format built
    // below, so inline comment indentation lines up under the code column.
    const GUTTER_WIDTH: usize = 12;
    let mut lines: Vec<ContentLine> = Vec::new();
    for (i, line) in diff.lines.iter().enumerate() {
        match line.kind {
            DiffLineKind::HunkHeader => {
                lines.push(ContentLine::styled(
                    format!("h-{i}"),
                    LineKind::DiffHunkHeader,
                    truncate_to_width(&line.content, width),
                    line_style_for_kind(LineKind::DiffHunkHeader, None),
                ));
                continue;
            }
            _ => {
                let (kind, marker) = match line.kind {
                    DiffLineKind::Added => (LineKind::DiffAdded, "+"),
                    DiffLineKind::Removed => (LineKind::DiffRemoved, "-"),
                    _ => (LineKind::DiffContext, " "),
                };
                let old_no = line
                    .old_line_no
                    .map(|n| n.to_string())
                    .unwrap_or_default();
                let new_no = line
                    .new_line_no
                    .map(|n| n.to_string())
                    .unwrap_or_default();
                let gutter = format!("{old_no:>4} {new_no:>4} {marker} ");
                let content_width = width.saturating_sub(gutter.chars().count()).max(1);
                let text = format!(
                    "{gutter}{}",
                    truncate_to_width(&line.content, content_width)
                );
                lines.push(ContentLine::styled(
                    format!("l-{i}"),
                    kind,
                    text,
                    line_style_for_kind(kind, None),
                ));
            }
        }

        if !line.comments.is_empty() {
            for comment in &line.comments {
                push_diff_comment_lines(&mut lines, i, comment, width, GUTTER_WIDTH);
            }
        }
    }
    lines
}

fn push_diff_comment_lines(
    lines: &mut Vec<ContentLine>,
    line_index: usize,
    comment: &PrComment,
    width: usize,
    gutter_width: usize,
) {
    let indent = " ".repeat(gutter_width);
    let prefix = format!("dc-{line_index}-{}", comment.id);
    let meta = format!(
        "{indent}│ {} · {}",
        comment.author,
        format_distance_to_now(parse_date(&comment.published_at), true)
    );
    lines.push(ContentLine::styled(
        format!("{prefix}-meta"),
        LineKind::DiffComment,
        truncate_to_width(&meta, width),
        line_style_for_kind(LineKind::DiffComment, None),
    ));
    let body_width = width.saturating_sub(gutter_width + 2).max(1);
    for (i, styled_line) in render_markdown_to_lines(&strip_html(&comment.content), body_width)
        .into_iter()
        .enumerate()
    {
        let text = format!("{indent}│ {}", plain_from_styled(&styled_line));
        lines.push(ContentLine::plain(
            format!("{prefix}-b-{i}"),
            LineKind::DiffComment,
            truncate_to_width(&text, width),
        ));
    }
}

fn plain_from_styled(line: &StyledLine) -> String {
    line.0.iter().map(|(t, _)| t.as_str()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn thread(id: i64, status: &str, file: Option<&str>) -> PrThread {
        PrThread {
            id,
            status: status.into(),
            file_path: file.map(|s| s.to_string()),
            line: Some(10),
            line_end: None,
            comments: vec![PrComment {
                id: 1,
                author: "Alice".into(),
                content: "hello".into(),
                published_at: "2024-01-01T00:00:00Z".into(),
            }],
        }
    }

    #[test]
    fn builds_general_and_file_sections() {
        let threads = vec![thread(1, "active", None), thread(2, "active", Some("a.rs"))];
        let sections = build_sections(&threads, CommentStatusFilter::All);
        assert_eq!(sections.len(), 2);
    }

    #[test]
    fn filters_resolved_vs_active() {
        let threads = vec![thread(1, "fixed", Some("a.rs")), thread(2, "active", Some("b.rs"))];
        let active_only = build_sections(&threads, CommentStatusFilter::Active);
        assert_eq!(active_only.len(), 1);
        let resolved_only = build_sections(&threads, CommentStatusFilter::Resolved);
        assert_eq!(resolved_only.len(), 1);
    }

    #[test]
    fn thread_header_includes_line_range() {
        let mut t = thread(1, "active", Some("a.rs"));
        t.line = Some(5);
        t.line_end = Some(9);
        assert_eq!(format_thread_header(&t), "a.rs: Line 5-9");
    }

    #[test]
    fn resolved_header_includes_status_tag() {
        let t = thread(1, "fixed", Some("a.rs"));
        assert_eq!(format_thread_header(&t), "a.rs: Line 10 [fixed]");
    }

    #[test]
    fn diff_lines_render_hunk_header_and_gutters() {
        use crate::api::types::{PrFileChange, PrFileChangeType};
        use crate::diffing::build_file_diff;

        let diff = build_file_diff(
            PrFileChange {
                path: "a.rs".into(),
                old_path: None,
                change_type: PrFileChangeType::Edit,
            },
            "a\nb\nc\n",
            "a\nX\nc\n",
        );
        let lines = build_diff_lines_for_file(&diff, 80);
        assert!(lines.iter().any(|l| l.kind == LineKind::DiffHunkHeader));
        assert!(lines.iter().any(|l| l.kind == LineKind::DiffAdded));
        assert!(lines.iter().any(|l| l.kind == LineKind::DiffRemoved));
    }

    #[test]
    fn diff_lines_render_binary_placeholder() {
        use crate::api::types::{FileDiff, PrFileChange, PrFileChangeType};

        let diff = FileDiff {
            file: PrFileChange {
                path: "img.png".into(),
                old_path: None,
                change_type: PrFileChangeType::Edit,
            },
            lines: vec![],
            additions: 0,
            deletions: 0,
            is_binary: true,
        };
        let lines = build_diff_lines_for_file(&diff, 80);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].plain_text().contains("Binary"));
    }

    #[test]
    fn diff_lines_include_attached_comments() {
        use crate::api::types::{PrFileChange, PrFileChangeType, PrThread};
        use crate::diffing::{attach_comments, build_file_diff};

        let mut diff = build_file_diff(
            PrFileChange {
                path: "a.rs".into(),
                old_path: None,
                change_type: PrFileChangeType::Edit,
            },
            "a\nb\nc\n",
            "a\nX\nc\n",
        );
        attach_comments(
            &mut diff,
            &[PrThread {
                id: 1,
                status: "active".into(),
                file_path: Some("a.rs".into()),
                line: Some(2),
                line_end: None,
                comments: vec![PrComment {
                    id: 1,
                    author: "Alice".into(),
                    content: "why?".into(),
                    published_at: "2024-01-01T00:00:00Z".into(),
                }],
            }],
        );
        let lines = build_diff_lines_for_file(&diff, 80);
        assert!(lines.iter().any(|l| l.kind == LineKind::DiffComment));
        assert!(lines.iter().any(|l| l.plain_text().contains("Alice")));
    }
}
