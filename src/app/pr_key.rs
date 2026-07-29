//! Stable keys for pull requests across repositories.

use crate::api::types::NormalizedPullRequest;

/// Creates a key unique across repositories in the current provider.
///
/// # Arguments
///
/// * `repository_id` - Provider-assigned repository identifier.
/// * `id` - Pull request number, unique within `repository_id`.
///
/// # Returns
///
/// A string of the form `"{repository_id}:{id}"`, stable across app runs and
/// suitable as a `HashMap`/`HashSet` key.
pub fn pr_key(repository_id: &str, id: i64) -> String {
    format!("{repository_id}:{id}")
}

/// Creates a stable key for a normalized pull request.
///
/// # Arguments
///
/// * `pr` - The pull request to key.
///
/// # Returns
///
/// The same key [`pr_key`] would produce from `pr.repository_id` and `pr.id`.
pub fn pr_key_for(pr: &NormalizedPullRequest) -> String {
    pr_key(&pr.repository_id, pr.id)
}
