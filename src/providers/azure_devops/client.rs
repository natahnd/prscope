//! Azure DevOps REST client and conversion into provider-neutral models.

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::de::DeserializeOwned;

use crate::api::types::{
    AdoReviewer, ApiListResponse, CodeSnippet, CodeSnippetLine, ConnectionData,
    NormalizedPullRequest, PrComment, PrThread, Project, PullRequestDetail, PullRequestFilters,
    PullRequestStatus, RawPullRequest, RawPullRequestDetail, RawThread, Repository,
};
use crate::providers::{CurrentUser, ProviderClient};

const API_VERSION: &str = "7.0";
/// `connectionData` is preview-only; stable 7.x returns HTTP 400.
const CONNECTION_DATA_API_VERSION: &str = "7.0-preview.1";

/// Read-only Azure DevOps implementation of [`ProviderClient`].
pub struct AzureDevOpsClient {
    token: String,
    organization: String,
    http: reqwest::Client,
}

impl AzureDevOpsClient {
    /// Creates a client for one organization using a bearer token or PAT.
    ///
    /// # Arguments
    ///
    /// * `token` - Bearer token or Personal Access Token to authenticate
    ///   requests with.
    /// * `organization` - Azure DevOps organization name (as in
    ///   `dev.azure.com/<organization>`).
    ///
    /// # Returns
    ///
    /// A new `AzureDevOpsClient` scoped to `organization`.
    pub fn new(token: String, organization: String) -> Self {
        AzureDevOpsClient {
            token,
            organization,
            http: reqwest::Client::new(),
        }
    }

    async fn fetch_json<T: DeserializeOwned>(&self, url: &str) -> Result<T> {
        let response = self
            .http
            .get(url)
            .bearer_auth(&self.token)
            .header("Accept", "application/json")
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            let truncated: String = body.chars().take(200).collect();
            return Err(anyhow!("Azure DevOps API error {}: {}", status.as_u16(), truncated));
        }

        let body = response.text().await?;
        serde_json::from_str::<T>(&body).map_err(|err| {
            let truncated: String = body.chars().take(500).collect();
            anyhow!("failed to decode Azure DevOps response: {err}\nbody: {truncated}")
        })
    }

    async fn fetch_text(&self, url: &str) -> Result<String> {
        let response = self
            .http
            .get(url)
            .bearer_auth(&self.token)
            .header("Accept", "text/plain")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Azure DevOps API error {}", response.status().as_u16()));
        }

        Ok(response.text().await?)
    }

    fn base_url(&self, project: Option<&str>) -> String {
        match project {
            Some(project) => format!(
                "https://dev.azure.com/{}/{}",
                self.organization,
                urlencoding::encode(project)
            ),
            None => format!("https://dev.azure.com/{}", self.organization),
        }
    }

    async fn get_connection_data(&self) -> Result<ConnectionData> {
        let url = format!(
            "{}/_apis/connectionData?api-version={}",
            self.base_url(None),
            CONNECTION_DATA_API_VERSION
        );
        self.fetch_json(&url).await
    }
}

#[async_trait]
impl ProviderClient for AzureDevOpsClient {
    async fn get_current_user(&self) -> Result<CurrentUser> {
        let data = self.get_connection_data().await?;
        Ok(CurrentUser {
            id: data.authenticated_user.id,
            display_name: data.authenticated_user.display_name,
        })
    }

    async fn list_projects(&self) -> Result<Vec<Project>> {
        let mut projects: Vec<Project> = Vec::new();
        let mut skip = 0i64;
        let top = 100i64;

        // This endpoint does not expose the continuation-token pattern used by
        // some Azure APIs, so page until a short `$top` response is returned.
        loop {
            let url = format!(
                "{}/_apis/projects?$top={}&$skip={}&api-version={}",
                self.base_url(None),
                top,
                skip,
                API_VERSION
            );
            let data: ApiListResponse<Project> = self.fetch_json(&url).await?;
            let len = data.value.len() as i64;
            projects.extend(data.value);
            if len < top {
                break;
            }
            skip += top;
        }

        projects.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        Ok(projects)
    }

    async fn list_repositories(&self, project: &str) -> Result<Vec<Repository>> {
        let url = format!(
            "{}/_apis/git/repositories?api-version={}",
            self.base_url(Some(project)),
            API_VERSION
        );
        let data: ApiListResponse<Repository> = self.fetch_json(&url).await?;
        let mut repos = data.value;
        repos.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        Ok(repos)
    }

