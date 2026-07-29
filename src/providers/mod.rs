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
    async fn get_current_user(&self) -> Result<CurrentUser>;
    /// Lists projects or equivalent top-level containers.
    async fn list_projects(&self) -> Result<Vec<Project>>;
    /// Lists repositories within a project.
    async fn list_repositories(&self, project: &str) -> Result<Vec<Repository>>;
    /// Lists pull requests matching provider-neutral filters.
    async fn list_pull_requests(
        &self,
        project: &str,
        filters: &PullRequestFilters,
        current_user_id: Option<&str>,
    ) -> Result<Vec<NormalizedPullRequest>>;
    /// Loads the full description and metadata for one pull request.
    async fn get_pull_request(
        &self,
        project: &str,
        repository_id: &str,
        pull_request_id: i64,
    ) -> Result<PullRequestDetail>;
    /// Loads user-authored comment threads for one pull request.
    async fn list_pull_request_threads(
        &self,
        project: &str,
        repository_id: &str,
        pull_request_id: i64,
    ) -> Result<Vec<PrThread>>;
    /// Loads source context around a file comment when supported.
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
