//! Pure, network-free diff construction shared by every provider. Builds
//! git-style hunks (3 lines of context) from two full-file text blobs via the
//! `similar` crate's Myers diff, then matches PR review threads onto the
//! resulting lines so the Files tab can render comments inline.

use similar::{ChangeTag, TextDiff};

use crate::api::types::{DiffLine, DiffLineKind, FileDiff, PrFileChange, PrThread};

const CONTEXT_LINES: usize = 3;

/// Builds a [`FileDiff`] for `file` from its old and new full-text contents.
/// Either side may be empty (adds/deletes); `comments` on every line starts
/// empty — use [`attach_comments`] to populate them once PR threads are known.
pub fn build_file_diff(file: PrFileChange, old_text: &str, new_text: &str) -> FileDiff {
    if looks_binary(old_text) || looks_binary(new_text) {
        return FileDiff {
            file,
            lines: vec![],
            additions: 0,
            deletions: 0,
            is_binary: true,
        };
    }

    let diff = TextDiff::from_lines(old_text, new_text);
    let mut lines: Vec<DiffLine> = Vec::new();
    let mut additions = 0usize;
    let mut deletions = 0usize;

    for group in diff.grouped_ops(CONTEXT_LINES) {
        if let (Some(first), Some(last)) = (group.first(), group.last()) {
            let old_start = first.old_range().start;
            let old_len: usize = group.iter().map(|op| op.old_range().len()).sum();
            let new_start = first.new_range().start;
            let new_len: usize = group.iter().map(|op| op.new_range().len()).sum();
            let _ = last;
            lines.push(DiffLine {
                kind: DiffLineKind::HunkHeader,
                old_line_no: None,
                new_line_no: None,
                content: format!(
                    "@@ -{},{} +{},{} @@",
                    old_start + 1,
                    old_len,
                    new_start + 1,
                    new_len
                ),
                comments: vec![],
            });
        }

        for op in group {
            for change in diff.iter_changes(&op) {
                let content = change.value().trim_end_matches(['\n', '\r']).to_string();
                let (kind, old_no, new_no) = match change.tag() {
                    ChangeTag::Equal => (
                        DiffLineKind::Context,
                        change.old_index().map(|i| i as i64 + 1),
                        change.new_index().map(|i| i as i64 + 1),
                    ),
                    ChangeTag::Delete => {
                        deletions += 1;
                        (DiffLineKind::Removed, change.old_index().map(|i| i as i64 + 1), None)
                    }
                    ChangeTag::Insert => {
                        additions += 1;
                        (DiffLineKind::Added, None, change.new_index().map(|i| i as i64 + 1))
                    }
                };
                lines.push(DiffLine {
                    kind,
                    old_line_no: old_no,
                    new_line_no: new_no,
                    content,
                    comments: vec![],
                });
            }
        }
    }

    FileDiff {
        file,
        lines,
        additions,
        deletions,
        is_binary: false,
    }
}

/// Heuristic binary-content sniff mirroring `git diff`'s NUL-byte check —
/// good enough since we only ever get here after already fetching content as
/// text.
fn looks_binary(text: &str) -> bool {
    text.chars().take(8000).any(|c| c == '\u{0}')
}

