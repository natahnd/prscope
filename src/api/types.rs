use serde::{Deserialize, Serialize};

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

/// Client-only filter on aggregate reviewer votes across a PR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ReviewStatusFilter {
    #[default]
    All,
    #[serde(rename = "needs-review")]
    NeedsReview,
    Approved,
    Rejected,
    #[serde(rename = "waiting-for-author")]
    WaitingForAuthor,
}

impl ReviewStatusFilter {
    pub fn label(self) -> &'static str {
        match self {
            ReviewStatusFilter::All => "All PRs",
            ReviewStatusFilter::NeedsReview => "Needs review",
            ReviewStatusFilter::Approved => "Approved",
            ReviewStatusFilter::Rejected => "Rejected",
            ReviewStatusFilter::WaitingForAuthor => "Waiting for author",
        }
    }

    pub fn filter_label(self) -> &'static str {
        match self {
            ReviewStatusFilter::All => "all",
            ReviewStatusFilter::NeedsReview => "needs-review",
            ReviewStatusFilter::Approved => "approved",
            ReviewStatusFilter::Rejected => "rejected",
            ReviewStatusFilter::WaitingForAuthor => "waiting-for-author",
        }
    }
}

/// Client-only filter on whether the signed-in user approved the PR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ApprovedByFilter {
    #[default]
    All,
    Me,
}

impl ApprovedByFilter {
    pub fn label(self) -> &'static str {
        match self {
            ApprovedByFilter::All => "All",
            ApprovedByFilter::Me => "Me",
        }
    }

    pub fn filter_label(self) -> &'static str {
        match self {
            ApprovedByFilter::All => "all",
            ApprovedByFilter::Me => "me",
        }
    }
}

/// Client-only filter on who authored the PR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum AuthorFilter {
    #[default]
    All,
    Me,
    #[serde(rename = "not-me")]
    NotMe,
}

impl AuthorFilter {
    pub fn label(self) -> &'static str {
        match self {
            AuthorFilter::All => "All",
            AuthorFilter::Me => "Me",
            AuthorFilter::NotMe => "Not me",
        }
    }

    pub fn filter_label(self) -> &'static str {
        match self {
            AuthorFilter::All => "all",
            AuthorFilter::Me => "me",
            AuthorFilter::NotMe => "not-me",
        }
    }
}

