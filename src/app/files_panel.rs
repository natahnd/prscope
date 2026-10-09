//! Combined multi-file diff view for the Files tab: builds one scrollable
//! stream of file headers + diff lines (inline or side-by-side), tracks
//! section boundaries so scroll position can drive file-list selection.

use std::collections::{HashMap, HashSet};

use ratatui::style::{Color, Modifier, Style};

use crate::api::types::{DiffLineKind, FileDiff, PrFileChange};
use crate::app::file_diff_cache::FileDiffState;
use crate::app::right_panel_lines::{
    line_style_for_kind, push_diff_comment_callout, ContentLine, LineKind,
};
use crate::app::ui::styles::file_change_accent;
use crate::app::text_width::{display_width, truncate_to_width};

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
    /// Line indices of every `╰─ comment` connector in [`lines`].
    pub comment_lines: Vec<usize>,
}

pub fn build_combined_file_diff_view(
    files: &[PrFileChange],
    file_indices: &[usize],
    file_diffs: &HashMap<String, FileDiffState>,
    pr_key: &str,
    view_mode: DiffViewMode,
    width: usize,
    collapsed_hunks: &HashSet<String>,
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
            comment_lines: vec![],
        };
    }
    if file_indices.is_empty() {
        return CombinedFileDiffView {
            lines: vec![ContentLine::plain(
                "no-match",
                LineKind::Empty,
                "No files match the current filter.",
            )],
            sections: vec![],
            hunk_lines: vec![],
            comment_lines: vec![],
        };
    }

    let mut lines: Vec<ContentLine> = Vec::new();
    let mut sections: Vec<FileDiffSection> = Vec::new();

    for (displayed_index, &orig_index) in file_indices.iter().enumerate() {
        let file = &files[orig_index];
        if !lines.is_empty() {
            push_file_separator(&mut lines, width, displayed_index);
        }

        let start_line = lines.len();
        sections.push(FileDiffSection {
            file_index: displayed_index,
            start_line,
        });
        let key = crate::app::file_diff_cache::file_diff_key(pr_key, &file.path);
        let header = format!(
            "{}  {}",
            file.path,
            file.change_type.icon()
        );
        lines.push(ContentLine {
            key: format!("fh-{orig_index}"),
            kind: LineKind::Header,
            spans: vec![(truncate_to_width(&header, width), Style::default())],
            accent: Some(file_change_accent(file.change_type)),
            hunk_band: None,
        });

        match file_diffs.get(&key) {
            None | Some(FileDiffState { loading: true, .. }) => {
                lines.push(ContentLine::plain(
                    format!("loading-{orig_index}"),
                    LineKind::Empty,
                    "Loading diff…",
                ));
            }
            Some(FileDiffState {
                error: Some(err), ..
            }) => {
                lines.push(ContentLine::plain(
                    format!("error-{orig_index}"),
                    LineKind::Empty,
                    err.clone(),
                ));
            }
            Some(FileDiffState {
                diff: Some(diff), ..
            }) => {
                lines.extend(diff_lines_for_file(
                    diff,
                    view_mode,
                    width,
                    orig_index,
                    collapsed_hunks,
                ));
            }
            Some(FileDiffState { diff: None, .. }) => {
                lines.push(ContentLine::plain(
                    format!("loading-{orig_index}"),
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
    let comment_lines: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.kind == LineKind::DiffCommentConnector)
        .map(|(i, _)| i)
        .collect();

    CombinedFileDiffView {
        lines,
        sections,
        hunk_lines,
        comment_lines,
    }
}

fn diff_lines_for_file(
    diff: &FileDiff,
    view_mode: DiffViewMode,
    width: usize,
    file_index: usize,
    collapsed_hunks: &HashSet<String>,
) -> Vec<ContentLine> {
    match view_mode {
        DiffViewMode::Inline => build_inline_diff_lines(diff, width, file_index, collapsed_hunks),
        DiffViewMode::SideBySide => {
            build_side_by_side_diff_lines(diff, width, file_index, collapsed_hunks)
        }
    }
}

fn hunk_context_collapsed(
    hunk_key: &str,
    collapsed_hunks: &HashSet<String>,
) -> bool {
    collapsed_hunks.contains(hunk_key)
}

fn push_file_separator(lines: &mut Vec<ContentLine>, width: usize, file_index: usize) {
    let rule = truncate_to_width(&"─".repeat(width.saturating_mul(2)), width);
    lines.push(ContentLine::styled(
        format!("file-sep-{file_index}"),
        LineKind::FileSeparator,
        rule,
        line_style_for_kind(LineKind::FileSeparator, None),
    ));
}

fn build_inline_diff_lines(
    diff: &FileDiff,
    width: usize,
    file_index: usize,
    collapsed_hunks: &HashSet<String>,
) -> Vec<ContentLine> {
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
    let mut current_hunk_key: Option<String> = None;
    let mut hunk_number: usize = 0;
    let mut current_band: u8 = 0;

    for (i, line) in diff.lines.iter().enumerate() {
        match line.kind {
            DiffLineKind::HunkHeader => {
                let hunk_key = format!("f{file_index}-h-{i}");
                current_hunk_key = Some(hunk_key.clone());
                current_band = (hunk_number % 2) as u8;
                hunk_number += 1;
                let collapsed = hunk_context_collapsed(&hunk_key, collapsed_hunks);
                let header = if collapsed {
                    format!("{}  ▸", truncate_to_width(&line.content, width.saturating_sub(4)))
                } else {
                    truncate_to_width(&line.content, width)
                };
                lines.push(
                    ContentLine::styled(
                        hunk_key,
                        LineKind::DiffHunkHeader,
                        header,
                        line_style_for_kind(LineKind::DiffHunkHeader, None),
                    )
                    .with_hunk_band(current_band),
                );
                continue;
            }
            DiffLineKind::Context => {
                if let Some(key) = &current_hunk_key {
                    if hunk_context_collapsed(key, collapsed_hunks) {
                        continue;
                    }
                }
            }
            _ => {}
        }

        if line.kind != DiffLineKind::HunkHeader {
            let (kind, marker) = match line.kind {
                DiffLineKind::Added => (LineKind::DiffAdded, "+"),
                DiffLineKind::Removed => (LineKind::DiffRemoved, "-"),
                DiffLineKind::Context => (LineKind::DiffContext, " "),
                DiffLineKind::HunkHeader => unreachable!(),
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
            lines.push(
                ContentLine::styled(
                    format!("f{file_index}-l-{i}"),
                    kind,
                    text,
                    line_style_for_kind(kind, None),
                )
                .with_hunk_band(current_band),
            );
        }

        if !line.comments.is_empty() {
            for comment in &line.comments {
                push_diff_comment_callout(
                    &mut lines,
                    &format!("f{file_index}-dc-{i}-{}", comment.id),
                    comment,
                    width,
                    GUTTER_WIDTH,
                    Some(current_band),
                );
            }
        }
    }
    lines
}

fn build_side_by_side_diff_lines(
    diff: &FileDiff,
    width: usize,
    file_index: usize,
    collapsed_hunks: &HashSet<String>,
) -> Vec<ContentLine> {
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

    const SEPARATOR: &str = " ║ ";
    const CALLOUT_PAD: usize = 2;
    let sep_width = display_width(SEPARATOR);
    let half_left = width.saturating_sub(sep_width) / 2;
    let half_right = width.saturating_sub(sep_width).saturating_sub(half_left);
    let half_content = half_left.saturating_sub(7).max(1);

    let mut lines = Vec::new();
    lines.push(side_by_side_column_header(
        file_index,
        width,
        half_left,
        half_right,
        SEPARATOR,
    ));

    let mut current_hunk_key: Option<String> = None;
    let mut hunk_number: usize = 0;
    let mut current_band: u8 = 0;

    for (i, line) in diff.lines.iter().enumerate() {
        match line.kind {
            DiffLineKind::HunkHeader => {
                let hunk_key = format!("f{file_index}-h-{i}");
                current_hunk_key = Some(hunk_key.clone());
                current_band = (hunk_number % 2) as u8;
                hunk_number += 1;
                let collapsed = hunk_context_collapsed(&hunk_key, collapsed_hunks);
                let header = if collapsed {
                    format!("{}  ▸", truncate_to_width(&line.content, width.saturating_sub(4)))
                } else {
                    truncate_to_width(&line.content, width)
                };
                lines.push(
                    ContentLine::styled(
                        hunk_key,
                        LineKind::DiffHunkHeader,
                        header,
                        line_style_for_kind(LineKind::DiffHunkHeader, None),
                    )
                    .with_hunk_band(current_band),
                );
            }
            DiffLineKind::Context => {
                if let Some(key) = &current_hunk_key {
                    if hunk_context_collapsed(key, collapsed_hunks) {
                        continue;
                    }
                }
                let (left, right) = side_by_side_halves(line, half_left, half_right, half_content);
                lines.push(side_by_side_line(
                    format!("f{file_index}-l-{i}"),
                    left,
                    right,
                    LineKind::DiffContext,
                    SEPARATOR,
                    width,
                    current_band,
                ));
            }
            _ => {
                let (left, right, kind) =
                    side_by_side_halves_typed(line, half_left, half_right, half_content);
                lines.push(side_by_side_line(
                    format!("f{file_index}-l-{i}"),
                    left,
                    right,
                    kind,
                    SEPARATOR,
                    width,
                    current_band,
                ));
            }
        }

        if !line.comments.is_empty() {
            for comment in &line.comments {
                push_diff_comment_callout(
                    &mut lines,
                    &format!("f{file_index}-dc-{i}-{}", comment.id),
                    comment,
                    width,
                    CALLOUT_PAD,
                    Some(current_band),
                );
            }
        }
    }
    lines
}

fn side_by_side_column_header(
    file_index: usize,
    _width: usize,
    half_left: usize,
    half_right: usize,
    separator: &str,
) -> ContentLine {
    let left = truncate_to_width(
        &format!("{:<width$}", "− removed", width = half_left),
        half_left,
    );
    let right = truncate_to_width(
        &format!("{:<width$}", "+ added", width = half_right),
        half_right,
    );
    let sep_style = Style::default().fg(Color::DarkGray);
    let dim = Style::default().add_modifier(Modifier::DIM);
    ContentLine {
        key: format!("f{file_index}-cols"),
        kind: LineKind::Meta,
        spans: vec![
            (left, dim),
            (separator.to_string(), sep_style),
            (right, dim),
        ],
        accent: None,
        hunk_band: None,
    }
}

fn side_by_side_line(
    key: String,
    left: String,
    right: String,
    kind: LineKind,
    separator: &str,
    width: usize,
    hunk_band: u8,
) -> ContentLine {
    let line_style = line_style_for_kind(kind, None);
    let sep_style = Style::default().fg(Color::DarkGray);
    let text_width = display_width(&left) + display_width(separator) + display_width(&right);
    let mut spans = vec![
        (left, line_style),
        (separator.to_string(), sep_style),
        (right, line_style),
    ];
    if text_width < width {
        spans.push((" ".repeat(width - text_width), Style::default()));
    }
    ContentLine {
        key,
        kind,
        spans,
        accent: None,
        hunk_band: Some(hunk_band),
    }
}

fn side_by_side_halves_typed(
    line: &crate::api::types::DiffLine,
    half_left: usize,
    half_right: usize,
    content_width: usize,
) -> (String, String, LineKind) {
    match line.kind {
        DiffLineKind::Removed => {
            let left = format_side_half(line.old_line_no, "-", &line.content, content_width, half_left);
            (left, blank_half(half_right), LineKind::DiffRemoved)
        }
        DiffLineKind::Added => {
            let right = format_side_half(line.new_line_no, "+", &line.content, content_width, half_right);
            (blank_half(half_left), right, LineKind::DiffAdded)
        }
        DiffLineKind::Context => {
            let left = format_side_half(line.old_line_no, " ", &line.content, content_width, half_left);
            let right = format_side_half(line.new_line_no, " ", &line.content, content_width, half_right);
            (left, right, LineKind::DiffContext)
        }
        DiffLineKind::HunkHeader => unreachable!(),
    }
}

fn side_by_side_halves(
    line: &crate::api::types::DiffLine,
    half_left: usize,
    half_right: usize,
    content_width: usize,
) -> (String, String) {
    let (left, right, _) = side_by_side_halves_typed(line, half_left, half_right, content_width);
    (left, right)
}

fn blank_half(half_width: usize) -> String {
    truncate_to_width(&" ".repeat(half_width.saturating_mul(2)), half_width)
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

/// Key of the hunk header at or above `scroll` in the combined view.
pub fn hunk_key_at_scroll(
    lines: &[ContentLine],
    hunk_lines: &[usize],
    scroll: usize,
) -> Option<String> {
    for &idx in hunk_lines.iter().rev() {
        if idx <= scroll {
            return Some(lines[idx].key.clone());
        }
    }
    None
}
/// Scroll offset for the next hunk strictly after `scroll`.
pub fn next_hunk_scroll(hunk_lines: &[usize], scroll: usize) -> Option<usize> {
    hunk_lines.iter().copied().find(|&line| line > scroll)
}

/// Scroll offset for the previous hunk strictly before `scroll`.
pub fn prev_hunk_scroll(hunk_lines: &[usize], scroll: usize) -> Option<usize> {
    hunk_lines.iter().rev().copied().find(|&line| line < scroll)
}

/// Scroll offset for the next inline comment strictly after `scroll`.
pub fn next_comment_scroll(comment_lines: &[usize], scroll: usize) -> Option<usize> {
    comment_lines.iter().copied().find(|&line| line > scroll)
}

/// Scroll offset for the previous inline comment strictly before `scroll`.
pub fn prev_comment_scroll(comment_lines: &[usize], scroll: usize) -> Option<usize> {
    comment_lines
        .iter()
        .rev()
        .copied()
        .find(|&line| line < scroll)
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

    fn all_indices(files: &[PrFileChange]) -> Vec<usize> {
        (0..files.len()).collect()
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
        let view = build_combined_file_diff_view(
            &files,
            &all_indices(&files),
            &diffs,
            "pr",
            DiffViewMode::Inline,
            80,
            &HashSet::new(),
        );
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
        let view = build_combined_file_diff_view(
            &files,
            &all_indices(&files),
            &diffs,
            "pr",
            DiffViewMode::Inline,
            80,
            &HashSet::new(),
        );
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

    #[test]
    fn comment_scroll_navigation() {
        let comments = vec![8, 25, 50];
        assert_eq!(next_comment_scroll(&comments, 0), Some(8));
        assert_eq!(next_comment_scroll(&comments, 8), Some(25));
        assert_eq!(next_comment_scroll(&comments, 9), Some(25));
        assert_eq!(next_comment_scroll(&comments, 50), None);
        assert_eq!(prev_comment_scroll(&comments, 50), Some(25));
        assert_eq!(prev_comment_scroll(&comments, 25), Some(8));
        assert_eq!(prev_comment_scroll(&comments, 8), None);
    }

    #[test]
    fn combined_view_collects_comment_line_indices() {
        use crate::api::types::{PrComment, PrThread};
        use crate::diffing::{attach_comments, build_file_diff};

        let files = vec![file("a.rs")];
        let mut diffs = HashMap::new();
        let mut diff = build_file_diff(
            file("a.rs"),
            "line1\nline2\nline3\n",
            "line1\nchanged\nline3\n",
        );
        attach_comments(
            &mut diff,
            &[PrThread {
                id: 1,
                status: "active".into(),
                file_path: Some("a.rs".into()),
                line: Some(1),
                line_end: None,
                comments: vec![PrComment {
                    id: 99,
                    author: "alice".into(),
                    content: "fix this".into(),
                    published_at: "2024-01-01T00:00:00Z".into(),
                }],
            }],
        );
        diffs.insert(
            "pr:a.rs".into(),
            FileDiffState {
                diff: Some(diff),
                loading: false,
                error: None,
            },
        );
        let view = build_combined_file_diff_view(
            &files,
            &all_indices(&files),
            &diffs,
            "pr",
            DiffViewMode::Inline,
            80,
            &HashSet::new(),
        );
        assert_eq!(view.comment_lines.len(), 1);
        assert_eq!(
            view.lines[view.comment_lines[0]].kind,
            LineKind::DiffCommentConnector
        );
    }

    #[test]
    fn collapsed_hunk_hides_context_lines() {
        let files = vec![file("a.rs")];
        let mut diffs = HashMap::new();
        let diff = build_file_diff(
            file("a.rs"),
            "line1\nline2\nline3\nline4\nline5\n",
            "line1\nchanged\nline3\nline4\nline5\n",
        );
        diffs.insert(
            "pr:a.rs".into(),
            FileDiffState {
                diff: Some(diff),
                loading: false,
                error: None,
            },
        );
        let expanded = build_combined_file_diff_view(
            &files,
            &all_indices(&files),
            &diffs,
            "pr",
            DiffViewMode::Inline,
            80,
            &HashSet::new(),
        );
        let context_count = expanded
            .lines
            .iter()
            .filter(|l| l.kind == LineKind::DiffContext)
            .count();
        assert!(context_count > 0);

        let hunk_key = expanded.lines[expanded.hunk_lines[0]].key.clone();
        let collapsed = build_combined_file_diff_view(
            &files,
            &all_indices(&files),
            &diffs,
            "pr",
            DiffViewMode::Inline,
            80,
            &HashSet::from([hunk_key]),
        );
        assert_eq!(
            collapsed
                .lines
                .iter()
                .filter(|l| l.kind == LineKind::DiffContext)
                .count(),
            0
        );
        assert!(collapsed.lines[expanded.hunk_lines[0]].plain_text().contains('▸'));
    }

    #[test]
    fn hunk_key_at_scroll_returns_active_hunk() {
        let mut lines: Vec<ContentLine> = (0..30)
            .map(|i| ContentLine::plain(format!("l-{i}"), LineKind::DiffContext, "x"))
            .collect();
        lines[5] = ContentLine::plain("hunk-a", LineKind::DiffHunkHeader, "@@");
        lines[20] = ContentLine::plain("hunk-b", LineKind::DiffHunkHeader, "@@");
        let hunks = vec![5, 20];
        assert_eq!(
            hunk_key_at_scroll(&lines, &hunks, 10),
            Some("hunk-a".to_string())
        );
        assert_eq!(
            hunk_key_at_scroll(&lines, &hunks, 20),
            Some("hunk-b".to_string())
        );
    }

    #[test]
    fn alternating_hunk_bands_within_file() {
        let files = vec![file("a.rs")];
        let mut diffs = HashMap::new();
        let mut old_lines: Vec<String> = (0..40).map(|i| format!("line{i}")).collect();
        let mut new_lines = old_lines.clone();
        old_lines[2] = "changed-old".to_string();
        new_lines[2] = "changed-new".to_string();
        old_lines[30] = "changed-old-2".to_string();
        new_lines[30] = "changed-new-2".to_string();
        let old = format!("{}\n", old_lines.join("\n"));
        let new = format!("{}\n", new_lines.join("\n"));
        let diff = build_file_diff(file("a.rs"), &old, &new);
        diffs.insert(
            "pr:a.rs".into(),
            FileDiffState {
                diff: Some(diff),
                loading: false,
                error: None,
            },
        );
        let view = build_combined_file_diff_view(
            &files,
            &all_indices(&files),
            &diffs,
            "pr",
            DiffViewMode::Inline,
            80,
            &HashSet::new(),
        );
        assert!(view.hunk_lines.len() >= 2);
        let bands: Vec<u8> = view
            .lines
            .iter()
            .filter_map(|l| l.hunk_band)
            .collect();
        assert!(bands.contains(&0) && bands.contains(&1));
    }

    #[test]
    fn file_separator_between_files() {
        let files = vec![file("a.rs"), file("b.rs")];
        let mut diffs = HashMap::new();
        for path in ["a.rs", "b.rs"] {
            diffs.insert(
                format!("pr:{path}").into(),
                FileDiffState {
                    diff: Some(build_file_diff(file(path), "a\n", "b\n")),
                    loading: false,
                    error: None,
                },
            );
        }
        let view = build_combined_file_diff_view(
            &files,
            &all_indices(&files),
            &diffs,
            "pr",
            DiffViewMode::Inline,
            80,
            &HashSet::new(),
        );
        assert!(
            view.lines
                .iter()
                .any(|l| l.kind == LineKind::FileSeparator)
        );
    }
}
