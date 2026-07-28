import type { AzureDevOpsProviderConfig } from "../../api/types.js";
import type { ProviderDefinition } from "../types.js";
import { resolveAzureDevOpsToken } from "./auth.js";
import { createAzureDevOpsClient } from "./client.js";
import { detectAzureDevOpsOrg } from "./detect-org.js";

export const azureDevOpsProvider: ProviderDefinition = {
  id: "azure-devops",
  name: "Azure DevOps",
  enabled: true,

  async detectConfig() {
    const organization = await detectAzureDevOpsOrg();
    return organization ? { organization } : {};
  },

  resolveToken: resolveAzureDevOpsToken,

  createClient(token, config) {
    return createAzureDevOpsClient(token, config as AzureDevOpsProviderConfig);
  },
};
