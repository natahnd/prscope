//! Provider-neutral application models and Azure DevOps wire-format types.

use serde::{Deserialize, Serialize};

/// Pull request lifecycle states accepted by the CLI and provider clients.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PullRequestStatus {
    Active,
    Completed,
    Abandoned,
    All,
}

impl Default for PullRequestStatus {
    fn default() -> Self {
        PullRequestStatus::Active
    }
}

impl std::fmt::Display for PullRequestStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            PullRequestStatus::Active => "active",
            PullRequestStatus::Completed => "completed",
            PullRequestStatus::Abandoned => "abandoned",
            PullRequestStatus::All => "all",
        };
        write!(f, "{s}")
    }
}

/// Controls whether draft pull requests appear in results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DraftFilter {
    All,
    Hide,
    Only,
}

impl Default for DraftFilter {
    fn default() -> Self {
        DraftFilter::All
    }
}

/// Controls whether the PR list is restricted to unresolved comment activity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommentActivityFilter {
    All,
    Only,
}

impl Default for CommentActivityFilter {
    fn default() -> Self {
        CommentActivityFilter::All
    }
}

/// Stable identifier used to select provider-specific configuration and clients.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderId {
    AzureDevops,
    Github,
}

impl std::fmt::Display for ProviderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ProviderId::AzureDevops => "azure-devops",
            ProviderId::Github => "github",
        };
        write!(f, "{s}")
    }
}

/// Persisted Azure DevOps settings and last-used UI state.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AzureDevOpsProviderConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization: Option<String>,
    #[serde(rename = "lastProject", skip_serializing_if = "Option::is_none")]
    pub last_project: Option<String>,
    #[serde(rename = "lastFilters", skip_serializing_if = "Option::is_none")]
    pub last_filters: Option<PullRequestFilters>,
    #[serde(rename = "lastListLayout", skip_serializing_if = "Option::is_none")]
    pub last_list_layout: Option<PrListLayout>,
}

/// Persisted GitHub settings and last-used UI state.
///
/// GitHub has no fixed upfront "organization" the way Azure DevOps does — the
/// owner (org or personal account) is chosen dynamically on the project
/// select screen, so this mirrors [`AzureDevOpsProviderConfig`] minus that
/// field.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GitHubProviderConfig {
    #[serde(rename = "lastProject", skip_serializing_if = "Option::is_none")]
    pub last_project: Option<String>,
    #[serde(rename = "lastFilters", skip_serializing_if = "Option::is_none")]
    pub last_filters: Option<PullRequestFilters>,
    #[serde(rename = "lastListLayout", skip_serializing_if = "Option::is_none")]
    pub last_list_layout: Option<PrListLayout>,
}

/// Provider-specific sections in the versioned application config.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ProviderConfigs {
    #[serde(rename = "azure-devops", skip_serializing_if = "Option::is_none")]
    pub azure_devops: Option<AzureDevOpsProviderConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub github: Option<GitHubProviderConfig>,
}

/// Permissive shape used only while reading the config file from disk, since
/// it may still be the legacy v1 (flat fields) or already-migrated v2 shape.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RawAppConfig {
    pub version: Option<u32>,
    #[serde(rename = "activeProvider")]
    pub active_provider: Option<ProviderId>,
    #[serde(default)]
    pub providers: ProviderConfigs,
    pub pat: Option<String>,
    // Legacy v1 flat fields — migrated automatically on load.
    pub organization: Option<String>,
    #[serde(rename = "lastProject")]
    pub last_project: Option<String>,
}

/// Current on-disk configuration schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfigV2 {
    pub version: u32,
    #[serde(rename = "activeProvider", skip_serializing_if = "Option::is_none")]
    pub active_provider: Option<ProviderId>,
    #[serde(default)]
    pub providers: ProviderConfigs,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pat: Option<String>,
}

impl Default for AppConfigV2 {
    fn default() -> Self {
        AppConfigV2 {
            version: 2,
            active_provider: None,
            providers: ProviderConfigs::default(),
            pat: None,
        }
    }
}

/// Authenticated identity returned by Azure DevOps connection metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdoIdentity {
    pub id: String,
    /// Azure DevOps's `connectionData` endpoint returns this identity's name
    /// under `providerDisplayName`, not `displayName` (unlike the reviewer/
    /// author identities elsewhere in the API). Default to empty rather than
    /// failing to decode — nothing in the app relies on this field.
    #[serde(rename = "displayName", default)]
    pub display_name: String,
}

