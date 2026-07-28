import { Box, Text } from "ink";
import type { AppScreen } from "../../api/types.js";

interface StatusBarProps {
  screen: AppScreen;
  modalOpen?: boolean;
  setupDetecting?: boolean;
}

export function StatusBar({ screen, modalOpen, setupDetecting }: StatusBarProps) {
  let hint = "";
  if (modalOpen) {
    hint = "↑↓ navigate · Space toggle · Enter apply · Esc cancel";
  } else if (screen === "providerSelect") {
    hint = "↑↓ navigate · Enter add · q quit";
  } else if (screen === "providerSetup") {
    hint = setupDetecting ? "Detecting… · q quit" : "Enter continue · q quit";
  } else if (screen === "projectSelect") {
    hint = "↑↓ navigate · Enter select · / search · r refresh · q quit";
  } else {
    hint =
      "↑↓ navigate · Enter open · f filter · p project · / search · r refresh · q quit";
  }

  return (
    <Box marginTop={1} borderStyle="single" borderColor="gray" paddingX={1}>
      <Text dimColor>{hint}</Text>
    </Box>
  );
}
