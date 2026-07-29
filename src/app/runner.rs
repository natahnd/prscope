//! Terminal lifecycle, application bootstrap, and asynchronous event loop.

use std::io::stdout;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{Event, EventStream};
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, terminal};
use futures::StreamExt;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::mpsc::{self, UnboundedSender};

use crate::api::types::{AppScreen, ProviderId, PullRequestFilters};
use crate::config::store::{get_last_list_layout, get_last_project, get_provider_display_context, load_config};
use crate::providers::registry;
use crate::providers::ProviderClient;

use super::state::{App, AppEvent};
use super::update::{handle_key, handle_mouse, on_filters_changed, sync_comment_activity, sync_detail_state};
use super::ui;

const MOUSE_ENABLE: &str = "\x1b[?1000h\x1b[?1002h\x1b[?1006h";
const MOUSE_DISABLE: &str = "\x1b[?1000l\x1b[?1002l\x1b[?1006l";

fn resolve_initial_screen(configured: bool, has_project: bool) -> AppScreen {
    if !configured {
        AppScreen::ProviderSelect
    } else if has_project {
        AppScreen::PrList
    } else {
        AppScreen::ProjectSelect
    }
}

async fn init_client(
    provider_id: ProviderId,
) -> Result<(Arc<dyn ProviderClient>, String, String, Option<String>)> {
    let config = load_config().await;
    let token = registry::resolve_provider_token(provider_id).await?;
    let client = registry::create_client_from_config(provider_id, token, &config)?;
    let user = client.get_current_user().await?;
    let display_context = get_provider_display_context(&config);
    let last_project = get_last_project(&config);
    Ok((client, user.id, display_context, last_project))
}

/// Loads projects in a background task and sends one completion event.
///
/// # Arguments
///
/// * `client` - Provider client to list projects from.
/// * `tx` - Channel used to report the result back to the main event loop.
///
/// # Returns
///
/// This function does not return a value; on success it sends
/// `AppEvent::ProjectsLoaded` with the persisted last project, display
/// context, and project list, or `AppEvent::ProjectsLoadFailed` with the
/// error message on failure. It never propagates the provider error to the
/// caller directly.
pub async fn load_projects(client: Arc<dyn ProviderClient>, tx: UnboundedSender<AppEvent>) {
    let config = load_config().await;
    let last_project = get_last_project(&config);
    let display_context = get_provider_display_context(&config);
    match client.list_projects().await {
        Ok(projects) => {
            let _ = tx.send(AppEvent::ProjectsLoaded {
                last_project,
                display_context,
                projects,
            });
        }
        Err(err) => {
            let _ = tx.send(AppEvent::ProjectsLoadFailed(err.to_string()));
        }
    }
}

/// Loads repositories and pull requests concurrently for one project.
///
/// # Arguments
///
/// * `client` - Provider client to fetch repositories and pull requests from.
/// * `project` - Project to load data for.
/// * `filters` - Filters to apply to the pull request list.
/// * `current_user_id` - Authenticated user's identity, used to resolve
///   `needsMyReview` and "me" filters.
/// * `tx` - Channel used to report the result back to the main event loop.
///
/// # Returns
///
/// This function does not return a value; on success it sends
/// `AppEvent::PrsLoaded` with both results, or `AppEvent::PrsLoadFailed`
/// with the error message if either the repository or pull request fetch
/// fails. It never propagates the provider error to the caller directly.
pub async fn load_pull_requests(
    client: Arc<dyn ProviderClient>,
    project: String,
    filters: PullRequestFilters,
    current_user_id: Option<String>,
    tx: UnboundedSender<AppEvent>,
) {
    let repos_fut = client.list_repositories(&project);
    let prs_fut = client.list_pull_requests(&project, &filters, current_user_id.as_deref());
    match tokio::try_join!(repos_fut, prs_fut) {
        Ok((repos, prs)) => {
            let _ = tx.send(AppEvent::PrsLoaded { repos, prs });
        }
        Err(err) => {
            let _ = tx.send(AppEvent::PrsLoadFailed(err.to_string()));
        }
    }
}

/// Starts best-effort Azure DevOps organization discovery.
///
/// # Arguments
///
/// * `tx` - Channel used to report the detected organization (or `None`).
///
/// # Returns
///
/// This function does not return a value; the background task always sends
/// `AppEvent::AdoOrgDetected` exactly once, with `None` if detection fails
/// or finds nothing, so a failed lookup never blocks the setup screen.
pub fn spawn_ado_org_detect(tx: UnboundedSender<AppEvent>) {
    tokio::spawn(async move {
        let org = crate::providers::azure_devops::detect_org::detect_azure_devops_org().await;
        let _ = tx.send(AppEvent::AdoOrgDetected(org));
    });
}

async fn bootstrap(app: &mut App, tx: &UnboundedSender<AppEvent>) {
    let config = load_config().await;
    let configured = crate::api::types::is_configured_provider(&config);
    let target_project = app.initial_project.clone().or_else(|| get_last_project(&config));
    app.screen = resolve_initial_screen(configured, target_project.is_some());
    if configured && target_project.is_some() {
        app.selected_project = target_project;
    }

    if configured {
        if let Some(active) = config.active_provider {
            match init_client(active).await {
                Ok((client, user_id, display_context, last_project)) => {
                    app.client = Some(client);
                    app.current_user_id = Some(user_id);
                    app.display_context = display_context;
                    app.last_project = last_project;
                }
                Err(err) => {
                    app.projects_error = Some(err.to_string());
                    app.screen = AppScreen::ProviderSelect;
                }
            }
        }
    }

    app.list_layout = get_last_list_layout(&config);

    app.bootstrapped = true;

    if app.screen == AppScreen::ProjectSelect {
        if let Some(client) = app.client.clone() {
            app.projects_loading = true;
            tokio::spawn(load_projects(client, tx.clone()));
        }
    } else if app.screen == AppScreen::PrList {
        on_filters_changed(app, tx);
    }

    // Persistence becomes safe only after stored values have initialized state.
    app.filters_hydrated = true;
}

