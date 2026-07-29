use crate::api::types::{
    AdoRepository, CommentActivityFilter, DraftFilter, PullRequestFilters, PullRequestStatus,
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

pub const SECTIONS: [FilterSection; 5] = [
    FilterSection::Status,
    FilterSection::Repo,
    FilterSection::Draft,
    FilterSection::Comments,
    FilterSection::Toggles,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterSection {
    Status,
    Repo,
    Draft,
    Comments,
    Toggles,
}

#[derive(Debug, Clone)]
pub struct FilterModalState {
    pub status_index: usize,
    pub repo_index: usize,
    pub draft_index: usize,
    pub comment_index: usize,
    pub reviewer_me: bool,
    pub author_me: bool,
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

    FilterModalState {
        status_index,
        repo_index,
        draft_index,
        comment_index,
        reviewer_me: filters.reviewer_me.unwrap_or(false),
        author_me: filters.author_me.unwrap_or(false),
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
    PullRequestFilters {
        status: STATUS_OPTIONS[state.status_index.min(STATUS_OPTIONS.len() - 1)],
        repository_name,
        reviewer_me: Some(state.reviewer_me),
        reviewer_id: if state.reviewer_me {
            Some(current_user_id.to_string())
        } else {
            None
        },
        author_me: Some(state.author_me),
        author_id: if state.author_me {
            Some(current_user_id.to_string())
        } else {
            None
        },
        draft_filter: Some(DRAFT_OPTIONS[state.draft_index].0),
        active_comments_filter: Some(COMMENT_OPTIONS[state.comment_index].0),
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
    if filters.author_me.unwrap_or(false) {
        labels.push("author:me".to_string());
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
            FilterSection::Toggles => {
                self.toggle_index = (self.toggle_index + 1).min(1);
            }
        }
    }

    pub fn move_up(&mut self) {
        match self.section {
            FilterSection::Status => self.status_index = self.status_index.saturating_sub(1),
            FilterSection::Repo => self.repo_index = self.repo_index.saturating_sub(1),
            FilterSection::Draft => self.draft_index = self.draft_index.saturating_sub(1),
            FilterSection::Comments => self.comment_index = self.comment_index.saturating_sub(1),
            FilterSection::Toggles => self.toggle_index = self.toggle_index.saturating_sub(1),
        }
    }

    pub fn toggle_space(&mut self) {
        if self.section == FilterSection::Toggles {
            if self.toggle_index == 0 {
                self.reviewer_me = !self.reviewer_me;
            } else {
                self.author_me = !self.author_me;
            }
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
