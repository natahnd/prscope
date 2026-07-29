//! Command-line definitions and top-level command dispatch.

mod config_cmd;
mod helpers;
mod list;
pub mod plain_output;

use clap::{Args, Parser, Subcommand};

/// Parsed command-line invocation.
#[derive(Parser, Debug)]
#[command(
    name = "pr-reviews",
    version = "0.1.0",
    about = "Pull request review TUI for Azure DevOps and GitHub"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<TopCommand>,

    #[command(flatten)]
    pub list: ListArgs,
}

/// Top-level commands supported by the binary.
#[derive(Subcommand, Debug)]
pub enum TopCommand {
    /// List pull requests (interactive TUI by default)
    List(ListArgs),
    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigCommand,
    },
}

/// Operations supported by `pr-reviews config`.
#[derive(Subcommand, Debug)]
pub enum ConfigCommand {
    /// Show current configuration
    Show,
    /// Set configuration values
    Set {
        /// Default organization
        #[arg(short = 'o', long)]
        org: Option<String>,
        /// Default project (lastProject)
        #[arg(short = 'p', long)]
        project: Option<String>,
        /// Personal access token
        #[arg(long)]
        pat: Option<String>,
    },
}

/// Arguments shared by the default invocation and the explicit `list` command.
#[derive(Args, Clone, Default, Debug)]
pub struct ListArgs {
    /// Provider to use with --plain ("azure-devops" or "github"); otherwise
    /// falls back to the configured active provider
    #[arg(long)]
    pub provider: Option<String>,
    /// Azure DevOps organization
    #[arg(short = 'o', long)]
    pub org: Option<String>,
    /// Azure DevOps project, or GitHub owner (org or username) — skips the
    /// picker
    #[arg(short = 'p', long)]
    pub project: Option<String>,
    /// PR status filter
    #[arg(short = 's', long)]
    pub status: Option<String>,
    /// Filter by repository name
    #[arg(long)]
    pub repo: Option<String>,
    /// Filter by author display name
    #[arg(long)]
    pub author: Option<String>,
    /// Filter by reviewer ("me" supported)
    #[arg(long)]
    pub reviewer: Option<String>,
    /// Show only PRs created by you
    #[arg(long)]
    pub mine: bool,
    /// Plain output (no TUI)
    #[arg(long)]
    pub plain: bool,
    /// Output format for --plain (table|json)
    #[arg(long)]
    pub format: Option<String>,
}

/// Parses command-line arguments, dispatches the command, and reports failures.
///
/// # Returns
///
/// This function does not return a value. On a dispatch error, it prints the
/// error to stderr via [`plain_output::print_error`] and exits the process
/// with status `1` instead of returning.
pub async fn run() {
    let cli = Cli::parse();

    let result = match cli.command {
        Some(TopCommand::List(args)) => list::run_list_command(args).await,
        Some(TopCommand::Config { action }) => config_cmd::run_config_command(action).await,
        None => list::run_list_command(cli.list).await,
    };

    if let Err(err) = result {
        plain_output::print_error(&err.to_string());
        std::process::exit(1);
    }
}
