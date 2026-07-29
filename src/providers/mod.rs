//! Provider abstraction shared by the TUI, CLI, and hosting-service clients.

pub mod azure_devops;
pub mod github;
pub mod registry;

use anyhow::Result;
use async_trait::async_trait;

use crate::api::types::{
    CodeSnippet, NormalizedPullRequest, PrThread, Project, PullRequestDetail, PullRequestFilters,
    Repository,
};

/// Identity of the authenticated provider user.
#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub id: String,
    #[allow(dead_code)]
    pub display_name: String,
}

/// Read-only provider operations required by the application.
///
/// Implementations normalize provider-specific responses into shared domain
/// models before returning them to the CLI or TUI.
#[async_trait]
pub trait ProviderClient: Send + Sync {
    /// Returns the user associated with the active credential.
    ///
    /// # Returns
    ///
    /// The authenticated `CurrentUser`, used to resolve "me" filters and
    /// review-vote ownership.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the credential is invalid or expired, or if the
    /// underlying API request fails.
    async fn get_current_user(&self) -> Result<CurrentUser>;

    /// Lists projects or equivalent top-level containers.
    ///
    /// # Returns
    ///
    /// All projects visible to the authenticated user in this organization.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the underlying API request fails.
    async fn list_projects(&self) -> Result<Vec<Project>>;

    /// Lists repositories within a project.
    ///
    /// # Arguments
    ///
    /// * `project` - Name of the project to list repositories for.
    ///
    /// # Returns
    ///
    /// All repositories in `project`.
    ///
    /// # Errors
    ///
    /// Returns `Err` if `project` doesn't exist or the underlying API
    /// request fails.
    async fn list_repositories(&self, project: &str) -> Result<Vec<Repository>>;

    /// Lists pull requests matching provider-neutral filters.
    ///
    /// # Arguments
    ///
    /// * `project` - Name of the project to search within.
    /// * `filters` - Status, repository, author/reviewer, draft, and search
    ///   criteria to apply.
    /// * `current_user_id` - The authenticated user's provider ID, used to
    ///   resolve `needsMyReview` and any "me"-relative filters; `None` if
    ///   not yet known.
    ///
    /// # Returns
    ///
    /// Normalized pull requests in `project` matching `filters`. Returns an
    /// empty `Vec` if none match.
    ///
    /// # Errors
    ///
    /// Returns `Err` if `project` doesn't exist or the underlying API
    /// request fails.
    async fn list_pull_requests(
        &self,
        project: &str,
        filters: &PullRequestFilters,
        current_user_id: Option<&str>,
    ) -> Result<Vec<NormalizedPullRequest>>;

    /// Loads the full description and metadata for one pull request.
    ///
    /// # Arguments
    ///
    /// * `project` - Name of the project the pull request belongs to.
    /// * `repository_id` - Repository containing the pull request.
    /// * `pull_request_id` - Pull request number within `repository_id`.
    ///
    /// # Returns
    ///
    /// The full `PullRequestDetail`, including description and source
    /// commit, for the specified pull request.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the pull request doesn't exist or the underlying
    /// API request fails.
    async fn get_pull_request(
        &self,
        project: &str,
        repository_id: &str,
        pull_request_id: i64,
    ) -> Result<PullRequestDetail>;

    /// Loads user-authored comment threads for one pull request.
    ///
    /// # Arguments
    ///
    /// * `project` - Name of the project the pull request belongs to.
    /// * `repository_id` - Repository containing the pull request.
    /// * `pull_request_id` - Pull request number within `repository_id`.
    ///
    /// # Returns
    ///
    /// All comment threads on the pull request, with system-generated
    /// comments already filtered out. Returns an empty `Vec` if there are no
    /// user comments.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the pull request doesn't exist or the underlying
    /// API request fails.
    async fn list_pull_request_threads(
        &self,
        project: &str,
        repository_id: &str,
        pull_request_id: i64,
    ) -> Result<Vec<PrThread>>;

    /// Loads source context around a file comment when supported.
    ///
    /// # Arguments
    ///
    /// * `project` - Name of the project the pull request belongs to.
    /// * `repository_id` - Repository containing the file.
    /// * `file_path` - Path of the commented file.
    /// * `commit_id` - Commit the comment was anchored to.
    /// * `line` - Line number to center the snippet on.
    /// * `radius` - Number of lines of context to include above and below
    ///   `line`.
    ///
    /// # Returns
    ///
    /// `Some(CodeSnippet)` with the surrounding lines if the provider
    /// supports source-context lookups and the file/commit/line are found;
    /// `None` if the provider has no such capability or nothing is found.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the underlying API request fails.
    #[allow(dead_code)]
    async fn get_file_snippet(
        &self,
        project: &str,
        repository_id: &str,
        file_path: &str,
        commit_id: &str,
        line: i64,
        radius: i64,
    ) -> Result<Option<CodeSnippet>>;
}

/// Metadata shown by the first-run provider picker.
#[derive(Debug, Clone, Copy)]
pub struct ProviderInfo {
    pub id: crate::api::types::ProviderId,
    pub name: &'static str,
    pub enabled: bool,
}
