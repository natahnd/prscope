use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton as CtMouseButton, MouseEvent, MouseEventKind};
use tokio::sync::mpsc::UnboundedSender;

use crate::api::types::{AppScreen, NormalizedPullRequest, ProviderId};
use crate::app::open_url::open_url;
use crate::app::clipboard::build_osc52_copy_sequence;
use crate::app::comment_activity::spawn_missing_comment_activity;
use crate::app::detail_cache::spawn_detail_fetch;
use crate::app::file_diff_cache::{file_diff_key, spawn_file_diff_fetch, FileDiffState};
use crate::app::file_list_rows::resolve_file_list_click_row;
use crate::app::files_panel::{
    build_combined_file_diff_view, file_index_at_scroll, hunk_key_at_scroll, next_comment_scroll,
    next_hunk_scroll, prev_comment_scroll, prev_hunk_scroll, scroll_offset_for_file,
    CombinedFileDiffView, FilesPanelFocus, FilesPanelLayout, FILES_DIVIDER_WIDTH,
    MAX_FILES_LIST_RATIO, MIN_FILES_LIST_RATIO,
};
use crate::app::filter_modal::{filter_modal_state_from_filters, modal_state_to_filters};
use crate::app::layout::{MAX_LEFT_WIDTH_RATIO, MIN_LEFT_WIDTH_RATIO};
use crate::app::pr_list_rows::resolve_list_click_row;
use crate::app::right_panel_lines::{
    build_right_panel_lines, get_visible_right_panel_lines, right_panel_content_width,
    BuildRightPanelLinesArgs, ContentLine, CHROME_ROWS, CONTENT_TOP_ROWS,
};
use crate::app::state::{App, AppEvent, FilesFilter, PanelFocus, RightPanelMode};
use crate::app::text_selection::{
    extract_selected_text, normalize_selection, SelectionPoint, SelectionRange,
};
use crate::providers::registry;

use super::runner::{load_projects, load_pull_requests, spawn_ado_org_detect};

pub fn current_right_panel_lines(app: &App) -> Vec<ContentLine> {
    if app.right_mode == RightPanelMode::Files {
        return combined_file_diff_view(app).lines;
    }
    let displayed = app.displayed_prs();
    let safe_index = app.safe_selected_index(displayed.len());
    let selected_pr = displayed.get(safe_index).copied();
    let geometry = app.geometry();
    let content_width = right_panel_content_width(geometry.right_width);
    build_right_panel_lines(BuildRightPanelLinesArgs {
        pull_request: selected_pr,
        detail_state: &app.detail_state,
        mode: app.right_mode,
        comment_status_filter: app.comment_status_filter,
        content_width,
    })
}

fn combined_file_diff_view(app: &App) -> CombinedFileDiffView {
    let layout = FilesPanelLayout::compute(app);
    let pr_key = app.current_detail_key.as_deref().unwrap_or("");
    let file_indices = app.displayed_file_indices();
    build_combined_file_diff_view(
        &app.detail_state.files,
        &file_indices,
        &app.file_diffs,
        pr_key,
        app.diff_view_mode,
        layout.diff_text_width,
        &app.collapsed_hunk_keys,
    )
}

/// Total content rows available below the tab strip in the detail panel.
pub fn panel_content_height(app: &App) -> usize {
    FilesPanelLayout::compute(app).content_height
}

fn sync_file_list_scroll(app: &mut App) {
    let layout = FilesPanelLayout::compute(app);
    let available = layout.list_inner_height;
    let selected = app.selected_file_index;
    app.file_list_scroll = if selected < app.file_list_scroll {
        selected
    } else if selected >= app.file_list_scroll + available {
        selected.saturating_sub(available.saturating_sub(1))
    } else {
        app.file_list_scroll
    };
}

fn sync_file_selection_from_scroll(app: &mut App) {
    let view = combined_file_diff_view(app);
    let idx = file_index_at_scroll(&view.sections, app.file_diff_scroll);
    if idx != app.selected_file_index {
        app.selected_file_index = idx;
        sync_file_list_scroll(app);
    }
}

fn diff_max_scroll(app: &App) -> usize {
    let layout = FilesPanelLayout::compute(app);
    let view = combined_file_diff_view(app);
    view.lines.len().saturating_sub(layout.diff_inner_height)
}

fn scroll_diff_by(app: &mut App, delta: i64, tx: &UnboundedSender<AppEvent>) {
    let max_scroll = diff_max_scroll(app);
    app.file_diff_scroll =
        (app.file_diff_scroll as i64 + delta).clamp(0, max_scroll as i64) as usize;
    sync_file_selection_from_scroll(app);
    ensure_file_diffs_through(app, app.selected_file_index + 2, tx);
}

