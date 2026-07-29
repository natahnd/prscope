//! Human-readable and JSON output for non-interactive CLI use.

use anyhow::Result;
use unicode_width::UnicodeWidthStr;

use crate::api::types::NormalizedPullRequest;
use crate::humanize::{format_distance_to_now, parse_date};

/// Prints pull requests as a width-aware terminal table.
///
/// # Arguments
///
/// * `prs` - Pull requests to print, in the order given.
///
/// # Returns
///
/// This function does not return a value; it prints
/// `"No pull requests found."` instead of a table if `prs` is empty.
pub fn print_plain_table(prs: &[NormalizedPullRequest]) {
    if prs.is_empty() {
        println!("No pull requests found.");
        return;
    }

    let headers = ["Repo", "#", "Title", "Author", "Reviewers", "Age", "Status"];
    let rows: Vec<[String; 7]> = prs
        .iter()
        .map(|pr| {
            [
                pr.repo_name.clone(),
                pr.id.to_string(),
                truncate(&pr.title, 40),
                truncate(&pr.author, 16),
                format_reviewers(pr),
                format_distance_to_now(parse_date(&pr.created_at), false),
                if pr.is_draft {
                    "draft".to_string()
                } else {
                    pr.status.clone()
                },
            ]
        })
        .collect();

    let widths: Vec<usize> = headers
        .iter()
        .enumerate()
        .map(|(i, h)| {
            rows.iter()
                .map(|r| r[i].width())
                .chain(std::iter::once(h.width()))
                .max()
                .unwrap_or(0)
        })
        .collect();

    println!(
        "{}",
        headers
            .iter()
            .enumerate()
            .map(|(i, h)| pad(h, widths[i]))
            .collect::<Vec<_>>()
            .join("  ")
    );
    println!(
        "{}",
        widths
            .iter()
            .map(|w| "-".repeat(*w))
            .collect::<Vec<_>>()
            .join("  ")
    );
    for row in &rows {
        println!(
            "{}",
            row.iter()
                .enumerate()
                .map(|(i, cell)| pad(cell, widths[i]))
                .collect::<Vec<_>>()
                .join("  ")
        );
    }
}

/// Serializes pull requests as pretty-printed JSON.
///
/// # Arguments
///
/// * `prs` - Pull requests to serialize, in the order given.
///
/// # Returns
///
/// `Ok(())` once the JSON has been printed to stdout.
///
/// # Errors
///
/// Returns `Err` if `prs` fails to serialize to JSON (not expected in
/// practice, since `NormalizedPullRequest` derives `Serialize` with no
/// fallible custom logic).
pub fn print_plain_json(prs: &[NormalizedPullRequest]) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(prs)?);
    Ok(())
}

fn format_reviewers(pr: &NormalizedPullRequest) -> String {
    if pr.reviewers.is_empty() {
        return "-".to_string();
    }
    pr.reviewers
        .iter()
        .map(|r| {
            let vote = if r.vote == 10 {
                "✓"
            } else if r.vote == -10 {
                "✗"
            } else if r.is_current_user {
                "⚠"
            } else {
                "○"
            };
            let name = if r.is_current_user {
                "you".to_string()
            } else {
                r.display_name
                    .split(' ')
                    .next()
                    .unwrap_or(&r.display_name)
                    .to_string()
            };
            format!("{vote}{name}")
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn truncate(value: &str, max: usize) -> String {
    if value.width() > max {
        let mut out = String::new();
        let mut width = 0;
        for ch in value.chars() {
            let w = UnicodeWidthStr::width(ch.to_string().as_str());
            if width + w > max.saturating_sub(1) {
                break;
            }
            width += w;
            out.push(ch);
        }
        out.push('…');
        out
    } else {
        value.to_string()
    }
}

fn pad(value: &str, width: usize) -> String {
    let current = value.width();
    if current >= width {
        value.to_string()
    } else {
        format!("{}{}", value, " ".repeat(width - current))
    }
}

/// Prints a CLI error using a red ANSI foreground.
///
/// # Arguments
///
/// * `message` - The error text to print, prefixed with `"Error: "`.
///
/// # Returns
///
/// This function does not return a value; it always writes to stderr.
pub fn print_error(message: &str) {
    eprintln!("\x1b[31mError: {message}\x1b[0m");
}
