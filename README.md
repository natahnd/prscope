# PR Reviews

> A fast, keyboard-first terminal UI for reviewing pull requests across Azure DevOps repositories.

[![Rust](https://img.shields.io/badge/Rust-2021-orange?logo=rust)](https://www.rust-lang.org/)
[![Azure DevOps](https://img.shields.io/badge/Azure_DevOps-supported-0078D7?logo=azuredevops)](#provider-support)
[![GitHub](https://img.shields.io/badge/GitHub-provider_planned-6e7681?logo=github)](#provider-support)

PR Reviews brings pull request triage into one focused terminal workspace. Browse every repository in an Azure DevOps project, inspect Markdown descriptions and comment threads, filter down to the work that needs attention, and open a PR in the browser when it is time to review.

Built with [Ratatui](https://ratatui.rs/) and [Crossterm](https://github.com/crossterm-rs/crossterm), it ships as a single Rust binary with both an interactive TUI and script-friendly output.

## Highlights

| | |
|---|---|
| **Cross-repository view** | See pull requests from every repository in the selected Azure DevOps project. |
| **Review context** | Read rendered Markdown descriptions, inline comments, thread locations, and resolution states without leaving the terminal. |
| **Action-focused filtering** | Filter by status, repository, draft state, author, reviewer, active comments, or free-text search. |
| **Flexible organization** | Group by repository or author, or sort by newest and oldest. |
| **Keyboard and mouse support** | Navigate with Vim-style keys, click rows, scroll panels, resize the split view, and select text. |
| **Persistent workspace** | Restore the last project, filters, and list layout automatically. |
| **Automation-friendly output** | Export pull requests as a terminal table or JSON with `--plain`. |

## Requirements

- A stable [Rust toolchain](https://rustup.rs/)
- Access to an Azure DevOps organization and project
- One of:
  - An authenticated [Azure CLI](https://learn.microsoft.com/cli/azure/install-azure-cli) session
  - An Azure DevOps Personal Access Token in `AZURE_DEVOPS_PAT`

The Azure CLI `azure-devops` extension is optional, but enables automatic organization detection during setup:

```bash
az extension add --name azure-devops
az devops configure --defaults organization=https://dev.azure.com/your-org
```

## Installation

Install the latest version directly from GitHub:

```bash
cargo install --git https://github.com/natahnd/prscope.git
```

Or build it locally:

```bash
git clone https://github.com/natahnd/prscope.git
cd prscope
cargo install --path .
```

The installed command is `pr-reviews`.

## Quick Start

Authenticate with either Azure CLI:

```bash
az login
```

Or a Personal Access Token:

```bash
export AZURE_DEVOPS_PAT="your-token"
```

Then launch the app:

```bash
pr-reviews
```

On first run:

1. Select **Azure DevOps** as the provider.
2. Confirm or enter your organization name.
3. Choose an Azure DevOps project.
4. Start browsing pull requests.

Future launches return directly to the last selected project.

## Authentication

Credentials are resolved in this order:

1. Azure CLI access token from the current `az login` session
2. `AZURE_DEVOPS_PAT` environment variable
3. PAT saved with `pr-reviews config set --pat <token>`

Using the environment variable is recommended for PAT-based authentication. A PAT saved through the config command is stored locally in the application config file; on Unix, the file is restricted to the current user with `0600` permissions.

## Usage

### Interactive TUI

```bash
# Open the last-used project
pr-reviews

# Open a specific project
pr-reviews --project "My Project"

# Start with a narrower pull request list
pr-reviews --project "My Project" --status active --repo api
```

Supported status values are `active`, `completed`, `abandoned`, and `all`.

Run `pr-reviews --help` or `pr-reviews list --help` for the complete CLI reference.

### Plain and JSON Output

Plain mode skips the TUI and requires a project:

```bash
# Human-readable table
pr-reviews list \
  --org my-org \
  --project "My Project" \
  --plain

# JSON for scripts, jq, or CI jobs
pr-reviews list \
  --org my-org \
  --project "My Project" \
  --status active \
  --reviewer me \
  --plain \
  --format json
```

If the organization is already configured, `--org` can be omitted.

### Configuration

```bash
# Set defaults
pr-reviews config set --org my-org --project "My Project"

# Inspect the current configuration
pr-reviews config show
```

Configuration is stored at:

```text
${XDG_CONFIG_HOME:-~/.config}/pr-reviews/config.json
```

The application automatically persists the last project, pull request filters, and list grouping. Explicit CLI filter flags override saved filters for that run.

> [!CAUTION]
> `pr-reviews config show` prints the complete configuration, including a saved PAT. Avoid running it in shared terminals or CI logs.

## Keybindings

### Pull Request View

| Key | Action |
|---|---|
| `j` / `k`, `↑` / `↓` | Move through pull requests or scroll the focused detail panel |
| `Tab` | Switch focus between the pull request list and detail panel |
| `h` / `l`, `←` / `→` | Switch between Description and Comments |
| `s` | Cycle comments through all, active, and resolved |
| `g` | Cycle repository, author, newest, and oldest layouts |
| `f` | Open the filter modal |
| `/` | Search pull requests |
| `y` | Copy all content from the current detail view |
| `Enter` / `o` | Open the selected pull request in a browser |
| `p` | Switch Azure DevOps project |
| `r` | Refresh pull requests |
| `q` / `Ctrl+C` | Quit |

### Pickers and Modals

| Context | Keys |
|---|---|
| Provider picker | `j` / `k` or arrows to navigate, `Enter` to select |
| Project picker | Type to search, `j` / `k` or arrows to navigate, `Enter` to select, `r` to refresh |
| Filter modal | `j` / `k` to move, `h` / `l` to change options, `Space` to toggle, `Enter` to apply, `Esc` to cancel |

### Mouse

- Click a pull request to select it.
- Click a panel to focus it.
- Scroll over either panel to navigate its content.
- Drag the divider to resize the split layout.
- Drag across the detail panel to highlight text.

Pressing `y` copies the complete rendered content of the active detail view, not only the visible lines or highlighted selection. Copying uses OSC 52 and works in most modern local and SSH terminals, including iTerm2, Kitty, WezTerm, and Windows Terminal.

## Filters and Comments

The filter modal supports:

- Pull request status
- Repository
- Drafts: show all, hide, or only
- Active comment threads
- Authored by you
- Assigned to you as reviewer
- Text search

Comment activity is loaded in the background. Pull requests with unresolved threads are marked in the list, while the Comments view can cycle between all, active, and resolved threads. File comments include their path and line range.

Descriptions and comments support terminal-styled Markdown, including headings, emphasis, lists, inline code, fenced code, and tables.

## Provider Support

| Provider | Status |
|---|---|
| Azure DevOps | Supported |
| GitHub | Planned; visible as a disabled provider in the setup screen |

Providers implement a shared `ProviderClient` interface, keeping the application ready for additional hosting services.

## Development

```bash
cargo build
cargo run
cargo test
cargo clippy --all-targets --all-features
```

Project structure:

```text
src/
├── api/          Shared pull request, filter, and configuration types
├── app/          TUI state, event loop, rendering, input, and detail views
├── cli/          Command definitions plus table and JSON output
├── config/       Config persistence and schema migration
└── providers/    Provider interface, registry, and Azure DevOps client
```

To add a provider:

1. Implement `ProviderClient` in `src/providers/<provider>/`.
2. Register it in `src/providers/registry.rs`.
3. Enable it in the provider registry when the implementation is ready.

## Terminal Notes

- **WSL:** Browser links open through Windows PowerShell automatically.
- **Remote sessions:** Ensure OSC 52 clipboard access is enabled in your terminal or multiplexer.
- **No color:** Set [`NO_COLOR`](https://no-color.org/) to disable foreground and background colors.

## Contributing

Issues and pull requests are welcome. Before submitting a change, run:

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features
```

### Commenting Standard

- Start each module with `//!` documentation that states its responsibility.
- Document every public item with `///`, exhaustively: a summary of its contract, an `# Arguments` (or per-parameter) section when it takes non-trivial parameters, a `# Returns` section describing the value, a `# Errors` section listing when and why a `Result` is `Err`, and a `# Panics` section for any panicking path. Omit a section only when it plainly doesn't apply (e.g. no `# Errors` on a function that can't fail).
- Use inline `//` comments to tell a story: constraints, invariants, compatibility decisions, and non-obvious failure handling. Never restate what the code already makes obvious.
- Explain why the code takes an approach; do not narrate syntax or repeat names.
- Describe the current design rather than preserving implementation history from earlier versions.
