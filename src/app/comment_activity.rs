//! Background discovery of unresolved comment activity for PR list rows.

use std::collections::HashSet;
use std::sync::Arc;

use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Semaphore;

use crate::api::types::NormalizedPullRequest;
use crate::providers::ProviderClient;

use super::comment_status::has_active_thread;
use super::pr_key::pr_key_for;
use super::AppEvent;

/// Summary of whether a pull request has actionable comment threads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentActivity {
    Active,
    Resolved,
    Error,
}

const CONCURRENCY: usize = 6;

/// Lazily fetches comment threads with bounded concurrency to determine which
/// pull requests still have unresolved feedback.
///
/// `requested` lives for the full application run, so repeated refreshes or
/// project switches do not enqueue duplicate work for an already-seen PR key.
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
