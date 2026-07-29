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

pub fn list_providers() -> Vec<ProviderInfo> {
    PROVIDERS.to_vec()
}

#[allow(dead_code)]
pub fn get_provider(id: ProviderId) -> Option<ProviderInfo> {
    PROVIDERS.iter().copied().find(|p| p.id == id)
}

pub async fn resolve_provider_token(id: ProviderId) -> Result<String> {
    match id {
        ProviderId::AzureDevops => azure_devops::auth::resolve_azure_devops_token().await,
        ProviderId::Github => bail!("GitHub provider is not yet available."),
    }
}

/// Builds a client from the persisted app config (used on app bootstrap).
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

/// Builds an Azure DevOps client directly from a freshly-entered organization
/// name (used by the setup flow's test-connection step, before it's saved).
pub fn create_azure_devops_client_direct(
    token: String,
    organization: String,
) -> Result<Arc<dyn ProviderClient>> {
    let client = azure_devops::client::create_azure_devops_client(token, Some(organization))?;
    Ok(Arc::new(client))
}
