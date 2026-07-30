use crate::api::types::{
    AdoRepository, ApprovedByFilter, AuthorFilter, CommentActivityFilter, DraftFilter,
    PullRequestFilters, PullRequestStatus, ReviewStatusFilter,
};

pub const STATUS_OPTIONS: [PullRequestStatus; 4] = [
    PullRequestStatus::Active,
    PullRequestStatus::Completed,
    PullRequestStatus::Abandoned,
    PullRequestStatus::All,
];

pub const DRAFT_OPTIONS: [(DraftFilter, &str); 3] = [
    (DraftFilter::All, "All PRs"),
    (DraftFilter::Hide, "Hide drafts"),
    (DraftFilter::Only, "Only drafts"),
];

pub const COMMENT_OPTIONS: [(CommentActivityFilter, &str); 2] = [
    (CommentActivityFilter::All, "All PRs"),
    (CommentActivityFilter::Only, "PRs with active comments"),
];

pub const REVIEW_STATUS_OPTIONS: [ReviewStatusFilter; 5] = [
    ReviewStatusFilter::All,
    ReviewStatusFilter::NeedsReview,
    ReviewStatusFilter::Approved,
    ReviewStatusFilter::Rejected,
    ReviewStatusFilter::WaitingForAuthor,
];

pub const APPROVED_BY_OPTIONS: [ApprovedByFilter; 2] = [
    ApprovedByFilter::All,
    ApprovedByFilter::Me,
];

pub const AUTHOR_OPTIONS: [AuthorFilter; 3] = [
    AuthorFilter::All,
    AuthorFilter::Me,
    AuthorFilter::NotMe,
];

