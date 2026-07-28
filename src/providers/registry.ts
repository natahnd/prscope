import type { AppConfig } from "../api/types.js";
import { azureDevOpsProvider } from "./azure-devops/index.js";
import { githubProvider } from "./github/index.js";
import type { ProviderDefinition, ProviderId } from "./types.js";

export const PROVIDERS: ProviderDefinition[] = [azureDevOpsProvider, githubProvider];

export function listProviders(): ProviderDefinition[] {
  return PROVIDERS;
}

export function getProvider(id: ProviderId): ProviderDefinition | undefined {
  return PROVIDERS.find((p) => p.id === id);
}

export function getActiveProvider(config: AppConfig): ProviderDefinition | undefined {
  if (!config.activeProvider) return undefined;
  return getProvider(config.activeProvider);
}

export async function resolveProviderToken(providerId: ProviderId): Promise<string> {
  const provider = getProvider(providerId);
  if (!provider) {
    throw new Error(`Unknown provider: ${providerId}`);
  }
  return provider.resolveToken();
}
