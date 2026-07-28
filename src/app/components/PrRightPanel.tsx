import cliTruncate from "cli-truncate";
import { Box, Text } from "ink";
import { formatDistanceToNow } from "date-fns";
import { useEffect, useMemo } from "react";
import type { NormalizedPullRequest, PrComment, PrThread } from "../../api/types.js";
import type { PullRequestDetailState } from "../hooks/usePullRequestDetail.js";
import { isResolvedStatus } from "../utils/commentStatus.js";
import { renderMarkdownToLines } from "../utils/renderMarkdown.js";
import { stripHtml } from "../utils/wrapText.js";

export type RightPanelMode = "description" | "comments";
export type CommentStatusFilter = "all" | "active" | "resolved";

export function nextCommentStatusFilter(current: CommentStatusFilter): CommentStatusFilter {
  if (current === "all") return "active";
  if (current === "active") return "resolved";
  return "all";
}

interface PrRightPanelProps {
  pullRequest?: NormalizedPullRequest;
  detailState: PullRequestDetailState;
  mode: RightPanelMode;
  commentStatusFilter: CommentStatusFilter;
  scrollOffset: number;
  width: number;
  height: number;
  focused: boolean;
  onMaxScrollChange: (maxScroll: number) => void;
}

interface ContentLine {
  key: string;
  kind: "header" | "meta" | "body" | "markdown" | "blank" | "empty";
  text: string;
  accent?: string;
}

type Section = { type: "general"; comments: PrComment[] } | { type: "file"; thread: PrThread };

// border (top+bottom) + title line + meta line + tab-strip margin + tab-strip line
const CHROME_ROWS = 6;

export function PrRightPanel({
  pullRequest,
  detailState,
  mode,
  commentStatusFilter,
  scrollOffset,
  width,
  height,
  focused,
  onMaxScrollChange,
}: PrRightPanelProps) {
  const contentWidth = Math.max(10, width - 4);
  const contentHeight = Math.max(1, height - CHROME_ROWS);
  const { detail, threads, loading, error } = detailState;

  const sections = useMemo(
    () => buildSections(threads, commentStatusFilter),
    [threads, commentStatusFilter],
  );

  const lines = useMemo<ContentLine[]>(() => {
    if (!pullRequest) return [];
    if (loading) {
      return [
        {
          key: "loading",
          kind: "empty",
          text: mode === "description" ? "Loading description…" : "Loading comments…",
        },
      ];
    }
    if (error) return [{ key: "error", kind: "empty", text: error }];

    if (mode === "description") {
      const description = detail?.description?.trim();
      if (!description) return [{ key: "empty", kind: "empty", text: "No description provided." }];
      return renderMarkdownToLines(description, contentWidth).map((text, i) => ({
        key: `d-${i}`,
        kind: "markdown",
        text,
      }));
    }

    if (sections.length === 0) {
      const emptyText =
        commentStatusFilter === "all"
          ? "No comments on this pull request."
          : `No ${commentStatusFilter} comments on this pull request.`;
      return [{ key: "empty", kind: "empty", text: emptyText }];
    }
    return buildCommentLines(sections, contentWidth);
  }, [
    pullRequest,
    loading,
    error,
    mode,
    detail?.description,
    sections,
    contentWidth,
    commentStatusFilter,
  ]);

  const maxScroll = Math.max(0, lines.length - contentHeight);

  useEffect(() => {
    onMaxScrollChange(maxScroll);
  }, [maxScroll, onMaxScrollChange]);

  if (!pullRequest) {
    return (
      <Box width={width} height={height} borderStyle="single" borderColor="gray" paddingX={1}>
        <Text dimColor>Select a pull request to see details.</Text>
      </Box>
    );
  }

  const clampedScroll = Math.min(scrollOffset, maxScroll);
  const visible = lines.slice(clampedScroll, clampedScroll + contentHeight);

  // Truncate ourselves with cli-truncate instead of Ink's `wrap="truncate-end"`,
  // and see the flexShrink={0} wrapper below for why — both are needed to
  // avoid a layout bug where these rows get merged with content below them.
  const titleText = cliTruncate(`#${pullRequest.id} ${pullRequest.title}`, contentWidth);
  const metaText = cliTruncate(
    `${pullRequest.targetBranch} · draft: ${pullRequest.isDraft ? "yes" : "no"} · conflicts: ` +
      `${pullRequest.mergeStatus === "conflicts" ? "yes" : "none"} · status: ${pullRequest.status}`,
    contentWidth,
  );

  return (
    <Box
      flexDirection="column"
      width={width}
      height={height}
      borderStyle="single"
      borderColor={focused ? "cyan" : "gray"}
      paddingX={1}
      overflow="hidden"
    >
      {/*
        flexShrink={0} works around a reproducible ink/Yoga layout bug (also
        present on the pre-upgrade ink@5/react@18, so unrelated to the react19
        upgrade): on re-renders that change the tab-strip row's content below,
        this fixed-height title/meta block could get squeezed by 1 row, and
        its content would merge onto the row below it. Isolating it in its
        own non-shrinkable Box prevents Yoga from ever reassigning its rows.
      */}
      <Box flexDirection="column" flexShrink={0}>
        <Text bold>{titleText}</Text>
        <Text dimColor>{metaText}</Text>
      </Box>
      <Box marginTop={1} justifyContent="space-between">
        <Box>
          <Text bold={mode === "description"} underline={mode === "description"} color={mode === "description" ? "cyan" : undefined} dimColor={mode !== "description"}>
            Description
          </Text>
          <Text> </Text>
          <Text bold={mode === "comments"} underline={mode === "comments"} color={mode === "comments" ? "cyan" : undefined} dimColor={mode !== "comments"}>
            {`Comments${sections.length ? ` (${sections.length})` : ""}`}
          </Text>
          <Text dimColor>
            {mode === "comments" && commentStatusFilter !== "all"
              ? ` · ${commentStatusFilter === "active" ? "active only" : "resolved only"}`
              : ""}
          </Text>
        </Box>
        {maxScroll > 0 && (
          <Text dimColor>
            {clampedScroll + 1}-{Math.min(lines.length, clampedScroll + contentHeight)}/{lines.length}
          </Text>
        )}
      </Box>
      <Box flexDirection="column">
        {visible.map((line) => (
          <ContentLineRow key={line.key} line={line} />
        ))}
      </Box>
    </Box>
  );
}

