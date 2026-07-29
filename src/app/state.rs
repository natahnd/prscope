use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

use crate::api::types::{
    AppScreen, CommentActivityFilter, NormalizedPullRequest, Project, PullRequestFilters,
    Repository,
};
use crate::providers::ProviderClient;

use super::comment_activity::CommentActivity;
use super::detail_cache::{DetailCache, PullRequestDetailState};
use super::filter_modal::FilterModalState;
use super::layout::{compute_layout_geometry, LayoutGeometry, LayoutOptions, DEFAULT_LEFT_WIDTH_RATIO};
use super::pr_key::pr_key_for;
use super::text_selection::SelectionRange;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelFocus {
    List,
    Detail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RightPanelMode {
    Description,
    Comments,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentStatusFilter {
    All,
    Active,
    Resolved,
}

impl CommentStatusFilter {
    pub fn next(self) -> Self {
        match self {
            CommentStatusFilter::All => CommentStatusFilter::Active,
            CommentStatusFilter::Active => CommentStatusFilter::Resolved,
            CommentStatusFilter::Resolved => CommentStatusFilter::All,
        }
    }
}

/// All background-task completions and terminal events the main loop reacts
/// to. Mirrors the many `useEffect`/callback closures in `App.tsx`, just
/// funneled through one channel instead of React state updates.
pub enum AppEvent {
    ProjectsLoaded {
        last_project: Option<String>,
        display_context: String,
        projects: Vec<Project>,
    },
    ProjectsLoadFailed(String),

    PrsLoaded {
        repos: Vec<Repository>,
        prs: Vec<NormalizedPullRequest>,
    },
    PrsLoadFailed(String),

    AdoOrgDetected(Option<String>),
    AdoSubmitSucceeded {
        client: Arc<dyn ProviderClient>,
        user_id: String,
        organization: String,
    },
    AdoSubmitFailed(String),

    DetailLoaded {
        key: String,
        state: PullRequestDetailState,
    },
    CommentActivityLoaded {
        key: String,
        activity: CommentActivity,
    },
}

pub struct App {
    pub should_quit: bool,
    pub bootstrapped: bool,

    pub client: Option<Arc<dyn ProviderClient>>,
    pub current_user_id: Option<String>,
    pub screen: AppScreen,

    // Provider setup (Azure DevOps org entry) screen state.
    pub setup_loading: bool,
    pub setup_detecting: bool,
    pub setup_error: Option<String>,
    pub setup_value: String,
    pub setup_detected_from_az: bool,
    pub spinner_frame: usize,
    pub provider_picker_index: usize,

    pub last_project: Option<String>,
    pub selected_project: Option<String>,
    pub display_context: String,
    pub initial_project: Option<String>,

    pub projects: Vec<Project>,
    pub projects_loading: bool,
    pub projects_error: Option<String>,
    pub project_search: String,
    pub project_index: usize,

    pub repositories: Vec<Repository>,
    pub pull_requests: Vec<NormalizedPullRequest>,
    pub prs_loading: bool,
    pub prs_error: Option<String>,
    pub refreshed_at: Option<Instant>,
    pub selected_index: usize,
    pub filters: PullRequestFilters,
    pub search_mode: bool,
    pub search_query: String,
    pub filter_modal_open: bool,
    pub filter_modal: Option<FilterModalState>,
    pub filters_hydrated: bool,

    pub panel_focus: PanelFocus,
    pub right_mode: RightPanelMode,
    pub comment_status_filter: CommentStatusFilter,
    pub description_scroll: usize,
    pub comments_scroll: usize,
    pub right_max_scroll: usize,

    pub left_width_ratio: f64,
    pub dragging_divider: bool,
    pub right_selection: Option<SelectionRange>,
    pub dragging_selection: bool,

    pub terminal_columns: u16,
    pub terminal_rows: u16,

    pub detail_cache: DetailCache,
    pub detail_state: PullRequestDetailState,
    pub current_detail_key: Option<String>,

    pub comment_activity: std::collections::HashMap<String, CommentActivity>,
    pub comment_activity_requested: HashSet<String>,
}

impl App {
    pub fn new(initial_project: Option<String>, initial_filters: PullRequestFilters) -> Self {
        App {
            should_quit: false,
            bootstrapped: false,

            client: None,
            current_user_id: None,
            screen: AppScreen::ProviderSelect,

            setup_loading: false,
            setup_detecting: true,
            setup_error: None,
            setup_value: String::new(),
            setup_detected_from_az: false,
            spinner_frame: 0,
            provider_picker_index: 0,

            last_project: None,
            selected_project: initial_project.clone(),
            display_context: String::new(),
            initial_project,

            projects: vec![],
            projects_loading: false,
            projects_error: None,
            project_search: String::new(),
            project_index: 0,

            repositories: vec![],
            pull_requests: vec![],
            prs_loading: false,
            prs_error: None,
            refreshed_at: None,
            selected_index: 0,
            filters: initial_filters,
            search_mode: false,
            search_query: String::new(),
            filter_modal_open: false,
            filter_modal: None,
            filters_hydrated: false,

            panel_focus: PanelFocus::List,
            right_mode: RightPanelMode::Description,
            comment_status_filter: CommentStatusFilter::All,
            description_scroll: 0,
            comments_scroll: 0,
            right_max_scroll: 0,

            left_width_ratio: DEFAULT_LEFT_WIDTH_RATIO,
            dragging_divider: false,
            right_selection: None,
            dragging_selection: false,

            terminal_columns: 80,
            terminal_rows: 24,

            detail_cache: DetailCache::default(),
            detail_state: PullRequestDetailState::default(),
            current_detail_key: None,

            comment_activity: std::collections::HashMap::new(),
            comment_activity_requested: HashSet::new(),
        }
    }

    pub fn geometry(&self) -> LayoutGeometry {
        compute_layout_geometry(
            self.terminal_columns,
            self.terminal_rows,
            LayoutOptions {
                has_error: self.prs_error.is_some(),
                left_width_ratio: self.left_width_ratio,
            },
        )
    }

    /// Filtered + sorted PR list, mirroring `displayedPrs` in App.tsx. Grouped
    /// by repo in the list panel, so this must stay sorted the same way.
    pub fn displayed_prs(&self) -> Vec<&NormalizedPullRequest> {
        let query = self
            .filters
            .search
            .clone()
            .unwrap_or_else(|| self.search_query.clone())
            .to_lowercase();

        let mut filtered: Vec<&NormalizedPullRequest> = if query.is_empty() {
            self.pull_requests.iter().collect()
        } else {
            self.pull_requests
                .iter()
                .filter(|pr| {
                    let haystack = format!("{} {} {}", pr.repo_name, pr.title, pr.author).to_lowercase();
                    haystack.contains(&query)
                })
                .collect()
        };

        if self.filters.active_comments_filter == Some(CommentActivityFilter::Only) {
            filtered.retain(|pr| match self.comment_activity.get(&pr_key_for(pr)) {
                None => true,
                Some(CommentActivity::Error) => true,
                Some(CommentActivity::Active) => true,
                Some(CommentActivity::Resolved) => false,
            });
        }

        filtered.sort_by_key(|pr| pr.repo_name.to_lowercase());
        filtered
    }

    pub fn safe_selected_index(&self, displayed_len: usize) -> usize {
        if displayed_len == 0 {
            0
        } else {
            self.selected_index.min(displayed_len - 1)
        }
    }

    /// Mirrors `sortProjects`/`filtered` in ProjectPicker.tsx: last-used
    /// project pinned first, then alphabetical, then narrowed by search.
    pub fn filtered_projects(&self) -> Vec<&Project> {
        let mut sorted: Vec<&Project> = self.projects.iter().collect();
        if let Some(last) = &self.last_project {
            sorted.sort_by(|a, b| {
                if &a.name == last {
                    std::cmp::Ordering::Less
                } else if &b.name == last {
                    std::cmp::Ordering::Greater
                } else {
                    a.name.to_lowercase().cmp(&b.name.to_lowercase())
                }
            });
        }
        if self.project_search.trim().is_empty() {
            return sorted;
        }
        let q = self.project_search.to_lowercase();
        sorted
            .into_iter()
            .filter(|p| p.name.to_lowercase().contains(&q))
            .collect()
    }

    pub fn filter_labels(&self) -> Vec<String> {
        let mut filters = self.filters.clone();
        if filters.search.is_none() && !self.search_query.is_empty() {
            filters.search = Some(self.search_query.clone());
        }
        super::filter_modal::filters_to_labels(&filters)
    }
}
