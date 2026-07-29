//! Projection of pull requests into grouped, scrollable visual rows.

use crate::api::types::{NormalizedPullRequest, PrListLayout};

/// One renderable row in the grouped pull request list.
#[derive(Debug, Clone)]
pub enum VisualRow {
    Header { key: String, label: String },
    Spacer { key: String },
    Title { key: String, pr_index: usize },
    Meta { key: String, pr_index: usize },
}

/// Rows above the scrollable list within the panel: border-top + "Pull
/// Requests (N)" line + "Filters: ..." line (+1 more while the inline search
/// box is showing). Shared by rendering and mouse hit-testing so they can
/// never drift out of sync.
pub const LIST_CHROME_ROWS: usize = 3;

/// Returns non-content rows above the scrollable list.
///
/// # Arguments
///
/// * `search_mode` - Whether the inline search box is currently showing.
///
/// # Returns
///
/// [`LIST_CHROME_ROWS`], plus one extra row when `search_mode` is `true`.
pub fn list_chrome_rows(search_mode: bool) -> usize {
    LIST_CHROME_ROWS + if search_mode { 1 } else { 0 }
}

fn group_key(pr: &NormalizedPullRequest, layout: PrListLayout) -> &str {
    match layout {
        PrListLayout::GroupByRepo => pr.repo_name.as_str(),
        PrListLayout::GroupByAuthor => pr.author.as_str(),
        PrListLayout::SortByAgeDesc | PrListLayout::SortByAgeAsc => "",
    }
}

/// Builds group headers, PR rows, and spacers for the selected list layout.
///
/// # Arguments
///
/// * `pull_requests` - Already-filtered and ordered pull requests to lay out.
/// * `layout` - Grouping/sort mode; determines whether group headers are
///   inserted (see [`PrListLayout::groups`]) and what key groups by.
///
/// # Returns
///
/// A flat `Vec<VisualRow>` alternating headers/spacers with each PR's title
/// and meta rows, in the order they should render top to bottom. Returns an
/// empty `Vec` if `pull_requests` is empty.
pub fn build_rows(pull_requests: &[&NormalizedPullRequest], layout: PrListLayout) -> Vec<VisualRow> {
    let mut rows: Vec<VisualRow> = Vec::new();
    let mut last_group: Option<&str> = None;

    for (index, pr) in pull_requests.iter().enumerate() {
        if layout.groups() {
            let current = group_key(pr, layout);
            if Some(current) != last_group {
                if !rows.is_empty() {
                    rows.push(VisualRow::Spacer {
                        key: format!("sp-{}-{}", pr.repository_id, pr.id),
                    });
                }
                rows.push(VisualRow::Header {
                    key: format!("h-{}-{}", pr.repository_id, current),
                    label: current.to_string(),
                });
                last_group = Some(current);
            } else {
                rows.push(VisualRow::Spacer {
                    key: format!("sp-{}-{}", pr.repository_id, pr.id),
                });
            }
        } else if index > 0 {
            rows.push(VisualRow::Spacer {
                key: format!("sp-{}-{}", pr.repository_id, pr.id),
            });
        }

        rows.push(VisualRow::Title {
            key: format!("t-{}-{}", pr.repository_id, pr.id),
            pr_index: index,
        });
        rows.push(VisualRow::Meta {
            key: format!("m-{}-{}", pr.repository_id, pr.id),
            pr_index: index,
        });
    }

    rows
}

/// Computes a scroll offset that keeps the selected visual row visible.
///
/// # Arguments
///
/// * `selected_row_index` - Index of the visual row that must stay visible.
/// * `total` - Total number of visual rows.
/// * `visible_rows` - Number of rows the viewport can show at once.
///
/// # Returns
///
/// `0` if `total` fits entirely within `visible_rows`. Otherwise, a scroll
/// offset centering `selected_row_index` in the viewport, clamped so the
/// viewport never scrolls past the first or last row.
pub fn compute_scroll_start(selected_row_index: usize, total: usize, visible_rows: usize) -> usize {
    if total <= visible_rows {
        return 0;
    }
    let start = selected_row_index as i64 - (visible_rows / 2) as i64;
    start.max(0).min((total - visible_rows) as i64) as usize
}

/// Complete and viewport-limited visual rows for one render pass.
pub struct VisiblePrRows {
    pub rows: Vec<VisualRow>,
    pub visible: Vec<VisualRow>,
    pub start: usize,
    #[allow(dead_code)]
    pub available_rows: usize,
}

/// Builds rows and extracts the viewport containing the selected PR.
///
/// # Arguments
///
/// * `pull_requests` - Already-filtered and ordered pull requests to lay out.
/// * `selected_index` - Index into `pull_requests` of the currently
///   selected PR.
/// * `panel_height` - Total panel height in terminal rows, including chrome.
/// * `search_mode` - Whether the inline search box is currently showing.
/// * `layout` - Grouping/sort mode to build rows with.
///
/// # Returns
///
/// A `VisiblePrRows` containing the full row list, the viewport slice
/// (`visible`), the viewport's start index into `rows`, and the number of
/// rows available for content (at least `1`, even on a very small panel).
pub fn get_visible_pr_rows(
    pull_requests: &[&NormalizedPullRequest],
    selected_index: usize,
    panel_height: usize,
    search_mode: bool,
    layout: PrListLayout,
) -> VisiblePrRows {
    let rows = build_rows(pull_requests, layout);
    let available_rows =
        (panel_height as i64 - (list_chrome_rows(search_mode) as i64 + 1)).max(1) as usize;
    let selected_row_index = rows
        .iter()
        .position(|row| matches!(row, VisualRow::Title { pr_index, .. } if *pr_index == selected_index))
        .unwrap_or(0);
    let start = compute_scroll_start(selected_row_index, rows.len(), available_rows);
    let visible = rows[start..(start + available_rows).min(rows.len())].to_vec();
    VisiblePrRows {
        rows,
        visible,
        start,
        available_rows,
    }
}

