//! Shared rules for classifying provider comment-thread statuses.

use crate::api::types::PrThread;

const RESOLVED_STATUSES: [&str; 4] = ["fixed", "wontfix", "closed", "bydesign"];

/// Returns whether a provider status represents a completed thread.
///
/// # Arguments
///
/// * `status` - Raw provider status string (e.g. `"fixed"`, `"active"`),
///   compared case-insensitively.
///
/// # Returns
///
/// `true` if `status` matches one of the known resolved states (`fixed`,
/// `wontfix`, `closed`, `bydesign`); `false` otherwise, including for
/// statuses the provider defines but this app doesn't recognize.
pub fn is_resolved_status(status: &str) -> bool {
    RESOLVED_STATUSES.contains(&status.to_lowercase().as_str())
}

/// Returns whether any thread in the list still needs action.
///
/// # Arguments
///
/// * `threads` - Comment threads to inspect.
///
/// # Returns
///
/// `true` if at least one thread's status is not resolved (see
/// [`is_resolved_status`]); `false` if `threads` is empty or every thread is
/// resolved.
pub fn has_active_thread(threads: &[PrThread]) -> bool {
    threads.iter().any(|t| !is_resolved_status(&t.status))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolved_statuses_are_case_insensitive() {
        assert!(is_resolved_status("Fixed"));
        assert!(is_resolved_status("WONTFIX"));
        assert!(!is_resolved_status("active"));
    }

    #[test]
    fn has_active_thread_detects_unresolved() {
        let threads = vec![
            PrThread {
                id: 1,
                status: "fixed".into(),
                file_path: None,
                line: None,
                line_end: None,
                comments: vec![],
            },
            PrThread {
                id: 2,
                status: "active".into(),
                file_path: None,
                line: None,
                line_end: None,
                comments: vec![],
            },
        ];
        assert!(has_active_thread(&threads));
        assert!(!has_active_thread(&threads[..1]));
    }
}
