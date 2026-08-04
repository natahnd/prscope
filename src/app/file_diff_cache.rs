use std::sync::Arc;

use tokio::sync::mpsc::UnboundedSender;

use crate::api::types::{FileDiff, PrFileChange, PrThread};
use crate::diffing::attach_comments;
use crate::providers::ProviderClient;

use super::AppEvent;

/// Per-file diff cache entry, mirroring `PullRequestDetailState`'s
/// loading/error shape. Once populated (success or failure), a file is never
/// re-fetched for the lifetime of the app run.
#[derive(Debug, Clone, Default)]
pub struct FileDiffState {
    pub diff: Option<FileDiff>,
    pub loading: bool,
    pub error: Option<String>,
}

pub fn file_diff_key(pr_key: &str, path: &str) -> String {
    format!("{pr_key}:{path}")
}

/// Spawns a background fetch of one file's diff and attaches any matching PR
/// comments before reporting back over `sender`. `threads` is a snapshot of
/// the PR's comment threads at request time (cheap — already loaded as part
/// of the PR detail fetch).
#[allow(clippy::too_many_arguments)]
pub fn spawn_file_diff_fetch(
    client: Arc<dyn ProviderClient>,
    project: String,
    repository_id: String,
    pull_request_id: i64,
    file: PrFileChange,
    threads: Vec<PrThread>,
    key: String,
    sender: UnboundedSender<AppEvent>,
) {
    tokio::spawn(async move {
        let result = client
            .get_pull_request_file_diff(&project, &repository_id, pull_request_id, &file)
            .await;
        let state = match result {
            Ok(mut diff) => {
                attach_comments(&mut diff, &threads);
                FileDiffState {
                    diff: Some(diff),
                    loading: false,
                    error: None,
                }
            }
            Err(err) => FileDiffState {
                diff: None,
                loading: false,
                error: Some(err.to_string()),
            },
        };
        let _ = sender.send(AppEvent::FileDiffLoaded { key, state });
    });
}
