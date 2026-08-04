//! Combined multi-file diff view for the Files tab: builds one scrollable
//! stream of file headers + diff lines (inline or side-by-side), tracks
//! section boundaries so scroll position can drive file-list selection.

use std::collections::HashMap;

use ratatui::style::{Color, Style};

use crate::api::types::{DiffLineKind, FileDiff, PrFileChange};
use crate::app::file_diff_cache::FileDiffState;
use crate::app::right_panel_lines::{line_style_for_kind, ContentLine, LineKind};
use crate::app::text_width::truncate_to_width;

/// Width of the draggable divider column between the file-list and diff cards.
pub const FILES_DIVIDER_WIDTH: u16 = 1;
pub const DEFAULT_FILES_LIST_WIDTH_RATIO: f64 = 0.32;
pub const MIN_FILES_LIST_RATIO: f64 = 0.18;
pub const MAX_FILES_LIST_RATIO: f64 = 0.55;
pub const MIN_FILES_LIST_WIDTH: u16 = 14;
pub const MIN_FILES_DIFF_WIDTH: u16 = 20;

/// Shared layout for the Files tab's two-card split. Both rendering and mouse
/// hit-testing use this so widths, scroll regions, and the divider position
/// never drift apart.
#[derive(Debug, Clone, Copy)]
pub struct FilesPanelLayout {
    /// Absolute terminal column of the draggable divider.
    pub divider_x: u16,
    /// Absolute terminal column where the file-list card begins.
    pub list_left: u16,
    /// Absolute terminal column where the diff card begins.
    pub diff_left: u16,
    pub list_card_width: u16,
    pub diff_card_width: u16,
    pub list_text_width: usize,
    pub diff_text_width: usize,
    pub list_inner_height: usize,
    pub diff_inner_height: usize,
    pub content_height: usize,
}

impl FilesPanelLayout {
    pub fn compute(app: &crate::app::state::App) -> Self {
        let geometry = app.geometry();
        let detail_left = geometry.left_width.saturating_add(1);
        let content_width = geometry.right_width.saturating_sub(2);
        let content_height =
            (geometry.panel_height as i64 - crate::app::right_panel_lines::CHROME_ROWS as i64)
                .max(1) as u16;

        let available = content_width.saturating_sub(FILES_DIVIDER_WIDTH);
        let max_list = available.saturating_sub(MIN_FILES_DIFF_WIDTH);

        let mut list_w =
            ((available as f64) * app.files_list_width_ratio).floor() as u16;
        list_w = list_w.clamp(MIN_FILES_LIST_WIDTH, max_list.max(MIN_FILES_LIST_WIDTH));
        let diff_w = available.saturating_sub(list_w);

        let divider_x = detail_left.saturating_add(list_w);
        let diff_left = divider_x.saturating_add(FILES_DIVIDER_WIDTH);

        let (list_text_w, list_inner_h) = card_text_size(list_w, content_height);
        let (diff_text_w, diff_inner_h) = card_text_size(diff_w, content_height);

        FilesPanelLayout {
            divider_x,
            list_left: detail_left,
            diff_left,
            list_card_width: list_w,
            diff_card_width: diff_w,
            list_text_width: list_text_w,
            diff_text_width: diff_text_w,
            list_inner_height: list_inner_h,
            diff_inner_height: diff_inner_h,
            content_height: content_height as usize,
        }
    }

    pub fn is_on_divider(&self, x: i64) -> bool {
        (x - self.divider_x as i64).abs() <= 1
    }

    pub fn is_in_list_card(&self, x: i64) -> bool {
        x >= self.list_left as i64 && x < self.divider_x as i64
    }

    pub fn is_in_diff_card(&self, x: i64) -> bool {
        x >= self.diff_left as i64
    }
}

