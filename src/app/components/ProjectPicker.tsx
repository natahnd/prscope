import { Box, Text, useInput } from "ink";
import { useMemo, useState } from "react";
import type { AdoProject } from "../../api/types.js";

interface ProjectPickerProps {
  organization: string;
  projects: AdoProject[];
  lastProject?: string;
  loading: boolean;
  error?: string;
  search: string;
  onSearchChange: (value: string) => void;
  onSelect: (project: AdoProject) => void;
  onRefresh: () => void;
  onQuit: () => void;
}

export function ProjectPicker({
  organization,
  projects,
  lastProject,
  loading,
  error,
  search,
  onSearchChange,
  onSelect,
  onRefresh,
  onQuit,
}: ProjectPickerProps) {
  const sorted = useMemo(() => sortProjects(projects, lastProject), [projects, lastProject]);

  const filtered = useMemo(() => {
    if (!search.trim()) return sorted;
    const q = search.toLowerCase();
    return sorted.filter((p) => p.name.toLowerCase().includes(q));
  }, [sorted, search]);

  const [index, setIndex] = useState(0);
  const safeIndex = filtered.length === 0 ? 0 : Math.min(index, filtered.length - 1);

  useInput((input, key) => {
    if (key.upArrow || input === "k") {
      setIndex((i) => Math.max(0, i - 1));
      return;
    }
    if (key.downArrow || input === "j") {
      setIndex((i) => Math.min(Math.max(filtered.length - 1, 0), i + 1));
      return;
    }
    if (key.return && filtered[safeIndex]) {
      onSelect(filtered[safeIndex]!);
      return;
    }
    if (input === "r") {
      onRefresh();
      return;
    }
    if (input === "q" || (key.ctrl && input === "c")) {
      onQuit();
      return;
    }
    if (key.backspace || key.delete) {
      onSearchChange(search.slice(0, -1));
      return;
    }
    if (input.length === 1 && !key.ctrl && !key.meta) {
      onSearchChange(search + input);
    }
  });

  return (
    <Box flexDirection="column">
      <Box borderStyle="round" borderColor="cyan" paddingX={1}>
        <Text bold color="cyan">
          Select Project
        </Text>
        <Text> — {organization}</Text>
        {loading && <Text dimColor> (loading…)</Text>}
      </Box>

      {error && (
        <Box marginTop={1}>
          <Text color="red">{error}</Text>
        </Box>
      )}

      <Box marginTop={1}>
        <Text dimColor>/ search: </Text>
        <Text>{search || "type to filter projects"}</Text>
      </Box>

      <Box flexDirection="column" marginTop={1} minHeight={10}>
        {filtered.length === 0 ? (
          <Text dimColor>No projects match.</Text>
        ) : (
          filtered.map((project, i) => {
            const selected = i === safeIndex;
            const isLast = project.name === lastProject;
            return (
              <Box key={project.id}>
                <Text color={selected ? "cyan" : undefined} bold={selected}>
                  {selected ? "▶ " : "  "}
                  {project.name}
                  {isLast ? " (last used)" : ""}
                </Text>
              </Box>
            );
          })
        )}
      </Box>

      <Box marginTop={1} borderStyle="single" borderColor="gray" paddingX={1}>
        <Text dimColor>↑↓ navigate · Enter select · / search · r refresh · q quit</Text>
      </Box>
    </Box>
  );
}

function sortProjects(projects: AdoProject[], lastProject?: string): AdoProject[] {
  const copy = [...projects];
  if (!lastProject) return copy;
  copy.sort((a, b) => {
    if (a.name === lastProject) return -1;
    if (b.name === lastProject) return 1;
    return a.name.localeCompare(b.name);
  });
  return copy;
}