fn jump_to_file(app: &mut App, file_index: usize, tx: &UnboundedSender<AppEvent>) {
    let indices = app.displayed_file_indices();
    if indices.is_empty() {
        return;
    }
    let file_index = file_index.min(indices.len() - 1);
    app.selected_file_index = file_index;
    let view = combined_file_diff_view(app);
    app.file_diff_scroll = scroll_offset_for_file(&view.sections, file_index);
    sync_file_list_scroll(app);
    ensure_file_diffs_through(app, file_index + 2, tx);
}

fn jump_to_hunk(app: &mut App, hunk_line: usize, tx: &UnboundedSender<AppEvent>) {
    app.files_panel_focus = FilesPanelFocus::Diff;
    app.file_diff_scroll = hunk_line.min(diff_max_scroll(app));
    sync_file_selection_from_scroll(app);
    ensure_file_diffs_through(app, app.selected_file_index + 2, tx);
}

/// Ensures diffs are loaded (or loading) for every file up through `through_index`.
pub fn ensure_file_diffs_through(
    app: &mut App,
    through_index: usize,
    tx: &UnboundedSender<AppEvent>,
) {
    if app.right_mode != RightPanelMode::Files {
        return;
    }
    if app.current_detail_key.is_none() {
        return;
    }
    let indices = app.displayed_file_indices();
    if indices.is_empty() {
        return;
    }
    let through = through_index.min(indices.len() - 1);
    for &orig_index in indices.iter().take(through + 1) {
        ensure_file_diff_insert_and_spawn(app, orig_index, tx);
    }
}

/// Ensures the diff for the currently selected file (and a few ahead) is
/// either cached or being fetched.
pub fn ensure_selected_file_diff(app: &mut App, tx: &UnboundedSender<AppEvent>) {
    ensure_file_diffs_through(app, app.selected_file_index + 2, tx);
}

fn ensure_file_diff_insert_and_spawn(
    app: &mut App,
    idx: usize,
    tx: &UnboundedSender<AppEvent>,
) {
    let Some(pr_key) = app.current_detail_key.clone() else {
        return;
    };
    let file = app.detail_state.files[idx].clone();
    let key = file_diff_key(&pr_key, &file.path);
    if app.file_diffs.contains_key(&key) {
        return;
    }
    let (Some(client), Some(project)) = (app.client.clone(), app.selected_project.clone()) else {
        return;
    };
    let displayed = app.displayed_prs();
    let safe_index = app.safe_selected_index(displayed.len());
    let Some(pr) = displayed.get(safe_index) else {
        return;
    };
    let repository_id = pr.repository_id.clone();
    let pull_request_id = pr.id;
    app.file_diffs.insert(
        key.clone(),
        FileDiffState {
            diff: None,
            loading: true,
            error: None,
        },
    );
    spawn_file_diff_fetch(
        client,
        project,
        repository_id,
        pull_request_id,
        file,
        app.detail_state.threads.clone(),
        key,
        tx.clone(),
    );
}

/// Keeps `detail_state`/`detail_cache` in sync with whatever PR is currently
/// selected, mirroring `usePullRequestDetail`. Called after every event that
/// could change the selection.
pub fn sync_detail_state(app: &mut App, tx: &UnboundedSender<AppEvent>) {
    let displayed = app.displayed_prs();
    let safe_index = app.safe_selected_index(displayed.len());
    let selected: Option<NormalizedPullRequest> = displayed.get(safe_index).map(|pr| (*pr).clone());

    let (Some(client), Some(project), Some(pr)) = (
        app.client.clone(),
        app.selected_project.clone(),
        selected.clone(),
    ) else {
        app.current_detail_key = None;
        app.detail_state = Default::default();
        return;
    };

    let key = crate::app::pr_key::pr_key_for(&pr);
    if app.current_detail_key.as_deref() == Some(key.as_str()) {
        return;
    }
    app.current_detail_key = Some(key.clone());
    app.description_scroll = 0;
    app.comments_scroll = 0;
    app.right_selection = None;
    app.selected_file_index = 0;
    app.file_diff_scroll = 0;
    app.file_list_scroll = 0;
    app.collapsed_hunk_keys.clear();
    app.files_filter = FilesFilter::All;

    if let Some(cached) = app.detail_cache.get(&key) {
        app.detail_state = cached.clone();
        ensure_selected_file_diff(app, tx);
        return;
    }

    app.detail_state = crate::app::detail_cache::PullRequestDetailState {
        loading: true,
        ..Default::default()
    };
    if !app.detail_cache.is_in_flight(&key) {
        app.detail_cache.mark_in_flight(key);
        spawn_detail_fetch(client, project, pr, tx.clone());
    }
}