/// Provider-neutral reviewer data displayed in list and plain output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdoReviewer {
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub vote: i32,
    #[serde(rename = "isRequired")]
    pub is_required: bool,
    #[serde(rename = "isCurrentUser")]
    pub is_current_user: bool,
}

/// Provider-neutral pull request summary used throughout the application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedPullRequest {
    pub id: i64,
    pub title: String,
    pub status: String,
    #[serde(rename = "isDraft")]
    pub is_draft: bool,
    #[serde(rename = "repoName")]
    pub repo_name: String,
    #[serde(rename = "repositoryId")]
    pub repository_id: String,
    pub author: String,
    pub reviewers: Vec<AdoReviewer>,
    #[serde(rename = "targetBranch")]
    pub target_branch: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    pub url: String,
    #[serde(rename = "mergeStatus")]
    pub merge_status: String,
    #[serde(rename = "needsMyReview")]
    pub needs_my_review: bool,
}

/// Pull request summary plus lazily loaded detail data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestDetail {
    #[serde(flatten)]
    pub pr: NormalizedPullRequest,
    pub description: String,
    #[serde(rename = "sourceCommitId", skip_serializing_if = "Option::is_none")]
    pub source_commit_id: Option<String>,
}

impl std::ops::Deref for PullRequestDetail {
    type Target = NormalizedPullRequest;
    fn deref(&self) -> &Self::Target {
        &self.pr
    }
}

/// Normalized user-authored pull request comment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrComment {
    pub id: i64,
    pub author: String,
    pub content: String,
    #[serde(rename = "publishedAt")]
    pub published_at: String,
}

/// Normalized comment thread, optionally anchored to a file and line range.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrThread {
    pub id: i64,
    pub status: String,
    #[serde(rename = "filePath", skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<i64>,
    #[serde(rename = "lineEnd", skip_serializing_if = "Option::is_none")]
    pub line_end: Option<i64>,
    pub comments: Vec<PrComment>,
}

/// One line of provider-supplied source context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSnippetLine {
    pub number: i64,
    pub content: String,
    pub highlight: bool,
}

/// Source context surrounding a file comment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSnippet {
    #[serde(rename = "filePath")]
    pub file_path: String,
    pub lines: Vec<CodeSnippetLine>,
    #[serde(rename = "centerLine")]
    pub center_line: i64,
}

/// Provider-neutral filters shared by interactive and plain listing modes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PullRequestFilters {
    pub status: PullRequestStatus,
    #[serde(rename = "repositoryId", skip_serializing_if = "Option::is_none")]
    pub repository_id: Option<String>,
    #[serde(rename = "repositoryName", skip_serializing_if = "Option::is_none")]
    pub repository_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(rename = "authorId", skip_serializing_if = "Option::is_none")]
    pub author_id: Option<String>,
    #[serde(rename = "reviewerId", skip_serializing_if = "Option::is_none")]
    pub reviewer_id: Option<String>,
    #[serde(rename = "reviewerMe", skip_serializing_if = "Option::is_none")]
    pub reviewer_me: Option<bool>,
    #[serde(rename = "authorMe", skip_serializing_if = "Option::is_none")]
    pub author_me: Option<bool>,
    #[serde(rename = "draftFilter", skip_serializing_if = "Option::is_none")]
    pub draft_filter: Option<DraftFilter>,
    /// Client-only: filters on whether a PR has any unresolved comment thread.
    #[serde(
        rename = "activeCommentsFilter",
        skip_serializing_if = "Option::is_none"
    )]
    pub active_comments_filter: Option<CommentActivityFilter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
}

impl Default for PullRequestFilters {
    fn default() -> Self {
        PullRequestFilters {
            status: PullRequestStatus::Active,
            repository_id: None,
            repository_name: None,
            author: None,
            author_id: None,
            reviewer_id: None,
            reviewer_me: None,
            author_me: None,
            draft_filter: None,
            active_comments_filter: None,
            search: None,
        }
    }
}

