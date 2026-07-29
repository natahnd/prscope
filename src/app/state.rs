//! Central application state and background-task event definitions.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

use crate::api::types::{
    AppScreen, CommentActivityFilter, NormalizedPullRequest, PrListLayout, Project,
    PullRequestFilters, Repository,
};
use crate::humanize::parse_date;
use crate::providers::ProviderClient;

use super::comment_activity::CommentActivity;
use super::detail_cache::{DetailCache, PullRequestDetailState};
use super::filter_modal::FilterModalState;
use super::layout::{compute_layout_geometry, LayoutGeometry, LayoutOptions, DEFAULT_LEFT_WIDTH_RATIO};
use super::pr_key::pr_key_for;
use super::text_selection::SelectionRange;

/// Panel that receives navigation and scrolling input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelFocus {
    List,
    Detail,
}

/// Content displayed in the right-hand detail panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RightPanelMode {
    Description,
    Comments,
}

/// Thread-status subset shown in the comments view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentStatusFilter {
    All,
    Active,
    Resolved,
}

impl CommentStatusFilter {
    /// Advances to the next status filter in the keyboard cycle.
    ///
    /// # Returns
    ///
    /// The `CommentStatusFilter` following `self`: All → Active → Resolved,
    /// wrapping back to All.
    pub fn next(self) -> Self {
        match self {
            CommentStatusFilter::All => CommentStatusFilter::Active,
            CommentStatusFilter::Active => CommentStatusFilter::Resolved,
            CommentStatusFilter::Resolved => CommentStatusFilter::All,
        }
    }
}

/// Results produced by background tasks and consumed by the main event loop.
///
/// A single channel serializes asynchronous state changes with terminal input,
/// keeping all mutations of [`App`] on the UI thread.
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

/// Complete mutable state owned by the terminal event loop.
pub struct App {
    // Lifecycle and active provider session.
    pub should_quit: bool,
    pub bootstrapped: bool,

    pub client: Option<Arc<dyn ProviderClient>>,
    pub current_user_id: Option<String>,
    pub screen: AppScreen,

    // Provider setup and organization entry.
    pub setup_loading: bool,
    pub setup_detecting: bool,
    pub setup_error: Option<String>,
    pub setup_value: String,
    pub setup_detected_from_az: bool,
    pub spinner_frame: usize,
    pub provider_picker_index: usize,

    // Project selection and provider context.
    pub last_project: Option<String>,
    pub selected_project: Option<String>,
    pub display_context: String,
    pub initial_project: Option<String>,

    // Project picker results and input state.
    pub projects: Vec<Project>,
    pub projects_loading: bool,
    pub projects_error: Option<String>,
    pub project_search: String,
    pub project_index: usize,

    // PR list data, filtering, and selection state.
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
    pub list_layout: PrListLayout,

    // Right-panel mode and independent scroll positions.
    pub panel_focus: PanelFocus,
    pub right_mode: RightPanelMode,
    pub comment_status_filter: CommentStatusFilter,
    pub description_scroll: usize,
    pub comments_scroll: usize,
    pub right_max_scroll: usize,

    // Pointer-driven layout resizing and text selection.
    pub left_width_ratio: f64,
    pub dragging_divider: bool,
    pub right_selection: Option<SelectionRange>,
    pub dragging_selection: bool,

    // Last known terminal dimensions, updated on resize events.
    pub terminal_columns: u16,
    pub terminal_rows: u16,

    // Detail data for the currently selected PR and prior selections.
    pub detail_cache: DetailCache,
    pub detail_state: PullRequestDetailState,
    pub current_detail_key: Option<String>,

    // Background unresolved-comment summaries keyed by repository and PR.
    pub comment_activity: std::collections::HashMap<String, CommentActivity>,
    pub comment_activity_requested: HashSet<String>,
}

