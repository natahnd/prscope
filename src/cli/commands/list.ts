import type { Command } from "commander";
import { render } from "ink";
import { createElement } from "react";
import { createAzureDevOpsClient } from "../../providers/azure-devops/client.js";
import { resolveProviderToken } from "../../providers/registry.js";
import { printError, printPlainJson, printPlainTable } from "../plain-output.js";
import { resolveOrganization } from "../config-helpers.js";
import { buildFilters, type ListCommandOptions } from "../../filters/apply.js";
import { getLastFilters, loadConfig } from "../../config/store.js";
import { App } from "../../app/App.js";

// CLI flags that map to PullRequestFilters fields. If the user explicitly
// passes any of these, that one-off run should win over whatever filters
// were last saved from the TUI — only fall back to saved filters when the
// user launched with none of them.
const FILTER_OPTION_KEYS = ["status", "repo", "author", "reviewer", "mine"] as const;

function hasExplicitFilterFlags(command?: Command): boolean {
  if (!command) return false;
  return FILTER_OPTION_KEYS.some((key) => command.getOptionValueSource(key) === "cli");
}

export async function runListCommand(options: ListCommandOptions, command?: Command): Promise<void> {
  try {
    if (options.plain) {
      const token = await resolveProviderToken("azure-devops");
      const organization = await resolveOrganization(options.org);
      const client = createAzureDevOpsClient(token, { organization });
      const connection = await client.getCurrentUser();
      const currentUserId = connection.id;
      const filters = buildFilters(options, currentUserId);

      if (!options.project) {
        throw new Error("--project is required with --plain mode.");
      }
      const prs = await client.listPullRequests(options.project, filters, currentUserId);
      if (options.format === "json") {
        printPlainJson(prs);
      } else {
        printPlainTable(prs);
      }
      return;
    }

    const savedFilters = hasExplicitFilterFlags(command)
      ? undefined
      : getLastFilters(await loadConfig());

    render(
      createElement(App, {
        initialProject: options.project,
        initialFilters: savedFilters ?? buildFilters(options),
      }),
    );
  } catch (error) {
    printError(error instanceof Error ? error.message : String(error));
    process.exit(1);
  }
}
