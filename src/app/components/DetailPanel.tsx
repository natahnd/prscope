import { Box, Text } from "ink";
import type { NormalizedPullRequest } from "../../api/types.js";

interface DetailPanelProps {
  pullRequest?: NormalizedPullRequest;
}

export function DetailPanel({ pullRequest }: DetailPanelProps) {
  if (!pullRequest) {
    return (
      <Box marginTop={1} borderStyle="single" borderColor="gray" paddingX={1} minHeight={3}>
        <Text dimColor>Select a pull request to see details.</Text>
      </Box>
    );
  }

  const reviewerText =
    pullRequest.reviewers.length === 0
      ? "none"
      : pullRequest.reviewers
          .map((r) => {
            const vote =
              r.vote === 10
                ? "approved"
                : r.vote === -10
                  ? "rejected"
                  : r.vote === -5
                    ? "waiting"
                    : "no vote";
            const name = r.isCurrentUser ? "you" : r.displayName;
            return `${name} (${vote})`;
          })
          .join(", ");

  return (
    <Box
      marginTop={1}
      borderStyle="single"
      borderColor="gray"
      paddingX={1}
      flexDirection="column"
    >
      <Text>
        #{pullRequest.id} {pullRequest.title} → {pullRequest.targetBranch}
      </Text>
      <Text dimColor>
        draft: {pullRequest.isDraft ? "yes" : "no"} · conflicts:{" "}
        {pullRequest.mergeStatus === "conflicts" ? "yes" : "none"} · status:{" "}
        {pullRequest.status}
      </Text>
      <Text dimColor>Reviewers: {reviewerText}</Text>
    </Box>
  );
}
