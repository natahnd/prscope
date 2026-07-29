//! Shared rules for classifying provider comment-thread statuses.

use crate::api::types::PrThread;

const RESOLVED_STATUSES: [&str; 4] = ["fixed", "wontfix", "closed", "bydesign"];

/// Returns whether a provider status represents a completed thread.
pub fn is_resolved_status(status: &str) -> bool {
    RESOLVED_STATUSES.contains(&status.to_lowercase().as_str())
}

/// Returns whether any thread in the list still needs action.
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
