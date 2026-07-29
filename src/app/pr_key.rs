use crate::api::types::NormalizedPullRequest;

pub fn pr_key(repository_id: &str, id: i64) -> String {
    format!("{repository_id}:{id}")
}

pub fn pr_key_for(pr: &NormalizedPullRequest) -> String {
    pr_key(&pr.repository_id, pr.id)
}
