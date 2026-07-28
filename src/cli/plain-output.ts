import chalk from "chalk";
import { formatDistanceToNow } from "date-fns";
import type { NormalizedPullRequest } from "../api/types.js";

export function printPlainTable(prs: NormalizedPullRequest[]): void {
  if (prs.length === 0) {
    console.log("No pull requests found.");
    return;
  }

  const headers = ["Repo", "#", "Title", "Author", "Reviewers", "Age", "Status"];
  const rows = prs.map((pr) => [
    pr.repoName,
    String(pr.id),
    truncate(pr.title, 40),
    truncate(pr.author, 16),
    formatReviewers(pr),
    formatDistanceToNow(new Date(pr.createdAt), { addSuffix: false }),
    pr.isDraft ? "draft" : pr.status,
  ]);

  const widths = headers.map((h, i) =>
    Math.max(h.length, ...rows.map((r) => r[i]?.length ?? 0)),
  );

  console.log(headers.map((h, i) => pad(h, widths[i]!)).join("  "));
  console.log(widths.map((w) => "-".repeat(w)).join("  "));
  for (const row of rows) {
    console.log(row.map((cell, i) => pad(cell, widths[i]!)).join("  "));
  }
}

export function printPlainJson(prs: NormalizedPullRequest[]): void {
  console.log(JSON.stringify(prs, null, 2));
}

function formatReviewers(pr: NormalizedPullRequest): string {
  if (pr.reviewers.length === 0) return "-";
  return pr.reviewers
    .map((r) => {
      const vote =
        r.vote === 10 ? "✓" : r.vote === -10 ? "✗" : r.isCurrentUser ? "⚠" : "○";
      const name = r.isCurrentUser ? "you" : r.displayName.split(" ")[0];
      return `${vote}${name}`;
    })
    .join(",");
}

function truncate(value: string, max: number): string {
  return value.length > max ? `${value.slice(0, max - 1)}…` : value;
}

function pad(value: string, width: number): string {
  return value.padEnd(width);
}

export function printError(message: string): void {
  console.error(chalk.red(`Error: ${message}`));
}
