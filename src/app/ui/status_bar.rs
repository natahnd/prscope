//! Context-sensitive keyboard hints shown at the bottom of the TUI.

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Padding, Paragraph};
use ratatui::Frame;

use crate::api::types::AppScreen;
use crate::app::state::App;

/// Renders controls relevant to the active screen or modal.
pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let hint: &str = if app.filter_modal_open {
        "↑↓ navigate · Space toggle · Enter apply · Esc cancel"
    } else {
        match app.screen {
            AppScreen::ProviderSelect => "↑↓ navigate · Enter add · q quit",
            AppScreen::ProviderSetup => {
                if app.setup_detecting {
                    "Detecting… · q quit"
                } else {
                    "Enter continue · q quit"
                }
            }
            AppScreen::ProjectSelect => "↑↓ navigate · Enter select · / search · r refresh · q quit",
            AppScreen::PrList => {
                "j/k navigate/scroll · Tab focus · h/l switch view · s comment filter · g group · y copy panel · Enter open · f filter · p project · / search · r refresh · q quit"
            }
        }
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(Color::Gray))
        .padding(Padding::horizontal(1));
    let para = Paragraph::new(Line::from(Span::styled(
        hint,
        Style::default().add_modifier(Modifier::DIM),
    )))
    .block(block);
    f.render_widget(para, area);
}
