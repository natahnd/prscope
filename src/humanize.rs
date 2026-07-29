//! Human-readable absolute and relative timestamp formatting.

use chrono::{DateTime, Datelike, Utc};

/// Parses an Azure DevOps ISO-8601 timestamp string. Falls back to "now" on
/// parse failure so a malformed date never panics the UI.
pub fn parse_date(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

const MINUTES_IN_DAY: i64 = 1440;
const MINUTES_IN_ALMOST_TWO_DAYS: i64 = 2520;
const MINUTES_IN_MONTH: i64 = 43200;
const MINUTES_IN_TWO_MONTHS: i64 = 86400;

/// Formats a timestamp using stable, human-readable English time buckets.
///
/// The thresholds intentionally match the wording already exposed by the CLI,
/// so output remains consistent across the TUI and plain table modes.
pub fn format_distance_to_now(date: DateTime<Utc>, add_suffix: bool) -> String {
    let now = Utc::now();
    let seconds = (now - date).num_seconds().max(0);
    let minutes = ((seconds as f64) / 60.0).round() as i64;

    let phrase = if minutes < 1 {
        if minutes == 0 {
            "less than a minute".to_string()
        } else {
            format!("{minutes} minutes")
        }
    } else if minutes == 1 {
        "1 minute".to_string()
    } else if minutes < 44 {
        format!("{minutes} minutes")
    } else if minutes < 90 {
        "about 1 hour".to_string()
    } else if minutes < MINUTES_IN_DAY {
        let hours = ((minutes as f64) / 60.0).round() as i64;
        format!("about {hours} hours")
    } else if minutes < MINUTES_IN_ALMOST_TWO_DAYS {
        "1 day".to_string()
    } else if minutes < MINUTES_IN_MONTH {
        let days = ((minutes as f64) / MINUTES_IN_DAY as f64).round() as i64;
        format!("{days} days")
    } else if minutes < MINUTES_IN_TWO_MONTHS {
        let months = ((minutes as f64) / MINUTES_IN_MONTH as f64).round() as i64;
        format!("about {months} months")
    } else {
        let months = month_difference(date, now);
        if months < 12 {
            let nearest_month = ((minutes as f64) / MINUTES_IN_MONTH as f64).round() as i64;
            format!("{nearest_month} months")
        } else {
            let months_since_start_of_year = months % 12;
            let years = months / 12;
            if months_since_start_of_year < 3 {
                format!("about {years} years")
            } else if months_since_start_of_year < 9 {
                format!("over {years} years")
            } else {
                format!("almost {} years", years + 1)
            }
        }
    };

    if add_suffix {
        format!("{phrase} ago")
    } else {
        phrase
    }
}

fn month_difference(earlier: DateTime<Utc>, later: DateTime<Utc>) -> i64 {
    let mut months =
        (later.date_naive().year() - earlier.date_naive().year()) as i64 * 12
            + (later.date_naive().month() as i64 - earlier.date_naive().month() as i64);
    if later.date_naive().day() < earlier.date_naive().day() {
        months -= 1;
    }
    months.max(0)
}

/// Formats a compact age such as `5d`, `3h`, or `2mo` for PR list rows.
pub fn format_compact_age(date: DateTime<Utc>) -> String {
    let seconds = (Utc::now() - date).num_seconds().max(0);
    if seconds < 60 {
        return format!("{seconds}s");
    }
    let minutes = seconds / 60;
    if minutes < 60 {
        return format!("{minutes}m");
    }
    let hours = minutes / 60;
    if hours < 24 {
        return format!("{hours}h");
    }
    let days = hours / 24;
    if days < 30 {
        return format!("{days}d");
    }
    let months = days / 30;
    if months < 12 {
        return format!("{months}mo");
    }
    format!("{}y", months / 12)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn compact_age_buckets() {
        let now = Utc::now();
        assert_eq!(format_compact_age(now - Duration::seconds(30)), "30s");
        assert_eq!(format_compact_age(now - Duration::minutes(5)), "5m");
        assert_eq!(format_compact_age(now - Duration::hours(3)), "3h");
        assert_eq!(format_compact_age(now - Duration::days(5)), "5d");
    }

    #[test]
    fn distance_to_now_hours() {
        let now = Utc::now();
        let phrase = format_distance_to_now(now - Duration::hours(15), true);
        assert_eq!(phrase, "about 15 hours ago");
    }

    #[test]
    fn distance_to_now_days() {
        let now = Utc::now();
        let phrase = format_distance_to_now(now - Duration::days(5), true);
        assert_eq!(phrase, "5 days ago");
    }
}
