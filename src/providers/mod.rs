pub mod azure_devops;
pub mod github;
pub mod registry;

use anyhow::Result;
use async_trait::async_trait;

use crate::api::types::{
    CodeSnippet, NormalizedPullRequest, PrThread, Project, PullRequestDetail, PullRequestFilters,
    Repository,
};

#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub id: String,
    #[allow(dead_code)]
    pub display_name: String,
}

#[async_trait]
pub trait ProviderClient: Send + Sync {
    async fn get_current_user(&self) -> Result<CurrentUser>;
    async fn list_projects(&self) -> Result<Vec<Project>>;
    async fn list_repositories(&self, project: &str) -> Result<Vec<Repository>>;
    async fn list_pull_requests(
        &self,
        project: &str,
        filters: &PullRequestFilters,
        current_user_id: Option<&str>,
    ) -> Result<Vec<NormalizedPullRequest>>;
    async fn get_pull_request(
        &self,
        project: &str,
        repository_id: &str,
        pull_request_id: i64,
    ) -> Result<PullRequestDetail>;
    async fn list_pull_request_threads(
        &self,
        project: &str,
        repository_id: &str,
        pull_request_id: i64,
    ) -> Result<Vec<PrThread>>;
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

#[derive(Debug, Clone, Copy)]
pub struct ProviderInfo {
    pub id: crate::api::types::ProviderId,
    pub name: &'static str,
    pub enabled: bool,
}
