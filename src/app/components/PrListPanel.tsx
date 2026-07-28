import { Box, Text } from "ink";
import type { NormalizedPullRequest } from "../../api/types.js";

interface PrListPanelProps {
  pullRequests: NormalizedPullRequest[];
  selectedIndex: number;
  filterLabels: string[];
  width: number;
  height: number;
  focused: boolean;
}

type VisualRow =
  | { type: "header"; key: string; repoName: string }
  | { type: "spacer"; key: string }
  | { type: "title"; key: string; pr: NormalizedPullRequest; prIndex: number }
  | { type: "meta"; key: string; pr: NormalizedPullRequest; prIndex: number };

export function PrListPanel({
  pullRequests,
  selectedIndex,
  filterLabels,
  width,
  height,
  focused,
}: PrListPanelProps) {
  const rows = buildRows(pullRequests);
  const availableRows = Math.max(1, height - 4);
  const selectedRowIndex = rows.findIndex(
    (row) => row.type === "title" && row.prIndex === selectedIndex,
  );
  const start = computeScrollStart(Math.max(0, selectedRowIndex), rows.length, availableRows);
  const visible = rows.slice(start, start + availableRows);

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
      <Text bold color={focused ? "cyan" : undefined}>
        Pull Requests {pullRequests.length > 0 ? `(${pullRequests.length})` : ""}
      </Text>
      <FilterLine filters={filterLabels} />
      {pullRequests.length === 0 ? (
        <Text dimColor>No pull requests match your filters.</Text>
      ) : (
        visible.map((row) => {
          if (row.type === "header") {
            return (
              <Text key={row.key} wrap="truncate-end" bold underline color="blueBright">
                {row.repoName}
              </Text>
            );
          }
          if (row.type === "spacer") {
            return <Text key={row.key}> </Text>;
          }
          if (row.type === "title") {
            const selected = row.prIndex === selectedIndex;
            const color = rowColor(row.pr, selected);
            const prefix = selected ? "▶ " : "  ";
            const tag = row.pr.isDraft ? " DRAFT" : "";
            const innerWidth = Math.max(1, width - 4);
            const titleWidth = Math.max(1, innerWidth - prefix.length - tag.length);
            const title = truncateText(row.pr.title, titleWidth);
            return (
              <Text key={row.key} wrap="truncate-end" color={color} bold={selected}>
                {prefix}
                {title}
                {tag ? (
                  <Text bold color="yellow">
                    {tag}
                  </Text>
                ) : null}
              </Text>
            );
          }
          return (
            <Text key={row.key} wrap="truncate-end" dimColor>
              {"    "}
              {row.pr.author} · {formatCompactAge(row.pr.createdAt)}
            </Text>
          );
        })
      )}
    </Box>
  );
}

function FilterLine({ filters }: { filters: string[] }) {
  return (
    <Text wrap="truncate-end" dimColor>
      Filters:{" "}
      {filters.length === 0 ? "none" : filters.map((f) => `[${f}]`).join(" · ")}
    </Text>
  );
}

function buildRows(pullRequests: NormalizedPullRequest[]): VisualRow[] {
  const rows: VisualRow[] = [];
  let lastRepo: string | null = null;

  pullRequests.forEach((pr, index) => {
    if (pr.repoName !== lastRepo) {
      if (rows.length > 0) {
        rows.push({ type: "spacer", key: `sp-${pr.repositoryId}-${pr.id}` });
      }
      rows.push({
        type: "header",
        key: `h-${pr.repositoryId}-${pr.repoName}`,
        repoName: pr.repoName,
      });
      lastRepo = pr.repoName;
    } else {
      rows.push({ type: "spacer", key: `sp-${pr.repositoryId}-${pr.id}` });
    }
    rows.push({ type: "title", key: `t-${pr.repositoryId}-${pr.id}`, pr, prIndex: index });
    rows.push({ type: "meta", key: `m-${pr.repositoryId}-${pr.id}`, pr, prIndex: index });
  });

  return rows;
}

function computeScrollStart(selectedRowIndex: number, total: number, visibleRows: number): number {
  if (total <= visibleRows) return 0;
  const start = selectedRowIndex - Math.floor(visibleRows / 2);
  return Math.max(0, Math.min(start, total - visibleRows));
}

function rowColor(pr: NormalizedPullRequest, selected: boolean): string | undefined {
  if (selected) return "cyan";
  if (pr.mergeStatus === "conflicts") return "red";
  if (pr.needsMyReview) return "yellow";
  return undefined;
}

function truncateText(text: string, maxWidth: number): string {
  if (maxWidth <= 0) return "";
  if (text.length <= maxWidth) return text;
  if (maxWidth === 1) return "…";
  return `${text.slice(0, maxWidth - 1)}…`;
}

function formatCompactAge(dateString: string): string {
  const seconds = Math.max(0, Math.floor((Date.now() - new Date(dateString).getTime()) / 1000));
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h`;
  const days = Math.floor(hours / 24);
  if (days < 30) return `${days}d`;
  const months = Math.floor(days / 30);
  if (months < 12) return `${months}mo`;
  return `${Math.floor(months / 12)}y`;
}
