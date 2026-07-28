import { Box, Text, useInput } from "ink";
import { useState } from "react";
import type { AdoRepository, PullRequestFilters, PullRequestStatus } from "../../api/types.js";

export interface FilterModalState {
  status: PullRequestStatus;
  repositoryName?: string;
  reviewerMe: boolean;
  authorMe: boolean;
}

interface FilterModalProps {
  repositories: AdoRepository[];
  initial: FilterModalState;
  onApply: (state: FilterModalState) => void;
  onClose: () => void;
}

const STATUS_OPTIONS: PullRequestStatus[] = ["active", "completed", "abandoned", "all"];

export function FilterModal({ repositories, initial, onApply, onClose }: FilterModalProps) {
  const [statusIndex, setStatusIndex] = useState(
    Math.max(0, STATUS_OPTIONS.indexOf(initial.status)),
  );
  const [repoIndex, setRepoIndex] = useState(() => {
    if (!initial.repositoryName) return 0;
    const idx = repositories.findIndex((r) => r.name === initial.repositoryName);
    return idx >= 0 ? idx + 1 : 0;
  });
  const [reviewerMe, setReviewerMe] = useState(initial.reviewerMe);
  const [authorMe, setAuthorMe] = useState(initial.authorMe);
  const [section, setSection] = useState<"status" | "repo" | "toggles">("status");

  useInput((input, key) => {
    if (key.escape) {
      onClose();
      return;
    }
    if (key.return) {
      onApply({
        status: STATUS_OPTIONS[statusIndex] ?? "active",
        repositoryName: repoIndex === 0 ? undefined : repositories[repoIndex - 1]?.name,
        reviewerMe,
        authorMe,
      });
      return;
    }
    if (input === "j" || key.downArrow) {
      if (section === "status") setStatusIndex((i) => Math.min(STATUS_OPTIONS.length - 1, i + 1));
      else if (section === "repo") setRepoIndex((i) => Math.min(repositories.length, i + 1));
      else {
        if (!reviewerMe) setReviewerMe(true);
        else setAuthorMe(true);
      }
      return;
    }
    if (input === "k" || key.upArrow) {
      if (section === "status") setStatusIndex((i) => Math.max(0, i - 1));
      else if (section === "repo") setRepoIndex((i) => Math.max(0, i - 1));
      else {
        if (authorMe) setAuthorMe(false);
        else setReviewerMe(false);
      }
      return;
    }
    if (input === "h" || key.leftArrow) {
      setSection((s) => (s === "toggles" ? "repo" : s === "repo" ? "status" : s));
      return;
    }
    if (input === "l" || key.rightArrow) {
      setSection((s) => (s === "status" ? "repo" : s === "repo" ? "toggles" : s));
      return;
    }
    if (input === " ") {
      if (section === "toggles") {
        if (reviewerMe && authorMe) setAuthorMe(false);
        else if (reviewerMe) setAuthorMe(true);
        else setReviewerMe(true);
      }
    }
  });

  const repoOptions = ["All repos", ...repositories.map((r) => r.name)];

  return (
    <Box
      flexDirection="column"
      borderStyle="double"
      borderColor="yellow"
      paddingX={1}
      marginTop={1}
    >
      <Text bold color="yellow">
        Filters
      </Text>
      <Box marginTop={1}>
        <Box flexDirection="column" width={16}>
          <Text bold={section === "status"}>Status</Text>
          {STATUS_OPTIONS.map((status, i) => (
            <Text key={status} color={i === statusIndex ? "cyan" : undefined}>
              {i === statusIndex ? "▶ " : "  "}
              {status}
            </Text>
          ))}
        </Box>
        <Box flexDirection="column" width={24} marginLeft={2}>
          <Text bold={section === "repo"}>Repository</Text>
          {repoOptions.map((name, i) => (
            <Text key={name} color={i === repoIndex ? "cyan" : undefined}>
              {i === repoIndex ? "▶ " : "  "}
              {name}
            </Text>
          ))}
        </Box>
        <Box flexDirection="column" marginLeft={2}>
          <Text bold={section === "toggles"}>People</Text>
          <Text color={reviewerMe ? "cyan" : undefined}>
            {reviewerMe ? "[x]" : "[ ]"} Reviewer: me
          </Text>
          <Text color={authorMe ? "cyan" : undefined}>
            {authorMe ? "[x]" : "[ ]"} Author: me
          </Text>
        </Box>
      </Box>
    </Box>
  );
}

export function filtersToLabels(filters: PullRequestFilters): string[] {
  const labels: string[] = [filters.status];
  if (filters.repositoryName) labels.push(`repo:${filters.repositoryName}`);
  if (filters.reviewerMe) labels.push("reviewer:me");
  if (filters.authorMe) labels.push("author:me");
  if (filters.search) labels.push(`search:${filters.search}`);
  return labels;
}

export function modalStateToFilters(
  state: FilterModalState,
  currentUserId: string,
): PullRequestFilters {
  return {
    status: state.status,
    repositoryName: state.repositoryName,
    reviewerMe: state.reviewerMe,
    reviewerId: state.reviewerMe ? currentUserId : undefined,
    authorMe: state.authorMe,
    authorId: state.authorMe ? currentUserId : undefined,
  };
}

export function filtersToModalState(filters: PullRequestFilters): FilterModalState {
  return {
    status: filters.status,
    repositoryName: filters.repositoryName,
    reviewerMe: filters.reviewerMe ?? false,
    authorMe: filters.authorMe ?? false,
  };
}
