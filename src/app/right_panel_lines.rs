use ratatui::style::{Color, Modifier, Style};

use crate::api::types::{NormalizedPullRequest, PrComment, PrThread};
use crate::humanize::{format_distance_to_now, parse_date};

use super::comment_status::is_resolved_status;
use super::detail_cache::PullRequestDetailState;
use super::markdown::{render_markdown_to_lines, StyledLine};
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
}

#[derive(Debug, Clone)]
pub struct ContentLine {
    pub key: String,
    pub kind: LineKind,
    pub spans: Vec<(String, Style)>,
    pub accent: Option<Color>,
}

impl ContentLine {
    fn plain(key: impl Into<String>, kind: LineKind, text: impl Into<String>) -> Self {
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

    pub fn plain_text(&self) -> String {
        self.spans.iter().map(|(t, _)| t.as_str()).collect()
    }
}

enum Section {
    General(Vec<PrComment>),
    File(PrThread),
}

/// border (top+bottom) + title line + meta line + tab-strip margin + tab-strip line
pub const CHROME_ROWS: usize = 6;
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
        let description = detail_state
            .detail
            .as_ref()
            .map(|d| d.description.trim().to_string())
            .unwrap_or_default();
        if description.is_empty() {
            return vec![ContentLine::plain(
                "empty",
                LineKind::Empty,
                "No description provided.",
            )];
        }
        return render_markdown_to_lines(&description, content_width)
            .into_iter()
            .enumerate()
            .map(|(i, line)| ContentLine::markdown(format!("d-{i}"), line))
            .collect();
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
    }
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
}