/// Runs the interactive application and restores terminal state on exit.
///
/// # Arguments
///
/// * `initial_project` - Project to open directly, bypassing the project
///   picker (typically from a `--project` CLI flag).
/// * `initial_filters` - Filters to apply on first load, overriding any
///   persisted filters for this run only.
///
/// # Returns
///
/// `Ok(())` after the event loop exits normally (the user quit).
///
/// # Errors
///
/// Returns `Err` if entering/leaving raw mode or the alternate screen fails,
/// or if the event loop itself returns an error (e.g. a terminal I/O
/// failure). Terminal state (raw mode, alternate screen, mouse reporting) is
/// always restored before this function returns, even on error.
pub async fn run_tui(initial_project: Option<String>, initial_filters: PullRequestFilters) -> Result<()> {
    terminal::enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen)?;
    print!("{MOUSE_ENABLE}");
    use std::io::Write;
    out.flush()?;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let result = run_app(&mut terminal, initial_project, initial_filters).await;

    print!("{MOUSE_DISABLE}");
    let mut out = stdout();
    out.flush().ok();
    execute!(out, LeaveAlternateScreen).ok();
    terminal::disable_raw_mode().ok();

    result
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    initial_project: Option<String>,
    initial_filters: PullRequestFilters,
) -> Result<()> {
    let mut app = App::new(initial_project, initial_filters);
    // Background tasks never mutate App directly; they report completions
    // through this channel for serialized handling in the main loop.
    let (tx, mut rx) = mpsc::unbounded_channel::<AppEvent>();

    terminal.draw(ui::render_loading)?;
    bootstrap(&mut app, &tx).await;

    if let Ok((cols, rows)) = terminal::size() {
        app.terminal_columns = cols;
        app.terminal_rows = rows;
    }

    let mut events = EventStream::new();
    let mut spinner_ticker = tokio::time::interval(Duration::from_millis(80));

    terminal.draw(|f| ui::render(f, &app))?;

    loop {
        if app.should_quit {
            break;
        }

        tokio::select! {
            maybe_event = events.next() => {
                match maybe_event {
                    Some(Ok(event)) => handle_terminal_event(&mut app, event, &tx),
                    Some(Err(_)) | None => break,
                }
            }
            msg = rx.recv() => {
                if let Some(event) = msg {
                    handle_app_event(&mut app, event, &tx);
                }
            }
            _ = spinner_ticker.tick() => {
                if app.setup_detecting {
                    app.spinner_frame = (app.spinner_frame + 1) % 10;
                }
            }
        }

        terminal.draw(|f| ui::render(f, &app))?;
    }

    Ok(())
}

fn handle_terminal_event(app: &mut App, event: Event, tx: &UnboundedSender<AppEvent>) {
    match event {
        Event::Key(key) => {
            if key.kind == crossterm::event::KeyEventKind::Release {
                return;
            }
            handle_key(app, key, tx);
        }
        Event::Mouse(mouse) => handle_mouse(app, mouse, tx),
        Event::Resize(cols, rows) => {
            app.terminal_columns = cols;
            app.terminal_rows = rows;
        }
        _ => {}
    }
}

fn handle_app_event(app: &mut App, event: AppEvent, tx: &UnboundedSender<AppEvent>) {
    match event {
        AppEvent::ProjectsLoaded {
            last_project,
            display_context,
            projects,
        } => {
            app.last_project = last_project;
            app.display_context = display_context;
            app.projects = projects;
            app.projects_loading = false;
            app.projects_error = None;
        }
        AppEvent::ProjectsLoadFailed(err) => {
            app.projects_loading = false;
            app.projects_error = Some(err);
        }
        AppEvent::PrsLoaded { repos, prs } => {
            app.repositories = repos;
            app.pull_requests = prs;
            app.prs_loading = false;
            app.prs_error = None;
            app.refreshed_at = Some(Instant::now());
            app.selected_index = 0;
            sync_comment_activity(app, tx);
            sync_detail_state(app, tx);
        }
        AppEvent::PrsLoadFailed(err) => {
            app.prs_loading = false;
            app.prs_error = Some(err);
        }
        AppEvent::AdoOrgDetected(org) => {
            app.setup_detecting = false;
            if let Some(org) = org {
                app.setup_value = org;
                app.setup_detected_from_az = true;
            }
        }
        AppEvent::AdoSubmitSucceeded {
            client,
            user_id,
            organization,
        } => {
            app.client = Some(client.clone());
            app.display_context = organization;
            app.current_user_id = Some(user_id);
            app.setup_loading = false;
            app.setup_error = None;
            if app.initial_project.is_some() {
                app.screen = AppScreen::PrList;
                app.selected_project = app.initial_project.clone();
                on_filters_changed(app, tx);
            } else {
                app.screen = AppScreen::ProjectSelect;
                app.projects_loading = true;
                tokio::spawn(load_projects(client, tx.clone()));
            }
        }
        AppEvent::AdoSubmitFailed(err) => {
            app.setup_loading = false;
            app.setup_error = Some(err);
        }
        AppEvent::DetailLoaded { key, state } => {
            app.detail_cache.insert(key.clone(), state.clone());
            if app.current_detail_key.as_deref() == Some(key.as_str()) {
                app.detail_state = state;
            }
        }
        AppEvent::CommentActivityLoaded { key, activity } => {
            app.comment_activity.insert(key, activity);
        }
    }
}
