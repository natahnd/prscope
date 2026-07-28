import type { ProviderConfigs, ProviderId } from "../api/types.js";
import type {
  NormalizedPullRequest,
  Project,
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
