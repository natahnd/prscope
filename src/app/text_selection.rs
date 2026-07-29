//! Character-based selection and extraction for rendered detail text.

/// Character coordinate in the full, unscrolled detail content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionPoint {
    /// Absolute index into the full (unscrolled) content lines array.
    pub row: usize,
    pub col: usize,
}

/// Drag selection preserving the original anchor and moving focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionRange {
    pub anchor: SelectionPoint,
    pub focus: SelectionPoint,
}

/// Selection reordered into a forward start and end range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NormalizedSelection {
    pub start_row: usize,
    pub start_col: usize,
    pub end_row: usize,
    pub end_col: usize,
}

/// Orders a drag selection regardless of drag direction.
///
/// # Arguments
///
/// * `range` - The raw anchor/focus pair from an in-progress or completed
///   drag; `focus` may be before or after `anchor`.
///
/// # Returns
///
/// A `NormalizedSelection` whose `start_*` fields are always at or before
/// `end_*`, regardless of which direction the user dragged.
pub fn normalize_selection(range: SelectionRange) -> NormalizedSelection {
    let anchor_first = range.anchor.row < range.focus.row
        || (range.anchor.row == range.focus.row && range.anchor.col <= range.focus.col);
    let (start, end) = if anchor_first {
        (range.anchor, range.focus)
    } else {
        (range.focus, range.anchor)
    };
    NormalizedSelection {
        start_row: start.row,
        start_col: start.col,
        end_row: end.row,
        end_col: end.col,
    }
}

/// Resolves the highlight column range for one content row, or `None` if the
/// row isn't part of the selection. Full-width (`usize::MAX`) for rows
/// strictly between the selection's start/end rows, matching how normal
/// multi-line text selection extends across whole intermediate lines.
///
/// # Arguments
///
/// * `row_index` - Index of the content row being rendered.
/// * `selection` - Current normalized selection, if any.
///
/// # Returns
///
/// `None` if `selection` is `None` or `row_index` falls outside its
/// start/end rows. Otherwise `Some((start_col, end_col))`: `end_col` is
/// `usize::MAX` for any row strictly between the selection's start and end
/// rows, meaning "highlight to the end of the line."
pub fn row_highlight_range(
    row_index: usize,
    selection: Option<&NormalizedSelection>,
) -> Option<(usize, usize)> {
    let selection = selection?;
    if row_index < selection.start_row || row_index > selection.end_row {
        return None;
    }
    let start = if row_index == selection.start_row {
        selection.start_col
    } else {
        0
    };
    let end = if row_index == selection.end_row {
        selection.end_col
    } else {
        usize::MAX
    };
    Some((start, end))
}

fn trim_trailing_whitespace(line: &str) -> String {
    line.trim_end_matches([' ', '\t']).to_string()
}

/// Slices selected text out of plain-text content lines (char-indexed, so
/// multi-byte characters are handled correctly), trimming trailing padding
/// per row.
///
/// # Arguments
///
/// * `lines` - Full plain-text content the selection was made against.
/// * `selection` - Normalized start/end row and column of the selection.
///
/// # Returns
///
/// The selected text, joined with `\n` and with trailing whitespace trimmed
/// from each row. Rows referenced by `selection` that don't exist in `lines`
/// are skipped; out-of-range columns are clamped to the row's length.
pub fn extract_selected_text(lines: &[String], selection: &NormalizedSelection) -> String {
    let mut out: Vec<String> = Vec::new();
    for row in selection.start_row..=selection.end_row {
        let Some(line) = lines.get(row) else {
            continue;
        };
        let chars: Vec<char> = line.chars().collect();
        let start = if row == selection.start_row {
            selection.start_col
        } else {
            0
        };
        let end = if row == selection.end_row {
            selection.end_col
        } else {
            chars.len().saturating_sub(1)
        };
        let clamped_start = start.min(chars.len());
        let clamped_end = end.min(chars.len().saturating_sub(1));
        let slice = if chars.is_empty() || clamped_end < clamped_start {
            String::new()
        } else {
            chars[clamped_start..=clamped_end].iter().collect()
        };
        out.push(trim_trailing_whitespace(&slice));
    }
    out.join("\n").trim_end_matches('\n').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_reversed_drag() {
        let range = SelectionRange {
            anchor: SelectionPoint { row: 2, col: 5 },
            focus: SelectionPoint { row: 0, col: 1 },
        };
        let normalized = normalize_selection(range);
        assert_eq!(normalized.start_row, 0);
        assert_eq!(normalized.start_col, 1);
        assert_eq!(normalized.end_row, 2);
        assert_eq!(normalized.end_col, 5);
    }

    #[test]
    fn row_highlight_range_covers_full_width_for_middle_rows() {
        let selection = NormalizedSelection {
            start_row: 0,
            start_col: 3,
            end_row: 2,
            end_col: 1,
        };
        assert_eq!(row_highlight_range(0, Some(&selection)), Some((3, usize::MAX)));
        assert_eq!(row_highlight_range(1, Some(&selection)), Some((0, usize::MAX)));
        assert_eq!(row_highlight_range(2, Some(&selection)), Some((0, 1)));
        assert_eq!(row_highlight_range(3, Some(&selection)), None);
    }

    #[test]
    fn extract_selected_text_joins_full_lines() {
        let lines = vec!["hello".to_string(), "world".to_string(), "third".to_string()];
        let selection = NormalizedSelection {
            start_row: 0,
            start_col: 0,
            end_row: 2,
            end_col: usize::MAX,
        };
        assert_eq!(extract_selected_text(&lines, &selection), "hello\nworld\nthird");
    }

    #[test]
    fn extract_selected_text_trims_trailing_whitespace_per_row() {
        let lines = vec!["hi   ".to_string()];
        let selection = NormalizedSelection {
            start_row: 0,
            start_col: 0,
            end_row: 0,
            end_col: 4,
        };
        assert_eq!(extract_selected_text(&lines, &selection), "hi");
    }
}
