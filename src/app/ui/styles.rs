//! Shared foreground/background palettes for selection and emphasis in the TUI.

use ratatui::style::{Color, Modifier, Style};

use crate::api::types::PrFileChangeType;
use crate::app::text_width::display_width;

/// Foreground + background pair for a tinted row or chip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TintedStyle {
    pub fg: Color,
    pub bg: Color,
}

pub fn file_change_tint(change_type: PrFileChangeType) -> TintedStyle {
    match change_type {
        PrFileChangeType::Add => TintedStyle {
            fg: Color::Rgb(140, 255, 160),
            bg: Color::Rgb(18, 52, 28),
        },
        PrFileChangeType::Edit => TintedStyle {
            fg: Color::Rgb(255, 220, 100),
            bg: Color::Rgb(58, 48, 12),
        },
        PrFileChangeType::Delete => TintedStyle {
            fg: Color::Rgb(255, 130, 130),
            bg: Color::Rgb(58, 22, 22),
        },
        PrFileChangeType::Rename => TintedStyle {
            fg: Color::Rgb(120, 230, 255),
            bg: Color::Rgb(14, 48, 58),
        },
        PrFileChangeType::Other => TintedStyle {
            fg: Color::Rgb(210, 210, 215),
            bg: Color::Rgb(42, 42, 48),
        },
    }
}

pub fn file_change_accent(change_type: PrFileChangeType) -> Color {
    file_change_tint(change_type).fg
}

/// Full-row style for one entry in the Files tab file list.
pub fn file_list_row_style(change_type: PrFileChangeType, selected: bool) -> Style {
    let tint = file_change_tint(change_type);
    if selected {
        Style::default()
            .fg(Color::White)
            .bg(brighten_rgb(tint.bg, 36))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(tint.fg)
    }
}

pub fn file_list_counts_style(bg: Color) -> Style {
    Style::default()
        .fg(Color::Gray)
        .bg(bg)
        .add_modifier(Modifier::DIM)
}

/// Active vs inactive tab chip on the detail panel.
pub fn detail_tab_style(active: bool) -> Style {
    if active {
        Style::default()
            .fg(Color::Rgb(240, 250, 255))
            .bg(Color::Rgb(0, 110, 130))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Color::DarkGray)
            .bg(Color::Rgb(32, 32, 38))
    }
}

/// Filter modal column header when that column has keyboard focus.
pub fn filter_column_header(active: bool) -> Style {
    if active {
        Style::default()
            .fg(Color::Rgb(255, 255, 200))
            .bg(Color::Rgb(72, 62, 10))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Color::DarkGray)
            .bg(Color::Rgb(28, 28, 32))
    }
}

/// One filter option row inside a column.
pub fn filter_option_style(column_active: bool, selected: bool) -> Style {
    if selected {
        Style::default()
            .fg(Color::Rgb(240, 250, 255))
            .bg(Color::Rgb(0, 110, 130))
            .add_modifier(Modifier::BOLD)
    } else if column_active {
        Style::default()
            .fg(Color::Rgb(200, 200, 210))
            .bg(Color::Rgb(38, 38, 46))
    } else {
        Style::default()
            .fg(Color::Gray)
            .bg(Color::Rgb(24, 24, 28))
    }
}

/// Pad `text` with spaces on the right so the background fills `width` columns.
pub fn pad_text_to_width(text: &str, width: usize, style: Style) -> ratatui::text::Span<'static> {
    let current = display_width(text);
    if current >= width {
        ratatui::text::Span::styled(text.to_string(), style)
    } else {
        ratatui::text::Span::styled(format!("{text}{}", " ".repeat(width - current)), style)
    }
}

fn brighten_rgb(color: Color, amount: u8) -> Color {
    match color {
        Color::Rgb(r, g, b) => Color::Rgb(
            r.saturating_add(amount).min(160),
            g.saturating_add(amount).min(160),
            b.saturating_add(amount).min(160),
        ),
        other => other,
    }
}
