import type { PullRequestFilters, PullRequestStatus } from "../api/types.js";

export interface ListCommandOptions {
  org?: string;
  project?: string;
  status?: string;
  repo?: string;
  author?: string;
  reviewer?: string;
  mine?: boolean;
  plain?: boolean;
  format?: string;
}

export function buildFilters(
  options: ListCommandOptions,
  currentUserId?: string,
): PullRequestFilters {
  const status = parseStatus(options.status);
  const filters: PullRequestFilters = { status };

  if (options.repo) {
    filters.repositoryName = options.repo;
  }

  if (options.mine && currentUserId) {
    filters.authorMe = true;
    filters.authorId = currentUserId;
  } else if (options.author) {
    filters.author = options.author;
  }

  if (options.reviewer === "me" && currentUserId) {
    filters.reviewerMe = true;
    filters.reviewerId = currentUserId;
  }

  return filters;
}

function parseStatus(value?: string): PullRequestStatus {
  if (!value || value === "active") return "active";
  if (value === "completed" || value === "abandoned" || value === "all") {
    return value;
  }
  throw new Error(`Invalid status "${value}". Use active, completed, abandoned, or all.`);
}
