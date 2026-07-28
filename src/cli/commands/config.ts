import { showConfig, setConfig } from "../config-helpers.js";

export async function runConfigShow(): Promise<void> {
  await showConfig();
}

export async function runConfigSet(options: {
  org?: string;
  project?: string;
  pat?: string;
}): Promise<void> {
  await setConfig(options);
}
