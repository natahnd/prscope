use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::mpsc::UnboundedSender;

use crate::api::types::{NormalizedPullRequest, PrThread, PullRequestDetail};
use crate::providers::ProviderClient;

use super::pr_key::pr_key_for;
use super::AppEvent;

#[derive(Debug, Clone, Default)]
pub struct PullRequestDetailState {
    pub detail: Option<PullRequestDetail>,
    pub threads: Vec<PrThread>,
    pub loading: bool,
    pub error: Option<String>,
}

/// Per-PR cache of fetched detail (description + comment threads), keyed by
/// `repositoryId:id`. Mirrors `usePullRequestDetail`'s ref-based cache: once
/// fetched for a PR, it's never re-fetched for the lifetime of the app run.
#[derive(Default)]
pub struct DetailCache {
    entries: HashMap<String, PullRequestDetailState>,
    in_flight: std::collections::HashSet<String>,
}

impl DetailCache {
    pub fn get(&self, key: &str) -> Option<&PullRequestDetailState> {
        self.entries.get(key)
    }

    pub fn insert(&mut self, key: String, state: PullRequestDetailState) {
        self.in_flight.remove(&key);
        self.entries.insert(key, state);
    }

    pub fn is_in_flight(&self, key: &str) -> bool {
        self.in_flight.contains(key)
    }

    pub fn mark_in_flight(&mut self, key: String) {
        self.in_flight.insert(key);
    }
}

/// Spawns a background fetch for one PR's description + comment threads and
/// reports the result back over `sender`. No cancellation token is needed:
/// results are cached by PR key, so a stale in-flight fetch for a PR the user
/// has since navigated away from is simply cached for next time, harmlessly.
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