pub fn sync_comment_activity(app: &mut App, tx: &UnboundedSender<AppEvent>) {
    let (Some(client), Some(project)) = (app.client.clone(), app.selected_project.clone()) else {
        return;
    };
    spawn_missing_comment_activity(
        client,
        project,
        &app.pull_requests,
        &mut app.comment_activity_requested,
        tx.clone(),
    );
}

pub fn handle_key(app: &mut App, key: KeyEvent, tx: &UnboundedSender<AppEvent>) {
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.should_quit = true;
        return;
    }

    match app.screen {
        AppScreen::ProviderSelect => handle_provider_select_key(app, key, tx),
        AppScreen::ProviderSetup => handle_provider_setup_key(app, key, tx),
        AppScreen::ProjectSelect => handle_project_select_key(app, key, tx),
        AppScreen::PrList => handle_pr_list_key(app, key, tx),
    }
}

fn is_plain_char(key: &KeyEvent, ch: char) -> bool {
    key.code == KeyCode::Char(ch) && !key.modifiers.contains(KeyModifiers::CONTROL)
}

fn typed_char(key: &KeyEvent) -> Option<char> {
    if key.modifiers.contains(KeyModifiers::CONTROL) || key.modifiers.contains(KeyModifiers::ALT) {
        return None;
    }
    match key.code {
        KeyCode::Char(c) => Some(c),
        _ => None,
    }
}

fn handle_provider_select_key(app: &mut App, key: KeyEvent, tx: &UnboundedSender<AppEvent>) {
    let providers = registry::list_providers();
    if providers.is_empty() {
        return;
    }
    match key.code {
        KeyCode::Up => app.provider_picker_index = app.provider_picker_index.saturating_sub(1),
        KeyCode::Down => {
            app.provider_picker_index = (app.provider_picker_index + 1).min(providers.len() - 1)
        }
        KeyCode::Char('k') => app.provider_picker_index = app.provider_picker_index.saturating_sub(1),
        KeyCode::Char('j') => {
            app.provider_picker_index = (app.provider_picker_index + 1).min(providers.len() - 1)
        }
        KeyCode::Enter => {
            let safe = app.provider_picker_index.min(providers.len() - 1);
            let provider = providers[safe];
            if provider.enabled && provider.id == ProviderId::AzureDevops {
                app.screen = AppScreen::ProviderSetup;
                app.setup_error = None;
                app.setup_detecting = true;
                app.setup_value.clear();
                app.setup_detected_from_az = false;
                spawn_ado_org_detect(tx.clone());
            }
        }
        KeyCode::Char('q') => app.should_quit = true,
        _ => {}
    }
}

fn handle_provider_setup_key(app: &mut App, key: KeyEvent, tx: &UnboundedSender<AppEvent>) {
    if app.setup_detecting {
        if is_plain_char(&key, 'q') {
            app.should_quit = true;
        }
        return;
    }
    if app.setup_loading {
        return;
    }

    match key.code {
        KeyCode::Enter => {
            let org = app.setup_value.trim().to_string();
            if org.is_empty() {
                app.setup_error = Some("Organization name cannot be empty.".to_string());
                return;
            }
            app.setup_loading = true;
            app.setup_error = None;
            let tx = tx.clone();
            tokio::spawn(async move {
                let result = async {
                    let token = registry::resolve_provider_token(ProviderId::AzureDevops).await?;
                    let client = registry::create_azure_devops_client_direct(token, org.clone())?;
                    client.get_current_user().await?;
                    client.list_projects().await?;
                    crate::config::store::save_provider_config(
                        ProviderId::AzureDevops,
                        Some(org.clone()),
                    )
                    .await?;
                    let user = client.get_current_user().await?;
                    anyhow::Ok((client, user.id, org))
                }
                .await;
                match result {
                    Ok((client, user_id, organization)) => {
                        let _ = tx.send(AppEvent::AdoSubmitSucceeded {
                            client,
                            user_id,
                            organization,
                        });
                    }
                    Err(err) => {
                        let _ = tx.send(AppEvent::AdoSubmitFailed(err.to_string()));
                    }
                }
            });
        }
        KeyCode::Char('q') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
        }
        KeyCode::Backspace | KeyCode::Delete => {
            app.setup_value.pop();
        }
        _ => {
            if let Some(c) = typed_char(&key) {
                app.setup_value.push(c);
            }
        }
    }
}

