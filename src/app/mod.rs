pub mod review_status;
pub mod open_url;
pub mod clipboard;
pub mod comment_activity;
pub mod comment_status;
pub mod detail_cache;
pub mod file_diff_cache;
pub mod file_list_rows;
pub mod files_panel;
pub mod filter_modal;
pub mod layout;
pub mod markdown;
pub mod pr_key;
pub mod pr_list_rows;
pub mod right_panel_lines;
pub mod state;
pub mod text_selection;
pub mod text_width;
pub mod ui;
pub mod update;
pub mod wrap_text;

pub use state::{App, AppEvent};

mod runner;
pub use runner::run_tui;
