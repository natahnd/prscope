import { Box, Text, useInput } from "ink";
import { useEffect, useMemo, useState } from "react";
import { detectAzureDevOpsOrg } from "../../providers/azure-devops/detect-org.js";
import { listProviders } from "../../providers/registry.js";
import type { ProviderDefinition } from "../../providers/types.js";

const SPINNER_FRAMES = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

interface ProviderPickerProps {
  onSelect: (provider: ProviderDefinition) => void;
  onQuit: () => void;
}

export function ProviderPicker({ onSelect, onQuit }: ProviderPickerProps) {
  const providers = useMemo(() => listProviders(), []);
  const [index, setIndex] = useState(0);
  const safeIndex = Math.min(index, Math.max(providers.length - 1, 0));
  const selected = providers[safeIndex];

  useInput((input, key) => {
    if (key.upArrow || input === "k") {
      setIndex((i) => Math.max(0, i - 1));
      return;
    }
    if (key.downArrow || input === "j") {
      setIndex((i) => Math.min(providers.length - 1, i + 1));
      return;
    }
    if (key.return && selected?.enabled) {
      onSelect(selected);
      return;
    }
    if (input === "q" || (key.ctrl && input === "c")) {
      onQuit();
    }
  });

  return (
    <Box flexDirection="column">
      <Box borderStyle="round" borderColor="cyan" paddingX={1}>
        <Text bold color="cyan">
          Add Provider
        </Text>
      </Box>

      <Box marginTop={1}>
        <Text>Select a source for pull requests:</Text>
      </Box>

      <Box flexDirection="column" marginTop={1} minHeight={6}>
        {providers.map((provider, i) => {
          const isSelected = i === safeIndex;
          return (
            <Box key={provider.id}>
              <Text
                color={isSelected && provider.enabled ? "cyan" : undefined}
                dimColor={!provider.enabled}
                bold={isSelected && provider.enabled}
              >
                {isSelected ? "▶ " : "  "}
                {provider.name}
              </Text>
            </Box>
          );
        })}
      </Box>
    </Box>
  );
}

interface AdoProviderSetupProps {
  loading: boolean;
  error?: string;
  onSubmit: (organization: string) => void;
  onQuit: () => void;
  onDetectingChange?: (detecting: boolean) => void;
}

export function AdoProviderSetup({
  loading,
  error,
  onSubmit,
  onQuit,
  onDetectingChange,
}: AdoProviderSetupProps) {
  const [detecting, setDetecting] = useState(true);
  const [value, setValue] = useState("");
  const [detectedFromAz, setDetectedFromAz] = useState(false);
  const [spinnerFrame, setSpinnerFrame] = useState(0);

  useEffect(() => {
    onDetectingChange?.(true);
    void detectAzureDevOpsOrg()
      .then((org) => {
        if (org) {
          setValue(org);
          setDetectedFromAz(true);
        }
      })
      .finally(() => {
        setDetecting(false);
        onDetectingChange?.(false);
      });
  }, [onDetectingChange]);

  useEffect(() => {
    if (!detecting) return;
    const id = setInterval(() => {
      setSpinnerFrame((f) => (f + 1) % SPINNER_FRAMES.length);
    }, 80);
    return () => clearInterval(id);
  }, [detecting]);

  useInput((input, key) => {
    if (detecting) {
      if (input === "q" || (key.ctrl && input === "c")) {
        onQuit();
      }
      return;
    }

    if (loading) return;

    if (key.return) {
      onSubmit(value.trim());
      return;
    }
    if (input === "q" || (key.ctrl && input === "c")) {
      onQuit();
      return;
    }
    if (key.backspace || key.delete) {
      setValue((v) => v.slice(0, -1));
      return;
    }
    if (input.length === 1 && !key.ctrl && !key.meta) {
      setValue((v) => v + input);
    }
  });

  return (
    <Box flexDirection="column">
      <Box borderStyle="round" borderColor="cyan" paddingX={1}>
        <Text bold color="cyan">
          Azure DevOps Setup
        </Text>
      </Box>

      {detecting ? (
        <Box marginTop={1}>
          <Text color="cyan">{SPINNER_FRAMES[spinnerFrame]} </Text>
          <Text>Detecting existing Azure DevOps configuration…</Text>
        </Box>
      ) : (
        <>
          <Box marginTop={1} flexDirection="column">
            <Text>Enter your Azure DevOps organization:</Text>
            <Text dimColor>This is the name in dev.azure.com/your-org</Text>
            {detectedFromAz && value && (
              <Text dimColor>(detected from az devops configure)</Text>
            )}
          </Box>

          <Box marginTop={1}>
            <Text color="cyan">&gt; </Text>
            <Text>{value}</Text>
            <Text dimColor>{loading ? " (connecting…)" : "█"}</Text>
          </Box>
        </>
      )}

      {error && (
        <Box marginTop={1}>
          <Text color="red">{error}</Text>
        </Box>
      )}
    </Box>
  );
}
