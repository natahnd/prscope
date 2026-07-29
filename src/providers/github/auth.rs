//! GitHub credential resolution from the `gh` CLI, environment, or config.

use std::time::Duration;

use anyhow::{anyhow, Result};
use tokio::process::Command;
use tokio::time::timeout;

use crate::config::store::load_config;

/// Resolves a GitHub credential in priority order.
///
/// The GitHub CLI is preferred, followed by `GITHUB_TOKEN`/`GH_TOKEN`, then a
/// PAT in the application config. `gh` lookup is capped at 15 seconds so a
/// missing or unhealthy CLI cannot stall startup indefinitely.
pub async fn resolve_github_token() -> Result<String> {
    if let Some(token) = get_token_from_gh_cli().await {
        return Ok(token);
    }

    for var in ["GITHUB_TOKEN", "GH_TOKEN"] {
        if let Ok(token) = std::env::var(var) {
            if !token.is_empty() {
                return Ok(token);
            }
        }
    }

    let config = load_config().await;
    if let Some(pat) = config.pat {
        if !pat.is_empty() {
            return Ok(pat);
        }
    }

    Err(anyhow!(
        "Authentication failed. Run `gh auth login` or set GITHUB_TOKEN."
    ))
}

async fn get_token_from_gh_cli() -> Option<String> {
    let run = Command::new("gh").args(["auth", "token"]).output();

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