fn handle_project_select_key(app: &mut App, key: KeyEvent, tx: &UnboundedSender<AppEvent>) {
    let filtered_len = app.filtered_projects().len();
    match key.code {
        KeyCode::Up => app.project_index = app.project_index.saturating_sub(1),
        KeyCode::Char('k') => app.project_index = app.project_index.saturating_sub(1),
        KeyCode::Down => {
            app.project_index = (app.project_index + 1).min(filtered_len.saturating_sub(1))
        }
        KeyCode::Char('j') => {
            app.project_index = (app.project_index + 1).min(filtered_len.saturating_sub(1))
        }
        KeyCode::Enter => {
            let filtered: Vec<crate::api::types::Project> =
                app.filtered_projects().into_iter().cloned().collect();
            let safe = if filtered.is_empty() {
                0
            } else {
                app.project_index.min(filtered.len() - 1)
            };
            if let Some(project) = filtered.get(safe).cloned() {
                app.selected_project = Some(project.name.clone());
                app.last_project = Some(project.name.clone());
                app.screen = AppScreen::PrList;
                app.search_query.clear();
                app.search_mode = false;
                let name = project.name.clone();
                tokio::spawn(async move {
                    let _ = crate::config::store::update_last_project(&name).await;
                });
                on_filters_changed(app, tx);
            }
        }
        KeyCode::Char('r') => {
            if let Some(client) = app.client.clone() {
                app.projects_loading = true;
                app.projects_error = None;
                let tx = tx.clone();
                tokio::spawn(load_projects(client, tx));
            }
        }
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Backspace | KeyCode::Delete => {
            app.project_search.pop();
            app.project_index = 0;
        }
        _ => {
            if let Some(c) = typed_char(&key) {
                app.project_search.push(c);
                app.project_index = 0;
            }
        }
    }
}

