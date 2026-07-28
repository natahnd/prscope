import { Box, Text } from "ink";
import { formatDistanceToNow } from "date-fns";
import type { NormalizedPullRequest } from "../../api/types.js";

interface FilterBarProps {
  filters: string[];
}

export function FilterBar({ filters }: FilterBarProps) {
  return (
    <Box marginTop={1}>
      <Text dimColor>Filters: </Text>
      {filters.length === 0 ? (
        <Text>none</Text>
      ) : (
        filters.map((f, i) => (
          <Text key={f}>
            {i > 0 ? " · " : ""}[{f}]
          </Text>
        ))
      )}
    </Box>
  );
}

interface PrTableProps {
  pullRequests: NormalizedPullRequest[];
  selectedIndex: number;
}

export function PrTable({ pullRequests, selectedIndex }: PrTableProps) {
  if (pullRequests.length === 0) {
    return (
      <Box marginTop={1} minHeight={8}>
        <Text dimColor>No pull requests match your filters.</Text>
      </Box>
    );
  }

  return (
    <Box flexDirection="column" marginTop={1}>
      <Box>
        <Text bold dimColor>
          {pad("Repo", 14)}
          {pad("#", 6)}
          {pad("Title", 28)}
          {pad("Author", 14)}
          {pad("Reviewers", 16)}
          Age
        </Text>
      </Box>
      {pullRequests.map((pr, i) => {
        const selected = i === selectedIndex;
        const color = rowColor(pr, selected);
        return (
          <Box key={`${pr.repoName}-${pr.id}`}>
            <Text color={color} bold={selected} dimColor={pr.isDraft && !selected}>
              {selected ? "▶ " : "  "}
              {pad(truncate(pr.repoName, 12), 12)}
              {pad(String(pr.id), 6)}
              {pad(truncate(pr.title, 26), 26)}
              {pad(truncate(pr.author, 12), 12)}
              {pad(formatReviewers(pr), 14)}
              {formatDistanceToNow(new Date(pr.createdAt), { addSuffix: false })}
            </Text>
          </Box>
        );
      })}
    </Box>
  );
}

function rowColor(pr: NormalizedPullRequest, selected: boolean): string | undefined {
  if (selected) return "cyan";
  if (pr.mergeStatus === "conflicts") return "red";
  if (pr.needsMyReview) return "yellow";
  if (pr.isDraft) return "gray";
  return undefined;
}

function formatReviewers(pr: NormalizedPullRequest): string {
  if (pr.reviewers.length === 0) return "-";
  const parts = pr.reviewers.slice(0, 2).map((r) => {
    const icon = r.vote === 10 ? "✓" : r.vote === -10 ? "✗" : r.isCurrentUser ? "⚠" : "○";
    const label = r.isCurrentUser ? "you" : r.displayName.split(" ")[0] ?? r.displayName;
    return `${icon}${label}`;
  });
  if (pr.reviewers.length > 2) parts.push("…");
  return parts.join(" ");
}

function truncate(value: string, max: number): string {
  return value.length > max ? `${value.slice(0, max - 1)}…` : value;
}

function pad(value: string, width: number): string {
  return value.padEnd(width);
}
