//! GitHub GraphQL client and conversion into provider-neutral models.
//!
//! Every [`ProviderClient`] method is answered over GraphQL rather than REST:
//! GraphQL's `search` query lets us fetch cross-repository pull requests with
//! full reviewer/branch fidelity in one shape, and it's the only GitHub API
//! that exposes review-thread resolution (`isResolved`) at all.

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::json;
use tokio::sync::OnceCell;

use crate::api::types::{
    AdoReviewer, CodeSnippet, CodeSnippetLine, NormalizedPullRequest, PrComment, PrThread,
    Project, PullRequestDetail, PullRequestFilters, PullRequestStatus, Repository,
};
use crate::providers::{matches_client_filters, CurrentUser, ProviderClient};

const GRAPHQL_URL: &str = "https://api.github.com/graphql";

/// Read-only GitHub implementation of [`ProviderClient`], backed entirely by
/// the GraphQL API.
pub struct GitHubClient {
    token: String,
    http: reqwest::Client,
    /// Memoized authenticated login, used to pick the `org:`/`user:` search
    /// qualifier without an extra round trip on repeat calls.
    viewer_login: OnceCell<String>,
}

impl GitHubClient {
    pub fn new(token: String) -> Self {
        GitHubClient {
            token,
            http: reqwest::Client::new(),
            viewer_login: OnceCell::new(),
        }
    }

    async fn graphql<T: DeserializeOwned>(&self, query: &str, variables: serde_json::Value) -> Result<T> {
        let body = json!({ "query": query, "variables": variables });
        let response = self
            .http
            .post(GRAPHQL_URL)
            .bearer_auth(&self.token)
            .header("User-Agent", "prscope")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            let truncated: String = text.chars().take(500).collect();
            return Err(anyhow!("GitHub API error {}: {}", status.as_u16(), truncated));
        }

        let text = response.text().await?;
        let parsed: GraphQlResponse<T> = serde_json::from_str(&text).map_err(|err| {
            let truncated: String = text.chars().take(500).collect();
            anyhow!("failed to decode GitHub response: {err}\nbody: {truncated}")
        })?;

        if !parsed.errors.is_empty() {
            let messages: Vec<String> = parsed.errors.into_iter().map(|e| e.message).collect();
            return Err(anyhow!("GitHub GraphQL error: {}", messages.join("; ")));
        }

        parsed
            .data
            .ok_or_else(|| anyhow!("GitHub GraphQL response missing data"))
    }

    /// Returns the authenticated user's login, fetching and caching it once.
    async fn viewer_login(&self) -> Result<String> {
        let login = self
            .viewer_login
            .get_or_try_init(|| async {
                let data: ViewerLoginData = self
                    .graphql("query { viewer { login } }", json!({}))
                    .await?;
                Ok::<String, anyhow::Error>(data.viewer.login)
            })
            .await?;
        Ok(login.clone())
    }
}

#[derive(Debug, Deserialize)]
struct GraphQlResponse<T> {
    data: Option<T>,
    #[serde(default)]
    errors: Vec<GraphQlError>,
}

#[derive(Debug, Deserialize)]
struct GraphQlError {
    message: String,
}

#[derive(Debug, Deserialize)]
struct PageInfo {
    #[serde(rename = "hasNextPage")]
    has_next_page: bool,
    #[serde(rename = "endCursor")]
    end_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AuthorRef {
    login: String,
}

#[derive(Debug, Deserialize)]
struct ViewerLoginData {
    viewer: ViewerLogin,
}

#[derive(Debug, Deserialize)]
struct ViewerLogin {
    login: String,
}

#[async_trait]
impl ProviderClient for GitHubClient {
    async fn get_current_user(&self) -> Result<CurrentUser> {
        #[derive(Debug, Deserialize)]
        struct Data {
            viewer: Viewer,
        }
        #[derive(Debug, Deserialize)]
        struct Viewer {
            login: String,
            #[serde(default)]
            name: Option<String>,
        }

        let data: Data = self
            .graphql("query { viewer { login name } }", json!({}))
            .await?;
        let login = data.viewer.login;
        // Best-effort seed of the memoized login; ignore if already set by a
        // concurrent call.
        let _ = self.viewer_login.set(login.clone());
        Ok(CurrentUser {
            id: login.clone(),
            display_name: data.viewer.name.unwrap_or(login),
        })
    }