/// Matches each thread's `(file_path, line)` onto the diff line it was left
/// on. Azure DevOps doesn't tell us whether a thread's `line` refers to the
/// old or new side once normalized, so a thread matches any diff line — added,
/// removed, or context — whose old or new line number agrees.
pub fn attach_comments(diff: &mut FileDiff, threads: &[PrThread]) {
    let relevant: Vec<&PrThread> = threads
        .iter()
        .filter(|t| t.file_path.as_deref() == Some(diff.file.path.as_str()) && t.line.is_some())
        .collect();
    if relevant.is_empty() {
        return;
    }

    for line in &mut diff.lines {
        for thread in &relevant {
            let target = thread.line;
            if target == line.old_line_no || target == line.new_line_no {
                line.comments.extend(thread.comments.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::types::{PrComment, PrFileChangeType};

    fn file(path: &str, change_type: PrFileChangeType) -> PrFileChange {
        PrFileChange {
            path: path.to_string(),
            old_path: None,
            change_type,
        }
    }

    #[test]
    fn builds_diff_for_added_file() {
        let diff = build_file_diff(file("a.rs", PrFileChangeType::Add), "", "line1\nline2\n");
        assert_eq!(diff.additions, 2);
        assert_eq!(diff.deletions, 0);
        assert!(diff.lines.iter().any(|l| l.kind == DiffLineKind::HunkHeader));
        assert!(diff
            .lines
            .iter()
            .all(|l| l.kind == DiffLineKind::HunkHeader || l.kind == DiffLineKind::Added));
    }

    #[test]
    fn builds_diff_for_deleted_file() {
        let diff = build_file_diff(file("a.rs", PrFileChangeType::Delete), "line1\nline2\n", "");
        assert_eq!(diff.additions, 0);
        assert_eq!(diff.deletions, 2);
    }

    #[test]
    fn builds_diff_with_context_for_edit() {
        let old = "a\nb\nc\nd\ne\n";
        let new = "a\nb\nX\nd\ne\n";
        let diff = build_file_diff(file("a.rs", PrFileChangeType::Edit), old, new);
        assert_eq!(diff.additions, 1);
        assert_eq!(diff.deletions, 1);
        let context_count = diff.lines.iter().filter(|l| l.kind == DiffLineKind::Context).count();
        // 3 lines of context on each side of the single changed line, but
        // there are only 4 unchanged lines total (a, b, d, e) — all should
        // be included since the file is short.
        assert_eq!(context_count, 4);
    }

    #[test]
    fn groups_distant_changes_into_separate_hunks() {
        let mut old_lines: Vec<String> = (0..40).map(|i| format!("line{i}")).collect();
        let mut new_lines = old_lines.clone();
        old_lines[2] = "changed-old".to_string();
        new_lines[2] = "changed-new".to_string();
        old_lines[30] = "changed-old-2".to_string();
        new_lines[30] = "changed-new-2".to_string();
        let old = format!("{}\n", old_lines.join("\n"));
        let new = format!("{}\n", new_lines.join("\n"));
        let diff = build_file_diff(file("a.rs", PrFileChangeType::Edit), &old, &new);
        let hunk_headers = diff.lines.iter().filter(|l| l.kind == DiffLineKind::HunkHeader).count();
        assert_eq!(hunk_headers, 2);
    }

    #[test]
    fn attaches_comments_matching_new_line_number() {
        let mut diff = build_file_diff(file("a.rs", PrFileChangeType::Edit), "a\nb\nc\n", "a\nX\nc\n");
        let thread = PrThread {
            id: 1,
            status: "active".into(),
            file_path: Some("a.rs".into()),
            line: Some(2),
            line_end: None,
            comments: vec![PrComment {
                id: 1,
                author: "Alice".into(),
                content: "why?".into(),
                published_at: "2024-01-01T00:00:00Z".into(),
            }],
        };
        attach_comments(&mut diff, &[thread]);
        let matched = diff
            .lines
            .iter()
            .filter(|l| !l.comments.is_empty())
            .count();
        assert!(matched >= 1);
    }

    #[test]
    fn ignores_comments_for_other_files() {
        let mut diff = build_file_diff(file("a.rs", PrFileChangeType::Edit), "a\nb\n", "a\nX\n");
        let thread = PrThread {
            id: 1,
            status: "active".into(),
            file_path: Some("other.rs".into()),
            line: Some(2),
            line_end: None,
            comments: vec![],
        };
        attach_comments(&mut diff, &[thread]);
        assert!(diff.lines.iter().all(|l| l.comments.is_empty()));
    }
}
