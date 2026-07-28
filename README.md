# pr-reviews

A keyboard-driven terminal UI for reviewing pull requests across repositories. Azure DevOps is supported today; the provider registry is ready for GitHub and others.

## Features

- **Add Provider** flow on first run — pick Azure DevOps (GitHub shown as coming soon)
- **Auto-detect org** from `az devops configure` when setting up Azure DevOps
- **Remembers your last project** — launches straight into it next time; the project picker only shows on first run or when you press `p`
- **Lazygit-style split layout** — PR list on the left, grouped by repository; a fixed-height panel on the right toggles between description and comments (`h`/`l`), both independently scrollable so the UI never jumps around
- **Filters** — status, repository, drafts (show all/hide/only), active comments (show all/hide/only — see which PRs still need action), reviewer/author (me), text search
- **Keyboard navigation** — j/k, Tab to switch panel focus, h/l to switch description/comments, Enter to open in browser, `p` to switch projects
- **Mouse support** — click a PR to select it, click either panel to focus it, drag the divider between panels to resize them, scroll the wheel to navigate/scroll
- **Dual auth** — Azure CLI session or Personal Access Token

## Requirements

- [Bun](https://bun.sh) 1.0+
- Azure CLI (`az`) with **azure-devops** extension, logged in **or** `AZURE_DEVOPS_PAT`

## Install

```bash
bun install
bun run build
bun link   # optional: install `pr-reviews` globally
```

## Quick start

```bash
# First run: Add Provider → Azure DevOps → org (auto-detected if az devops configured)
bun run dev

# Returning user: straight to your last-used project's PR list
bun run dev

# Or after linking
pr-reviews
```

## First-run flow

1. **Add Provider** — select Azure DevOps (GitHub is visible but disabled)
2. **Azure DevOps setup** — org pre-filled from `az devops configure` if available
3. **Project picker** — choose a project
4. **PR list** — review pull requests

## Authentication

1. **Azure CLI (preferred):** `az login` — uses your existing session for Azure DevOps API calls.
2. **PAT fallback:** Set `AZURE_DEVOPS_PAT` or `pr-reviews config set --pat <token>`.

Configure your default DevOps org once with Azure CLI (optional, enables auto-detect):

```bash
az devops configure --defaults organization=https://dev.azure.com/your-org
```

Config is stored at `~/.config/pr-reviews/config.json` (v2 schema, respects `XDG_CONFIG_HOME`).

## Usage

```bash
# Interactive TUI
pr-reviews

# Plain output for scripting (skips TUI)
pr-reviews list --org myorg --project MyProject --plain
pr-reviews list --org myorg --project MyProject --plain --format json

# Config
pr-reviews config set --org myorg
pr-reviews config show
```

## Adding a provider

1. Create `src/providers/<id>/index.ts` exporting a `ProviderDefinition`
2. Register it in `src/providers/registry.ts` (`PROVIDERS` array)
3. Set `enabled: true` when ready

## Keybindings

### Add Provider

| Key | Action |
|-----|--------|
| ↑/↓ or j/k | Navigate |
| Enter | Add selected provider (disabled providers ignored) |
| q | Quit |

### Azure DevOps setup

| Key | Action |
|-----|--------|
| Enter | Save org and continue |
| q | Quit |

### Project picker

| Key | Action |
|-----|--------|
| ↑/↓ or j/k | Navigate |
| Enter | Select project |
| / | Search projects |
| r | Refresh |
| q | Quit |

### PR list

The right side is a single fixed-height panel that toggles between **Description** and **Comments** for the selected PR (`h`/`l` to switch, `Tab` to move focus there). Both views are independently scrollable so long descriptions or comment threads never resize the UI. Comments are shown as `path/to/file: Line X` (or `Line X-Y` for a range) followed by the full, untruncated comment text. Descriptions and comment bodies are rendered as Markdown (glow-style, via `marked`/`marked-terminal`) — headings, bold/italic, lists, code spans, and tables get proper terminal styling instead of raw `#`/`**`/`` ` `` syntax. In Comments, press `s` to cycle between all comments, active-only, and resolved-only; resolved threads are also tagged (e.g. `[fixed]`) and shown in green even when the filter is off.

The list itself fetches each PR's comment thread status in the background and tags rows with unresolved threads as `· active comments`, so you can tell at a glance which PRs still need action. Use the Filter modal's **Comments** column to hide or isolate them across the whole list.

| Key | Action |
|-----|--------|
| ↑/↓ or j/k | Navigate PR list, or scroll the description/comments panel when it's focused |
| Tab | Toggle focus between PR list and the description/comments panel |
| h/l or ←/→ | Switch the right panel between Description and Comments (when it's focused) |
| s | Cycle the comments filter: all → active → resolved (when Comments is focused) |
| Enter / o | Open PR in browser |
| p | Switch project |
| f | Filter modal |
| / | Search PRs |
| r | Refresh |
| q | Quit |

**Mouse:** click a PR row to select it and view its details; click the list or the right panel to focus it; drag the border between the two panels left/right to resize them; scroll the wheel over either panel to navigate the list or scroll the focused content.

## Development

```bash
bun run dev
bun run build
```