    async fn list_projects(&self) -> Result<Vec<Project>> {
        const QUERY: &str = r#"
            query($cursor: String) {
              viewer {
                login
                organizations(first: 100, after: $cursor) {
                  nodes { login description }
                  pageInfo { hasNextPage endCursor }
                }
              }
            }
        "#;

        #[derive(Debug, Deserialize)]
        struct Data {
            viewer: Viewer,
        }
        #[derive(Debug, Deserialize)]
        struct Viewer {
            login: String,
            organizations: OrgConnection,
        }
        #[derive(Debug, Deserialize)]
        struct OrgConnection {
            nodes: Vec<OrgNode>,
            #[serde(rename = "pageInfo")]
            page_info: PageInfo,
        }
        #[derive(Debug, Deserialize)]
        struct OrgNode {
            login: String,
            #[serde(default)]
            description: Option<String>,
        }

        let mut projects: Vec<Project> = Vec::new();
        let mut cursor: Option<String> = None;
        let mut self_login: Option<String> = None;

        loop {
            let data: Data = self.graphql(QUERY, json!({ "cursor": cursor })).await?;
            self_login.get_or_insert(data.viewer.login);
            for org in data.viewer.organizations.nodes {
                projects.push(Project {
                    id: org.login.clone(),
                    name: org.login,
                    description: org.description,
                });
            }
            let page_info = data.viewer.organizations.page_info;
            if page_info.has_next_page {
                cursor = page_info.end_cursor;
            } else {
                break;
            }
        }

        if let Some(login) = self_login {
            projects.push(Project {
                id: login.clone(),
                name: login,
                description: Some("Personal account".to_string()),
            });
        }

        projects.sort_by_key(|p| p.name.to_lowercase());
        Ok(projects)
    }

    async fn list_repositories(&self, project: &str) -> Result<Vec<Repository>> {
        const QUERY: &str = r#"
            query($login: String!, $cursor: String) {
              repositoryOwner(login: $login) {
                ... on Organization {
                  orgRepos: repositories(first: 100, after: $cursor) {
                    nodes { name }
                    pageInfo { hasNextPage endCursor }
                  }
                }
                ... on User {
                  userRepos: repositories(first: 100, after: $cursor, ownerAffiliations: [OWNER]) {
                    nodes { name }
                    pageInfo { hasNextPage endCursor }
                  }
                }
              }
            }
        "#;

        #[derive(Debug, Deserialize)]
        struct Data {
            #[serde(rename = "repositoryOwner")]
            repository_owner: Option<Owner>,
        }
        #[derive(Debug, Deserialize)]
        struct Owner {
            #[serde(rename = "orgRepos")]
            org_repos: Option<RepoConnection>,
            #[serde(rename = "userRepos")]
            user_repos: Option<RepoConnection>,
        }
        #[derive(Debug, Deserialize)]
        struct RepoConnection {
            nodes: Vec<RepoNode>,
            #[serde(rename = "pageInfo")]
            page_info: PageInfo,
        }
        #[derive(Debug, Deserialize)]
        struct RepoNode {
            name: String,
        }

        let mut repos: Vec<Repository> = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            let data: Data = self
                .graphql(QUERY, json!({ "login": project, "cursor": cursor }))
                .await?;
            let owner = data
                .repository_owner
                .ok_or_else(|| anyhow!("GitHub owner '{project}' not found"))?;
            let connection = owner
                .org_repos
                .or(owner.user_repos)
                .ok_or_else(|| anyhow!("GitHub owner '{project}' has no repositories connection"))?;

            for node in connection.nodes {
                repos.push(Repository {
                    id: format!("{project}/{}", node.name),
                    name: node.name,
                });
            }

            if connection.page_info.has_next_page {
                cursor = connection.page_info.end_cursor;
            } else {
                break;
            }
        }

        repos.sort_by_key(|r| r.name.to_lowercase());
        Ok(repos)
    }

