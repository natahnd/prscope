mod config_cmd;
mod helpers;
mod list;
pub mod plain_output;

use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "pr-reviews",
    version = "0.1.0",
    about = "Azure DevOps pull request review TUI"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<TopCommand>,

    #[command(flatten)]
    pub list: ListArgs,
}

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

#[derive(Args, Clone, Default, Debug)]
pub struct ListArgs {
    /// Azure DevOps organization
    #[arg(short = 'o', long)]
    pub org: Option<String>,
    /// Azure DevOps project (skips picker)
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