pub const SECTIONS: [FilterSection; 8] = [
    FilterSection::Status,
    FilterSection::Repo,
    FilterSection::Draft,
    FilterSection::Comments,
    FilterSection::Review,
    FilterSection::ApprovedBy,
    FilterSection::Author,
    FilterSection::Toggles,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterSection {
    Status,
    Repo,
    Draft,
    Comments,
    Review,
    ApprovedBy,
    Author,
    Toggles,
}

#[derive(Debug, Clone)]
pub struct FilterModalState {
    pub status_index: usize,
    pub repo_index: usize,
    pub draft_index: usize,
    pub comment_index: usize,
    pub review_status_index: usize,
    pub approved_by_index: usize,
    pub author_filter_index: usize,
    pub reviewer_me: bool,
    pub toggle_index: usize,
    pub section: FilterSection,
}

pub fn filter_modal_state_from_filters(
    filters: &PullRequestFilters,
    repositories: &[AdoRepository],
) -> FilterModalState {
    let status_index = STATUS_OPTIONS
        .iter()
        .position(|s| *s == filters.status)
        .unwrap_or(0);
    let repo_index = match &filters.repository_name {
        None => 0,
        Some(name) => repositories
            .iter()
            .position(|r| &r.name == name)
            .map(|i| i + 1)
            .unwrap_or(0),
    };
    let draft_filter = filters.draft_filter.unwrap_or(DraftFilter::All);
    let draft_index = DRAFT_OPTIONS
        .iter()
        .position(|(v, _)| *v == draft_filter)
        .unwrap_or(0);
    let comment_filter = filters.active_comments_filter.unwrap_or(CommentActivityFilter::All);
    let comment_index = COMMENT_OPTIONS
        .iter()
        .position(|(v, _)| *v == comment_filter)
        .unwrap_or(0);
    let review_status_filter = filters.review_status_filter.unwrap_or(ReviewStatusFilter::All);
    let review_status_index = REVIEW_STATUS_OPTIONS
        .iter()
        .position(|v| *v == review_status_filter)
        .unwrap_or(0);
    let approved_by_filter = filters.approved_by_filter.unwrap_or(ApprovedByFilter::All);
    let approved_by_index = APPROVED_BY_OPTIONS
        .iter()
        .position(|v| *v == approved_by_filter)
        .unwrap_or(0);
    let author_filter = filters.effective_author_filter();
    let author_filter_index = AUTHOR_OPTIONS
        .iter()
        .position(|v| *v == author_filter)
        .unwrap_or(0);

    FilterModalState {
        status_index,
        repo_index,
        draft_index,
        comment_index,
        review_status_index,
        approved_by_index,
        author_filter_index,
        reviewer_me: filters.reviewer_me.unwrap_or(false),
        toggle_index: 0,
        section: FilterSection::Status,
    }
}

pub fn modal_state_to_filters(
    state: &FilterModalState,
    repositories: &[AdoRepository],
    current_user_id: &str,
) -> PullRequestFilters {
    let repository_name = if state.repo_index == 0 {
        None
    } else {
        repositories.get(state.repo_index - 1).map(|r| r.name.clone())
    };
    let author_filter =
        AUTHOR_OPTIONS[state.author_filter_index.min(AUTHOR_OPTIONS.len() - 1)];
    PullRequestFilters {
        status: STATUS_OPTIONS[state.status_index.min(STATUS_OPTIONS.len() - 1)],
        repository_name,
        reviewer_me: Some(state.reviewer_me),
        reviewer_id: if state.reviewer_me {
            Some(current_user_id.to_string())
        } else {
            None
        },
        author_me: None,
        author_id: if author_filter == AuthorFilter::Me {
            Some(current_user_id.to_string())
        } else {
            None
        },
        author_filter: Some(author_filter),
        draft_filter: Some(DRAFT_OPTIONS[state.draft_index].0),
        active_comments_filter: Some(COMMENT_OPTIONS[state.comment_index].0),
        review_status_filter: Some(
            REVIEW_STATUS_OPTIONS[state.review_status_index.min(REVIEW_STATUS_OPTIONS.len() - 1)],
        ),
        approved_by_filter: Some(
            APPROVED_BY_OPTIONS[state.approved_by_index.min(APPROVED_BY_OPTIONS.len() - 1)],
        ),
        ..Default::default()
    }
}

pub fn filters_to_labels(filters: &PullRequestFilters) -> Vec<String> {
    let mut labels = vec![filters.status.to_string()];
    if let Some(repo) = &filters.repository_name {
        labels.push(format!("repo:{repo}"));
    }
    if filters.reviewer_me.unwrap_or(false) {
        labels.push("reviewer:me".to_string());
    }
    match filters.effective_author_filter() {
        AuthorFilter::All => {}
        AuthorFilter::Me => labels.push("author:me".to_string()),
        AuthorFilter::NotMe => labels.push("author:not-me".to_string()),
    }
    if let Some(draft) = filters.draft_filter {
        if draft != DraftFilter::All {
            labels.push(if draft == DraftFilter::Hide {
                "drafts:hidden".to_string()
            } else {
                "drafts:only".to_string()
            });
        }
    }
    if filters.active_comments_filter == Some(CommentActivityFilter::Only) {
        labels.push("comments:active".to_string());
    }
    if let Some(review) = filters.review_status_filter {
        if review != ReviewStatusFilter::All {
            labels.push(format!("review:{}", review.filter_label()));
        }
    }
    if let Some(approved_by) = filters.approved_by_filter {
        if approved_by != ApprovedByFilter::All {
            labels.push(format!("approved-by:{}", approved_by.filter_label()));
        }
    }
    if let Some(search) = &filters.search {
        labels.push(format!("search:{search}"));
    }
    labels
}

impl FilterModalState {
    pub fn move_left(&mut self) {
        let idx = SECTIONS.iter().position(|s| *s == self.section).unwrap_or(0);
        self.section = SECTIONS[idx.saturating_sub(1)];
    }

    pub fn move_right(&mut self) {
        let idx = SECTIONS.iter().position(|s| *s == self.section).unwrap_or(0);
        self.section = SECTIONS[(idx + 1).min(SECTIONS.len() - 1)];
    }

    pub fn move_down(&mut self, repo_count: usize) {
        match self.section {
            FilterSection::Status => {
                self.status_index = (self.status_index + 1).min(STATUS_OPTIONS.len() - 1);
            }
            FilterSection::Repo => {
                self.repo_index = (self.repo_index + 1).min(repo_count);
            }
            FilterSection::Draft => {
                self.draft_index = (self.draft_index + 1).min(DRAFT_OPTIONS.len() - 1);
            }
            FilterSection::Comments => {
                self.comment_index = (self.comment_index + 1).min(COMMENT_OPTIONS.len() - 1);
            }
            FilterSection::Review => {
                self.review_status_index =
                    (self.review_status_index + 1).min(REVIEW_STATUS_OPTIONS.len() - 1);
            }
            FilterSection::ApprovedBy => {
                self.approved_by_index =
                    (self.approved_by_index + 1).min(APPROVED_BY_OPTIONS.len() - 1);
            }
            FilterSection::Author => {
                self.author_filter_index =
                    (self.author_filter_index + 1).min(AUTHOR_OPTIONS.len() - 1);
            }
            FilterSection::Toggles => {
                self.toggle_index = 0;
            }
        }
    }

    pub fn move_up(&mut self) {
        match self.section {
            FilterSection::Status => self.status_index = self.status_index.saturating_sub(1),
            FilterSection::Repo => self.repo_index = self.repo_index.saturating_sub(1),
            FilterSection::Draft => self.draft_index = self.draft_index.saturating_sub(1),
            FilterSection::Comments => self.comment_index = self.comment_index.saturating_sub(1),
            FilterSection::Review => {
                self.review_status_index = self.review_status_index.saturating_sub(1)
            }
            FilterSection::ApprovedBy => {
                self.approved_by_index = self.approved_by_index.saturating_sub(1)
            }
            FilterSection::Author => {
                self.author_filter_index = self.author_filter_index.saturating_sub(1)
            }
            FilterSection::Toggles => {}
        }
    }

    pub fn toggle_space(&mut self) {
        if self.section == FilterSection::Toggles {
            self.reviewer_me = !self.reviewer_me;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_to_labels_includes_active_extras() {
        let filters = PullRequestFilters {
            status: PullRequestStatus::Active,
            reviewer_me: Some(true),
            draft_filter: Some(DraftFilter::Hide),
            ..Default::default()
        };
        let labels = filters_to_labels(&filters);
        assert_eq!(labels, vec!["active", "reviewer:me", "drafts:hidden"]);
    }

    #[test]
    fn filters_to_labels_includes_review_filters() {
        let filters = PullRequestFilters {
            review_status_filter: Some(ReviewStatusFilter::Approved),
            approved_by_filter: Some(ApprovedByFilter::Me),
            ..Default::default()
        };
        let labels = filters_to_labels(&filters);
        assert_eq!(labels, vec!["active", "review:approved", "approved-by:me"]);
    }

    #[test]
    fn filters_to_labels_includes_author_not_me() {
        let filters = PullRequestFilters {
            author_filter: Some(AuthorFilter::NotMe),
            ..Default::default()
        };
        let labels = filters_to_labels(&filters);
        assert!(labels.contains(&"author:not-me".to_string()));
    }

    #[test]
    fn modal_roundtrip_preserves_status() {
        let filters = PullRequestFilters {
            status: PullRequestStatus::Completed,
            ..Default::default()
        };
        let state = filter_modal_state_from_filters(&filters, &[]);
        let rebuilt = modal_state_to_filters(&state, &[], "me");
        assert_eq!(rebuilt.status, PullRequestStatus::Completed);
    }
}
