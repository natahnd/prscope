//! Execution of interactive and plain pull request listing modes.

use anyhow::{anyhow, Result};

use super::helpers::resolve_organization;
use super::plain_output;
use super::ListArgs;
use crate::api::types::ProviderId;
use crate::config::store::{get_last_filters, load_config};
use crate::filters::{build_filters, ListCommandOptions};
use crate::providers::registry;

fn to_filter_options(args: &ListArgs) -> ListCommandOptions {
    ListCommandOptions {
        org: args.org.clone(),
        project: args.project.clone(),
        status: args.status.clone(),
        repo: args.repo.clone(),
        author: args.author.clone(),
        reviewer: args.reviewer.clone(),
        mine: args.mine,
        plain: args.plain,
        format: args.format.clone(),
    }
}

/// CLI flags that map to `PullRequestFilters` fields. If the user explicitly
/// passes any of these, that one-off run should win over whatever filters
/// were last saved from the TUI — only fall back to saved filters when the
/// user launched with none of them.
fn has_explicit_filter_flags(args: &ListArgs) -> bool {
    args.status.is_some()
        || args.repo.is_some()
        || args.author.is_some()
        || args.reviewer.is_some()
        || args.mine
}

/// Runs either the interactive TUI or the non-interactive plain listing path.
///
/// # Arguments
///
/// * `options` - Parsed CLI arguments from the default invocation or the
///   explicit `list` subcommand.
///
/// # Returns
///
/// `Ok(())` after plain mode finishes printing, or after the interactive TUI
/// exits normally.
///
/// # Errors
///
/// In plain mode (`options.plain`), returns `Err` if authentication fails,
/// the organization can't be resolved, `--project` is missing, the filter
/// flags are invalid, or the pull request fetch fails. In interactive mode,
/// returns `Err` if filter flags are invalid or if [`crate::app::run_tui`]
/// returns an error.
pub async fn run_list_command(options: ListArgs) -> Result<()> {
    if options.plain {
        let token = registry::resolve_provider_token(ProviderId::AzureDevops).await?;
        let organization = resolve_organization(options.org.clone()).await?;
        let client =
            registry::create_azure_devops_client_direct(token, organization)?;
        let user = client.get_current_user().await?;
        let filter_opts = to_filter_options(&options);
        let filters = build_filters(&filter_opts, Some(&user.id))?;

        let project = options
            .project
            .clone()
            .ok_or_else(|| anyhow!("--project is required with --plain mode."))?;

        let prs = client
            .list_pull_requests(&project, &filters, Some(&user.id))
            .await?;

        if options.format.as_deref() == Some("json") {
            plain_output::print_plain_json(&prs)?;
        } else {
            plain_output::print_plain_table(&prs);
        }
        return Ok(());
    }

    let saved_filters = if has_explicit_filter_flags(&options) {
        None
    } else {
        let config = load_config().await;
        get_last_filters(&config)
    };

    let filter_opts = to_filter_options(&options);
    let initial_filters = match saved_filters {
        Some(f) => f,
        None => build_filters(&filter_opts, None)?,
    };

    crate::app::run_tui(options.project.clone(), initial_filters).await
}
