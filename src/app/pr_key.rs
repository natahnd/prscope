//! Stable keys for pull requests across repositories.

use crate::api::types::NormalizedPullRequest;

/// Creates a key unique across repositories in the current provider.
pub fn pr_key(repository_id: &str, id: i64) -> String {
    format!("{repository_id}:{id}")
}

/// Creates a stable key for a normalized pull request.
pub fn pr_key_for(pr: &NormalizedPullRequest) -> String {
    pr_key(&pr.repository_id, pr.id)
}
