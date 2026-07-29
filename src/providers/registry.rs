//! Provider metadata, authentication dispatch, and client construction.

use std::sync::Arc;

use anyhow::{bail, Result};

use crate::api::types::{get_azure_devops_config, AppConfigV2, ProviderId};

use super::azure_devops;
use super::{ProviderClient, ProviderInfo};

const PROVIDERS: [ProviderInfo; 2] = [
    ProviderInfo {
        id: ProviderId::AzureDevops,
        name: "Azure DevOps",
        enabled: true,
    },
    ProviderInfo {
        id: ProviderId::Github,
        name: "GitHub (coming soon)",
        enabled: false,
    },
];

/// Returns providers in their display order.
///
/// # Returns
///
/// All known `ProviderInfo` entries (enabled and disabled/"coming soon"),
/// in the order they should appear in the first-run picker.
pub fn list_providers() -> Vec<ProviderInfo> {
    PROVIDERS.to_vec()
}

/// Looks up metadata for a provider identifier.
///
/// # Arguments
///
/// * `id` - Provider identifier to look up.
///
/// # Returns
///
/// `Some` with the matching `ProviderInfo`; `None` should not occur in
/// practice since `PROVIDERS` covers every `ProviderId` variant.
#[allow(dead_code)]
pub fn get_provider(id: ProviderId) -> Option<ProviderInfo> {
    PROVIDERS.iter().copied().find(|p| p.id == id)
}

/// Resolves credentials using the selected provider's authentication strategy.
///
/// # Arguments
///
/// * `id` - Provider to resolve a credential for.
///
/// # Returns
///
/// A token or access credential string usable to construct a client for
/// `id`.
///
/// # Errors
///
/// Returns `Err` if `id` is `ProviderId::Github` (not yet implemented), or
/// if Azure DevOps credential resolution fails (no Azure CLI session and no
/// `AZURE_DEVOPS_PAT`/saved PAT).
pub async fn resolve_provider_token(id: ProviderId) -> Result<String> {
    match id {
        ProviderId::AzureDevops => azure_devops::auth::resolve_azure_devops_token().await,
        ProviderId::Github => bail!("GitHub provider is not yet available."),
    }
}

/// Builds a provider client from persisted application config.
///
/// # Arguments
///
/// * `id` - Provider to build a client for.
/// * `token` - Resolved credential from [`resolve_provider_token`].
/// * `config` - Loaded application configuration, used for provider-specific
///   settings (e.g. the Azure DevOps organization).
///
/// # Returns
///
/// A boxed `ProviderClient` for `id`, ready to make API calls.
///
/// # Errors
///
/// Returns `Err` if `id` is `ProviderId::Github` (not yet implemented), or
/// if building the Azure DevOps client fails (e.g. no organization
/// configured).
pub fn create_client_from_config(
    id: ProviderId,
    token: String,
    config: &AppConfigV2,
) -> Result<Arc<dyn ProviderClient>> {
    match id {
        ProviderId::AzureDevops => {
            let ado = get_azure_devops_config(config);
            let client = azure_devops::client::create_azure_devops_client(token, ado.organization)?;
            Ok(Arc::new(client))
        }
        ProviderId::Github => bail!("GitHub provider is not yet available."),
    }
}

/// Builds an Azure DevOps client before setup values have been persisted.
///
/// # Arguments
///
/// * `token` - Resolved Azure DevOps credential.
/// * `organization` - Organization name entered during setup, not yet saved
///   to config.
///
/// # Returns
///
/// A boxed `ProviderClient` for Azure DevOps, ready to make API calls.
///
/// # Errors
///
/// Returns `Err` if the underlying HTTP client fails to construct (e.g. an
/// invalid base URL derived from `organization`).
pub fn create_azure_devops_client_direct(
    token: String,
    organization: String,
) -> Result<Arc<dyn ProviderClient>> {
    let client = azure_devops::client::create_azure_devops_client(token, Some(organization))?;
    Ok(Arc::new(client))
}
