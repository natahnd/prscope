import { loadConfig } from "../../config/store.js";
import { execFile } from "node:child_process";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);
const AZURE_DEVOPS_RESOURCE = "499b84ac-1321-427f-aa17-267ca6975798";

export async function resolveAzureDevOpsToken(): Promise<string> {
  const azToken = await getTokenFromAzCli();
  if (azToken) return azToken;

  if (process.env.AZURE_DEVOPS_PAT) {
    return process.env.AZURE_DEVOPS_PAT;
  }

  const config = await loadConfig();
  if (config.pat) return config.pat;

  throw new Error(
    "Authentication failed. Run `az login` or set AZURE_DEVOPS_PAT environment variable.",
  );
}

async function getTokenFromAzCli(): Promise<string | null> {
  try {
    const { stdout } = await execFileAsync(
      "az",
      [
        "account",
        "get-access-token",
        "--resource",
        AZURE_DEVOPS_RESOURCE,
        "--query",
        "accessToken",
        "-o",
        "tsv",
      ],
      { timeout: 15_000 },
    );
    const token = stdout.trim();
    return token.length > 0 ? token : null;
  } catch {
    return null;
  }
}
