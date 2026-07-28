import type { ProviderDefinition } from "../types.js";

export const githubProvider: ProviderDefinition = {
  id: "github",
  name: "GitHub (coming soon)",
  enabled: false,

  async resolveToken() {
    throw new Error("GitHub provider is not yet available.");
  },

  createClient() {
    throw new Error("GitHub provider is not yet available.");
  },
};
