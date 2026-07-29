//! Versioned config loading, migration, persistence, and provider accessors.

use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::api::types::{
    AppConfigV2, AzureDevOpsProviderConfig, PrListLayout, ProviderId, PullRequestFilters,
    RawAppConfig,
};

fn config_dir() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg).join("pr-reviews");
        }
    }
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config")
        .join("pr-reviews")
}

fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

fn migrate_config(raw: RawAppConfig) -> AppConfigV2 {
    if raw.version == Some(2) {
        return AppConfigV2 {
            version: 2,
            active_provider: raw.active_provider,
            providers: raw.providers,
            pat: raw.pat,
        };
    }

    let mut ado = AzureDevOpsProviderConfig::default();
    if let Some(org) = raw.organization {
        ado.organization = Some(org);
    }
    if let Some(project) = raw.last_project {
        ado.last_project = Some(project);
    }

    let mut migrated = AppConfigV2 {
        version: 2,
        active_provider: None,
        providers: Default::default(),
        pat: raw.pat,
    };

    if ado.organization.is_some() || ado.last_project.is_some() {
        migrated.active_provider = Some(ProviderId::AzureDevops);
        migrated.providers.azure_devops = Some(ado);
    }

    migrated
}

/// Loads and, when necessary, migrates the application config.
///
/// Missing, unreadable, or invalid files intentionally produce a default
/// config so first-run setup can recover without manual file cleanup.
///
/// # Returns
///
/// The loaded and migrated `AppConfigV2`, or `AppConfigV2::default()` if the
/// file is missing, unreadable, or fails to parse. This function never
/// returns an error or panics.
pub async fn load_config() -> AppConfigV2 {
    match load_config_inner().await {
        Ok(config) => config,
        Err(_) => AppConfigV2::default(),
    }
}

async fn load_config_inner() -> Result<AppConfigV2> {
    let contents = tokio::fs::read_to_string(config_path()).await?;
    let raw: RawAppConfig = serde_json::from_str(&contents)?;
    let needs_migration = raw.version != Some(2);
    let config = migrate_config(raw);
    if needs_migration {
        save_config(&config).await?;
    }
    Ok(config)
}

/// Persists config as formatted JSON and restricts Unix permissions to `0600`.
///
/// # Arguments
///
/// * `config` - The full configuration to write to disk.
///
/// # Returns
///
/// `Ok(())` once the config file has been written.
///
/// # Errors
///
/// Returns `Err` if the config directory can't be created, the config can't
/// be serialized, or the file can't be written. Failing to apply `0600`
/// permissions on Unix is intentionally best-effort and does not cause this
/// function to return `Err`.
pub async fn save_config(config: &AppConfigV2) -> Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("creating config directory {}", parent.display()))?;
    }
    let mut body = serde_json::to_string_pretty(config)?;
    body.push('\n');
    tokio::fs::write(&path, &body)
        .await
        .with_context(|| format!("writing config file {}", path.display()))?;
    // Permission hardening is best-effort because the config is already
    // durable and some filesystems do not support Unix permission changes.
    set_owner_only_permissions(&path).await;
    Ok(())
}

#[cfg(unix)]
async fn set_owner_only_permissions(path: &PathBuf) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(metadata) = tokio::fs::metadata(path).await {
        let mut perms = metadata.permissions();
        perms.set_mode(0o600);
        let _ = tokio::fs::set_permissions(path, perms).await;
    }
}

#[cfg(not(unix))]
async fn set_owner_only_permissions(_path: &PathBuf) {}

/// Updates the last project for the active provider.
///
/// # Arguments
///
/// * `project` - Name of the project to remember as last-used.
///
/// # Returns
///
/// `Ok(())` once saved. Does nothing (but still returns `Ok(())`) if no
/// provider is currently active, or if the active provider isn't Azure
/// DevOps (the only provider with a persisted "last project" today).
///
/// # Errors
///
/// Returns `Err` if the updated config fails to save (see
/// [`save_config`]).
pub async fn update_last_project(project: &str) -> Result<()> {
    let mut config = load_config().await;
    let Some(active) = config.active_provider else {
        return Ok(());
    };
    if active == ProviderId::AzureDevops {
        let mut ado = config.providers.azure_devops.clone().unwrap_or_default();
        ado.last_project = Some(project.to_string());
        config.providers.azure_devops = Some(ado);
    } else if active == ProviderId::Github {
        let mut github = config.providers.github.clone().unwrap_or_default();
        github.last_project = Some(project.to_string());
        config.providers.github = Some(github);
    }
    save_config(&config).await
}

