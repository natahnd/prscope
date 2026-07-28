import { getConfigPath, loadConfig, setConfigValues } from "../config/store.js";
import { getAzureDevOpsConfig } from "../api/types.js";

export async function showConfig(): Promise<void> {
  const config = await loadConfig();
  console.log(`Config file: ${getConfigPath()}`);
  console.log(JSON.stringify(config, null, 2));
}

export async function setConfig(options: {
  org?: string;
  project?: string;
  pat?: string;
}): Promise<void> {
  if (!options.org && !options.project && !options.pat) {
    console.error("Nothing to set. Use --org, --project, or --pat.");
    process.exit(1);
  }

  const config = await setConfigValues(options);
  console.log("Config updated:");
  console.log(JSON.stringify(config, null, 2));
}

export async function resolveOrganization(flagOrg?: string): Promise<string> {
  if (flagOrg) return flagOrg;
  const config = await loadConfig();
  const ado = getAzureDevOpsConfig(config);
  if (ado.organization) return ado.organization;
  throw new Error(
    "Organization required. Pass --org or configure Azure DevOps via the TUI.",
  );
}