/// How the PR list panel groups and sorts pull requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PrListLayout {
    /// Group under repository headers; meta line shows author and age.
    #[default]
    #[serde(rename = "repos")]
    GroupByRepo,
    /// Group under author headers; meta line shows repository and age.
    #[serde(rename = "author")]
    GroupByAuthor,
    /// Flat list, newest first; meta line shows repository, author, and age.
    #[serde(rename = "age-desc")]
    SortByAgeDesc,
    /// Flat list, oldest first; meta line shows repository, author, and age.
    #[serde(rename = "age-asc")]
    SortByAgeAsc,
}

impl PrListLayout {
    /// Advances to the next layout in the user-facing cycle.
    ///
    /// # Returns
    ///
    /// The `PrListLayout` that follows `self`: repos → author → age↓ → age↑,
    /// wrapping back to repos.
    pub fn next(self) -> Self {
        match self {
            PrListLayout::GroupByRepo => PrListLayout::GroupByAuthor,
            PrListLayout::GroupByAuthor => PrListLayout::SortByAgeDesc,
            PrListLayout::SortByAgeDesc => PrListLayout::SortByAgeAsc,
            PrListLayout::SortByAgeAsc => PrListLayout::GroupByRepo,
        }
    }

    /// Returns the compact label displayed in the list panel.
    ///
    /// # Returns
    ///
    /// A short static string identifying `self`: `"repos"`, `"author"`,
    /// `"age↓"`, or `"age↑"`.
    pub fn label(self) -> &'static str {
        match self {
            PrListLayout::GroupByRepo => "repos",
            PrListLayout::GroupByAuthor => "author",
            PrListLayout::SortByAgeDesc => "age↓",
            PrListLayout::SortByAgeAsc => "age↑",
        }
    }

    /// Returns whether this layout inserts group headers between PR rows.
    ///
    /// # Returns
    ///
    /// `true` for `GroupByRepo`/`GroupByAuthor`; `false` for the two
    /// `SortByAge*` layouts, which render as a flat list.
    pub fn groups(self) -> bool {
        !matches!(self, PrListLayout::SortByAgeDesc | PrListLayout::SortByAgeAsc)
    }
}

/// Azure DevOps project payload used by the project picker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdoProject {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Provider-facing project model.
pub type Project = AdoProject;

/// Azure DevOps repository payload used by filters and PR metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdoRepository {
    pub id: String,
    pub name: String,
}

/// Provider-facing repository model.
pub type Repository = AdoRepository;

/// Azure DevOps connection metadata response.
#[derive(Debug, Clone, Deserialize)]
pub struct ConnectionData {
    #[serde(rename = "authenticatedUser")]
    pub authenticated_user: AdoIdentity,
}

/// Generic Azure DevOps collection response.
#[derive(Debug, Clone, Deserialize)]
pub struct ApiListResponse<T> {
    #[allow(dead_code)]
    pub count: i64,
    pub value: Vec<T>,
}

/// Repository reference embedded in an Azure DevOps pull request payload.
#[derive(Debug, Clone, Deserialize)]
pub struct RawRepositoryRef {
    pub id: String,
    pub name: String,
}

/// Identity reference embedded in Azure DevOps API payloads.
#[derive(Debug, Clone, Deserialize)]
pub struct RawIdentityRef {
    #[allow(dead_code)]
    pub id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
}

/// Reviewer representation returned by Azure DevOps.
#[derive(Debug, Clone, Deserialize)]
pub struct RawReviewer {
    pub id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub vote: i32,
    #[serde(rename = "isRequired")]
    pub is_required: Option<bool>,
}

/// Raw Azure DevOps pull request payload before normalization.
#[derive(Debug, Clone, Deserialize)]
pub struct RawPullRequest {
    #[serde(rename = "pullRequestId")]
    pub pull_request_id: i64,
    pub title: String,
    pub status: String,
    #[serde(rename = "isDraft")]
    pub is_draft: Option<bool>,
    #[serde(rename = "creationDate")]
    pub creation_date: String,
    #[serde(rename = "targetRefName")]
    pub target_ref_name: String,
    #[serde(rename = "mergeStatus")]
    pub merge_status: String,
    pub repository: RawRepositoryRef,
    #[serde(rename = "createdBy")]
    pub created_by: RawIdentityRef,
    #[serde(default)]
    pub reviewers: Option<Vec<RawReviewer>>,
}

/// Source commit reference embedded in pull request detail responses.
#[derive(Debug, Clone, Deserialize)]
pub struct RawLastMergeSourceCommit {
    #[serde(rename = "commitId")]
    pub commit_id: String,
}