    async fn list_pull_requests(
        &self,
        project: &str,
        filters: &PullRequestFilters,
        current_user_id: Option<&str>,
    ) -> Result<Vec<NormalizedPullRequest>> {
        // Push identity and lifecycle filters to Azure DevOps when the API
        // supports them. Name, draft, and text filters run after normalization.
        let mut params: Vec<(String, String)> =
            vec![("api-version".into(), API_VERSION.into())];
        if filters.status != PullRequestStatus::All {
            params.push(("searchCriteria.status".into(), filters.status.to_string()));
        }
        if let Some(repo_id) = &filters.repository_id {
            params.push(("searchCriteria.repositoryId".into(), repo_id.clone()));
        }
        if let Some(author_id) = &filters.author_id {
            params.push(("searchCriteria.creatorId".into(), author_id.clone()));
        }
        if let Some(reviewer_id) = &filters.reviewer_id {
            params.push(("searchCriteria.reviewerId".into(), reviewer_id.clone()));
        }

        let mut all: Vec<RawPullRequest> = Vec::new();
        let mut skip = 0i64;
        let top = 100i64;

        loop {
            let mut query = params.clone();
            query.push(("$top".into(), top.to_string()));
            query.push(("$skip".into(), skip.to_string()));
            let qs = query
                .iter()
                .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
                .collect::<Vec<_>>()
                .join("&");
            let url = format!(
                "{}/_apis/git/pullrequests?{}",
                self.base_url(Some(project)),
                qs
            );
            let data: ApiListResponse<RawPullRequest> = self.fetch_json(&url).await?;
            let len = data.value.len() as i64;
            all.extend(data.value);
            if len < top {
                break;
            }
            skip += top;
        }

        let normalized: Vec<NormalizedPullRequest> = all
            .into_iter()
            .map(|pr| normalize_pull_request(pr, &self.organization, project, current_user_id))
            .filter(|pr| matches_client_filters(pr, filters))
            .collect();
        Ok(normalized)
    }

    async fn get_pull_request(
        &self,
        project: &str,
        repository_id: &str,
        pull_request_id: i64,
    ) -> Result<PullRequestDetail> {
        let url = format!(
            "{}/_apis/git/repositories/{}/pullrequests/{}?api-version={}",
            self.base_url(Some(project)),
            repository_id,
            pull_request_id,
            API_VERSION
        );
        let raw: RawPullRequestDetail = self.fetch_json(&url).await?;
        let description = raw.description.clone().unwrap_or_default();
        let source_commit_id = raw
            .last_merge_source_commit
            .as_ref()
            .map(|c| c.commit_id.clone());
        let pr = normalize_pull_request(raw.pr, &self.organization, project, None);
        Ok(PullRequestDetail {
            pr,
            description,
            source_commit_id,
        })
    }

    async fn list_pull_request_threads(
        &self,
        project: &str,
        repository_id: &str,
        pull_request_id: i64,
    ) -> Result<Vec<PrThread>> {
        let url = format!(
            "{}/_apis/git/repositories/{}/pullrequests/{}/threads?api-version={}",
            self.base_url(Some(project)),
            repository_id,
            pull_request_id,
            API_VERSION
        );
        let data: ApiListResponse<RawThread> = self.fetch_json(&url).await?;

        // Azure DevOps also returns system threads (reviewer voted, status
        // changed, branch updated, etc). Those have no real feedback once
        // their system-generated comments are stripped in normalize_thread,
        // but they still carry a thread `status` (often "active", since ADO
        // doesn't mark them resolved) — so they must be dropped here, after
        // normalizing, rather than by the raw comment count, or they'd be
        // miscounted as unresolved human feedback.
        Ok(data
            .value
            .into_iter()
            .filter(|thread| !thread.is_deleted.unwrap_or(false))
            .map(normalize_thread)
            .filter(|thread| !thread.comments.is_empty())
            .collect())
    }

    async fn get_file_snippet(
        &self,
        project: &str,
        repository_id: &str,
        file_path: &str,
        commit_id: &str,
        line: i64,
        radius: i64,
    ) -> Result<Option<CodeSnippet>> {
        let result: Result<CodeSnippet> = async {
            let qs = format!(
                "path={}&versionDescriptor.version={}&versionDescriptor.versionType=commit&api-version={}&$format=text",
                urlencoding::encode(file_path),
                urlencoding::encode(commit_id),
                API_VERSION
            );
            let url = format!(
                "{}/_apis/git/repositories/{}/items?{}",
                self.base_url(Some(project)),
                repository_id,
                qs
            );
            let text = self.fetch_text(&url).await?;
            let all_lines: Vec<&str> = text.split(['\n']).map(|l| l.trim_end_matches('\r')).collect();

            let start_line = 1.max(line - radius);
            let end_line = (all_lines.len() as i64).min(line + radius);
            let mut lines = Vec::new();
            let mut n = start_line;
            while n <= end_line {
                lines.push(CodeSnippetLine {
                    number: n,
                    content: all_lines.get((n - 1) as usize).copied().unwrap_or("").to_string(),
                    highlight: n == line,
                });
                n += 1;
            }

            Ok(CodeSnippet {
                file_path: file_path.to_string(),
                lines,
                center_line: line,
            })
        }
        .await;

        // Source context is optional enrichment; permission or file lookup
        // failures should not prevent comments themselves from rendering.
        Ok(result.ok())
    }
}

