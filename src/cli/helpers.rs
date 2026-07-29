//! Shared configuration and organization-resolution helpers for CLI commands.

use anyhow::{anyhow, Result};

use crate::api::types::get_azure_devops_config;
use crate::config::store::{get_config_path, load_config, set_config_values, ConfigValuesPatch};

/// Prints the config path and deserialized config document.
///
/// # Returns
///
/// This function does not return a value; it always prints, since
/// [`load_config`] falls back to a default config rather than failing.
pub async fn show_config() {
    let config = load_config().await;
    println!("Config file: {}", get_config_path().display());
    println!("{}", serde_json::to_string_pretty(&config).unwrap());
}

/// Applies values supplied to `config set` and prints the resulting document.
///
/// # Arguments
///
/// * `values` - Parsed `--org`/`--project`/`--pat` values from the CLI.
///
/// # Returns
///
/// `Ok(())` once the updated config has been saved and printed. If `values`
/// has no fields set, this function does not return at all: it prints a
/// usage message to stderr and terminates the process with
/// `std::process::exit(1)`.
///
/// # Errors
///
/// Returns `Err` if saving the updated config fails.
pub async fn set_config(values: ConfigValuesPatch) -> Result<()> {
    if values.org.is_none() && values.project.is_none() && values.pat.is_none() {
        eprintln!("Nothing to set. Use --org, --project, or --pat.");
        std::process::exit(1);
    }

    let config = set_config_values(values).await?;
    println!("Config updated:");
    println!("{}", serde_json::to_string_pretty(&config).unwrap());
    Ok(())
}

/// Resolves an organization from the CLI flag, then persisted config.
///
/// # Arguments
///
/// * `flag_org` - The `--org` value passed on the command line, if any.
///
/// # Returns
///
/// `flag_org` if set; otherwise the organization saved in the persisted
/// Azure DevOps config.
///
/// # Errors
///
/// Returns `Err` if `flag_org` is `None` and no organization is saved in the
/// config, with a message telling the user to pass `--org` or configure one
/// via the TUI.
pub async fn resolve_organization(flag_org: Option<String>) -> Result<String> {
    if let Some(org) = flag_org {
        return Ok(org);
    }
    let config = load_config().await;
    let ado = get_azure_devops_config(&config);
    if let Some(org) = ado.organization {
        return Ok(org);
    }
    Err(anyhow!(
        "Organization required. Pass --org or configure Azure DevOps via the TUI."
    ))
}
