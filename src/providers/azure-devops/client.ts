import type {
  ApiListResponse,
  ConnectionData,
  NormalizedPullRequest,
  Project,
  PullRequestFilters,
  RawPullRequest,
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
    author: pr.createdBy.displayName,
    reviewers,
    targetBranch: pr.targetRefName.replace(/^refs\/heads\//, ""),
    createdAt: pr.creationDate,
    url: `https://dev.azure.com/${organization}/${encodeURIComponent(project)}/_git/${encodeURIComponent(repoName)}/pullrequest/${pr.pullRequestId}`,
    mergeStatus: pr.mergeStatus,
    needsMyReview,
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
  if (filters.search) {
    const q = filters.search.toLowerCase();
    const haystack = `${pr.repoName} ${pr.title} ${pr.author}`.toLowerCase();
    if (!haystack.includes(q)) return false;
  }
  return true;
}
