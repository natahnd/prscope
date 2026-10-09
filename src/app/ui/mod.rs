pub mod filter_modal;
pub mod header;
pub mod pr_list_panel;
pub mod pr_right_panel;
pub mod project_picker;
pub mod provider_picker;
pub mod status_bar;
pub mod styles;

use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::api::types::AppScreen;

use super::state::App;

pub fn render_loading(f: &mut Frame) {
    let p = Paragraph::new("Loading…").style(Style::default().add_modifier(Modifier::DIM));
    f.render_widget(p, f.area());
}

pub fn render(f: &mut Frame, app: &App) {
    if !app.bootstrapped {
        render_loading(f);
        return;
    }
    match app.screen {
        AppScreen::ProviderSelect => render_provider_select(f, app),
        AppScreen::ProviderSetup => render_provider_setup(f, app),
        AppScreen::ProjectSelect => render_project_select(f, app),
        AppScreen::PrList => render_pr_list(f, app),
    }
}

fn render_provider_select(f: &mut Frame, app: &App) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(area);
    provider_picker::render_picker(f, chunks[0], app);
    status_bar::render(f, chunks[1], app);
}

fn render_provider_setup(f: &mut Frame, app: &App) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(area);
    provider_picker::render_setup(f, chunks[0], app);
    status_bar::render(f, chunks[1], app);
}

fn render_project_select(f: &mut Frame, app: &App) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(area);
    project_picker::render(f, chunks[0], app);
    status_bar::render(f, chunks[1], app);
}

fn render_pr_list(f: &mut Frame, app: &App) {
    let geometry = app.geometry();
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(geometry.reserved_top),
            Constraint::Length(geometry.panel_height),
            Constraint::Length(3),
        ])
        .split(area);

    header::render(f, chunks[0], app);

    if app.filter_modal_open {
        filter_modal::render(f, chunks[1], app);
    } else {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(geometry.left_width),
                Constraint::Length(geometry.right_width),
            ])
            .split(chunks[1]);
        pr_list_panel::render(f, cols[0], app);
        pr_right_panel::render(f, cols[1], app);
    }

    status_bar::render(f, chunks[2], app);
}