function ContentLineRow({ line }: { line: ContentLine }) {
  if (line.kind === "empty") {
    return <Text dimColor>{line.text}</Text>;
  }
  if (line.kind === "header") {
    return (
      <Text bold color={line.accent ?? "magenta"}>
        {line.text}
      </Text>
    );
  }
  if (line.kind === "meta") {
    return <Text dimColor>{line.text}</Text>;
  }
  if (line.kind === "blank") {
    return <Text> </Text>;
  }
  if (line.kind === "markdown") {
    // Already pre-truncated to width by renderMarkdownToLines (cli-truncate),
    // so no need for Ink's own wrap="truncate-end" (which has a re-render bug).
    return <Text>{line.text || " "}</Text>;
  }
  return <Text>{line.text || " "}</Text>;
}

function buildSections(threads: PrThread[], statusFilter: CommentStatusFilter): Section[] {
  const filtered = threads.filter((thread) => {
    if (statusFilter === "all") return true;
    const resolved = isResolvedStatus(thread.status);
    return statusFilter === "resolved" ? resolved : !resolved;
  });
  const generalComments = filtered.filter((t) => !t.filePath).flatMap((t) => t.comments);
  const fileThreads = filtered.filter((t) => t.filePath);
  const sections: Section[] = [];
  if (generalComments.length > 0) sections.push({ type: "general", comments: generalComments });
  for (const thread of fileThreads) sections.push({ type: "file", thread });
  return sections;
}

function buildCommentLines(sections: Section[], width: number): ContentLine[] {
  const lines: ContentLine[] = [];

  for (const section of sections) {
    if (section.type === "general") {
      lines.push({ key: "general-header", kind: "header", text: "General", accent: "cyan" });
      for (const comment of section.comments) {
        pushCommentLines(lines, "general", comment, width);
      }
      continue;
    }

    const header = formatThreadHeader(section.thread);
    const accent = isResolvedStatus(section.thread.status) ? "green" : "magenta";
    lines.push({ key: `t-${section.thread.id}-header`, kind: "header", text: header, accent });
    for (const comment of section.thread.comments) {
      pushCommentLines(lines, `t-${section.thread.id}`, comment, width);
    }
  }

  while (lines.length > 0 && lines[lines.length - 1]?.kind === "blank") {
    lines.pop();
  }

  return lines;
}

function pushCommentLines(lines: ContentLine[], scope: string, comment: PrComment, width: number): void {
  const prefix = `${scope}-c-${comment.id}`;
  lines.push({
    key: `${prefix}-meta`,
    kind: "meta",
    text: `${comment.author} · ${formatDistanceToNow(new Date(comment.publishedAt), { addSuffix: true })}`,
  });
  renderMarkdownToLines(stripHtml(comment.content), width).forEach((text, i) => {
    lines.push({ key: `${prefix}-b-${i}`, kind: "markdown", text });
  });
  lines.push({ key: `${prefix}-blank`, kind: "blank", text: "" });
}

function formatThreadHeader(thread: PrThread): string {
  const base = (() => {
    if (!thread.filePath) return "Comment";
    if (!thread.line) return thread.filePath;
    const lineLabel = thread.lineEnd ? `Line ${thread.line}-${thread.lineEnd}` : `Line ${thread.line}`;
    return `${thread.filePath}: ${lineLabel}`;
  })();
  return isResolvedStatus(thread.status) ? `${base} [${thread.status}]` : base;
}
