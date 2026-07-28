export type PullRequestStatus = "active" | "completed" | "abandoned" | "all";
export type DraftFilter = "all" | "hide" | "only";
export type CommentActivityFilter = "all" | "hide" | "only";

export type ProviderId = "azure-devops" | "github";

export interface AzureDevOpsProviderConfig {
  organization?: string;
  lastProject?: string;
}

export interface GitHubProviderConfig {
  owner?: string;
  lastRepo?: string;
}

export interface ProviderConfigs {
  "azure-devops"?: AzureDevOpsProviderConfig;
  github?: GitHubProviderConfig;
}

/** @deprecated v1 flat fields — migrated automatically on load */
export interface LegacyAppConfig {
  organization?: string;
  lastProject?: string;
  pat?: string;
}

export interface AppConfigV2 {
  version: 2;
  activeProvider?: ProviderId;
  providers: ProviderConfigs;
  pat?: string;
}

export type AppConfig = AppConfigV2 & LegacyAppConfig;

export interface AdoIdentity {
  id: string;
  displayName: string;
}

export interface AdoReviewer {
  displayName: string;
  vote: number;
  isRequired: boolean;
  isCurrentUser: boolean;
}

export interface NormalizedPullRequest {
  id: number;
  title: string;
  status: string;
  isDraft: boolean;
  repoName: string;
  repositoryId: string;
  author: string;
  reviewers: AdoReviewer[];
  targetBranch: string;
  createdAt: string;
  url: string;
  mergeStatus: string;
  needsMyReview: boolean;
  description?: string;
}

export interface PullRequestDetail extends NormalizedPullRequest {
  description: string;
  sourceCommitId?: string;
}

export interface PrComment {
  id: number;
  author: string;
  content: string;
  publishedAt: string;
}

export interface PrThread {
  id: number;
  status: string;
  filePath?: string;
  line?: number;
  lineEnd?: number;
  comments: PrComment[];
}

export interface CodeSnippetLine {
  number: number;
  content: string;
  highlight: boolean;
}

export interface CodeSnippet {
  filePath: string;
  lines: CodeSnippetLine[];
  centerLine: number;
}

export interface PullRequestFilters {
  status: PullRequestStatus;
  repositoryId?: string;
  repositoryName?: string;
  author?: string;
  authorId?: string;
  reviewerId?: string;
  reviewerMe?: boolean;
  authorMe?: boolean;
  draftFilter?: DraftFilter;
  /** Client-only: filters on whether a PR has any unresolved comment thread. */
  activeCommentsFilter?: CommentActivityFilter;
  search?: string;
}

export type Project = AdoProject;
export type Repository = AdoRepository;

export interface AdoProject {
  id: string;
  name: string;
  description?: string;
}

export interface AdoRepository {
  id: string;
  name: string;
}

export interface ConnectionData {
  authenticatedUser: AdoIdentity;
}

export interface ApiListResponse<T> {
  count: number;
  value: T[];
}

export interface RawPullRequest {
  pullRequestId: number;
  title: string;
  status: string;
  isDraft?: boolean;
  creationDate: string;
  targetRefName: string;
  mergeStatus: string;
  repository: { id: string; name: string };
  createdBy: { id: string; displayName: string };
  reviewers?: Array<{
    id: string;
    displayName: string;
    vote: number;
    isRequired?: boolean;
  }>;
}

export interface RawPullRequestDetail extends RawPullRequest {
  description?: string;
  lastMergeSourceCommit?: { commitId: string };
}

export interface RawCommentPosition {
  line: number;
  offset: number;
}

export interface RawCommentThreadContext {
  filePath: string;
  rightFileStart?: RawCommentPosition;
  rightFileEnd?: RawCommentPosition;
  leftFileStart?: RawCommentPosition;
  leftFileEnd?: RawCommentPosition;
}

export interface RawComment {
  id: number;
  content?: string;
  publishedDate: string;
  commentType?: string;
  author: { displayName: string };
}

export interface RawThread {
  id: number;
  status?: string;
  isDeleted?: boolean;
  threadContext?: RawCommentThreadContext | null;
  comments: RawComment[];
}

export type AppScreen =
  | "providerSelect"
  | "providerSetup"
  | "projectSelect"
  | "prList";

export function isConfiguredProvider(config: AppConfig): boolean {
  if (config.version !== 2 || !config.activeProvider) return false;
  const providerConfig = config.providers[config.activeProvider];
  if (!providerConfig) return false;
  if (config.activeProvider === "azure-devops") {
    return Boolean(
      (providerConfig as AzureDevOpsProviderConfig).organization,
    );
  }
  return false;
}

export function getAzureDevOpsConfig(config: AppConfig): AzureDevOpsProviderConfig {
  if (config.version === 2 && config.providers["azure-devops"]) {
    return config.providers["azure-devops"];
  }
  return {
    organization: config.organization,
    lastProject: config.lastProject,
  };
}
