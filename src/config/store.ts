import { mkdir, readFile, writeFile } from "node:fs/promises";
import { homedir } from "node:os";
import { dirname, join } from "node:path";
import type {
  AppConfig,
  AppConfigV2,
  AzureDevOpsProviderConfig,
} from "../api/types.js";
import type { ProviderId } from "../api/types.js";

function configDir(): string {
  const xdg = process.env.XDG_CONFIG_HOME;
  if (xdg) return join(xdg, "pr-reviews");
  return join(homedir(), ".config", "pr-reviews");
}

function configPath(): string {
  return join(configDir(), "config.json");
}

function migrateConfig(raw: AppConfig): AppConfigV2 {
  if (raw.version === 2) {
    return {
      version: 2,
      activeProvider: raw.activeProvider,
      providers: raw.providers ?? {},
      pat: raw.pat,
    };
  }

  const ado: AzureDevOpsProviderConfig = {};
  if (raw.organization) ado.organization = raw.organization;
  if (raw.lastProject) ado.lastProject = raw.lastProject;

  const migrated: AppConfigV2 = {
    version: 2,
    providers: {},
    pat: raw.pat,
  };

  if (ado.organization || ado.lastProject) {
    migrated.activeProvider = "azure-devops";
    migrated.providers["azure-devops"] = ado;
  }

  return migrated;
}

export async function loadConfig(): Promise<AppConfigV2> {
  try {
    const raw = JSON.parse(await readFile(configPath(), "utf8")) as AppConfig;
    const config = migrateConfig(raw);
    if (raw.version !== 2) {
      await saveConfig(config);
    }
    return config;
  } catch {
    return { version: 2, providers: {} };
  }
}

export async function saveConfig(config: AppConfigV2): Promise<void> {
  const path = configPath();
  await mkdir(dirname(path), { recursive: true });
  await writeFile(path, `${JSON.stringify(config, null, 2)}\n`, { mode: 0o600 });
}

export async function updateLastProject(project: string): Promise<void> {
  const config = await loadConfig();
  if (!config.activeProvider) return;
  const providerConfig = config.providers[config.activeProvider] ?? {};
  if (config.activeProvider === "azure-devops") {
    config.providers["azure-devops"] = {
      ...(providerConfig as AzureDevOpsProviderConfig),
      lastProject: project,
    };
  }
  await saveConfig(config);
}

export async function saveProviderConfig(
  providerId: ProviderId,
  providerConfig: Record<string, string | undefined>,
): Promise<AppConfigV2> {
  const config = await loadConfig();
  config.activeProvider = providerId;
  config.providers[providerId] = {
    ...config.providers[providerId],
    ...providerConfig,
  };
  await saveConfig(config);
  return config;
}

export async function setConfigValues(values: {
  org?: string;
  project?: string;
  pat?: string;
}): Promise<AppConfigV2> {
  const config = await loadConfig();

  if (values.pat) {
    config.pat = values.pat;
  }

  if (values.org || values.project) {
    config.activeProvider = "azure-devops";
    const ado = { ...(config.providers["azure-devops"] ?? {}) };
    if (values.org) ado.organization = values.org;
    if (values.project) ado.lastProject = values.project;
    config.providers["azure-devops"] = ado;
  }

  await saveConfig(config);
  return config;
}

export function getConfigPath(): string {
  return configPath();
}

export function getProviderDisplayContext(config: AppConfigV2): string {
  if (config.activeProvider === "azure-devops") {
    return config.providers["azure-devops"]?.organization ?? "";
  }
  return config.activeProvider ?? "";
}

export function getLastProject(config: AppConfigV2): string | undefined {
  if (config.activeProvider === "azure-devops") {
    return config.providers["azure-devops"]?.lastProject;
  }
  return undefined;
}
