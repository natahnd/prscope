use anyhow::{anyhow, Result};

use crate::api::types::get_azure_devops_config;
use crate::config::store::{get_config_path, load_config, set_config_values, ConfigValuesPatch};

pub async fn show_config() {
    let config = load_config().await;
    println!("Config file: {}", get_config_path().display());
    println!("{}", serde_json::to_string_pretty(&config).unwrap());
}

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
