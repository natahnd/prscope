//! Azure DevOps credential resolution from Azure CLI or PAT sources.

use std::time::Duration;

use anyhow::{anyhow, Result};
use tokio::process::Command;
use tokio::time::timeout;

use crate::config::store::load_config;

const AZURE_DEVOPS_RESOURCE: &str = "499b84ac-1321-427f-aa17-267ca6975798";

/// Resolves an Azure DevOps credential in priority order.
///
/// Azure CLI is preferred, followed by `AZURE_DEVOPS_PAT`, then a PAT in the
/// application config. Azure CLI lookup is capped at 15 seconds so a missing or
/// unhealthy CLI cannot stall startup indefinitely.
///
/// # Returns
///
/// A usable access token or PAT string, from whichever source succeeds
/// first.
///
/// # Errors
///
/// Returns `Err` if the Azure CLI lookup fails or times out, no
/// `AZURE_DEVOPS_PAT` environment variable is set (or it's empty), and no
/// PAT is saved in the application config.
pub async fn resolve_azure_devops_token() -> Result<String> {
    if let Some(token) = get_token_from_az_cli().await {
        return Ok(token);
    }

    if let Ok(pat) = std::env::var("AZURE_DEVOPS_PAT") {
        if !pat.is_empty() {
            return Ok(pat);
        }
    }

    let config = load_config().await;
    if let Some(pat) = config.pat {
        return Ok(pat);
    }

    Err(anyhow!(
        "Authentication failed. Run `az login` or set AZURE_DEVOPS_PAT environment variable."
    ))
}

async fn get_token_from_az_cli() -> Option<String> {
    let run = Command::new("az")
        .args([
            "account",
            "get-access-token",
            "--resource",
            AZURE_DEVOPS_RESOURCE,
            "--query",
            "accessToken",
            "-o",
            "tsv",
        ])
        .output();

    let output = timeout(Duration::from_secs(15), run).await.ok()?.ok()?;
    if !output.status.success() {
        return None;
    }
    let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if token.is_empty() {
        None
    } else {
        Some(token)
    }
}