fn handle_pr_list_key(app: &mut App, key: KeyEvent, tx: &UnboundedSender<AppEvent>) {
    if app.filter_modal_open {
        handle_filter_modal_key(app, key, tx);
        return;
    }

    if app.search_mode {
        match key.code {
            KeyCode::Enter => {
                app.search_mode = false;
                app.filters.search = if app.search_query.is_empty() {
                    None
                } else {
                    Some(app.search_query.clone())
                };
                apply_filters_change(app, tx);
            }
            KeyCode::Esc => {
                app.search_mode = false;
                app.search_query.clear();
                app.filters.search = None;
                apply_filters_change(app, tx);
            }
            KeyCode::Backspace | KeyCode::Delete => {
                app.search_query.pop();
            }
            _ => {
                if let Some(c) = typed_char(&key) {
                    app.search_query.push(c);
                }
            }
        }
        return;
    }

    let displayed_len = app.displayed_prs().len();

    match key.code {
        KeyCode::Tab => {
            app.panel_focus = match app.panel_focus {
                PanelFocus::List => PanelFocus::Detail,
                PanelFocus::Detail => PanelFocus::List,
            };
        }
        KeyCode::Up => {
            move_selection_up(app, tx);
            sync_detail_state(app, tx);
        }
        KeyCode::Char('k') => {
            move_selection_up(app, tx);
            sync_detail_state(app, tx);
        }
        KeyCode::Down => {
            move_selection_down(app, displayed_len, tx);
            sync_detail_state(app, tx);
        }
        KeyCode::Char('j') => {
            move_selection_down(app, displayed_len, tx);
            sync_detail_state(app, tx);
        }
        KeyCode::PageUp if app.panel_focus == PanelFocus::Detail && app.right_mode == RightPanelMode::Files => {
            let page = panel_content_height(app);
            scroll_diff_by(app, -(page as i64), tx);
        }
        KeyCode::PageDown if app.panel_focus == PanelFocus::Detail && app.right_mode == RightPanelMode::Files => {
            let page = panel_content_height(app);
            scroll_diff_by(app, page as i64, tx);
        }
        KeyCode::Char('v')
            if app.panel_focus == PanelFocus::Detail && app.right_mode == RightPanelMode::Files =>
        {
            app.diff_view_mode = app.diff_view_mode.toggle();
            let view = combined_file_diff_view(app);
            app.file_diff_scroll =
                scroll_offset_for_file(&view.sections, app.selected_file_index);
        }
        KeyCode::Char(']')
            if app.panel_focus == PanelFocus::Detail && app.right_mode == RightPanelMode::Files =>
        {
            let view = combined_file_diff_view(app);
            if let Some(offset) = next_hunk_scroll(&view.hunk_lines, app.file_diff_scroll) {
                jump_to_hunk(app, offset, tx);
            }
        }
        KeyCode::Char('[')
            if app.panel_focus == PanelFocus::Detail && app.right_mode == RightPanelMode::Files =>
        {
            let view = combined_file_diff_view(app);
            if let Some(offset) = prev_hunk_scroll(&view.hunk_lines, app.file_diff_scroll) {
                jump_to_hunk(app, offset, tx);
            }
        }
        KeyCode::Char('n')
            if app.panel_focus == PanelFocus::Detail && app.right_mode == RightPanelMode::Files =>
        {
            let view = combined_file_diff_view(app);
            if let Some(offset) = next_comment_scroll(&view.comment_lines, app.file_diff_scroll) {
                jump_to_hunk(app, offset, tx);
            }
        }
        KeyCode::Char('N')
            if app.panel_focus == PanelFocus::Detail && app.right_mode == RightPanelMode::Files =>
        {
            let view = combined_file_diff_view(app);
            if let Some(offset) = prev_comment_scroll(&view.comment_lines, app.file_diff_scroll) {
                jump_to_hunk(app, offset, tx);
            }
        }
        KeyCode::Char('z')
            if app.panel_focus == PanelFocus::Detail && app.right_mode == RightPanelMode::Files =>
        {
            let view = combined_file_diff_view(app);
            if let Some(key) =
                hunk_key_at_scroll(&view.lines, &view.hunk_lines, app.file_diff_scroll)
            {
                if app.collapsed_hunk_keys.contains(&key) {
                    app.collapsed_hunk_keys.remove(&key);
                } else {
                    app.collapsed_hunk_keys.insert(key);
                }
            }
        }
        KeyCode::Left | KeyCode::Char('h') if app.panel_focus == PanelFocus::Detail => {
            app.right_mode = app.right_mode.prev();
            app.right_selection = None;
            ensure_selected_file_diff(app, tx);
        }
        KeyCode::Right | KeyCode::Char('l') if app.panel_focus == PanelFocus::Detail => {
            app.right_mode = app.right_mode.next();
            app.right_selection = None;
            ensure_selected_file_diff(app, tx);
        }
        KeyCode::Char('s')
            if app.panel_focus == PanelFocus::Detail && app.right_mode == RightPanelMode::Comments =>
        {
            app.comment_status_filter = app.comment_status_filter.next();
            app.comments_scroll = 0;
            app.right_selection = None;
        }
        KeyCode::Char('s')
            if app.panel_focus == PanelFocus::Detail && app.right_mode == RightPanelMode::Files =>
        {
            app.files_filter = app.files_filter.next();
            app.selected_file_index = 0;
            app.file_list_scroll = 0;
            app.file_diff_scroll = 0;
            app.clamp_file_selection();
            ensure_selected_file_diff(app, tx);
        }
        KeyCode::Char('y') => {
            let lines = current_right_panel_lines(app);
            if !lines.is_empty() {
                let plain: Vec<String> = lines.iter().map(|l| l.plain_text()).collect();
                let selection = SelectionRange {
                    anchor: SelectionPoint { row: 0, col: 0 },
                    focus: SelectionPoint {
                        row: lines.len() - 1,
                        col: usize::MAX,
                    },
                };
                let normalized = normalize_selection(selection);
                let text = extract_selected_text(&plain, &normalized);
                if !text.is_empty() {
                    write_clipboard(&text);
                }
            }
        }
        KeyCode::Enter | KeyCode::Char('o') => {
            let displayed = app.displayed_prs();
            let safe = app.safe_selected_index(displayed.len());
            if let Some(pr) = displayed.get(safe) {
                let _ = open_url(&pr.url);
            }
        }
        KeyCode::Char('p') => {
            app.screen = AppScreen::ProjectSelect;
            app.project_search.clear();
            app.project_index = 0;
            if let Some(client) = app.client.clone() {
                app.projects_loading = true;
                app.projects_error = None;
                tokio::spawn(load_projects(client, tx.clone()));
            }
        }
        KeyCode::Char('f') => {
            app.filter_modal = Some(filter_modal_state_from_filters(
                &app.filters,
                &app.repositories,
            ));
            app.filter_modal_open = true;
        }
        KeyCode::Char('g') => {
            app.list_layout = app.list_layout.next();
            app.selected_index = 0;
            persist_list_layout(app);
        }
        KeyCode::Char('/') => {
            app.search_mode = true;
        }
        KeyCode::Char('r') => {
            if let (Some(client), Some(project)) = (app.client.clone(), app.selected_project.clone()) {
                app.prs_loading = true;
                app.prs_error = None;
                tokio::spawn(load_pull_requests(
                    client,
                    project,
                    app.filters.clone(),
                    app.current_user_id.clone(),
                    tx.clone(),
                ));
            }
        }
        KeyCode::Char('q') => app.should_quit = true,
        _ => {}
    }
}

