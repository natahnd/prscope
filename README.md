# pr-reviews

A keyboard-driven terminal UI for reviewing pull requests across repositories. Built with [ratatui](https://ratatui.rs) and [crossterm](https://github.com/crossterm-rs/crossterm) as a single self-contained binary. Azure DevOps is supported today; the provider registry is ready for GitHub and others.

## Features

- **Add Provider** flow on first run — pick Azure DevOps (GitHub shown as coming soon)
- **Auto-detect org** from `az devops configure` when setting up Azure DevOps
- **Remembers your last project** — launches straight into it next time; the project picker only shows on first run or when you press `p`
- **Remembers your filters** — whatever you set via the Filter modal or search is saved and restored on the next launch; passing explicit CLI filter flags (`--status`, `--repo`, `--author`, `--reviewer`, `--mine`) overrides the saved filters for that run only
- **Lazygit-style split layout** — PR list on the left, grouped by repository; a fixed-height panel on the right toggles between description and comments (`h`/`l`), both independently scrollable so the UI never jumps around
- **Filters** — status, repository, drafts (show all/hide/only), active comments (all PRs / only PRs with active comments — see which PRs still need action), reviewer/author (me), text search
- **Keyboard navigation** — j/k, Tab to switch panel focus, h/l to switch description/comments, Enter to open in browser, `p` to switch projects
- **Mouse support** — click a PR to select it, click either panel to focus it, drag the divider between panels to resize them, scroll the wheel to navigate/scroll, drag-select text in the description/comments panel (confined to that panel, never bleeds into the PR list); press `y` any time to copy the whole panel's current content (respecting whatever filter is applied)
- **Dual auth** — Azure CLI session or Personal Access Token
- **Structured styling** — content is rendered as `(text, Style)` spans rather than baked-in ANSI escape codes, so clipboard copy is always clean plain text

## Requirements

- [Rust](https://rustup.rs) 1.75+ (stable toolchain, 2021 edition)
- Azure CLI (`az`) with **azure-devops** extension, logged in **or** `AZURE_DEVOPS_PAT`

## Install

```bash
cargo build --release
# binary at target/release/pr-reviews
```

Optionally install it onto your `PATH`:

```bash
cargo install --path .
```

## Quick start

```bash
# First run: Add Provider → Azure DevOps → org (auto-detected if az devops configured)
cargo run

# Returning user: straight to your last-used project's PR list
cargo run

# Or after `cargo install --path .`
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

1. Implement the `ProviderClient` trait (`src/providers/mod.rs`) for the new provider in `src/providers/<id>/`
2. Register it in `src/providers/registry.rs` (`list_providers()`)
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

The right side is a single fixed-height panel that toggles between **Description** and **Comments** for the selected PR (`h`/`l` to switch, `Tab` to move focus there). Both views are independently scrollable so long descriptions or comment threads never resize the UI. Comments are shown as `path/to/file: Line X` (or `Line X-Y` for a range) followed by the full, untruncated comment text. Descriptions and comment bodies are rendered as Markdown (via `pulldown-cmark`) — headings, bold/italic, lists, code spans, and tables get proper terminal styling instead of raw `#`/`**`/`` ` `` syntax. In Comments, press `s` to cycle between all comments, active-only, and resolved-only; resolved threads are also tagged (e.g. `[fixed]`) and shown in green even when the filter is off.

The list itself fetches each PR's comment thread status in the background and tags rows with unresolved threads as `· active comments`, so you can tell at a glance which PRs still need action. Use the Filter modal's **Comments** column to isolate them across the whole list.

| Key | Action |
|-----|--------|
| ↑/↓ or j/k | Navigate PR list, or scroll the description/comments panel when it's focused |
| Tab | Toggle focus between PR list and the description/comments panel |
| h/l or ←/→ | Switch the right panel between Description and Comments (when it's focused) |
| s | Cycle the comments filter: all → active → resolved (when Comments is focused) |
| g | Cycle list grouping: repos → author → age↓ → age↑ |
| y | Copy the whole description/comments panel's current content (respects the active filter) |
| Enter / o | Open PR in browser |
| p | Switch project |
| f | Filter modal |
| / | Search PRs |
| r | Refresh |
| q | Quit |

**Mouse:** click a PR row to select it and view its details; click the list or the right panel to focus it; drag the border between the two panels left/right to resize them; scroll the wheel over either panel to navigate the list or scroll the focused content.

**Text selection:** click-and-drag over the description/comments panel selects text the same way you'd expect in a normal terminal, but the selection is confined to that panel — dragging out over the PR list (or past the panel's edges) never pulls list content into it. The selection stays highlighted after you release the drag, purely for reading/reference — it doesn't drive what gets copied. A plain click with no drag just focuses the panel and clears any existing selection.

**Copying (`y`):** press `y` at any time to copy the *entire* current content of the description/comments panel — not just what's on screen or selected — to your system clipboard via an OSC 52 escape sequence, which works locally and over SSH on most modern terminals (iTerm2, Kitty, WezTerm, Windows Terminal, etc). Whatever's currently filtered/toggled is exactly what gets copied: e.g. if the comments filter is set to active-only, `y` copies just the active comment threads (with their `path/to/file: Line X` headers and full text), not the full unfiltered list.

## Project layout

```
src/
  api/           Shared types (PRs, filters, config schema)
  cli/           Clap CLI definitions, plain/JSON output, dispatch
  config/        Config file load/save + v1→v2 migration
  providers/     ProviderClient trait, registry, Azure DevOps implementation
  app/           TUI: state, event loop, rendering (ui/), input handling,
                 markdown renderer, layout geometry, text selection
```

## Development

```bash
cargo build
cargo run
cargo test
cargo clippy
```

### WSL browser opening

On WSL, `Enter`/`o` opens URLs via Windows PowerShell (`Start "url"`), so no extra config is needed. For other CLI tools in WSL that use `xdg-open`, install `wslu` and set `export BROWSER=wslview` in your shell profile.

### A note on `NO_COLOR`

Colors are rendered via `crossterm`, which honors the [`NO_COLOR`](https://no-color.org/) convention: if `NO_COLOR` is set in the environment, all foreground/background colors are suppressed (text attributes like bold/dim still render). Don't set `NO_COLOR` if you want the normal styled UI, including the text-selection highlight.
