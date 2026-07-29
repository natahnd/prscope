//! Execution of configuration subcommands.

use anyhow::Result;

use super::helpers::{set_config, show_config};
use super::ConfigCommand;
use crate::config::store::ConfigValuesPatch;

/// Executes a parsed configuration subcommand.
///
/// # Arguments
///
/// * `action` - The parsed `config show` or `config set` subcommand.
///
/// # Returns
///
/// `Ok(())` once the subcommand has completed.
///
/// # Errors
///
/// Returns `Err` if `config set` fails to save (see
/// [`super::helpers::set_config`]). `config show` never fails.
pub async fn run_config_command(action: ConfigCommand) -> Result<()> {
    match action {
        ConfigCommand::Show => {
            show_config().await;
            Ok(())
        }
        ConfigCommand::Set { org, project, pat } => {
            set_config(ConfigValuesPatch { org, project, pat }).await
        }
    }
}