    async fn list_pull_requests(
        &self,
        project: &str,
        filters: &PullRequestFilters,
        _current_user_id: Option<&str>,
    ) -> Result<Vec<NormalizedPullRequest>> {
        const QUERY: &str = r#"
            query($q: String!, $cursor: String) {
              search(query: $q, type: ISSUE, first: 50, after: $cursor) {
                pageInfo { hasNextPage endCursor }
                nodes {
                  ... on PullRequest {
                    number title state isDraft createdAt url merged mergeable baseRefName
                    repository { name owner { login } }
                    author { login }
                    reviewRequests(first: 20) {
                      nodes { requestedReviewer { ... on User { login } } }
                    }
                  }
                }
              }
            }
        "#;

        let viewer_login = self.viewer_login().await?;
        let query_string = build_search_query(project, filters, &viewer_login);

        let mut all: Vec<NormalizedPullRequest> = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            let data: SearchData = self
                .graphql(QUERY, json!({ "q": query_string, "cursor": cursor }))
                .await?;
            for node in data.search.nodes {
                if let Some((fields, owner, repo_name)) = extract_search_fields(node) {
                    all.push(normalize_pr(fields, &owner, &repo_name, &viewer_login));
                }
            }
            if data.search.page_info.has_next_page {
                cursor = data.search.page_info.end_cursor;
            } else {
                break;
            }
        }

