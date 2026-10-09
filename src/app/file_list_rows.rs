use crate::app::pr_list_rows::compute_scroll_start;

pub struct VisibleFileRows {
    pub start: usize,
    pub visible_count: usize,
}

pub fn get_visible_file_rows(
    file_count: usize,
    selected_index: usize,
    available_rows: usize,
) -> VisibleFileRows {
    let visible_count = file_count.min(available_rows.max(1));
    let start = compute_scroll_start(selected_index, file_count, visible_count);
    VisibleFileRows { start, visible_count }
}

/// Resolves a mouse click at `local_row` (0 = the first row inside the file-list
/// card) to the file index it landed on, or `None` if it missed.
pub fn resolve_file_list_click_row(
    file_count: usize,
    selected_index: usize,
    list_scroll: usize,
    local_row: i64,
    available_rows: usize,
) -> Option<usize> {
    if local_row < 0 {
        return None;
    }
    let visible = get_visible_file_rows(file_count, selected_index, available_rows);
    let idx = list_scroll + local_row as usize;
    if idx < file_count && (local_row as usize) < visible.visible_count {
        Some(idx)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::types::{PrFileChange, PrFileChangeType};

    fn files(n: usize) -> Vec<PrFileChange> {
        (0..n)
            .map(|i| PrFileChange {
                path: format!("f{i}.rs"),
                old_path: None,
                change_type: PrFileChangeType::Edit,
            })
            .collect()
    }

    #[test]
    fn click_maps_to_file_index() {
        let files = files(5);
        assert_eq!(resolve_file_list_click_row(files.len(), 0, 0, 0, 10), Some(0));
        assert_eq!(resolve_file_list_click_row(files.len(), 0, 0, 2, 10), Some(2));
        assert_eq!(resolve_file_list_click_row(files.len(), 0, 0, 10, 10), None);
    }

    #[test]
    fn click_respects_list_scroll() {
        let files = files(20);
        assert_eq!(resolve_file_list_click_row(files.len(), 10, 5, 0, 10), Some(5));
        assert_eq!(resolve_file_list_click_row(files.len(), 10, 5, 4, 10), Some(9));
    }
}
