//! Best-effort discovery of the user's default Azure DevOps organization.

use std::time::Duration;

use tokio::process::Command;
use tokio::time::timeout;

/// Finds the default organization configured by the Azure DevOps CLI.
///
/// CLI output is attempted first, then the extension's config file is read
/// directly. Detection is best-effort and returns `None` on any failure.
pub async fn detect_azure_devops_org() -> Option<String> {
    if let Some(org) = detect_from_az_cli().await {
        return Some(org);
    }
    detect_from_config_file().await
}

async fn detect_from_az_cli() -> Option<String> {
    let run = Command::new("az")
        .args(["devops", "configure", "--list"])
        .output();
    let output = timeout(Duration::from_secs(10), run).await.ok()?.ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_org_from_configure_output(&stdout)
}

async fn detect_from_config_file() -> Option<String> {
    let home = dirs::home_dir()?;
    let path = home.join(".azure").join("azuredevops").join("config");
    let content = tokio::fs::read_to_string(path).await.ok()?;
    parse_org_from_configure_output(&content)
}

fn parse_org_from_configure_output(text: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("organization") {
            continue;
        }
        let value = trimmed.splitn(2, '=').nth(1)?.trim().to_string();
        if let Some(org) = extract_org_from_url(&value) {
            return Some(org);
        }
        if !value.is_empty() && !value.contains('/') {
            return Some(value);
        }
    }
    None
}

fn extract_org_from_url(value: &str) -> Option<String> {
    let lower = value.to_lowercase();
    let marker = "dev.azure.com/";
    let idx = lower.find(marker)?;
    let rest = &value[idx + marker.len()..];
    let org: String = rest
        .chars()
        .take_while(|c| !c.is_whitespace() && *c != '/')
        .collect();
    if org.is_empty() {
        None
    } else {
        Some(org)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_organization_url() {
        let text = "organization = https://dev.azure.com/my-org\nother = value";
        assert_eq!(
            parse_org_from_configure_output(text).as_deref(),
            Some("my-org")
        );
    }

    #[test]
    fn parses_bare_organization_name() {
        let text = "organization = my-org";
        assert_eq!(
            parse_org_from_configure_output(text).as_deref(),
            Some("my-org")
        );
    }

    #[test]
    fn returns_none_when_missing() {
        let text = "defaults.project = Something";
        assert_eq!(parse_org_from_configure_output(text), None);
    }
}
