import type { ProviderId } from "../providers/types.js";
import { resolveProviderToken } from "../providers/registry.js";

export async function resolveToken(providerId: ProviderId = "azure-devops"): Promise<string> {
  return resolveProviderToken(providerId);
}

export { resolveProviderToken };
