//! Filters the Files tab file list by whether PR comment threads exist on each
//! path, optionally scoped by thread status (active vs resolved).

use std::collections::HashSet;

use crate::api::types::{PrFileChange, PrThread};

use super::comment_status::is_resolved_status;
use super::state::{CommentStatusFilter, FilesFilter};

/// Original indices into `files` that should appear in the file list and diff
/// stream for the current filter.
pub fn file_indices_for_filter(
    files: &[PrFileChange],
    threads: &[PrThread],
    filter: FilesFilter,
) -> Vec<usize> {
    match filter {
        FilesFilter::All => (0..files.len()).collect(),
        FilesFilter::WithComments(status) => {
            let paths = file_paths_with_comment_threads(threads, status);
            files
                .iter()
                .enumerate()
                .filter(|(_, file)| paths.contains(&file.path))
                .map(|(index, _)| index)
                .collect()
        }
    }
}

fn file_paths_with_comment_threads(
    threads: &[PrThread],
    status: CommentStatusFilter,
) -> HashSet<String> {
    threads
        .iter()
        .filter(|thread| thread.file_path.as_ref().is_some_and(|p| !p.is_empty()))
        .filter(|thread| thread_matches_status(thread, status))
        .map(|thread| thread.file_path.clone().unwrap())
        .collect()
}

fn thread_matches_status(thread: &PrThread, status: CommentStatusFilter) -> bool {
    match status {
        CommentStatusFilter::All => true,
        CommentStatusFilter::Active => !is_resolved_status(&thread.status),
        CommentStatusFilter::Resolved => is_resolved_status(&thread.status),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::types::PrFileChangeType;

    fn file(path: &str) -> PrFileChange {
        PrFileChange {
            path: path.to_string(),
            old_path: None,
            change_type: PrFileChangeType::Edit,
        }
    }

    fn thread(id: i64, status: &str, path: &str) -> PrThread {
        PrThread {
            id,
            status: status.into(),
            file_path: Some(path.into()),
            line: Some(1),
            line_end: None,
            comments: vec![],
        }
    }

    #[test]
    fn all_filter_returns_every_file() {
        let files = vec![file("a.rs"), file("b.rs")];
        let indices = file_indices_for_filter(&files, &[], FilesFilter::All);
        assert_eq!(indices, vec![0, 1]);
    }

    #[test]
    fn with_comments_any_status() {
        let files = vec![file("a.rs"), file("b.rs"), file("c.rs")];
        let threads = vec![thread(1, "active", "b.rs"), thread(2, "fixed", "c.rs")];
        let indices = file_indices_for_filter(
            &files,
            &threads,
            FilesFilter::WithComments(CommentStatusFilter::All),
        );
        assert_eq!(indices, vec![1, 2]);
    }

    #[test]
    fn with_comments_active_only() {
        let files = vec![file("a.rs"), file("b.rs"), file("c.rs")];
        let threads = vec![thread(1, "active", "b.rs"), thread(2, "fixed", "c.rs")];
        let indices = file_indices_for_filter(
            &files,
            &threads,
            FilesFilter::WithComments(CommentStatusFilter::Active),
        );
        assert_eq!(indices, vec![1]);
    }

    #[test]
    fn with_comments_resolved_only() {
        let files = vec![file("a.rs"), file("b.rs")];
        let threads = vec![thread(1, "active", "a.rs"), thread(2, "fixed", "b.rs")];
        let indices = file_indices_for_filter(
            &files,
            &threads,
            FilesFilter::WithComments(CommentStatusFilter::Resolved),
        );
        assert_eq!(indices, vec![1]);
    }
}
