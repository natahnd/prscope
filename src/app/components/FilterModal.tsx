import { Box, Text, useInput } from "ink";
import { useState } from "react";
import type {
  AdoRepository,
  CommentActivityFilter,
  DraftFilter,
  PullRequestFilters,
  PullRequestStatus,
} from "../../api/types.js";

export interface FilterModalState {
  status: PullRequestStatus;
  repositoryName?: string;
  reviewerMe: boolean;
  authorMe: boolean;
  draftFilter: DraftFilter;
  activeCommentsFilter: CommentActivityFilter;
}

interface FilterModalProps {
  repositories: AdoRepository[];
  initial: FilterModalState;
  onApply: (state: FilterModalState) => void;
  onClose: () => void;
}

const STATUS_OPTIONS: PullRequestStatus[] = ["active", "completed", "abandoned", "all"];
const DRAFT_OPTIONS: { value: DraftFilter; label: string }[] = [
  { value: "all", label: "All PRs" },
  { value: "hide", label: "Hide drafts" },
  { value: "only", label: "Only drafts" },
];
const COMMENT_OPTIONS: { value: CommentActivityFilter; label: string }[] = [
  { value: "all", label: "All PRs" },
  { value: "hide", label: "Hide active comments" },
  { value: "only", label: "Only active comments" },
];

export function FilterModal({ repositories, initial, onApply, onClose }: FilterModalProps) {
  const [statusIndex, setStatusIndex] = useState(
    Math.max(0, STATUS_OPTIONS.indexOf(initial.status)),
  );
  const [repoIndex, setRepoIndex] = useState(() => {
    if (!initial.repositoryName) return 0;
    const idx = repositories.findIndex((r) => r.name === initial.repositoryName);
    return idx >= 0 ? idx + 1 : 0;
  });
  const [draftIndex, setDraftIndex] = useState(
    Math.max(
      0,
      DRAFT_OPTIONS.findIndex((o) => o.value === initial.draftFilter),
    ),
  );
  const [commentIndex, setCommentIndex] = useState(
    Math.max(
      0,
      COMMENT_OPTIONS.findIndex((o) => o.value === initial.activeCommentsFilter),
    ),
  );
  const [reviewerMe, setReviewerMe] = useState(initial.reviewerMe);
  const [authorMe, setAuthorMe] = useState(initial.authorMe);
  const [toggleIndex, setToggleIndex] = useState(0); // 0 = Reviewer: me, 1 = Author: me
  const SECTIONS = ["status", "repo", "draft", "comments", "toggles"] as const;
  type Section = (typeof SECTIONS)[number];
  const [section, setSection] = useState<Section>("status");

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
        draftFilter: DRAFT_OPTIONS[draftIndex]?.value ?? "all",
        activeCommentsFilter: COMMENT_OPTIONS[commentIndex]?.value ?? "all",
      });
      return;
    }
    if (input === "j" || key.downArrow) {
      if (section === "status") setStatusIndex((i) => Math.min(STATUS_OPTIONS.length - 1, i + 1));
      else if (section === "repo") setRepoIndex((i) => Math.min(repositories.length, i + 1));
      else if (section === "draft")
        setDraftIndex((i) => Math.min(DRAFT_OPTIONS.length - 1, i + 1));
      else if (section === "comments")
        setCommentIndex((i) => Math.min(COMMENT_OPTIONS.length - 1, i + 1));
      else setToggleIndex((i) => Math.min(1, i + 1));
      return;
    }
    if (input === "k" || key.upArrow) {
      if (section === "status") setStatusIndex((i) => Math.max(0, i - 1));
      else if (section === "repo") setRepoIndex((i) => Math.max(0, i - 1));
      else if (section === "draft") setDraftIndex((i) => Math.max(0, i - 1));
      else if (section === "comments") setCommentIndex((i) => Math.max(0, i - 1));
      else setToggleIndex((i) => Math.max(0, i - 1));
      return;
    }
    if (input === "h" || key.leftArrow) {
      setSection((s) => SECTIONS[Math.max(0, SECTIONS.indexOf(s) - 1)]);
      return;
    }
    if (input === "l" || key.rightArrow) {
      setSection((s) => SECTIONS[Math.min(SECTIONS.length - 1, SECTIONS.indexOf(s) + 1)]);
      return;
    }
    if (input === " ") {
      if (section === "toggles") {
        if (toggleIndex === 0) setReviewerMe((v) => !v);
        else setAuthorMe((v) => !v);
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
            <Text key={status} wrap="truncate-end" color={i === statusIndex ? "cyan" : undefined}>
              {i === statusIndex ? "▶ " : "  "}
              {status}
            </Text>
          ))}
        </Box>
        <Box flexDirection="column" width={26} marginLeft={2}>
          <Text bold={section === "repo"}>Repository</Text>
          {repoOptions.map((name, i) => (
            <Text key={name} wrap="truncate-end" color={i === repoIndex ? "cyan" : undefined}>
              {i === repoIndex ? "▶ " : "  "}
              {name}
            </Text>
          ))}
        </Box>
        <Box flexDirection="column" width={16} marginLeft={2}>
          <Text bold={section === "draft"}>Drafts</Text>
          {DRAFT_OPTIONS.map((option, i) => (
            <Text key={option.value} wrap="truncate-end" color={i === draftIndex ? "cyan" : undefined}>
              {i === draftIndex ? "▶ " : "  "}
              {option.label}
            </Text>
          ))}
        </Box>
        <Box flexDirection="column" width={24} marginLeft={2}>
          <Text bold={section === "comments"}>Comments</Text>
          {COMMENT_OPTIONS.map((option, i) => (
            <Text
              key={option.value}
              wrap="truncate-end"
              color={i === commentIndex ? "cyan" : undefined}
            >
              {i === commentIndex ? "▶ " : "  "}
              {option.label}
            </Text>
          ))}
        </Box>
        <Box flexDirection="column" marginLeft={2}>
          <Text bold={section === "toggles"}>People</Text>
          <Text
            wrap="truncate-end"
            color={section === "toggles" && toggleIndex === 0 ? "cyan" : undefined}
          >
            {section === "toggles" && toggleIndex === 0 ? "▶ " : "  "}
            {reviewerMe ? "[x]" : "[ ]"} Reviewer: me
          </Text>
          <Text
            wrap="truncate-end"
            color={section === "toggles" && toggleIndex === 1 ? "cyan" : undefined}
          >
            {section === "toggles" && toggleIndex === 1 ? "▶ " : "  "}
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
  if (filters.draftFilter && filters.draftFilter !== "all") {
    labels.push(filters.draftFilter === "hide" ? "drafts:hidden" : "drafts:only");
  }
  if (filters.activeCommentsFilter && filters.activeCommentsFilter !== "all") {
    labels.push(
      filters.activeCommentsFilter === "hide" ? "comments:actioned" : "comments:active",
    );
  }
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
    draftFilter: state.draftFilter,
    activeCommentsFilter: state.activeCommentsFilter,
  };
}

export function filtersToModalState(filters: PullRequestFilters): FilterModalState {
  return {
    status: filters.status,
    repositoryName: filters.repositoryName,
    reviewerMe: filters.reviewerMe ?? false,
    authorMe: filters.authorMe ?? false,
    draftFilter: filters.draftFilter ?? "all",
    activeCommentsFilter: filters.activeCommentsFilter ?? "all",
  };
}
