import { Box, Text } from "ink";

interface HeaderProps {
  organization: string;
  project: string;
  count: number;
  loading: boolean;
  error?: string;
  refreshedAt?: Date;
}

export function Header({
  organization,
  project,
  count,
  loading,
  error,
  refreshedAt,
}: HeaderProps) {
  const refreshed = refreshedAt
    ? formatAge(refreshedAt)
    : "";

  return (
    <Box flexDirection="column">
      <Box borderStyle="round" borderColor="cyan" paddingX={1}>
        <Text bold color="cyan">
          PR Reviews
        </Text>
        <Text>
          {" "}
          — {organization} / {project} — {count} PR{count === 1 ? "" : "s"}
        </Text>
        {loading && <Text dimColor> (refreshing…)</Text>}
        {refreshed && !loading && <Text dimColor> — refreshed {refreshed} ago</Text>}
      </Box>
      {error && (
        <Box marginTop={1}>
          <Text color="red">{error}</Text>
        </Box>
      )}
    </Box>
  );
}

function formatAge(date: Date): string {
  const seconds = Math.floor((Date.now() - date.getTime()) / 1000);
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  return `${hours}h`;
}
