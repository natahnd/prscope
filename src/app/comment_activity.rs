use std::collections::HashSet;
use std::sync::Arc;

use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Semaphore;

use crate::api::types::NormalizedPullRequest;
use crate::providers::ProviderClient;

use super::comment_status::has_active_thread;
use super::pr_key::pr_key_for;
use super::AppEvent;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentActivity {
    Active,
    Resolved,
    Error,
}

const CONCURRENCY: usize = 6;

/// Lazily fetches each PR's comment threads in the background (bounded
/// concurrency via a semaphore) to determine whether it has any unresolved
/// thread. `requested` persists for the whole app run (like the TS hook's
/// ref), so a project switch only fetches PRs not already seen.
pub fn spawn_missing_comment_activity(
    client: Arc<dyn ProviderClient>,
    project: String,
    pull_requests: &[NormalizedPullRequest],
    requested: &mut HashSet<String>,
    sender: UnboundedSender<AppEvent>,
) {
    let pending: Vec<NormalizedPullRequest> = pull_requests
        .iter()
        .filter(|pr| !requested.contains(&pr_key_for(pr)))
        .cloned()
        .collect();
    for pr in &pending {
        requested.insert(pr_key_for(pr));
    }
    if pending.is_empty() {
        return;
    }

    let semaphore = Arc::new(Semaphore::new(CONCURRENCY));
    for pr in pending {
        let client = client.clone();
        let project = project.clone();
        let sender = sender.clone();
        let semaphore = semaphore.clone();
        tokio::spawn(async move {
            let _permit = semaphore.acquire_owned().await.ok();
            let key = pr_key_for(&pr);
            let activity = match client
                .list_pull_request_threads(&project, &pr.repository_id, pr.id)
                .await
            {
                Ok(threads) => {
                    if has_active_thread(&threads) {
                        CommentActivity::Active
                    } else {
                        CommentActivity::Resolved
                    }
                }
                Err(_) => CommentActivity::Error,
            };
            let _ = sender.send(AppEvent::CommentActivityLoaded { key, activity });
        });
    }
}
