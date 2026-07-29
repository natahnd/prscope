use crate::api::types::NormalizedPullRequest;

#[derive(Debug, Clone)]
pub enum VisualRow {
    Header { key: String, repo_name: String },
    Spacer { key: String },
    Title { key: String, pr_index: usize },
    Meta { key: String, pr_index: usize },
}

/// Rows above the scrollable list within the panel: border-top + "Pull
/// Requests (N)" line + "Filters: ..." line (+1 more while the inline search
/// box is showing). Shared by rendering and mouse hit-testing so they can
/// never drift out of sync.
pub const LIST_CHROME_ROWS: usize = 3;

pub fn list_chrome_rows(search_mode: bool) -> usize {
    LIST_CHROME_ROWS + if search_mode { 1 } else { 0 }
}

pub fn build_rows(pull_requests: &[&NormalizedPullRequest]) -> Vec<VisualRow> {
    let mut rows: Vec<VisualRow> = Vec::new();
    let mut last_repo: Option<&str> = None;

    for (index, pr) in pull_requests.iter().enumerate() {
        if Some(pr.repo_name.as_str()) != last_repo {
            if !rows.is_empty() {
                rows.push(VisualRow::Spacer {
                    key: format!("sp-{}-{}", pr.repository_id, pr.id),
                });
            }
            rows.push(VisualRow::Header {
                key: format!("h-{}-{}", pr.repository_id, pr.repo_name),
                repo_name: pr.repo_name.clone(),
            });
            last_repo = Some(pr.repo_name.as_str());
        } else {
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

pub fn compute_scroll_start(selected_row_index: usize, total: usize, visible_rows: usize) -> usize {
    if total <= visible_rows {
        return 0;
    }
    let start = selected_row_index as i64 - (visible_rows / 2) as i64;
    start.max(0).min((total - visible_rows) as i64) as usize
}

pub struct VisiblePrRows {
    pub rows: Vec<VisualRow>,
    pub visible: Vec<VisualRow>,
    pub start: usize,
    #[allow(dead_code)]
    pub available_rows: usize,
}

pub fn get_visible_pr_rows(
    pull_requests: &[&NormalizedPullRequest],
    selected_index: usize,
    panel_height: usize,
    search_mode: bool,
) -> VisiblePrRows {
    let rows = build_rows(pull_requests);
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
pub fn resolve_list_click_row(
    pull_requests: &[&NormalizedPullRequest],
    selected_index: usize,
    panel_height: usize,
    local_row: i64,
    search_mode: bool,
) -> Option<usize> {
    let result = get_visible_pr_rows(pull_requests, selected_index, panel_height, search_mode);
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

    fn pr(id: i64, repo: &str) -> NormalizedPullRequest {
        NormalizedPullRequest {
            id,
            title: format!("PR {id}"),
            status: "active".into(),
            is_draft: false,
            repo_name: repo.into(),
            repository_id: format!("repo-{repo}"),
            author: "Author".into(),
            reviewers: vec![],
            target_branch: "main".into(),
            created_at: "2024-01-01T00:00:00Z".into(),
            url: "https://example.com".into(),
            merge_status: "none".into(),
            needs_my_review: false,
        }
    }

    #[test]
    fn groups_rows_by_repo_with_spacers_and_header() {
        let prs = vec![pr(1, "alpha"), pr(2, "alpha"), pr(3, "beta")];
        let refs: Vec<&NormalizedPullRequest> = prs.iter().collect();
        let rows = build_rows(&refs);
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
                "header", "title", "meta", // alpha #1
                "spacer", "title", "meta", // alpha #2 (same repo -> spacer, no new header)
                "spacer", "header", "title", "meta", // beta #3 (new repo -> spacer + header)
            ]
        );
    }

    #[test]
    fn resolve_list_click_row_maps_to_pr_index() {
        let prs = vec![pr(1, "alpha"), pr(2, "beta")];
        let refs: Vec<&NormalizedPullRequest> = prs.iter().collect();
        // Row 0 border, chrome rows 0..3 non-clickable, row 3 = header, 4 = title(pr0), 5 = meta(pr0)
        let clicked = resolve_list_click_row(&refs, 0, 20, 4, false);
        assert_eq!(clicked, Some(0));
    }

    #[test]
    fn resolve_list_click_row_above_content_returns_none() {
        let prs = vec![pr(1, "alpha")];
        let refs: Vec<&NormalizedPullRequest> = prs.iter().collect();
        assert_eq!(resolve_list_click_row(&refs, 0, 20, 0, false), None);
    }
}
