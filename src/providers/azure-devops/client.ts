import type {
  ApiListResponse,
  CodeSnippet,
  ConnectionData,
  NormalizedPullRequest,
  PrThread,
  Project,
  PullRequestDetail,
  PullRequestFilters,
  RawPullRequest,
  RawPullRequestDetail,
  RawThread,
  Repository,
} from "../../api/types.js";
import type { AzureDevOpsProviderConfig } from "../../api/types.js";
import type { ProviderClient } from "../types.js";

const API_VERSION = "7.0";
/** connectionData is preview-only; stable 7.x returns HTTP 400 without -preview */
const CONNECTION_DATA_API_VERSION = "7.0-preview.1";

export class AzureDevOpsClient implements ProviderClient {
  constructor(
    private readonly token: string,
    private readonly organization: string,
  ) {}

  private async fetchJson<T>(url: string): Promise<T> {
    const response = await fetch(url, {
      headers: {
        Authorization: `Bearer ${this.token}`,
        Accept: "application/json",
      },
    });

    if (!response.ok) {
      const body = await response.text();
      throw new Error(
        `Azure DevOps API error ${response.status}: ${body.slice(0, 200)}`,
      );
    }

    return (await response.json()) as T;
  }

  private async fetchText(url: string): Promise<string> {
    const response = await fetch(url, {
      headers: {
        Authorization: `Bearer ${this.token}`,
        Accept: "text/plain",
      },
    });

    if (!response.ok) {
      throw new Error(`Azure DevOps API error ${response.status}`);
    }

    return response.text();
  }

  private baseUrl(project?: string): string {
    if (project) {
      return `https://dev.azure.com/${this.organization}/${encodeURIComponent(project)}`;
    }
    return `https://dev.azure.com/${this.organization}`;
  }

  async getCurrentUser(): Promise<{ id: string; displayName: string }> {
    const data = await this.getConnectionData();
    return data.authenticatedUser;
  }

  async listProjects(): Promise<Project[]> {
    const projects: Project[] = [];
    let skip = 0;
    const top = 100;

    while (true) {
      const url =
        `${this.baseUrl()}/_apis/projects?` +
        `$top=${top}&$skip=${skip}&api-version=${API_VERSION}`;
      const data = await this.fetchJson<ApiListResponse<Project>>(url);
      projects.push(...data.value);
      if (data.value.length < top) break;
      skip += top;
    }

    return projects.sort((a, b) => a.name.localeCompare(b.name));
  }

  async listRepositories(project: string): Promise<Repository[]> {
    const url =
      `${this.baseUrl(project)}/_apis/git/repositories?api-version=${API_VERSION}`;
    const data = await this.fetchJson<ApiListResponse<Repository>>(url);
    return data.value.sort((a, b) => a.name.localeCompare(b.name));
  }

  private async getConnectionData(): Promise<ConnectionData> {
    const url =
      `${this.baseUrl()}/_apis/connectionData?api-version=${CONNECTION_DATA_API_VERSION}`;
    return this.fetchJson<ConnectionData>(url);
  }

  async listPullRequests(
    project: string,
    filters: PullRequestFilters,
    currentUserId?: string,
  ): Promise<NormalizedPullRequest[]> {
    const params = new URLSearchParams({ "api-version": API_VERSION });
    const status = filters.status === "all" ? undefined : filters.status;
    if (status) params.set("searchCriteria.status", status);
    if (filters.repositoryId) {
      params.set("searchCriteria.repositoryId", filters.repositoryId);
    }
    if (filters.authorId) {
      params.set("searchCriteria.creatorId", filters.authorId);
    }
    if (filters.reviewerId) {
      params.set("searchCriteria.reviewerId", filters.reviewerId);
    }

    const all: RawPullRequest[] = [];
    let skip = 0;
    const top = 100;

    while (true) {
      params.set("$top", String(top));
      params.set("$skip", String(skip));
      const url = `${this.baseUrl(project)}/_apis/git/pullrequests?${params.toString()}`;
      const data = await this.fetchJson<ApiListResponse<RawPullRequest>>(url);
      all.push(...data.value);
      if (data.value.length < top) break;
      skip += top;
    }

    return all
      .map((pr) => normalizePullRequest(pr, this.organization, project, currentUserId))
      .filter((pr) => matchesClientFilters(pr, filters));
  }

  async getPullRequest(
    project: string,
    repositoryId: string,
    pullRequestId: number,
  ): Promise<PullRequestDetail> {
    const url =
      `${this.baseUrl(project)}/_apis/git/repositories/${repositoryId}` +
      `/pullrequests/${pullRequestId}?api-version=${API_VERSION}`;
    const raw = await this.fetchJson<RawPullRequestDetail>(url);
    return {
      ...normalizePullRequest(raw, this.organization, project),
      description: raw.description ?? "",
      sourceCommitId: raw.lastMergeSourceCommit?.commitId,
    };
  }