fn move_selection_up(app: &mut App, tx: &UnboundedSender<AppEvent>) {
    if app.panel_focus == PanelFocus::Detail {
        match app.right_mode {
            RightPanelMode::Description => {
                app.description_scroll = app.description_scroll.saturating_sub(1);
            }
            RightPanelMode::Comments => {
                app.comments_scroll = app.comments_scroll.saturating_sub(1);
            }
            RightPanelMode::Files => match app.files_panel_focus {
                FilesPanelFocus::Diff => scroll_diff_by(app, -1, tx),
                FilesPanelFocus::FileList => {
                    if app.selected_file_index > 0 {
                        jump_to_file(app, app.selected_file_index - 1, tx);
                    }
                }
            },
        }
    } else {
        app.selected_index = app.selected_index.saturating_sub(1);
    }
}

fn move_selection_down(app: &mut App, displayed_len: usize, tx: &UnboundedSender<AppEvent>) {
    if app.panel_focus == PanelFocus::Detail {
        match app.right_mode {
            RightPanelMode::Description => {
                let max_scroll = right_panel_max_scroll(app);
                app.right_max_scroll = max_scroll;
                app.description_scroll = (app.description_scroll + 1).min(max_scroll);
            }
            RightPanelMode::Comments => {
                let max_scroll = right_panel_max_scroll(app);
                app.right_max_scroll = max_scroll;
                app.comments_scroll = (app.comments_scroll + 1).min(max_scroll);
            }
            RightPanelMode::Files => match app.files_panel_focus {
                FilesPanelFocus::Diff => scroll_diff_by(app, 1, tx),
                FilesPanelFocus::FileList => {
                    let max_index = app.displayed_file_count().saturating_sub(1);
                    if app.selected_file_index < max_index {
                        jump_to_file(app, app.selected_file_index + 1, tx);
                    }
                }
            },
        }
    } else {
        let max_index = displayed_len.saturating_sub(1);
        app.selected_index = (app.selected_index + 1).min(max_index);
    }
}

/// Recomputes how far the right panel can scroll, mirroring the same
/// content-height math the renderer uses (`ui/pr_right_panel.rs`), so
/// keyboard/mouse scroll clamping never lags behind what's actually on
/// screen (see `app.right_max_scroll` for why this can't just be cached).
fn right_panel_max_scroll(app: &App) -> usize {
    let content_height = panel_content_height(app);
    let lines = current_right_panel_lines(app);
    lines.len().saturating_sub(content_height)
}

fn handle_filter_modal_key(app: &mut App, key: KeyEvent, tx: &UnboundedSender<AppEvent>) {
    let Some(state) = app.filter_modal.as_mut() else {
        app.filter_modal_open = false;
        return;
    };
    let repo_count = app.repositories.len();

    match key.code {
        KeyCode::Esc => {
            app.filter_modal_open = false;
            app.filter_modal = None;
        }
        KeyCode::Enter => {
            if let Some(user_id) = app.current_user_id.clone() {
                let mut next = modal_state_to_filters(state, &app.repositories, &user_id);
                if !app.search_query.is_empty() {
                    next.search = Some(app.search_query.clone());
                }
                app.filters = next;
                app.filter_modal_open = false;
                app.filter_modal = None;
                app.selected_index = 0;
                apply_filters_change(app, tx);
            }
        }
        KeyCode::Down | KeyCode::Char('j') => state.move_down(repo_count),
        KeyCode::Up | KeyCode::Char('k') => state.move_up(),
        KeyCode::Left | KeyCode::Char('h') => state.move_left(),
        KeyCode::Right | KeyCode::Char('l') => state.move_right(),
        KeyCode::Char(' ') => state.toggle_space(),
        _ => {}
    }
}

pub fn persist_filters(app: &App) {
    if !app.filters_hydrated {
        return;
    }
    let filters = app.filters.clone();
    tokio::spawn(async move {
        let _ = crate::config::store::update_last_filters(&filters).await;
    });
}

pub fn persist_list_layout(app: &App) {
    if !app.filters_hydrated {
        return;
    }
    let layout = app.list_layout;
    tokio::spawn(async move {
        let _ = crate::config::store::update_last_list_layout(layout).await;
    });
}

/// Mirrors the TS app's `filters` effect: persist (skipping the very first
/// hydration) and re-fetch the PR list under the new filters.
pub fn apply_filters_change(app: &mut App, tx: &UnboundedSender<AppEvent>) {
    persist_filters(app);
    on_filters_changed(app, tx);
}