impl App {
    /// Creates pre-bootstrap state using any CLI-supplied project and filters.
    ///
    /// # Arguments
    ///
    /// * `initial_project` - Project to open directly once bootstrap
    ///   completes, bypassing the project picker.
    /// * `initial_filters` - Filters to seed `self.filters` with; overrides
    ///   persisted filters for this run only.
    ///
    /// # Returns
    ///
    /// A fresh `App` with `screen` set to `ProviderSelect` and
    /// `bootstrapped: false`; `bootstrap` (in `runner.rs`) resolves the real
    /// starting screen and provider client asynchronously.
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
            list_layout: PrListLayout::default(),

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

    /// Computes current panel geometry from terminal size and UI state.
    ///
    /// # Returns
    ///
    /// The `LayoutGeometry` for the current `terminal_columns`/
    /// `terminal_rows`, `left_width_ratio`, and whether a PR-load error line
    /// is currently showing.
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

    /// Filtered + sorted PR list for the list panel. Ordering must match how
    /// `build_rows` groups rows for the active `list_layout`.
    ///
    /// # Returns
    ///
    /// `self.pull_requests`, narrowed by the active search text and
    /// active-comments filter, then sorted according to `self.list_layout`.
    /// Returns an empty `Vec` if no pull requests match.
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

        match self.list_layout {
            PrListLayout::GroupByRepo => {
                filtered.sort_by(|a, b| {
                    a.repo_name
                        .to_lowercase()
                        .cmp(&b.repo_name.to_lowercase())
                        .then_with(|| a.id.cmp(&b.id))
                });
            }
            PrListLayout::GroupByAuthor => {
                filtered.sort_by(|a, b| {
                    a.author
                        .to_lowercase()
                        .cmp(&b.author.to_lowercase())
                        .then_with(|| a.repo_name.to_lowercase().cmp(&b.repo_name.to_lowercase()))
                        .then_with(|| a.id.cmp(&b.id))
                });
            }
            PrListLayout::SortByAgeDesc => {
                filtered.sort_by(|a, b| {
                    parse_date(&b.created_at)
                        .cmp(&parse_date(&a.created_at))
                        .then_with(|| a.id.cmp(&b.id))
                });
            }
            PrListLayout::SortByAgeAsc => {
                filtered.sort_by(|a, b| {
                    parse_date(&a.created_at)
                        .cmp(&parse_date(&b.created_at))
                        .then_with(|| a.id.cmp(&b.id))
                });
            }
        }

        filtered
    }

    /// Clamps the selected PR index to the current filtered result length.
    ///
    /// # Arguments
    ///
    /// * `displayed_len` - Length of the currently filtered/sorted PR list
    ///   (from [`App::displayed_prs`]).
    ///
    /// # Returns
    ///
    /// `0` if `displayed_len` is `0`; otherwise `self.selected_index` capped
    /// at `displayed_len - 1`, so a filter change that shrinks the list never
    /// leaves the selection pointing past its end.
    pub fn safe_selected_index(&self, displayed_len: usize) -> usize {
        if displayed_len == 0 {
            0
        } else {
            self.selected_index.min(displayed_len - 1)
        }
    }

    /// Returns projects with the last-used project pinned first, followed by
    /// alphabetical order, then narrowed by the current search text.
    ///
    /// # Returns
    ///
    /// `self.projects`, reordered so `self.last_project` (if any) sorts
    /// first, then filtered to those whose name contains
    /// `self.project_search` (case-insensitive). Returns an empty `Vec` if
    /// no project matches the current search text.
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

    /// Returns compact labels describing all active PR filters.
    ///
    /// # Returns
    ///
    /// The same labels as [`super::filter_modal::filters_to_labels`], with
    /// `self.search_query` folded in as the active search term when
    /// `self.filters.search` isn't already set.
    pub fn filter_labels(&self) -> Vec<String> {
        let mut filters = self.filters.clone();
        if filters.search.is_none() && !self.search_query.is_empty() {
            filters.search = Some(self.search_query.clone());
        }
        super::filter_modal::filters_to_labels(&filters)
    }
}
