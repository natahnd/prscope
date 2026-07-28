import { readFile } from "node:fs/promises";
import { homedir } from "node:os";
import { join } from "node:path";
import { execFile } from "node:child_process";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);

const ORG_URL_PATTERN =
  /https?:\/\/dev\.azure\.com\/([^/\s]+)/i;

export async function detectAzureDevOpsOrg(): Promise<string | null> {
  const fromCli = await detectFromAzCli();
  if (fromCli) return fromCli;
  return detectFromConfigFile();
}

async function detectFromAzCli(): Promise<string | null> {
  try {
    const { stdout } = await execFileAsync(
      "az",
      ["devops", "configure", "--list"],
      { timeout: 10_000 },
    );
    return parseOrgFromConfigureOutput(stdout);
  } catch {
    return null;
  }
}

async function detectFromConfigFile(): Promise<string | null> {
  try {
    const path = join(homedir(), ".azure", "azuredevops", "config");
    const content = await readFile(path, "utf8");
    return parseOrgFromConfigureOutput(content);
  } catch {
    return null;
  }
}

function parseOrgFromConfigureOutput(text: string): string | null {
  for (const line of text.split("\n")) {
    const trimmed = line.trim();
    if (!trimmed.startsWith("organization")) continue;
    const value = trimmed.split("=").slice(1).join("=").trim();
    const match = value.match(ORG_URL_PATTERN);
    if (match?.[1]) return match[1];
    if (value && !value.includes("/")) return value;
  }
  return null;
}
