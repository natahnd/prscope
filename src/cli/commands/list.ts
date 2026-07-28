import { render } from "ink";
import { createElement } from "react";
import { createAzureDevOpsClient } from "../../providers/azure-devops/client.js";
import { resolveProviderToken } from "../../providers/registry.js";
import { printError, printPlainJson, printPlainTable } from "../plain-output.js";
import { resolveOrganization } from "../config-helpers.js";
import { buildFilters, type ListCommandOptions } from "../../filters/apply.js";
import { App } from "../../app/App.js";

export async function runListCommand(options: ListCommandOptions): Promise<void> {
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

    render(
      createElement(App, {
        initialProject: options.project,
        initialFilters: buildFilters(options),
      }),
    );
  } catch (error) {
    printError(error instanceof Error ? error.message : String(error));
    process.exit(1);
  }
}