fn write_clipboard(text: &str) {
    use std::io::Write;
    let sequence = build_osc52_copy_sequence(text);
    let mut stdout = std::io::stdout();
    let _ = stdout.write_all(sequence.as_bytes());
    let _ = stdout.flush();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Down,
    Up,
    Move,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Button {
    Left,
    Other,
    WheelUp,
    WheelDown,
}

fn decode_mouse(kind: MouseEventKind) -> Option<(Action, Button)> {
    match kind {
        MouseEventKind::Down(CtMouseButton::Left) => Some((Action::Down, Button::Left)),
        MouseEventKind::Down(_) => Some((Action::Down, Button::Other)),
        MouseEventKind::Up(_) => Some((Action::Up, Button::Other)),
        MouseEventKind::Drag(_) => Some((Action::Move, Button::Other)),
        MouseEventKind::Moved => Some((Action::Move, Button::Other)),
        MouseEventKind::ScrollUp => Some((Action::Down, Button::WheelUp)),
        MouseEventKind::ScrollDown => Some((Action::Down, Button::WheelDown)),
        MouseEventKind::ScrollLeft | MouseEventKind::ScrollRight => None,
    }
}

fn resolve_right_panel_point(
    app: &App,
    lines: &[ContentLine],
    x: i64,
    y: i64,
    strict: bool,
) -> Option<SelectionPoint> {
    if lines.is_empty() {
        return None;
    }
    let geometry = app.geometry();
    let content_width = right_panel_content_width(geometry.right_width);
    let content_height = (geometry.panel_height as i64 - CHROME_ROWS as i64).max(1) as usize;
    let current_scroll = match app.right_mode {
        RightPanelMode::Description => app.description_scroll,
        RightPanelMode::Comments => app.comments_scroll,
        RightPanelMode::Files => app.file_diff_scroll,
    };
    let visible = get_visible_right_panel_lines(lines, current_scroll, content_height);
    let clamped_scroll = visible.clamped_scroll;

    let viewport_row = y - geometry.reserved_top as i64 - CONTENT_TOP_ROWS as i64;
    if strict && (viewport_row < 0 || viewport_row >= content_height as i64) {
        return None;
    }
    let col = (x - geometry.left_width as i64 - 2).clamp(0, content_width as i64 - 1);
    let clamped_viewport_row = viewport_row.clamp(0, content_height as i64 - 1);
    let row = (clamped_scroll as i64 + clamped_viewport_row).clamp(0, lines.len() as i64 - 1);
    Some(SelectionPoint {
        row: row as usize,
        col: col as usize,
    })
}

pub fn handle_mouse(app: &mut App, event: MouseEvent, tx: &UnboundedSender<AppEvent>) {
    if app.screen != AppScreen::PrList || app.filter_modal_open {
        return;
    }
    let Some((action, button)) = decode_mouse(event.kind) else {
        return;
    };

    let geometry = app.geometry();
    let x = event.column as i64;
    let y = event.row as i64;
    let within_main_row = y >= geometry.reserved_top as i64
        && y < geometry.reserved_top as i64 + geometry.panel_height as i64;

    if action == Action::Move {
        if app.dragging_divider {
            let ratio = x as f64 / (app.terminal_columns as f64).max(1.0);
            app.left_width_ratio = ratio.clamp(MIN_LEFT_WIDTH_RATIO, MAX_LEFT_WIDTH_RATIO);
            return;
        }
        if app.dragging_files_divider {
            let geometry = app.geometry();
            let detail_content_left = geometry.left_width as i64 + 1;
            let available = geometry.right_width as i64 - 2 - FILES_DIVIDER_WIDTH as i64;
            if available > 0 {
                let list_w = (x - detail_content_left).clamp(0, available);
                app.files_list_width_ratio =
                    (list_w as f64 / available as f64)
                        .clamp(MIN_FILES_LIST_RATIO, MAX_FILES_LIST_RATIO);
            }
            return;
        }
        if app.dragging_selection {
            if let Some(current) = app.right_selection {
                let lines = current_right_panel_lines(app);
                let point = resolve_right_panel_point(app, &lines, x, y, false);
                if let Some(point) = point {
                    app.right_selection = Some(SelectionRange {
                        anchor: current.anchor,
                        focus: point,
                    });
                }
            }
        }
        return;
    }

    if action == Action::Up {
        app.dragging_divider = false;
        app.dragging_files_divider = false;
        if app.dragging_selection {
            app.dragging_selection = false;
            let moved = app
                .right_selection
                .map(|s| s.anchor.row != s.focus.row || s.anchor.col != s.focus.col)
                .unwrap_or(false);
            if !moved {
                app.right_selection = None;
            }
        }
        return;
    }

    // action == Down from here on.
    if button == Button::WheelUp || button == Button::WheelDown {
        if !within_main_row {
            return;
        }
        let delta: i64 = if button == Button::WheelUp { -1 } else { 1 };
        if x < geometry.left_width as i64 {
            let displayed_len = app.displayed_prs().len();
            let max_index = displayed_len.saturating_sub(1) as i64;
            app.selected_index = (app.selected_index as i64 + delta).clamp(0, max_index.max(0)) as usize;
            sync_detail_state(app, tx);
        } else if app.right_mode == RightPanelMode::Files {
            let layout = FilesPanelLayout::compute(app);
            if layout.is_in_list_card(x) {
                let max_scroll = app
                    .displayed_file_count()
                    .saturating_sub(layout.list_inner_height);
                app.file_list_scroll =
                    (app.file_list_scroll as i64 + delta).clamp(0, max_scroll as i64) as usize;
            } else if layout.is_in_diff_card(x) {
                scroll_diff_by(app, delta, tx);
            }
        } else {
            let max_scroll = right_panel_max_scroll(app);
            app.right_max_scroll = max_scroll;
            if app.right_mode == RightPanelMode::Description {
                app.description_scroll =
                    (app.description_scroll as i64 + delta).clamp(0, max_scroll as i64) as usize;
            } else {
                app.comments_scroll =
                    (app.comments_scroll as i64 + delta).clamp(0, max_scroll as i64) as usize;
            }
        }
        return;
    }

    if !within_main_row {
        return;
    }

    app.right_selection = None;

    if (x - geometry.left_width as i64).abs() <= 1 {
        app.dragging_divider = true;
        return;
    }

    if app.right_mode == RightPanelMode::Files && x >= geometry.left_width as i64 {
        let layout = FilesPanelLayout::compute(app);
        if layout.is_on_divider(x) {
            app.dragging_files_divider = true;
            return;
        }
    }

    if x < geometry.left_width as i64 {
        app.panel_focus = PanelFocus::List;
        let local_row = y - geometry.reserved_top as i64;
        let displayed = app.displayed_prs();
        let safe_index = app.safe_selected_index(displayed.len());
        if let Some(pr_index) = resolve_list_click_row(
            &displayed,
            safe_index,
            geometry.panel_height as usize,
            local_row,
            app.search_mode,
            app.list_layout,
        ) {
            app.selected_index = pr_index;
            sync_detail_state(app, tx);
        }
    } else {
        app.panel_focus = PanelFocus::Detail;
        if button == Button::Left {
            if app.right_mode == RightPanelMode::Files {
                let layout = FilesPanelLayout::compute(app);
                if layout.is_in_list_card(x) {
                    app.files_panel_focus = FilesPanelFocus::FileList;
                    if let Some(local_row) = files_list_local_row(app, y) {
                        let displayed_count = app.displayed_file_count();
                        if let Some(idx) = resolve_file_list_click_row(
                            displayed_count,
                            app.selected_file_index,
                            app.file_list_scroll,
                            local_row,
                            layout.list_inner_height,
                        ) {
                            jump_to_file(app, idx, tx);
                        }
                    }
                } else if layout.is_in_diff_card(x) {
                    app.files_panel_focus = FilesPanelFocus::Diff;
                }
            } else {
                let lines = current_right_panel_lines(app);
                let point = resolve_right_panel_point(app, &lines, x, y, true);
                if let Some(point) = point {
                    app.right_selection = Some(SelectionRange {
                        anchor: point,
                        focus: point,
                    });
                    app.dragging_selection = true;
                }
            }
        }
    }
}

/// Row within the file-list card's scrollable content, 0-based.
fn files_list_local_row(app: &App, y: i64) -> Option<i64> {
    let geometry = app.geometry();
    // Below tab strip + file-list card top border (title shares that row).
    let row = y - geometry.reserved_top as i64 - CONTENT_TOP_ROWS as i64 - 1;
    if row < 0 {
        None
    } else {
        Some(row)
    }
}

pub fn on_filters_changed(app: &mut App, tx: &UnboundedSender<AppEvent>) {
    if let (Some(client), Some(project)) = (app.client.clone(), app.selected_project.clone()) {
        app.prs_loading = true;
        app.prs_error = None;
        tokio::spawn(load_pull_requests(
            client,
            project,
            app.filters.clone(),
            app.current_user_id.clone(),
            tx.clone(),
        ));
    }
}

