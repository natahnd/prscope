import cliTruncate from "cli-truncate";
import { Box, Text } from "ink";
import { useEffect, useMemo } from "react";
import type { NormalizedPullRequest } from "../../api/types.js";
import type { PullRequestDetailState } from "../hooks/usePullRequestDetail.js";
import {
  buildRightPanelLines,
  buildSections,
  CHROME_ROWS,
  type CommentStatusFilter,
  type ContentLine,
  getVisibleRightPanelLines,
  rightPanelContentWidth,
  type RightPanelMode,
} from "../utils/rightPanelLines.js";
import { rowHighlightRange, type NormalizedSelection } from "../utils/textSelection.js";

export type { CommentStatusFilter, RightPanelMode };

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
  selection: NormalizedSelection | null;
}

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
  selection,
}: PrRightPanelProps) {
  const contentWidth = rightPanelContentWidth(width);
  const contentHeight = Math.max(1, height - CHROME_ROWS);
  const { threads } = detailState;

  const sections = useMemo(
    () => buildSections(threads, commentStatusFilter),
    [threads, commentStatusFilter],
  );

  const lines = useMemo<ContentLine[]>(
    () => buildRightPanelLines({ pullRequest, detailState, mode, commentStatusFilter, contentWidth }),
    [pullRequest, detailState, mode, commentStatusFilter, contentWidth],
  );

  const { visible, clampedScroll, maxScroll } = getVisibleRightPanelLines(
    lines,
    scrollOffset,
    contentHeight,
  );

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
        {visible.map((line, i) => (
          <ContentLineRow
            key={line.key}
            line={line}
            highlight={rowHighlightRange(clampedScroll + i, selection)}
            contentWidth={contentWidth}
          />
        ))}
      </Box>
    </Box>
  );
}

function ContentLineRow({
  line,
  highlight,
  contentWidth,
}: {
  line: ContentLine;
  highlight: { start: number; end: number } | null;
  contentWidth: number;
}) {
  if (line.kind === "empty") {
    return <Text dimColor>{line.text}</Text>;
  }
  if (line.kind === "header") {
    return (
      <Text bold color={line.accent ?? "magenta"}>
        {highlightedText(line.text, highlight, contentWidth)}
      </Text>
    );
  }
  if (line.kind === "meta") {
    return <Text dimColor>{highlightedText(line.text, highlight, contentWidth)}</Text>;
  }
  if (line.kind === "blank") {
    return <Text>{highlightedText(" ", highlight, contentWidth)}</Text>;
  }
  if (line.kind === "markdown") {
    // Already pre-truncated to width by renderMarkdownToLines (cli-truncate),
    // so no need for Ink's own wrap="truncate-end" (which has a re-render bug).
    return <Text>{highlightedText(line.text || " ", highlight, contentWidth)}</Text>;
  }
  return <Text>{highlightedText(line.text || " ", highlight, contentWidth)}</Text>;
}

/**
 * Splits `text` into up to three segments around the highlighted column
 * range and renders the middle one with a selection background. Full-row
 * highlights get padded to `contentWidth` so short lines in the middle of a
 * multi-line selection still read as a continuous highlighted block.
 */
function highlightedText(
  text: string,
  highlight: { start: number; end: number } | null,
  contentWidth: number,
) {
  if (!highlight) return text;

  const isFullRow = highlight.end >= contentWidth - 1 && highlight.start === 0;
  const source = isFullRow && text.length < contentWidth ? text.padEnd(contentWidth) : text;
  const start = Math.max(0, Math.min(highlight.start, source.length));
  const end = Math.min(highlight.end, source.length - 1);
  if (end < start) return text;

  const before = source.slice(0, start);
  const middle = source.slice(start, end + 1);
  const after = source.slice(end + 1);

  return (
    <>
      {before}
      <Text backgroundColor="cyan" color="black">
        {middle}
      </Text>
      {after}
    </>
  );
}