fn card_text_size(card_width: u16, card_height: u16) -> (usize, usize) {
    // Block borders (2) + horizontal padding (2); title shares the top border row.
    let text_w = (card_width as i64 - 4).max(1) as usize;
    let inner_h = (card_height as i64 - 2).max(1) as usize;
    (text_w, inner_h)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffViewMode {
    Inline,
    SideBySide,
}

impl DiffViewMode {
    pub fn toggle(self) -> Self {
        match self {
            DiffViewMode::Inline => DiffViewMode::SideBySide,
            DiffViewMode::SideBySide => DiffViewMode::Inline,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            DiffViewMode::Inline => "inline",
            DiffViewMode::SideBySide => "split",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilesPanelFocus {
    FileList,
    Diff,
}

/// Marks where each file's block begins inside [`CombinedFileDiffView::lines`].
#[derive(Debug, Clone)]
pub struct FileDiffSection {
    pub file_index: usize,
    pub start_line: usize,
}

#[derive(Debug, Clone)]
pub struct CombinedFileDiffView {
    pub lines: Vec<ContentLine>,
    pub sections: Vec<FileDiffSection>,
    /// Line indices of every `@@` hunk header in [`lines`].
    pub hunk_lines: Vec<usize>,
}

pub fn build_combined_file_diff_view(
    files: &[PrFileChange],
    file_diffs: &HashMap<String, FileDiffState>,
    pr_key: &str,
    view_mode: DiffViewMode,
    width: usize,
) -> CombinedFileDiffView {
    if files.is_empty() {
        return CombinedFileDiffView {
            lines: vec![ContentLine::plain(
                "no-files",
                LineKind::Empty,
                "No files changed.",
            )],
            sections: vec![],
            hunk_lines: vec![],
        };
    }

    let mut lines: Vec<ContentLine> = Vec::new();
    let mut sections: Vec<FileDiffSection> = Vec::new();

    for (file_index, file) in files.iter().enumerate() {
        if !lines.is_empty() {
            lines.push(ContentLine::plain(
                format!("gap-{file_index}"),
                LineKind::Blank,
                "",
            ));
        }

        let start_line = lines.len();
        sections.push(FileDiffSection {
            file_index,
            start_line,
        });

        let key = crate::app::file_diff_cache::file_diff_key(pr_key, &file.path);
        let header = format!(
            "{}  {}",
            file.path,
            file.change_type.icon()
        );
        lines.push(ContentLine {
            key: format!("fh-{file_index}"),
            kind: LineKind::Header,
            spans: vec![(truncate_to_width(&header, width), Style::default())],
            accent: Some(file_change_color(file.change_type)),
        });

        match file_diffs.get(&key) {
            None | Some(FileDiffState { loading: true, .. }) => {
                lines.push(ContentLine::plain(
                    format!("loading-{file_index}"),
                    LineKind::Empty,
                    "Loading diff…",
                ));
            }
            Some(FileDiffState {
                error: Some(err), ..
            }) => {
                lines.push(ContentLine::plain(
                    format!("error-{file_index}"),
                    LineKind::Empty,
                    err.clone(),
                ));
            }
            Some(FileDiffState {
                diff: Some(diff), ..
            }) => {
                lines.extend(diff_lines_for_file(diff, view_mode, width, file_index));
            }
            Some(FileDiffState { diff: None, .. }) => {
                lines.push(ContentLine::plain(
                    format!("loading-{file_index}"),
                    LineKind::Empty,
                    "Loading diff…",
                ));
            }
        }
    }

    let hunk_lines: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.kind == LineKind::DiffHunkHeader)
        .map(|(i, _)| i)
        .collect();

    CombinedFileDiffView {
        lines,
        sections,
        hunk_lines,
    }
}

fn diff_lines_for_file(
    diff: &FileDiff,
    view_mode: DiffViewMode,
    width: usize,
    file_index: usize,
) -> Vec<ContentLine> {
    match view_mode {
        DiffViewMode::Inline => build_inline_diff_lines(diff, width, file_index),
        DiffViewMode::SideBySide => build_side_by_side_diff_lines(diff, width, file_index),
    }
}

fn build_inline_diff_lines(diff: &FileDiff, width: usize, file_index: usize) -> Vec<ContentLine> {
    if diff.is_binary {
        return vec![ContentLine::plain(
            format!("binary-{file_index}"),
            LineKind::Empty,
            "Binary file not shown.",
        )];
    }
    if diff.lines.is_empty() {
        return vec![ContentLine::plain(
            format!("empty-{file_index}"),
            LineKind::Empty,
            "No changes to display.",
        )];
    }

    const GUTTER_WIDTH: usize = 12;
    let mut lines = Vec::new();
    for (i, line) in diff.lines.iter().enumerate() {
        match line.kind {
            DiffLineKind::HunkHeader => {
                lines.push(ContentLine::styled(
                    format!("f{file_index}-h-{i}"),
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
                    format!("f{file_index}-l-{i}"),
                    kind,
                    text,
                    line_style_for_kind(kind, None),
                ));
            }
        }

        if !line.comments.is_empty() {
            for comment in &line.comments {
                push_inline_comment_lines(&mut lines, file_index, i, comment, width, GUTTER_WIDTH);
            }
        }
    }
    lines
}

fn push_inline_comment_lines(
    lines: &mut Vec<ContentLine>,
    file_index: usize,
    line_index: usize,
    comment: &crate::api::types::PrComment,
    width: usize,
    gutter_width: usize,
) {
    use crate::app::markdown::render_markdown_to_lines;
    use crate::app::wrap_text::strip_html;
    use crate::humanize::{format_distance_to_now, parse_date};

    let indent = " ".repeat(gutter_width);
    let prefix = format!("f{file_index}-dc-{line_index}-{}", comment.id);
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
        let text: String = styled_line.0.iter().map(|(t, _)| t.as_str()).collect();
        lines.push(ContentLine::plain(
            format!("{prefix}-b-{i}"),
            LineKind::DiffComment,
            truncate_to_width(&format!("{indent}│ {text}"), width),
        ));
    }
}

fn build_side_by_side_diff_lines(diff: &FileDiff, width: usize, file_index: usize) -> Vec<ContentLine> {
    if diff.is_binary {
        return vec![ContentLine::plain(
            format!("binary-{file_index}"),
            LineKind::Empty,
            "Binary file not shown.",
        )];
    }
    if diff.lines.is_empty() {
        return vec![ContentLine::plain(
            format!("empty-{file_index}"),
            LineKind::Empty,
            "No changes to display.",
        )];
    }

    let separator = " │ ";
    let sep_width = separator.chars().count();
    let half = width.saturating_sub(sep_width) / 2;
    let half_content = half.saturating_sub(7).max(1); // " 1234 - " gutter

    let mut lines = Vec::new();
    for (i, line) in diff.lines.iter().enumerate() {
        match line.kind {
            DiffLineKind::HunkHeader => {
                lines.push(ContentLine::styled(
                    format!("f{file_index}-h-{i}"),
                    LineKind::DiffHunkHeader,
                    truncate_to_width(&line.content, width),
                    line_style_for_kind(LineKind::DiffHunkHeader, None),
                ));
            }
            _ => {
                let (left, right, kind) = side_by_side_halves(line, half, half_content);
                let text = format!("{left}{separator}{right}");
                lines.push(ContentLine::styled(
                    format!("f{file_index}-l-{i}"),
                    kind,
                    truncate_to_width(&text, width),
                    line_style_for_kind(kind, None),
                ));
            }
        }

        if !line.comments.is_empty() {
            for comment in &line.comments {
                let text = format!(
                    "  │ {} · {}",
                    comment.author, comment.content.chars().take(half_content).collect::<String>()
                );
                lines.push(ContentLine::styled(
                    format!("f{file_index}-c-{i}-{}", comment.id),
                    LineKind::DiffComment,
                    truncate_to_width(&text, width),
                    line_style_for_kind(LineKind::DiffComment, None),
                ));
            }
        }
    }
    lines
}

fn side_by_side_halves(
    line: &crate::api::types::DiffLine,
    half: usize,
    content_width: usize,
) -> (String, String, LineKind) {
    match line.kind {
        DiffLineKind::Removed => {
            let left = format_side_half(
                line.old_line_no,
                "-",
                &line.content,
                content_width,
                half,
            );
            (left, " ".repeat(half.min(40)), LineKind::DiffRemoved)
        }
        DiffLineKind::Added => {
            let right = format_side_half(
                line.new_line_no,
                "+",
                &line.content,
                content_width,
                half,
            );
            (" ".repeat(half.min(40)), right, LineKind::DiffAdded)
        }
        DiffLineKind::Context => {
            let left = format_side_half(
                line.old_line_no,
                " ",
                &line.content,
                content_width,
                half,
            );
            let right = format_side_half(
                line.new_line_no,
                " ",
                &line.content,
                content_width,
                half,
            );
            (left, right, LineKind::DiffContext)
        }
        DiffLineKind::HunkHeader => unreachable!(),
    }
}

fn format_side_half(
    line_no: Option<i64>,
    marker: &str,
    content: &str,
    content_width: usize,
    half_width: usize,
) -> String {
    let no = line_no.map(|n| n.to_string()).unwrap_or_default();
    let gutter = format!("{no:>4} {marker} ");
    let avail = half_width.saturating_sub(gutter.chars().count()).max(1);
    let truncated = truncate_to_width(content, avail.min(content_width));
    let text = format!("{gutter}{truncated}");
    truncate_to_width(&text, half_width)
}

pub fn file_index_at_scroll(sections: &[FileDiffSection], scroll: usize) -> usize {
    sections
        .iter()
        .rev()
        .find(|s| s.start_line <= scroll)
        .map(|s| s.file_index)
        .unwrap_or(0)
}

pub fn scroll_offset_for_file(sections: &[FileDiffSection], file_index: usize) -> usize {
    sections
        .iter()
        .find(|s| s.file_index == file_index)
        .map(|s| s.start_line)
        .unwrap_or(0)
}

/// Scroll offset for the next hunk strictly after `scroll`.
pub fn next_hunk_scroll(hunk_lines: &[usize], scroll: usize) -> Option<usize> {
    hunk_lines.iter().copied().find(|&line| line > scroll)
}

/// Scroll offset for the previous hunk strictly before `scroll`.
pub fn prev_hunk_scroll(hunk_lines: &[usize], scroll: usize) -> Option<usize> {
    hunk_lines.iter().rev().copied().find(|&line| line < scroll)
}

fn file_change_color(change_type: crate::api::types::PrFileChangeType) -> Color {
    use crate::api::types::PrFileChangeType;
    match change_type {
        PrFileChangeType::Add => Color::Green,
        PrFileChangeType::Edit => Color::Yellow,
        PrFileChangeType::Delete => Color::Red,
        PrFileChangeType::Rename => Color::Cyan,
        PrFileChangeType::Other => Color::Gray,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::types::PrFileChangeType;
    use crate::diffing::build_file_diff;

    fn file(path: &str) -> PrFileChange {
        PrFileChange {
            path: path.to_string(),
            old_path: None,
            change_type: PrFileChangeType::Edit,
        }
    }

    #[test]
    fn combined_view_creates_section_per_file() {
        let files = vec![file("a.rs"), file("b.rs")];
        let mut diffs = HashMap::new();
        let diff_a = build_file_diff(file("a.rs"), "a\n", "b\n");
        diffs.insert(
            "pr:a.rs".into(),
            FileDiffState {
                diff: Some(diff_a),
                loading: false,
                error: None,
            },
        );
        let view = build_combined_file_diff_view(&files, &diffs, "pr", DiffViewMode::Inline, 80);
        assert_eq!(view.sections.len(), 2);
        assert!(view.lines.len() > 2);
    }

    #[test]
    fn scroll_position_maps_to_file_index() {
        let sections = vec![
            FileDiffSection {
                file_index: 0,
                start_line: 0,
            },
            FileDiffSection {
                file_index: 1,
                start_line: 10,
            },
            FileDiffSection {
                file_index: 2,
                start_line: 25,
            },
        ];
        assert_eq!(file_index_at_scroll(&sections, 0), 0);
        assert_eq!(file_index_at_scroll(&sections, 9), 0);
        assert_eq!(file_index_at_scroll(&sections, 10), 1);
        assert_eq!(file_index_at_scroll(&sections, 24), 1);
        assert_eq!(file_index_at_scroll(&sections, 25), 2);
    }

    #[test]
    fn scroll_offset_for_file_index() {
        let sections = vec![
            FileDiffSection {
                file_index: 0,
                start_line: 0,
            },
            FileDiffSection {
                file_index: 1,
                start_line: 10,
            },
        ];
        assert_eq!(scroll_offset_for_file(&sections, 1), 10);
    }

    #[test]
    fn diff_view_mode_toggles() {
        assert_eq!(
            DiffViewMode::Inline.toggle(),
            DiffViewMode::SideBySide
        );
        assert_eq!(
            DiffViewMode::SideBySide.toggle(),
            DiffViewMode::Inline
        );
    }

    #[test]
    fn combined_view_collects_hunk_line_indices() {
        let files = vec![file("a.rs")];
        let mut diffs = HashMap::new();
        let diff = build_file_diff(
            file("a.rs"),
            "line1\nline2\nline3\n",
            "line1\nchanged\nline3\nline4\n",
        );
        diffs.insert(
            "pr:a.rs".into(),
            FileDiffState {
                diff: Some(diff),
                loading: false,
                error: None,
            },
        );
        let view = build_combined_file_diff_view(&files, &diffs, "pr", DiffViewMode::Inline, 80);
        assert!(!view.hunk_lines.is_empty());
        assert_eq!(
            view.lines[view.hunk_lines[0]].kind,
            LineKind::DiffHunkHeader
        );
    }

    #[test]
    fn hunk_scroll_navigation() {
        let hunks = vec![5, 20, 40];
        assert_eq!(next_hunk_scroll(&hunks, 0), Some(5));
        assert_eq!(next_hunk_scroll(&hunks, 5), Some(20));
        assert_eq!(next_hunk_scroll(&hunks, 6), Some(20));
        assert_eq!(next_hunk_scroll(&hunks, 40), None);
        assert_eq!(prev_hunk_scroll(&hunks, 40), Some(20));
        assert_eq!(prev_hunk_scroll(&hunks, 20), Some(5));
        assert_eq!(prev_hunk_scroll(&hunks, 5), None);
    }
}
