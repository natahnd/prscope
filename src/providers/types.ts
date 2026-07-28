import type { ProviderConfigs, ProviderId } from "../api/types.js";
import type {
  CodeSnippet,
  NormalizedPullRequest,
  Project,
  PrThread,
  PullRequestDetail,
  PullRequestFilters,
  Repository,
} from "../api/types.js";

export type { ProviderId };

export type ProviderConfig = NonNullable<ProviderConfigs[ProviderId]>;

export interface ProviderClient {
  getCurrentUser(): Promise<{ id: string; displayName: string }>;
  listProjects(): Promise<Project[]>;
  listRepositories(project: string): Promise<Repository[]>;
  listPullRequests(
    project: string,
    filters: PullRequestFilters,
    currentUserId?: string,
  ): Promise<NormalizedPullRequest[]>;
  getPullRequest(
    project: string,
    repositoryId: string,
    pullRequestId: number,
  ): Promise<PullRequestDetail>;
  listPullRequestThreads(
    project: string,
    repositoryId: string,
    pullRequestId: number,
  ): Promise<PrThread[]>;
  getFileSnippet(
    project: string,
    repositoryId: string,
    filePath: string,
    commitId: string,
    line: number,
    radius?: number,
  ): Promise<CodeSnippet | null>;
}

export interface ProviderDefinition {
  id: ProviderId;
  name: string;
  enabled: boolean;
  detectConfig?(): Promise<Partial<ProviderConfig>>;
  resolveToken(): Promise<string>;
  createClient(token: string, config: ProviderConfig): ProviderClient;
}

export interface ProviderSetupProps {
  detectedHint?: string;
  initialValue?: string;
  loading: boolean;
  error?: string;
  onSubmit: (value: string) => void;
  onQuit: () => void;
}