/// Creates an Azure DevOps client and validates that an organization is set.
///
/// # Arguments
///
/// * `token` - Bearer token or Personal Access Token to authenticate
///   requests with.
/// * `organization` - Organization name, if one has been configured or
///   entered during setup.
///
/// # Returns
///
/// A new `AzureDevOpsClient` scoped to `organization`.
///
/// # Errors
///
/// Returns `Err` if `organization` is `None`.
pub fn create_azure_devops_client(
    token: String,
    organization: Option<String>,
) -> Result<AzureDevOpsClient> {
    let organization = organization.ok_or_else(|| anyhow!("Azure DevOps organization is required."))?;
    Ok(AzureDevOpsClient::new(token, organization))
}

fn normalize_pull_request(
    pr: RawPullRequest,
    organization: &str,
    project: &str,
    current_user_id: Option<&str>,
) -> NormalizedPullRequest {
    let repo_name = pr.repository.name.clone();
    let reviewers: Vec<AdoReviewer> = pr
        .reviewers
        .unwrap_or_default()
        .into_iter()
        .map(|r| AdoReviewer {
            display_name: r.display_name,
            vote: r.vote,
            is_required: r.is_required.unwrap_or(false),
            is_current_user: current_user_id.map(|id| id == r.id).unwrap_or(false),
        })
        .collect();

    let needs_my_review = reviewers
        .iter()
        .any(|r| r.is_current_user && r.vote == 0 && !pr.is_draft.unwrap_or(false));

    let target_branch = pr
        .target_ref_name
        .strip_prefix("refs/heads/")
        .unwrap_or(&pr.target_ref_name)
        .to_string();

    let url = format!(
        "https://dev.azure.com/{}/{}/_git/{}/pullrequest/{}",
        organization,
        urlencoding::encode(project),
        urlencoding::encode(&repo_name),
        pr.pull_request_id
    );

    NormalizedPullRequest {
        id: pr.pull_request_id,
        title: pr.title,
        status: pr.status,
        is_draft: pr.is_draft.unwrap_or(false),
        repo_name,
        repository_id: pr.repository.id,
        author: pr.created_by.display_name,
        reviewers,
        target_branch,
        created_at: pr.creation_date,
        url,
        merge_status: pr.merge_status,
        needs_my_review,
    }
}

fn normalize_thread(thread: RawThread) -> PrThread {
    let context = thread.thread_context;
    // Prefer the right side of the diff (the proposed file), falling back to
    // the left side for comments attached to removed lines.
    let position = context
        .as_ref()
        .and_then(|c| c.right_file_start.clone().or(c.left_file_start.clone()));
    let end_position = context
        .as_ref()
        .and_then(|c| c.right_file_end.clone().or(c.left_file_end.clone()));
    let line_end = match (&position, &end_position) {
        (Some(p), Some(e)) if e.line != p.line => Some(e.line),
        _ => None,
    };

    let comments: Vec<PrComment> = thread
        .comments
        .into_iter()
        .filter(|c| c.comment_type.as_deref().unwrap_or("text") != "system" && c.content.is_some())
        .map(|c| PrComment {
            id: c.id,
            author: c.author.display_name,
            content: c.content.unwrap_or_default(),
            published_at: c.published_date,
        })
        .collect();

    PrThread {
        id: thread.id,
        status: thread.status.unwrap_or_else(|| "unknown".to_string()),
        file_path: context.map(|c| c.file_path),
        line: position.map(|p| p.line),
        line_end,
        comments,
    }
}

fn matches_client_filters(pr: &NormalizedPullRequest, filters: &PullRequestFilters) -> bool {
    if let Some(repo_name) = &filters.repository_name {
        if &pr.repo_name != repo_name {
            return false;
        }
    }
    if let Some(author) = &filters.author {
        if pr.author.to_lowercase() != author.to_lowercase() {
            return false;
        }
    }
    if filters.reviewer_me.unwrap_or(false) && !pr.reviewers.iter().any(|r| r.is_current_user) {
        return false;
    }
    if let Some(draft_filter) = filters.draft_filter {
        use crate::api::types::DraftFilter;
        if draft_filter == DraftFilter::Hide && pr.is_draft {
            return false;
        }
        if draft_filter == DraftFilter::Only && !pr.is_draft {
            return false;
        }
    }
    if let Some(search) = &filters.search {
        let q = search.to_lowercase();
        let haystack = format!("{} {} {}", pr.repo_name, pr.title, pr.author).to_lowercase();
        if !haystack.contains(&q) {
            return false;
        }
    }
    true
}