/// Resolves a mouse click at `local_row` (0 = the panel's own top border row)
/// to the pull request index it landed on, or `None` if it hit chrome/spacer.
///
/// # Arguments
///
/// * `pull_requests` - Already-filtered and ordered pull requests backing
///   the visible list.
/// * `selected_index` - Index into `pull_requests` of the currently
///   selected PR, used to compute the current viewport.
/// * `panel_height` - Total panel height in terminal rows, including chrome.
/// * `local_row` - Row clicked, relative to the panel's own top border.
/// * `search_mode` - Whether the inline search box is currently showing.
/// * `layout` - Grouping/sort mode currently in effect.
///
/// # Returns
///
/// `Some(index)` into `pull_requests` if `local_row` lands on a PR's title
/// or meta row; `None` if it lands above the content area, on a group
/// header/spacer, or past the end of the list.
pub fn resolve_list_click_row(
    pull_requests: &[&NormalizedPullRequest],
    selected_index: usize,
    panel_height: usize,
    local_row: i64,
    search_mode: bool,
    layout: PrListLayout,
) -> Option<usize> {
    let result = get_visible_pr_rows(pull_requests, selected_index, panel_height, search_mode, layout);
    let content_row = local_row - list_chrome_rows(search_mode) as i64;
    if content_row < 0 {
        return None;
    }
    let row = result.rows.get(result.start + content_row as usize)?;
    match row {
        VisualRow::Title { pr_index, .. } | VisualRow::Meta { pr_index, .. } => Some(*pr_index),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pr(id: i64, repo: &str, author: &str, created_at: &str) -> NormalizedPullRequest {
        NormalizedPullRequest {
            id,
            title: format!("PR {id}"),
            status: "active".into(),
            is_draft: false,
            repo_name: repo.into(),
            repository_id: format!("repo-{repo}"),
            author: author.into(),
            reviewers: vec![],
            target_branch: "main".into(),
            created_at: created_at.into(),
            url: "https://example.com".into(),
            merge_status: "none".into(),
            needs_my_review: false,
        }
    }

    #[test]
    fn groups_rows_by_repo_with_spacers_and_header() {
        let prs = vec![
            pr(1, "alpha", "Alice", "2024-01-03T00:00:00Z"),
            pr(2, "alpha", "Bob", "2024-01-02T00:00:00Z"),
            pr(3, "beta", "Alice", "2024-01-01T00:00:00Z"),
        ];
        let refs: Vec<&NormalizedPullRequest> = prs.iter().collect();
        let rows = build_rows(&refs, PrListLayout::GroupByRepo);
        let kinds: Vec<&str> = rows
            .iter()
            .map(|r| match r {
                VisualRow::Header { .. } => "header",
                VisualRow::Spacer { .. } => "spacer",
                VisualRow::Title { .. } => "title",
                VisualRow::Meta { .. } => "meta",
            })
            .collect();
        assert_eq!(
            kinds,
            vec![
                "header", "title", "meta", "spacer", "title", "meta", "spacer", "header", "title",
                "meta",
            ]
        );
    }

    #[test]
    fn groups_rows_by_author_with_spacers_and_header() {
        let prs = vec![
            pr(1, "alpha", "Alice", "2024-01-03T00:00:00Z"),
            pr(2, "beta", "Alice", "2024-01-02T00:00:00Z"),
            pr(3, "beta", "Bob", "2024-01-01T00:00:00Z"),
        ];
        let refs: Vec<&NormalizedPullRequest> = prs.iter().collect();
        let rows = build_rows(&refs, PrListLayout::GroupByAuthor);
        let headers: Vec<&str> = rows
            .iter()
            .filter_map(|r| match r {
                VisualRow::Header { label, .. } => Some(label.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(headers, vec!["Alice", "Bob"]);
    }

    #[test]
    fn age_layout_has_no_group_headers() {
        let prs = vec![
            pr(1, "alpha", "Alice", "2024-01-03T00:00:00Z"),
            pr(2, "beta", "Bob", "2024-01-01T00:00:00Z"),
        ];
        let refs: Vec<&NormalizedPullRequest> = prs.iter().collect();
        let rows = build_rows(&refs, PrListLayout::SortByAgeDesc);
        assert!(!rows.iter().any(|r| matches!(r, VisualRow::Header { .. })));
    }

    #[test]
    fn resolve_list_click_row_maps_to_pr_index() {
        let prs = vec![pr(1, "alpha", "Alice", "2024-01-01T00:00:00Z")];
        let refs: Vec<&NormalizedPullRequest> = prs.iter().collect();
        let clicked = resolve_list_click_row(&refs, 0, 20, 4, false, PrListLayout::GroupByRepo);
        assert_eq!(clicked, Some(0));
    }

    #[test]
    fn resolve_list_click_row_above_content_returns_none() {
        let prs = vec![pr(1, "alpha", "Alice", "2024-01-01T00:00:00Z")];
        let refs: Vec<&NormalizedPullRequest> = prs.iter().collect();
        assert_eq!(
            resolve_list_click_row(&refs, 0, 20, 0, false, PrListLayout::GroupByRepo),
            None
        );
    }
}