impl PullRequestFilters {
    pub fn effective_author_filter(&self) -> AuthorFilter {
        self.author_filter.unwrap_or_else(|| {
            if self.author_me.unwrap_or(false) {
                AuthorFilter::Me
            } else {
                AuthorFilter::All
            }
        })
    }
}

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

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GitHubProviderConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(rename = "lastRepo", skip_serializing_if = "Option::is_none")]
    pub last_repo: Option<String>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdoReviewer {
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub vote: i32,
    #[serde(rename = "isRequired")]
    pub is_required: bool,
    #[serde(rename = "isCurrentUser")]
    pub is_current_user: bool,
    /// True when the reviewer is a team or group (`isContainer` in the API).
    #[serde(rename = "isTeam", default)]
    pub is_team: bool,
}

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
    #[serde(rename = "isAuthorCurrentUser", default)]
    pub is_author_current_user: bool,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrComment {
    pub id: i64,
    pub author: String,
    pub content: String,
    #[serde(rename = "publishedAt")]
    pub published_at: String,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSnippetLine {
    pub number: i64,
    pub content: String,
    pub highlight: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSnippet {
    #[serde(rename = "filePath")]
    pub file_path: String,
    pub lines: Vec<CodeSnippetLine>,
    #[serde(rename = "centerLine")]
    pub center_line: i64,
}

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
    /// Client-only: filters on aggregate reviewer votes for the PR.
    #[serde(rename = "reviewStatusFilter", skip_serializing_if = "Option::is_none")]
    pub review_status_filter: Option<ReviewStatusFilter>,
    /// Client-only: filters to PRs approved by the signed-in user.
    #[serde(rename = "approvedByFilter", skip_serializing_if = "Option::is_none")]
    pub approved_by_filter: Option<ApprovedByFilter>,
    /// Client-only: filters on who authored the PR.
    #[serde(rename = "authorFilter", skip_serializing_if = "Option::is_none")]
    pub author_filter: Option<AuthorFilter>,
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
            review_status_filter: None,
            approved_by_filter: None,
            author_filter: None,
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
    pub fn next(self) -> Self {
        match self {
            PrListLayout::GroupByRepo => PrListLayout::GroupByAuthor,
            PrListLayout::GroupByAuthor => PrListLayout::SortByAgeDesc,
            PrListLayout::SortByAgeDesc => PrListLayout::SortByAgeAsc,
            PrListLayout::SortByAgeAsc => PrListLayout::GroupByRepo,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            PrListLayout::GroupByRepo => "repos",
            PrListLayout::GroupByAuthor => "author",
            PrListLayout::SortByAgeDesc => "age↓",
            PrListLayout::SortByAgeAsc => "age↑",
        }
    }

    pub fn groups(self) -> bool {
        !matches!(self, PrListLayout::SortByAgeDesc | PrListLayout::SortByAgeAsc)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdoProject {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

pub type Project = AdoProject;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdoRepository {
    pub id: String,
    pub name: String,
}

pub type Repository = AdoRepository;

#[derive(Debug, Clone, Deserialize)]
pub struct ConnectionData {
    #[serde(rename = "authenticatedUser")]
    pub authenticated_user: AdoIdentity,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiListResponse<T> {
    #[allow(dead_code)]
    pub count: i64,
    pub value: Vec<T>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawRepositoryRef {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawIdentityRef {
    #[allow(dead_code)]
    pub id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawReviewer {
    pub id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub vote: i32,
    #[serde(rename = "isRequired")]
    pub is_required: Option<bool>,
    #[serde(rename = "isContainer", default)]
    pub is_container: bool,
}

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

#[derive(Debug, Clone, Deserialize)]
pub struct RawLastMergeSourceCommit {
    #[serde(rename = "commitId")]
    pub commit_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawPullRequestDetail {
    #[serde(flatten)]
    pub pr: RawPullRequest,
    pub description: Option<String>,
    #[serde(rename = "lastMergeSourceCommit")]
    pub last_merge_source_commit: Option<RawLastMergeSourceCommit>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawCommentPosition {
    pub line: i64,
    #[allow(dead_code)]
    pub offset: i64,
}

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

/// Change kind for one file in a pull request iteration, parsed from Azure
/// DevOps's comma-joined `changeType` string (e.g. `"edit, rename"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrFileChangeType {
    Add,
    Edit,
    Delete,
    Rename,
    Other,
}

impl PrFileChangeType {
    /// Priority order (most to least specific) when ADO reports more than
    /// one change kind for a file, e.g. a rename that also edited content.
    pub fn parse(raw: &str) -> Self {
        let lower = raw.to_lowercase();
        let parts: Vec<&str> = lower.split(',').map(|p| p.trim()).collect();
        if parts.contains(&"delete") {
            PrFileChangeType::Delete
        } else if parts.contains(&"add") {
            PrFileChangeType::Add
        } else if parts.contains(&"rename") {
            PrFileChangeType::Rename
        } else if parts.contains(&"edit") {
            PrFileChangeType::Edit
        } else {
            PrFileChangeType::Other
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            PrFileChangeType::Add => "A",
            PrFileChangeType::Edit => "M",
            PrFileChangeType::Delete => "D",
            PrFileChangeType::Rename => "R",
            PrFileChangeType::Other => "?",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PrFileChange {
    pub path: String,
    pub old_path: Option<String>,
    pub change_type: PrFileChangeType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineKind {
    Context,
    Added,
    Removed,
    HunkHeader,
}

#[derive(Debug, Clone)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub old_line_no: Option<i64>,
    pub new_line_no: Option<i64>,
    pub content: String,
    pub comments: Vec<PrComment>,
}

#[derive(Debug, Clone)]
pub struct FileDiff {
    pub file: PrFileChange,
    pub lines: Vec<DiffLine>,
    pub additions: usize,
    pub deletions: usize,
    pub is_binary: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawGitCommitRef {
    #[serde(rename = "commitId")]
    pub commit_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawPullRequestIteration {
    #[allow(dead_code)]
    pub id: i64,
    #[serde(rename = "sourceRefCommit")]
    pub source_ref_commit: Option<RawGitCommitRef>,
    #[serde(rename = "commonRefCommit")]
    pub common_ref_commit: Option<RawGitCommitRef>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawChangeItem {
    #[serde(default)]
    pub path: Option<String>,
    #[serde(rename = "isFolder", default)]
    pub is_folder: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawChangeEntry {
    pub item: RawChangeItem,
    #[serde(rename = "changeType")]
    pub change_type: String,
    #[serde(rename = "sourceServerItem", default)]
    pub source_server_item: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawIterationChanges {
    #[serde(rename = "changeEntries")]
    pub change_entries: Vec<RawChangeEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppScreen {
    ProviderSelect,
    ProviderSetup,
    ProjectSelect,
    PrList,
}

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
        ProviderId::Github => false,
    }
}

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
    fn file_change_type_parses_simple_kinds() {
        assert_eq!(PrFileChangeType::parse("add"), PrFileChangeType::Add);
        assert_eq!(PrFileChangeType::parse("edit"), PrFileChangeType::Edit);
        assert_eq!(PrFileChangeType::parse("delete"), PrFileChangeType::Delete);
        assert_eq!(PrFileChangeType::parse("rename"), PrFileChangeType::Rename);
        assert_eq!(PrFileChangeType::parse("encoding"), PrFileChangeType::Other);
    }

    #[test]
    fn file_change_type_parses_combined_kinds() {
        assert_eq!(PrFileChangeType::parse("edit, rename"), PrFileChangeType::Rename);
        assert_eq!(PrFileChangeType::parse("rename, edit"), PrFileChangeType::Rename);
        assert_eq!(PrFileChangeType::parse("delete, edit"), PrFileChangeType::Delete);
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