/// Raw Azure DevOps detail payload before normalization.
#[derive(Debug, Clone, Deserialize)]
pub struct RawPullRequestDetail {
    #[serde(flatten)]
    pub pr: RawPullRequest,
    pub description: Option<String>,
    #[serde(rename = "lastMergeSourceCommit")]
    pub last_merge_source_commit: Option<RawLastMergeSourceCommit>,
}

/// Line and character position in an Azure DevOps thread context.
#[derive(Debug, Clone, Deserialize)]
pub struct RawCommentPosition {
    pub line: i64,
    #[allow(dead_code)]
    pub offset: i64,
}

/// File location metadata attached to an Azure DevOps comment thread.
#[derive(Debug, Clone, Deserialize)]
pub struct RawCommentThreadContext {
    #[serde(rename = "filePath")]
    pub file_path: String,
    #[serde(rename = "rightFileStart")]
    pub right_file_start: Option<RawCommentPosition>,
    #[serde(rename = "rightFileEnd")]
    pub right_file_end: Option<RawCommentPosition>,
    #[serde(rename = "leftFileStart")]
    pub left_file_start: Option<RawCommentPosition>,
    #[serde(rename = "leftFileEnd")]
    pub left_file_end: Option<RawCommentPosition>,
}

/// Raw Azure DevOps comment payload before system comments are removed.
#[derive(Debug, Clone, Deserialize)]
pub struct RawComment {
    #[allow(dead_code)]
    pub id: i64,
    pub content: Option<String>,
    #[serde(rename = "publishedDate")]
    pub published_date: String,
    #[serde(rename = "commentType")]
    pub comment_type: Option<String>,
    pub author: RawIdentityRef,
}

/// Raw Azure DevOps thread payload before status and location normalization.
#[derive(Debug, Clone, Deserialize)]
pub struct RawThread {
    pub id: i64,
    pub status: Option<String>,
    #[serde(rename = "isDeleted")]
    pub is_deleted: Option<bool>,
    #[serde(rename = "threadContext")]
    pub thread_context: Option<RawCommentThreadContext>,
    pub comments: Vec<RawComment>,
}

/// Top-level screens in the interactive setup and review flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppScreen {
    ProviderSelect,
    ProviderSetup,
    ProjectSelect,
    PrList,
}

/// Checks whether the active provider has enough persisted data to skip setup.
///
/// # Arguments
///
/// * `config` - The loaded application configuration to inspect.
///
/// # Returns
///
/// `true` if `config.active_provider` is set and that provider's section has
/// the minimum data needed to build a client (for Azure DevOps, an
/// organization name). `false` if no provider is active or its section is
/// missing that data, meaning the app should show the setup flow.
pub fn is_configured_provider(config: &AppConfigV2) -> bool {
    let Some(active) = config.active_provider else {
        return false;
    };
    match active {
        ProviderId::AzureDevops => config
            .providers
            .azure_devops
            .as_ref()
            .and_then(|c| c.organization.as_ref())
            .is_some(),
        // GitHub has no required upfront field like ADO's organization —
        // reaching this arm already proves `active_provider == Github`.
        ProviderId::Github => true,
    }
}

/// Reads the persisted Azure DevOps settings from the application config.
///
/// # Arguments
///
/// * `config` - The loaded application configuration to read from.
///
/// # Returns
///
/// The stored `AzureDevOpsProviderConfig`, or `AzureDevOpsProviderConfig::default()`
/// if no Azure DevOps section has been persisted yet (e.g. before first setup).
pub fn get_azure_devops_config(config: &AppConfigV2) -> AzureDevOpsProviderConfig {
    config.providers.azure_devops.clone().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_id_serializes_kebab_case() {
        let json = serde_json::to_string(&ProviderId::AzureDevops).unwrap();
        assert_eq!(json, "\"azure-devops\"");
    }

    #[test]
    fn is_configured_provider_requires_organization() {
        let mut config = AppConfigV2::default();
        assert!(!is_configured_provider(&config));

        config.active_provider = Some(ProviderId::AzureDevops);
        config.providers.azure_devops = Some(AzureDevOpsProviderConfig::default());
        assert!(!is_configured_provider(&config));

        config.providers.azure_devops.as_mut().unwrap().organization = Some("acme".into());
        assert!(is_configured_provider(&config));
    }
}
