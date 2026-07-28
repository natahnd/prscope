# pr-reviews

A keyboard-driven terminal UI for reviewing pull requests across repositories. Azure DevOps is supported today; the provider registry is ready for GitHub and others.

## Features

- **Add Provider** flow on first run — pick Azure DevOps (GitHub shown as coming soon)
- **Auto-detect org** from `az devops configure` when setting up Azure DevOps
- **Project picker** with last-used project pre-selected
- **PR overview** — all active PRs across every repo in one table
- **Filters** — status, repository, reviewer/author (me), text search
- **Keyboard navigation** — j/k, Enter to open in browser, `p` to switch projects
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

# Returning user: straight to project picker
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

| Key | Action |
|-----|--------|
| ↑/↓ or j/k | Navigate |
| Enter / o | Open PR in browser |
| p | Switch project |
| f | Filter modal |
| / | Search PRs |
| r | Refresh |
| q | Quit |

## Development

```bash
bun run dev
bun run build
```
