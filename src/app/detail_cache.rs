//! In-memory caching and background loading for pull request details.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::mpsc::UnboundedSender;

use crate::api::types::{NormalizedPullRequest, PrThread, PullRequestDetail};
use crate::providers::ProviderClient;

use super::pr_key::pr_key_for;
use super::AppEvent;

/// Loading state for a pull request's description and comment threads.
#[derive(Debug, Clone, Default)]
pub struct PullRequestDetailState {
    pub detail: Option<PullRequestDetail>,
    pub threads: Vec<PrThread>,
    pub loading: bool,
    pub error: Option<String>,
}

/// Per-PR cache of fetched descriptions and comment threads, keyed by
/// `repositoryId:id`.
///
/// Entries are immutable for the lifetime of the application run. An explicit
/// list refresh does not invalidate detail data.
#[derive(Default)]
pub struct DetailCache {
    entries: HashMap<String, PullRequestDetailState>,
    in_flight: std::collections::HashSet<String>,
}

impl DetailCache {
    /// Returns cached detail for a PR key.
    ///
    /// # Arguments
    ///
    /// * `key` - PR key in `repositoryId:id` form, from [`pr_key_for`].
    ///
    /// # Returns
    ///
    /// `Some` with the cached state if `key` has previously completed a
    /// fetch (via [`DetailCache::insert`]); `None` if it hasn't been fetched
    /// yet or is still in flight.
    pub fn get(&self, key: &str) -> Option<&PullRequestDetailState> {
        self.entries.get(key)
    }

    /// Stores a completed result and clears its in-flight marker.
    ///
    /// # Arguments
    ///
    /// * `key` - PR key the result belongs to.
    /// * `state` - Completed load result (success or error) to cache.
    pub fn insert(&mut self, key: String, state: PullRequestDetailState) {
        self.in_flight.remove(&key);
        self.entries.insert(key, state);
    }

    /// Returns whether a detail request is already running for a PR key.
    ///
    /// # Arguments
    ///
    /// * `key` - PR key to check.
    ///
    /// # Returns
    ///
    /// `true` if [`DetailCache::mark_in_flight`] was called for `key` and no
    /// matching [`DetailCache::insert`] has completed yet; `false` otherwise.
    pub fn is_in_flight(&self, key: &str) -> bool {
        self.in_flight.contains(key)
    }

    /// Marks a PR key as having an active detail request.
    ///
    /// # Arguments
    ///
    /// * `key` - PR key to mark as in flight, so concurrent callers don't
    ///   spawn a duplicate fetch for it.
    pub fn mark_in_flight(&mut self, key: String) {
        self.in_flight.insert(key);
    }
}

/// Fetches one PR's description and comment threads in the background.
///
/// Results are returned over `sender` and keyed by PR, so completion remains
/// safe even if the user navigates to another row before the request finishes.
///
/// # Arguments
///
/// * `client` - Provider client used to fetch the PR detail and its threads.
/// * `project` - Project the pull request belongs to.
/// * `pr` - Pull request summary to load full detail for.
/// * `sender` - Channel used to deliver the resulting `AppEvent::DetailLoaded`.
///
/// # Returns
///
/// This function does not return a value; the fetch runs in a spawned task
/// and its result (success or failure) is sent over `sender` as a
/// `PullRequestDetailState` with `error` set on failure. It never propagates
/// the underlying provider error to the caller directly.
pub fn spawn_detail_fetch(
    client: Arc<dyn ProviderClient>,
    project: String,
    pr: NormalizedPullRequest,
    sender: UnboundedSender<AppEvent>,
) {
    let key = pr_key_for(&pr);
    tokio::spawn(async move {
        let result = tokio::try_join!(
            client.get_pull_request(&project, &pr.repository_id, pr.id),
            client.list_pull_request_threads(&project, &pr.repository_id, pr.id)
        );
        let state = match result {
            Ok((detail, threads)) => PullRequestDetailState {
                detail: Some(detail),
                threads,
                loading: false,
                error: None,
            },
            Err(err) => PullRequestDetailState {
                detail: None,
                threads: vec![],
                loading: false,
                error: Some(err.to_string()),
            },
        };
        let _ = sender.send(AppEvent::DetailLoaded { key, state });
    });
}
