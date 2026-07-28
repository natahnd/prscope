import { formatDistanceToNow } from "date-fns";
import type { NormalizedPullRequest, PrComment, PrThread } from "../../api/types.js";
import type { PullRequestDetailState } from "../hooks/usePullRequestDetail.js";
import { isResolvedStatus } from "./commentStatus.js";
import { renderMarkdownToLines } from "./renderMarkdown.js";
import { stripHtml } from "./wrapText.js";

export type RightPanelMode = "description" | "comments";
export type CommentStatusFilter = "all" | "active" | "resolved";

export interface ContentLine {
  key: string;
  kind: "header" | "meta" | "body" | "markdown" | "blank" | "empty";
  text: string;
  accent?: string;
}

type Section = { type: "general"; comments: PrComment[] } | { type: "file"; thread: PrThread };

// border (top+bottom) + title line + meta line + tab-strip margin + tab-strip line
export const CHROME_ROWS = 6;
// Rows before the scrollable content starts: everything in CHROME_ROWS except the
// bottom border. Shared by rendering (PrRightPanel) and mouse hit-testing/selection
// (App) so they can never drift out of sync.
export const CONTENT_TOP_ROWS = CHROME_ROWS - 1;

export function rightPanelContentWidth(panelWidth: number): number {
  return Math.max(10, panelWidth - 4);
}

export function buildRightPanelLines({
  pullRequest,
  detailState,
  mode,
  commentStatusFilter,
  contentWidth,
}: {
  pullRequest?: NormalizedPullRequest;
  detailState: PullRequestDetailState;
  mode: RightPanelMode;
  commentStatusFilter: CommentStatusFilter;
  contentWidth: number;
}): ContentLine[] {
  const { detail, threads, loading, error } = detailState;
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

  const sections = buildSections(threads, commentStatusFilter);
  if (sections.length === 0) {
    const emptyText =
      commentStatusFilter === "all"
        ? "No comments on this pull request."
        : `No ${commentStatusFilter} comments on this pull request.`;
    return [{ key: "empty", kind: "empty", text: emptyText }];
  }
  return buildCommentLines(sections, contentWidth);
}

export function buildSections(threads: PrThread[], statusFilter: CommentStatusFilter): Section[] {
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

export function getVisibleRightPanelLines(
  lines: ContentLine[],
  scrollOffset: number,
  contentHeight: number,
): { visible: ContentLine[]; clampedScroll: number; maxScroll: number } {
  const maxScroll = Math.max(0, lines.length - contentHeight);
  const clampedScroll = Math.min(scrollOffset, maxScroll);
  const visible = lines.slice(clampedScroll, clampedScroll + contentHeight);
  return { visible, clampedScroll, maxScroll };
}
