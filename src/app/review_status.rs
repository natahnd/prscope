use ratatui::style::{Color, Modifier, Style};

use crate::api::types::{ApprovedByFilter, AuthorFilter, NormalizedPullRequest, ReviewStatusFilter};

/// Azure DevOps reviewer vote values.
pub const VOTE_APPROVED: i32 = 10;
pub const VOTE_APPROVED_WITH_SUGGESTIONS: i32 = 5;
pub const VOTE_NO_VOTE: i32 = 0;
pub const VOTE_WAITING_FOR_AUTHOR: i32 = -5;
pub const VOTE_REJECTED: i32 = -10;

pub fn current_user_reviewer<'a>(
    pr: &'a NormalizedPullRequest,
) -> Option<&'a crate::api::types::AdoReviewer> {
    pr.reviewers.iter().find(|r| r.is_current_user)
}

pub fn is_approved_vote(vote: i32) -> bool {
    vote == VOTE_APPROVED || vote == VOTE_APPROVED_WITH_SUGGESTIONS
}

pub fn review_vote_symbol(vote: i32) -> &'static str {
    match vote {
        VOTE_APPROVED | VOTE_APPROVED_WITH_SUGGESTIONS => "✓",
        VOTE_REJECTED => "✗",
        VOTE_WAITING_FOR_AUTHOR => "⏳",
        VOTE_NO_VOTE => "○",
        _ => "·",
    }
}

pub fn review_vote_color(vote: i32) -> Color {
    match vote {
        VOTE_APPROVED | VOTE_APPROVED_WITH_SUGGESTIONS => Color::Green,
        VOTE_REJECTED => Color::Red,
        VOTE_WAITING_FOR_AUTHOR => Color::Magenta,
        VOTE_NO_VOTE => Color::Yellow,
        _ => Color::Gray,
    }
}

pub fn reviewer_line_style(vote: i32, is_current_user: bool) -> Style {
    let mut style = Style::default().fg(review_vote_color(vote));
    if is_current_user {
        style = style.add_modifier(Modifier::BOLD);
    }
    style
}

pub fn reviewer_detail_name(reviewer: &crate::api::types::AdoReviewer) -> String {
    if reviewer.is_current_user {
        return "you".to_string();
    }
    reviewer.display_name.clone()
}

pub fn reviewer_count_label(pr: &NormalizedPullRequest) -> Option<String> {
    if pr.is_draft || pr.reviewers.is_empty() {
        return None;
    }
    let count = pr.reviewers.len();
    Some(format!(
        " · {count} reviewer{}",
        if count == 1 { "" } else { "s" }
    ))
}

pub fn matches_review_status_filter(pr: &NormalizedPullRequest, filter: ReviewStatusFilter) -> bool {
    if pr.is_draft {
        return filter == ReviewStatusFilter::All;
    }
    match filter {
        ReviewStatusFilter::All => true,
        ReviewStatusFilter::NeedsReview => pr.reviewers.iter().any(|r| r.vote == VOTE_NO_VOTE),
        ReviewStatusFilter::Approved => pr.reviewers.iter().any(|r| is_approved_vote(r.vote)),
        ReviewStatusFilter::Rejected => pr.reviewers.iter().any(|r| r.vote == VOTE_REJECTED),
        ReviewStatusFilter::WaitingForAuthor => {
            pr.reviewers.iter().any(|r| r.vote == VOTE_WAITING_FOR_AUTHOR)
        }
    }
}

pub fn matches_approved_by_filter(pr: &NormalizedPullRequest, filter: ApprovedByFilter) -> bool {
    match filter {
        ApprovedByFilter::All => true,
        ApprovedByFilter::Me => current_user_reviewer(pr).is_some_and(|r| is_approved_vote(r.vote)),
    }
}

pub fn matches_author_filter(pr: &NormalizedPullRequest, filter: AuthorFilter) -> bool {
    match filter {
        AuthorFilter::All => true,
        AuthorFilter::Me => pr.is_author_current_user,
        AuthorFilter::NotMe => !pr.is_author_current_user,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::types::AdoReviewer;

    fn pr_with_reviewers(
        reviewers: Vec<AdoReviewer>,
        is_draft: bool,
        is_author_current_user: bool,
    ) -> NormalizedPullRequest {
        let needs_my_review = reviewers
            .iter()
            .any(|r| r.is_current_user && r.vote == VOTE_NO_VOTE && !is_draft);
        NormalizedPullRequest {
            id: 1,
            title: "Test".into(),
            status: "active".into(),
            is_draft,
            repo_name: "repo".into(),
            repository_id: "r1".into(),
            author: "Author".into(),
            is_author_current_user,
            reviewers,
            target_branch: "main".into(),
            created_at: "2024-01-01T00:00:00Z".into(),
            url: "https://example.com".into(),
            merge_status: "none".into(),
            needs_my_review,
        }
    }

    fn reviewer(name: &str, vote: i32, is_current_user: bool) -> AdoReviewer {
        AdoReviewer {
            display_name: name.into(),
            vote,
            is_required: false,
            is_current_user,
            is_team: false,
        }
    }

    fn team_reviewer(name: &str, vote: i32) -> AdoReviewer {
        AdoReviewer {
            display_name: name.into(),
            vote,
            is_required: false,
            is_current_user: false,
            is_team: true,
        }
    }

    #[test]
    fn approved_filter_matches_when_any_reviewer_approved() {
        let pr = pr_with_reviewers(
            vec![
                reviewer("Kyle", VOTE_APPROVED, false),
                reviewer("Me", VOTE_NO_VOTE, true),
            ],
            false,
            false,
        );
        assert!(matches_review_status_filter(&pr, ReviewStatusFilter::Approved));
    }

    #[test]
    fn approved_by_me_requires_current_user_approval() {
        let pr = pr_with_reviewers(
            vec![
                reviewer("Kyle", VOTE_APPROVED, false),
                reviewer("Me", VOTE_NO_VOTE, true),
            ],
            false,
            false,
        );
        assert!(!matches_approved_by_filter(&pr, ApprovedByFilter::Me));

        let pr = pr_with_reviewers(
            vec![
                reviewer("Kyle", VOTE_APPROVED, false),
                reviewer("Me", VOTE_APPROVED, true),
            ],
            false,
            false,
        );
        assert!(matches_approved_by_filter(&pr, ApprovedByFilter::Me));
    }

    #[test]
    fn author_not_me_excludes_own_prs() {
        let mine = pr_with_reviewers(vec![], false, true);
        let theirs = pr_with_reviewers(vec![], false, false);
        assert!(!matches_author_filter(&mine, AuthorFilter::NotMe));
        assert!(matches_author_filter(&theirs, AuthorFilter::NotMe));
    }

    #[test]
    fn reviewer_count_label_pluralizes() {
        let pr = pr_with_reviewers(
            vec![reviewer("A", VOTE_NO_VOTE, false), reviewer("B", VOTE_NO_VOTE, false)],
            false,
            false,
        );
        assert_eq!(reviewer_count_label(&pr).as_deref(), Some(" · 2 reviewers"));
    }

    #[test]
    fn reviewer_line_style_uses_vote_color() {
        let style = reviewer_line_style(VOTE_APPROVED, false);
        assert_eq!(style.fg, Some(Color::Green));
    }

    #[test]
    fn team_reviewer_name_is_preserved() {
        let reviewer = team_reviewer("[Digital Product Platform]/Digital Product Platform", VOTE_NO_VOTE);
        assert_eq!(
            reviewer_detail_name(&reviewer),
            "[Digital Product Platform]/Digital Product Platform"
        );
    }
}