/// Updates the last interactive filter state for the active provider.
///
/// # Arguments
///
/// * `filters` - The filter selections to persist for next launch.
///
/// # Returns
///
/// `Ok(())` once saved. Does nothing (but still returns `Ok(())`) if no
/// provider is active or the active provider isn't Azure DevOps.
///
/// # Errors
///
/// Returns `Err` if the updated config fails to save (see
/// [`save_config`]).
pub async fn update_last_filters(filters: &PullRequestFilters) -> Result<()> {
    let mut config = load_config().await;
    let Some(active) = config.active_provider else {
        return Ok(());
    };
    if active == ProviderId::AzureDevops {
        let mut ado = config.providers.azure_devops.clone().unwrap_or_default();
        ado.last_filters = Some(filters.clone());
        config.providers.azure_devops = Some(ado);
    } else if active == ProviderId::Github {
        let mut github = config.providers.github.clone().unwrap_or_default();
        github.last_filters = Some(filters.clone());
        config.providers.github = Some(github);
    }
    save_config(&config).await
}

/// Updates the last PR list grouping or sort layout for the active provider.
///
/// # Arguments
///
/// * `layout` - The list layout to persist for next launch.
///
/// # Returns
///
/// `Ok(())` once saved. Does nothing (but still returns `Ok(())`) if no
/// provider is active or the active provider isn't Azure DevOps.
///
/// # Errors
///
/// Returns `Err` if the updated config fails to save (see
/// [`save_config`]).
pub async fn update_last_list_layout(layout: PrListLayout) -> Result<()> {
    let mut config = load_config().await;
    let Some(active) = config.active_provider else {
        return Ok(());
    };
    if active == ProviderId::AzureDevops {
        let mut ado = config.providers.azure_devops.clone().unwrap_or_default();
        ado.last_list_layout = Some(layout);
        config.providers.azure_devops = Some(ado);
    } else if active == ProviderId::Github {
        let mut github = config.providers.github.clone().unwrap_or_default();
        github.last_list_layout = Some(layout);
        config.providers.github = Some(github);
    }
    save_config(&config).await
}

/// Activates a provider and saves the settings collected during setup.
///
/// # Arguments
///
/// * `provider_id` - Provider to activate.
/// * `organization` - Organization name to persist, if the provider needs
///   one (Azure DevOps does).
///
/// # Returns
///
/// The updated `AppConfigV2` after the change has been saved.
///
/// # Errors
///
/// Returns `Err` if the updated config fails to save (see
/// [`save_config`]).
pub async fn save_provider_config(
    provider_id: ProviderId,
    organization: Option<String>,
) -> Result<AppConfigV2> {
    let mut config = load_config().await;
    config.active_provider = Some(provider_id);
    if provider_id == ProviderId::AzureDevops {
        let mut ado = config.providers.azure_devops.clone().unwrap_or_default();
        if let Some(org) = organization {
            ado.organization = Some(org);
        }
        config.providers.azure_devops = Some(ado);
    }
    save_config(&config).await?;
    Ok(config)
}

/// Partial update accepted by the `config set` command.
#[derive(Debug, Clone, Default)]
pub struct ConfigValuesPatch {
    pub org: Option<String>,
    pub project: Option<String>,
    pub pat: Option<String>,
}

/// Applies a partial config update without clearing unspecified values.
///
/// # Arguments
///
/// * `values` - Fields to update; any field left `None` keeps its existing
///   persisted value.
///
/// # Returns
///
/// The updated `AppConfigV2` after the change has been saved.
///
/// # Errors
///
/// Returns `Err` if the updated config fails to save (see
/// [`save_config`]).
pub async fn set_config_values(values: ConfigValuesPatch) -> Result<AppConfigV2> {
    let mut config = load_config().await;

    if let Some(pat) = values.pat {
        config.pat = Some(pat);
    }

    if values.org.is_some() || values.project.is_some() {
        config.active_provider = Some(ProviderId::AzureDevops);
        let mut ado = config.providers.azure_devops.clone().unwrap_or_default();
        if let Some(org) = values.org {
            ado.organization = Some(org);
        }
        if let Some(project) = values.project {
            ado.last_project = Some(project);
        }
        config.providers.azure_devops = Some(ado);
    }

    save_config(&config).await?;
    Ok(config)
}

/// Returns the resolved path to the application config file.
///
/// # Returns
///
/// The absolute path to `config.json`, honoring `XDG_CONFIG_HOME` when set,
/// otherwise `~/.config/pr-reviews/config.json`.
pub fn get_config_path() -> PathBuf {
    config_path()
}

