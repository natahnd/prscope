import { Command } from "commander";
import { runListCommand } from "./commands/list.js";
import { runConfigSet, runConfigShow } from "./commands/config.js";

function addListOptions(command: Command): Command {
  return command
    .option("-o, --org <organization>", "Azure DevOps organization")
    .option("-p, --project <project>", "Azure DevOps project (skips picker)")
    .option("-s, --status <status>", "PR status filter", "active")
    .option("--repo <repository>", "Filter by repository name")
    .option("--author <author>", "Filter by author display name")
    .option("--reviewer <reviewer>", 'Filter by reviewer ("me" supported)')
    .option("--mine", "Show only PRs created by you")
    .option("--plain", "Plain output (no TUI)")
    .option("--format <format>", "Output format for --plain (table|json)", "table");
}

export function createProgram(): Command {
  const program = new Command();

  program
    .name("pr-reviews")
    .description("Azure DevOps pull request review TUI")
    .version("0.1.0");

  addListOptions(
    program
      .command("list", { isDefault: true })
      .description("List pull requests (interactive TUI by default)")
      .action(async (options) => {
        await runListCommand(options);
      }),
  );

  const config = program.command("config").description("Manage configuration");

  config
    .command("show")
    .description("Show current configuration")
    .action(async () => {
      await runConfigShow();
    });

  config
    .command("set")
    .description("Set configuration values")
    .option("-o, --org <organization>", "Default organization")
    .option("-p, --project <project>", "Default project (lastProject)")
    .option("--pat <token>", "Personal access token")
    .action(async (options) => {
      await runConfigSet(options);
    });

  return program;
}
