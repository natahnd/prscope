use anyhow::Result;

use super::helpers::{set_config, show_config};
use super::ConfigCommand;
use crate::config::store::ConfigValuesPatch;

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