  async listPullRequestThreads(
    project: string,
    repositoryId: string,
    pullRequestId: number,
  ): Promise<PrThread[]> {
    const url =
      `${this.baseUrl(project)}/_apis/git/repositories/${repositoryId}` +
      `/pullrequests/${pullRequestId}/threads?api-version=${API_VERSION}`;
    const data = await this.fetchJson<ApiListResponse<RawThread>>(url);

    // Azure DevOps also returns system threads (reviewer voted, status
    // changed, branch updated, etc). Those have no real feedback once their
    // system-generated comments are stripped in normalizeThread, but they
    // still carry a thread `status` (often "active", since ADO doesn't mark
    // them resolved) — so they must be dropped here, after normalizing,
    // rather than by the raw comment count, or they'd be miscounted as
    // unresolved human feedback.
    return data.value
      .filter((thread) => !thread.isDeleted)
      .map((thread) => normalizeThread(thread))
      .filter((thread) => thread.comments.length > 0);
  }

  async getFileSnippet(
    project: string,
    repositoryId: string,
    filePath: string,
    commitId: string,
    line: number,
    radius = 5,
  ): Promise<CodeSnippet | null> {
    try {
      const params = new URLSearchParams({
        path: filePath,
        "versionDescriptor.version": commitId,
        "versionDescriptor.versionType": "commit",
        "api-version": API_VERSION,
        $format: "text",
      });
      const url =
        `${this.baseUrl(project)}/_apis/git/repositories/${repositoryId}` +
        `/items?${params.toString()}`;
      const text = await this.fetchText(url);
      const allLines = text.split(/\r\n|\n/);

      const startLine = Math.max(1, line - radius);
      const endLine = Math.min(allLines.length, line + radius);
      const lines = [];
      for (let n = startLine; n <= endLine; n++) {
        lines.push({
          number: n,
          content: allLines[n - 1] ?? "",
          highlight: n === line,
        });
      }

      return { filePath, lines, centerLine: line };
    } catch {
      return null;
    }
  }
}

export function createAzureDevOpsClient(
  token: string,
  config: AzureDevOpsProviderConfig,
): AzureDevOpsClient {
  if (!config.organization) {
    throw new Error("Azure DevOps organization is required.");
  }
  return new AzureDevOpsClient(token, config.organization);
}

function normalizePullRequest(
  pr: RawPullRequest,
  organization: string,
  project: string,
  currentUserId?: string,
): NormalizedPullRequest {
  const repoName = pr.repository.name;
  const reviewers = (pr.reviewers ?? []).map((r) => ({
    displayName: r.displayName,
    vote: r.vote,
    isRequired: r.isRequired ?? false,
    isCurrentUser: currentUserId ? r.id === currentUserId : false,
  }));

  const needsMyReview = reviewers.some(
    (r) => r.isCurrentUser && r.vote === 0 && !pr.isDraft,
  );

  return {
    id: pr.pullRequestId,
    title: pr.title,
    status: pr.status,
    isDraft: pr.isDraft ?? false,
    repoName,
    repositoryId: pr.repository.id,
    author: pr.createdBy.displayName,
    reviewers,
    targetBranch: pr.targetRefName.replace(/^refs\/heads\//, ""),
    createdAt: pr.creationDate,
    url: `https://dev.azure.com/${organization}/${encodeURIComponent(project)}/_git/${encodeURIComponent(repoName)}/pullrequest/${pr.pullRequestId}`,
    mergeStatus: pr.mergeStatus,
    needsMyReview,
  };
}

function normalizeThread(thread: RawThread): PrThread {
  const context = thread.threadContext;
  const position = context?.rightFileStart ?? context?.leftFileStart;
  const endPosition = context?.rightFileEnd ?? context?.leftFileEnd;
  const lineEnd =
    endPosition && position && endPosition.line !== position.line ? endPosition.line : undefined;

  return {
    id: thread.id,
    status: thread.status ?? "unknown",
    filePath: context?.filePath,
    line: position?.line,
    lineEnd,
    comments: thread.comments
      .filter((c) => (c.commentType ?? "text") !== "system" && c.content)
      .map((c) => ({
        id: c.id,
        author: c.author.displayName,
        content: c.content ?? "",
        publishedAt: c.publishedDate,
      })),
  };
}

function matchesClientFilters(
  pr: NormalizedPullRequest,
  filters: PullRequestFilters,
): boolean {
  if (filters.repositoryName && pr.repoName !== filters.repositoryName) {
    return false;
  }
  if (filters.author && pr.author.toLowerCase() !== filters.author.toLowerCase()) {
    return false;
  }
  if (filters.reviewerMe) {
    if (!pr.reviewers.some((r) => r.isCurrentUser)) return false;
  }
  if (filters.draftFilter === "hide" && pr.isDraft) return false;
  if (filters.draftFilter === "only" && !pr.isDraft) return false;
  if (filters.search) {
    const q = filters.search.toLowerCase();
    const haystack = `${pr.repoName} ${pr.title} ${pr.author}`.toLowerCase();
    if (!haystack.includes(q)) return false;
  }
  return true;
}