/// Returns the provider context shown in screen headers.
///
/// # Arguments
///
/// * `config` - The loaded application configuration.
///
/// # Returns
///
/// The Azure DevOps organization name if that's the active provider and one
/// is saved; otherwise the active provider's display name, or an empty
/// string if no provider is active.
pub fn get_provider_display_context(config: &AppConfigV2) -> String {
    if config.active_provider == Some(ProviderId::AzureDevops) {
        return config
            .providers
            .azure_devops
            .as_ref()
            .and_then(|c| c.organization.clone())
            .unwrap_or_default();
    }
    config
        .active_provider
        .map(|p| p.to_string())
        .unwrap_or_default()
}

/// Returns the last project for the active provider.
///
/// # Arguments
///
/// * `config` - The loaded application configuration.
///
/// # Returns
///
/// `Some` with the saved project name if the active provider is Azure
/// DevOps and one is saved; `None` otherwise.
pub fn get_last_project(config: &AppConfigV2) -> Option<String> {
    if config.active_provider == Some(ProviderId::AzureDevops) {
        return config
            .providers
            .azure_devops
            .as_ref()
            .and_then(|c| c.last_project.clone());
    }
    if config.active_provider == Some(ProviderId::Github) {
        return config
            .providers
            .github
            .as_ref()
            .and_then(|c| c.last_project.clone());
    }
    None
}

/// Returns the last filters saved by the interactive view.
///
/// # Arguments
///
/// * `config` - The loaded application configuration.
///
/// # Returns
///
/// `Some` with the saved filters if the active provider is Azure DevOps and
/// filters have been persisted; `None` otherwise.
pub fn get_last_filters(config: &AppConfigV2) -> Option<PullRequestFilters> {
    if config.active_provider == Some(ProviderId::AzureDevops) {
        return config
            .providers
            .azure_devops
            .as_ref()
            .and_then(|c| c.last_filters.clone());
    }
    if config.active_provider == Some(ProviderId::Github) {
        return config
            .providers
            .github
            .as_ref()
            .and_then(|c| c.last_filters.clone());
    }
    None
}

/// Returns the saved PR list layout or its default.
///
/// # Arguments
///
/// * `config` - The loaded application configuration.
///
/// # Returns
///
/// The saved layout if the active provider is Azure DevOps and one is
/// persisted; otherwise `PrListLayout::default()`.
pub fn get_last_list_layout(config: &AppConfigV2) -> PrListLayout {
    if config.active_provider == Some(ProviderId::AzureDevops) {
        if let Some(layout) = config
            .providers
            .azure_devops
            .as_ref()
            .and_then(|c| c.last_list_layout)
        {
            return layout;
        }
    }
    if config.active_provider == Some(ProviderId::Github) {
        if let Some(layout) = config.providers.github.as_ref().and_then(|c| c.last_list_layout) {
            return layout;
        }
    }
    PrListLayout::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::types::ProviderConfigs;

    #[test]
    fn migrates_legacy_v1_config() {
        let raw = RawAppConfig {
            version: None,
            active_provider: None,
            providers: ProviderConfigs::default(),
            pat: Some("secret".into()),
            organization: Some("acme".into()),
            last_project: Some("MyProject".into()),
        };
        let migrated = migrate_config(raw);
        assert_eq!(migrated.version, 2);
        assert_eq!(migrated.active_provider, Some(ProviderId::AzureDevops));
        let ado = migrated.providers.azure_devops.unwrap();
        assert_eq!(ado.organization.as_deref(), Some("acme"));
        assert_eq!(ado.last_project.as_deref(), Some("MyProject"));
        assert_eq!(migrated.pat.as_deref(), Some("secret"));
    }

    #[test]
    fn v2_config_passes_through_unchanged() {
        let mut providers = ProviderConfigs::default();
        providers.azure_devops = Some(AzureDevOpsProviderConfig {
            organization: Some("acme".into()),
            ..Default::default()
        });
        let raw = RawAppConfig {
            version: Some(2),
            active_provider: Some(ProviderId::AzureDevops),
            providers,
            pat: None,
            organization: None,
            last_project: None,
        };
        let migrated = migrate_config(raw);
        assert_eq!(migrated.version, 2);
        assert_eq!(
            migrated.providers.azure_devops.unwrap().organization.as_deref(),
            Some("acme")
        );
    }

    #[test]
    fn empty_v1_config_has_no_active_provider() {
        let raw = RawAppConfig {
            version: None,
            active_provider: None,
            providers: ProviderConfigs::default(),
            pat: None,
            organization: None,
            last_project: None,
        };
        let migrated = migrate_config(raw);
        assert_eq!(migrated.active_provider, None);
    }
}