        Ok(all
            .into_iter()
            .filter(|pr| matches_client_filters(pr, filters))
            .collect())
    }

    async fn get_pull_request(
        &self,
        project: &str,
        repository_id: &str,
        pull_request_id: i64,
    ) -> Result<PullRequestDetail> {
        const QUERY: &str = r#"
            query($owner: String!, $repo: String!, $number: Int!) {
              repository(owner: $owner, name: $repo) {
                pullRequest(number: $number) {
                  number title state isDraft createdAt url merged mergeable baseRefName body
                  headRefOid
                  author { login }
                  reviewRequests(first: 20) {
                    nodes { requestedReviewer { ... on User { login } } }
                  }
                }
              }
            }
        "#;

        #[derive(Debug, Deserialize)]
        struct Data {
            repository: Option<RepoWrapper>,
        }
        #[derive(Debug, Deserialize)]
        struct RepoWrapper {
            #[serde(rename = "pullRequest")]
            pull_request: Option<DetailNode>,
        }
        #[derive(Debug, Deserialize)]
        struct DetailNode {
            #[serde(flatten)]
            fields: SearchPrFields,
            #[serde(default)]
            body: Option<String>,
            #[serde(rename = "headRefOid")]
            head_ref_oid: Option<String>,
        }

        let (owner, repo) = split_repository_id(repository_id, project);
        let viewer_login = self.viewer_login().await?;

        let data: Data = self
            .graphql(
                QUERY,
                json!({ "owner": owner, "repo": repo, "number": pull_request_id }),
            )
            .await?;
        let node = data
            .repository
            .and_then(|r| r.pull_request)
            .ok_or_else(|| anyhow!("pull request #{pull_request_id} not found in {owner}/{repo}"))?;

        let fields = extract_pr_fields(node.fields)
            .ok_or_else(|| anyhow!("GitHub returned an incomplete pull request payload"))?;
        let pr = normalize_pr(fields, &owner, &repo, &viewer_login);

        Ok(PullRequestDetail {
            pr,
            description: node.body.unwrap_or_default(),
            source_commit_id: node.head_ref_oid,
        })
    }

    async fn list_pull_request_threads(
        &self,
        project: &str,
        repository_id: &str,
        pull_request_id: i64,
    ) -> Result<Vec<PrThread>> {
        const QUERY: &str = r#"
            query($owner: String!, $repo: String!, $number: Int!, $cursor: String) {
              repository(owner: $owner, name: $repo) {
                pullRequest(number: $number) {
                  comments(first: 100) {
                    nodes { databaseId body createdAt author { login } }
                  }
                  reviewThreads(first: 50, after: $cursor) {
                    pageInfo { hasNextPage endCursor }
                    nodes {
                      isResolved
                      path
                      line
                      originalLine
                      startLine
                      comments(first: 50) {
                        nodes { databaseId body createdAt author { login } }
                      }
                    }
                  }
                }
              }
            }
        "#;

        #[derive(Debug, Deserialize)]
        struct Data {
            repository: Option<RepoWrapper>,
        }
        #[derive(Debug, Deserialize)]
        struct RepoWrapper {
            #[serde(rename = "pullRequest")]
            pull_request: Option<PrWrapper>,
        }
        #[derive(Debug, Deserialize)]
        struct PrWrapper {
            comments: CommentConnection,
            #[serde(rename = "reviewThreads")]
            review_threads: ReviewThreadConnection,
        }
        #[derive(Debug, Deserialize)]
        struct CommentConnection {
            nodes: Vec<RawComment>,
        }
        #[derive(Debug, Deserialize)]
        struct RawComment {
            #[serde(rename = "databaseId")]
            database_id: Option<i64>,
            body: String,
            #[serde(rename = "createdAt")]
            created_at: String,
            author: Option<AuthorRef>,
        }
        #[derive(Debug, Deserialize)]
        struct ReviewThreadConnection {
            nodes: Vec<ReviewThreadNode>,
            #[serde(rename = "pageInfo")]
            page_info: PageInfo,
        }
        #[derive(Debug, Deserialize)]
        struct ReviewThreadNode {
            #[serde(rename = "isResolved")]
            is_resolved: bool,
            path: Option<String>,
            line: Option<i64>,
            #[serde(rename = "originalLine")]
            original_line: Option<i64>,
            #[serde(rename = "startLine")]
            start_line: Option<i64>,
            comments: CommentConnection,
        }

        fn normalize_comment(raw: RawComment) -> PrComment {
            PrComment {
                id: raw.database_id.unwrap_or_default(),
                author: raw.author.map(|a| a.login).unwrap_or_default(),
                content: raw.body,
                published_at: raw.created_at,
            }
        }

        let (owner, repo) = split_repository_id(repository_id, project);
        let mut threads: Vec<PrThread> = Vec::new();
        let mut cursor: Option<String> = None;
        let mut general_added = false;

        loop {
            let data: Data = self
                .graphql(
                    QUERY,
                    json!({ "owner": owner, "repo": repo, "number": pull_request_id, "cursor": cursor }),
                )
                .await?;
            let pr = data
                .repository
                .and_then(|r| r.pull_request)
                .ok_or_else(|| anyhow!("pull request #{pull_request_id} not found in {owner}/{repo}"))?;

            if !general_added {
                for raw in pr.comments.nodes {
                    let comment = normalize_comment(raw);
                    threads.push(PrThread {
                        id: comment.id,
                        status: "active".to_string(),
                        file_path: None,
                        line: None,
                        line_end: None,
                        comments: vec![comment],
                    });
                }
                general_added = true;
            }

            for node in pr.review_threads.nodes {
                let comments: Vec<PrComment> = node.comments.nodes.into_iter().map(normalize_comment).collect();
                let Some(first) = comments.first() else {
                    continue;
                };
                let end_line = node.line.or(node.original_line);
                let start_line = node.start_line.or(end_line);
                let line_end = match (start_line, end_line) {
                    (Some(s), Some(e)) if s != e => Some(e),
                    _ => None,
                };
                threads.push(PrThread {
                    id: first.id,
                    // GitHub's REST API can't tell us whether a review thread
                    // is resolved; GraphQL's `isResolved` is mapped onto the
                    // "closed" status so it plugs into
                    // `comment_status::RESOLVED_STATUSES` unchanged.
                    status: if node.is_resolved { "closed".to_string() } else { "active".to_string() },
                    file_path: node.path,
                    line: start_line,
                    line_end,
                    comments,
                });
            }

            if pr.review_threads.page_info.has_next_page {
                cursor = pr.review_threads.page_info.end_cursor;
            } else {
                break;
            }
        }

        Ok(threads)
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
        const QUERY: &str = r#"
            query($owner: String!, $repo: String!, $expression: String!) {
              repository(owner: $owner, name: $repo) {
                object(expression: $expression) {
                  ... on Blob { text }
                }
              }
            }
        "#;

        #[derive(Debug, Deserialize)]
        struct Data {
            repository: Option<RepoWrapper>,
        }
        #[derive(Debug, Deserialize)]
        struct RepoWrapper {
            object: Option<Blob>,
        }
        #[derive(Debug, Deserialize)]
        struct Blob {
            text: Option<String>,
        }

        let (owner, repo) = split_repository_id(repository_id, project);
        let result: Result<CodeSnippet> = async {
            let expression = format!("{commit_id}:{file_path}");
            let data: Data = self
                .graphql(QUERY, json!({ "owner": owner, "repo": repo, "expression": expression }))
                .await?;
            let text = data
                .repository
                .and_then(|r| r.object)
                .and_then(|o| o.text)
                .ok_or_else(|| anyhow!("file not found"))?;

            let all_lines: Vec<&str> = text.split('\n').collect();
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

#[derive(Debug, Deserialize)]
struct SearchData {
    search: SearchConnection,
}

#[derive(Debug, Deserialize)]
struct SearchConnection {
    nodes: Vec<SearchNode>,
    #[serde(rename = "pageInfo")]
    page_info: PageInfo,
}

/// One search result node. Every field is optional: a node that doesn't
/// resolve to `PullRequest` (shouldn't happen given `is:pr`, but the search
/// union isn't statically guaranteed) contributes no fields at all, and
/// [`extract_search_fields`] treats a node missing any required field as
/// unusable rather than failing the whole page.
#[derive(Debug, Deserialize)]
struct SearchNode {
    number: Option<i64>,
    title: Option<String>,
    state: Option<String>,
    #[serde(rename = "isDraft")]
    is_draft: Option<bool>,
    #[serde(rename = "createdAt")]
    created_at: Option<String>,
    url: Option<String>,
    merged: Option<bool>,
    mergeable: Option<String>,
    #[serde(rename = "baseRefName")]
    base_ref_name: Option<String>,
    repository: Option<RepoRef>,
    #[serde(default)]
    author: Option<AuthorRef>,
    #[serde(rename = "reviewRequests", default)]
    review_requests: Option<ReviewRequestConnection>,
}

/// Shared field shape between search results and a single-PR detail fetch —
/// [`get_pull_request`]'s query flattens this alongside `body`/`headRefOid`.
type SearchPrFields = SearchNode;

#[derive(Debug, Deserialize)]
struct RepoRef {
    name: String,
    owner: OwnerLogin,
}

#[derive(Debug, Deserialize)]
struct OwnerLogin {
    login: String,
}

#[derive(Debug, Deserialize)]
struct ReviewRequestConnection {
    nodes: Vec<ReviewRequestNode>,
}

#[derive(Debug, Deserialize)]
struct ReviewRequestNode {
    #[serde(rename = "requestedReviewer", default)]
    requested_reviewer: Option<RequestedReviewer>,
}

#[derive(Debug, Deserialize)]
struct RequestedReviewer {
    #[serde(default)]
    login: Option<String>,
}

/// Provider-neutral-adjacent fields extracted from a GraphQL PR node, ready
/// for [`normalize_pr`] once the owning repository is known.
struct RawPrFields {
    number: i64,
    title: String,
    state: String,
    is_draft: bool,
    created_at: String,
    url: String,
    merged: bool,
    mergeable: Option<String>,
    base_ref_name: String,
    author_login: Option<String>,
    reviewer_logins: Vec<String>,
}

fn reviewer_logins_from(connection: Option<ReviewRequestConnection>) -> Vec<String> {
    connection
        .map(|c| c.nodes)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|n| n.requested_reviewer)
        .filter_map(|r| r.login)
        .collect()
}

/// Extracts fields from a search result node, discarding nodes missing any
/// field required for normalization (see [`SearchNode`]'s doc comment).
fn extract_search_fields(node: SearchNode) -> Option<(RawPrFields, String, String)> {
    let repo = node.repository?;
    let fields = RawPrFields {
        number: node.number?,
        title: node.title?,
        state: node.state?,
        is_draft: node.is_draft.unwrap_or(false),
        created_at: node.created_at?,
        url: node.url?,
        merged: node.merged.unwrap_or(false),
        mergeable: node.mergeable,
        base_ref_name: node.base_ref_name?,
        author_login: node.author.map(|a| a.login),
        reviewer_logins: reviewer_logins_from(node.review_requests),
    };
    Some((fields, repo.owner.login, repo.name))
}

/// Extracts fields from a single-PR detail node (no `repository` field,
/// since the caller already knows owner/repo).
fn extract_pr_fields(node: SearchPrFields) -> Option<RawPrFields> {
    Some(RawPrFields {
        number: node.number?,
        title: node.title?,
        state: node.state?,
        is_draft: node.is_draft.unwrap_or(false),
        created_at: node.created_at?,
        url: node.url?,
        merged: node.merged.unwrap_or(false),
        mergeable: node.mergeable,
        base_ref_name: node.base_ref_name?,
        author_login: node.author.map(|a| a.login),
        reviewer_logins: reviewer_logins_from(node.review_requests),
    })
}

fn normalize_status(state: &str, merged: bool) -> String {
    if merged {
        "completed".to_string()
    } else if state.eq_ignore_ascii_case("open") {
        "active".to_string()
    } else {
        "abandoned".to_string()
    }
}

fn normalize_pr(fields: RawPrFields, owner: &str, repo_name: &str, viewer_login: &str) -> NormalizedPullRequest {
    let status = normalize_status(&fields.state, fields.merged);
    let merge_status = if fields.mergeable.as_deref() == Some("CONFLICTING") {
        "conflicts".to_string()
    } else {
        "none".to_string()
    };

    let reviewers: Vec<AdoReviewer> = fields
        .reviewer_logins
        .into_iter()
        .map(|login| {
            let is_current_user = login.eq_ignore_ascii_case(viewer_login);
            AdoReviewer {
                display_name: login,
                // GitHub's "requested reviewers" list is exactly the set of
                // people who haven't finished reviewing yet, so vote 0 (no
                // vote) is accurate, not just a placeholder — it makes
                // `needs_my_review` below work without needing a further
                // per-PR call for review state.
                vote: 0,
                is_required: false,
                is_current_user,
            }
        })
        .collect();

    let needs_my_review =
        !fields.is_draft && reviewers.iter().any(|r| r.is_current_user && r.vote == 0);

    NormalizedPullRequest {
        id: fields.number,
        title: fields.title,
        status,
        is_draft: fields.is_draft,
        repo_name: repo_name.to_string(),
        repository_id: format!("{owner}/{repo_name}"),
        author: fields.author_login.unwrap_or_default(),
        reviewers,
        target_branch: fields.base_ref_name,
        created_at: fields.created_at,
        url: fields.url,
        merge_status,
        needs_my_review,
    }
}

/// Splits a `"{owner}/{repo}"` repository id. Falls back to treating the
/// whole value as the repo name under `fallback_owner` if it somehow lacks a
/// slash — repository ids are always constructed as `owner/repo` by this
/// client, so this only guards against unexpected input.
fn split_repository_id(repository_id: &str, fallback_owner: &str) -> (String, String) {
    match repository_id.rsplit_once('/') {
        Some((owner, repo)) => (owner.to_string(), repo.to_string()),
        None => (fallback_owner.to_string(), repository_id.to_string()),
    }
}

/// Builds a GitHub search query string, pushing every filter the search API
/// can accept server-side (identity and lifecycle qualifiers); the rest are
/// applied afterwards via `matches_client_filters`, mirroring the Azure
/// DevOps client's server/client filter split.
fn build_search_query(project: &str, filters: &PullRequestFilters, viewer_login: &str) -> String {
    let mut parts: Vec<String> = vec!["is:pr".to_string()];

    if project.eq_ignore_ascii_case(viewer_login) {
        parts.push(format!("user:{project}"));
    } else {
        parts.push(format!("org:{project}"));
    }

    match filters.status {
        PullRequestStatus::Active => parts.push("is:open".to_string()),
        PullRequestStatus::Completed => parts.push("is:merged".to_string()),
        PullRequestStatus::Abandoned => {
            parts.push("is:closed".to_string());
            parts.push("is:unmerged".to_string());
        }
        PullRequestStatus::All => {}
    }

    if let Some(repo_name) = &filters.repository_name {
        parts.push(format!("repo:{project}/{repo_name}"));
    }
    if let Some(author_id) = &filters.author_id {
        parts.push(format!("author:{author_id}"));
    }
    if let Some(reviewer_id) = &filters.reviewer_id {
        parts.push(format!("review-requested:{reviewer_id}"));
    }

    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::types::DraftFilter;

    fn base_filters() -> PullRequestFilters {
        PullRequestFilters {
            status: PullRequestStatus::Active,
            ..Default::default()
        }
    }

    #[test]
    fn build_search_query_uses_user_qualifier_for_viewer() {
        let q = build_search_query("octocat", &base_filters(), "octocat");
        assert!(q.contains("user:octocat"));
        assert!(!q.contains("org:octocat"));
    }

    #[test]
    fn build_search_query_uses_org_qualifier_otherwise() {
        let q = build_search_query("acme-corp", &base_filters(), "octocat");
        assert!(q.contains("org:acme-corp"));
    }

    #[test]
    fn build_search_query_maps_status() {
        let mut filters = base_filters();
        filters.status = PullRequestStatus::Abandoned;
        let q = build_search_query("acme", &filters, "octocat");
        assert!(q.contains("is:closed"));
        assert!(q.contains("is:unmerged"));
    }

    #[test]
    fn build_search_query_pushes_repo_author_reviewer() {
        let mut filters = base_filters();
        filters.repository_name = Some("web".to_string());
        filters.author_id = Some("alice".to_string());
        filters.reviewer_id = Some("bob".to_string());
        let q = build_search_query("acme", &filters, "octocat");
        assert!(q.contains("repo:acme/web"));
        assert!(q.contains("author:alice"));
        assert!(q.contains("review-requested:bob"));
    }

    #[test]
    fn normalize_pr_maps_status_and_merge_conflicts() {
        let fields = RawPrFields {
            number: 42,
            title: "Fix bug".to_string(),
            state: "OPEN".to_string(),
            is_draft: false,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            url: "https://github.com/acme/web/pull/42".to_string(),
            merged: false,
            mergeable: Some("CONFLICTING".to_string()),
            base_ref_name: "main".to_string(),
            author_login: Some("alice".to_string()),
            reviewer_logins: vec!["octocat".to_string()],
        };
        let pr = normalize_pr(fields, "acme", "web", "octocat");
        assert_eq!(pr.status, "active");
        assert_eq!(pr.merge_status, "conflicts");
        assert_eq!(pr.repository_id, "acme/web");
        assert!(pr.needs_my_review);
        assert!(pr.reviewers[0].is_current_user);
    }

    #[test]
    fn normalize_pr_merged_wins_over_state() {
        let fields = RawPrFields {
            number: 1,
            title: "t".to_string(),
            state: "CLOSED".to_string(),
            is_draft: false,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            url: "u".to_string(),
            merged: true,
            mergeable: None,
            base_ref_name: "main".to_string(),
            author_login: None,
            reviewer_logins: vec![],
        };
        let pr = normalize_pr(fields, "acme", "web", "octocat");
        assert_eq!(pr.status, "completed");
    }

    #[test]
    fn normalize_pr_draft_never_needs_review() {
        let fields = RawPrFields {
            number: 1,
            title: "t".to_string(),
            state: "OPEN".to_string(),
            is_draft: true,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            url: "u".to_string(),
            merged: false,
            mergeable: None,
            base_ref_name: "main".to_string(),
            author_login: None,
            reviewer_logins: vec!["octocat".to_string()],
        };
        let pr = normalize_pr(fields, "acme", "web", "octocat");
        assert!(!pr.needs_my_review);
    }

    #[test]
    fn split_repository_id_splits_on_last_slash() {
        assert_eq!(
            split_repository_id("acme/web", "fallback"),
            ("acme".to_string(), "web".to_string())
        );
    }

    #[test]
    fn split_repository_id_falls_back_without_slash() {
        assert_eq!(
            split_repository_id("web", "acme"),
            ("acme".to_string(), "web".to_string())
        );
    }

    #[test]
    fn matches_client_filters_still_applies_draft_filter() {
        let fields = RawPrFields {
            number: 1,
            title: "t".to_string(),
            state: "OPEN".to_string(),
            is_draft: true,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            url: "u".to_string(),
            merged: false,
            mergeable: None,
            base_ref_name: "main".to_string(),
            author_login: None,
            reviewer_logins: vec![],
        };
        let pr = normalize_pr(fields, "acme", "web", "octocat");
        let mut filters = base_filters();
        filters.draft_filter = Some(DraftFilter::Hide);
        assert!(!matches_client_filters(&pr, &filters));
    }
}
