use anyhow::{bail, Result};

use crate::api::types::{PullRequestFilters, PullRequestStatus};

#[derive(Debug, Clone, Default)]
pub struct ListCommandOptions {
    #[allow(dead_code)]
    pub org: Option<String>,
    pub project: Option<String>,
    pub status: Option<String>,
    pub repo: Option<String>,
    pub author: Option<String>,
    pub reviewer: Option<String>,
    pub mine: bool,
    pub plain: bool,
    pub format: Option<String>,
}

pub fn build_filters(
    options: &ListCommandOptions,
    current_user_id: Option<&str>,
) -> Result<PullRequestFilters> {
    let status = parse_status(options.status.as_deref())?;
    let mut filters = PullRequestFilters {
        status,
        ..Default::default()
    };

    if let Some(repo) = &options.repo {
        filters.repository_name = Some(repo.clone());
    }

    if options.mine && current_user_id.is_some() {
        filters.author_me = Some(true);
        filters.author_id = current_user_id.map(|s| s.to_string());
    } else if let Some(author) = &options.author {
        filters.author = Some(author.clone());
    }

    if options.reviewer.as_deref() == Some("me") && current_user_id.is_some() {
        filters.reviewer_me = Some(true);
        filters.reviewer_id = current_user_id.map(|s| s.to_string());
    }

    Ok(filters)
}

fn parse_status(value: Option<&str>) -> Result<PullRequestStatus> {
    match value {
        None => Ok(PullRequestStatus::Active),
        Some("active") => Ok(PullRequestStatus::Active),
        Some("completed") => Ok(PullRequestStatus::Completed),
        Some("abandoned") => Ok(PullRequestStatus::Abandoned),
        Some("all") => Ok(PullRequestStatus::All),
        Some(other) => bail!(
            "Invalid status \"{}\". Use active, completed, abandoned, or all.",
            other
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_status_to_active() {
        let filters = build_filters(&ListCommandOptions::default(), None).unwrap();
        assert_eq!(filters.status, PullRequestStatus::Active);
    }

    #[test]
    fn mine_sets_author_me_when_user_known() {
        let options = ListCommandOptions {
            mine: true,
            ..Default::default()
        };
        let filters = build_filters(&options, Some("user-1")).unwrap();
        assert_eq!(filters.author_me, Some(true));
        assert_eq!(filters.author_id.as_deref(), Some("user-1"));
    }

    #[test]
    fn invalid_status_errors() {
        let options = ListCommandOptions {
            status: Some("bogus".into()),
            ..Default::default()
        };
        assert!(build_filters(&options, None).is_err());
    }
}
