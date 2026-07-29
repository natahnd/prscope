//! Shared terminal geometry used by rendering and mouse hit-testing.

/// Pixel-free terminal dimensions shared by rendering and pointer handling.
#[derive(Debug, Clone, Copy)]
pub struct LayoutGeometry {
    /// Absolute terminal row where the list/detail panel row begins.
    pub reserved_top: u16,
    pub panel_height: u16,
    pub left_width: u16,
    pub right_width: u16,
}

/// Height of the bottom keyboard-hint bar.
pub const STATUS_BAR_HEIGHT: u16 = 3;
/// Height of the PR-list screen header.
pub const HEADER_HEIGHT: u16 = 3;
/// Extra height reserved while a PR loading error is visible.
pub const ERROR_LINE_HEIGHT: u16 = 2;
/// Minimum width assigned to either split panel.
pub const MIN_PANEL_WIDTH: u16 = 20;
/// Minimum height assigned to the main panel row.
pub const MIN_PANEL_HEIGHT: u16 = 5;
/// Smallest user-selectable width ratio for the left panel.
pub const MIN_LEFT_WIDTH_RATIO: f64 = 0.15;
/// Largest user-selectable width ratio for the left panel.
pub const MAX_LEFT_WIDTH_RATIO: f64 = 0.7;
/// Initial width ratio for the left panel.
pub const DEFAULT_LEFT_WIDTH_RATIO: f64 = 0.35;

/// Inputs that affect top-level panel geometry.
#[derive(Debug, Clone, Copy)]
pub struct LayoutOptions {
    pub has_error: bool,
    pub left_width_ratio: f64,
}

/// Computes the shared panel layout used both to render the app and to
/// hit-test mouse clicks — kept in one place so the two can never drift out
/// of sync. The search box lives inside the PR list panel's own header, not
/// here, so it doesn't affect this geometry.
///
/// # Arguments
///
/// * `columns` - Current terminal width in columns.
/// * `rows` - Current terminal height in rows.
/// * `options` - Whether an error line is showing and the user's chosen
///   left-panel width ratio.
///
/// # Returns
///
/// The resolved `LayoutGeometry`. Panel height and width are each floored at
/// [`MIN_PANEL_HEIGHT`]/[`MIN_PANEL_WIDTH`], so this never panics or
/// produces a negative/zero-sized panel even on a very small terminal.
pub fn compute_layout_geometry(columns: u16, rows: u16, options: LayoutOptions) -> LayoutGeometry {
    let reserved_top = HEADER_HEIGHT + if options.has_error { ERROR_LINE_HEIGHT } else { 0 };
    let panel_height = (rows as i32 - reserved_top as i32 - STATUS_BAR_HEIGHT as i32)
        .max(MIN_PANEL_HEIGHT as i32) as u16;
    let left_width =
        ((columns as f64) * options.left_width_ratio).floor().max(MIN_PANEL_WIDTH as f64) as u16;
    let right_width = ((columns as i32) - (left_width as i32)).max(MIN_PANEL_WIDTH as i32) as u16;
    LayoutGeometry {
        reserved_top,
        panel_height,
        left_width,
        right_width,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_geometry_for_typical_terminal() {
        let geometry = compute_layout_geometry(
            120,
            40,
            LayoutOptions {
                has_error: false,
                left_width_ratio: DEFAULT_LEFT_WIDTH_RATIO,
            },
        );
        assert_eq!(geometry.reserved_top, 3);
        assert_eq!(geometry.panel_height, 40 - 3 - 3);
        assert_eq!(geometry.left_width, 42);
        assert_eq!(geometry.right_width, 78);
    }

    #[test]
    fn error_line_adds_reserved_rows() {
        let geometry = compute_layout_geometry(
            120,
            40,
            LayoutOptions {
                has_error: true,
                left_width_ratio: DEFAULT_LEFT_WIDTH_RATIO,
            },
        );
        assert_eq!(geometry.reserved_top, 5);
    }

    #[test]
    fn enforces_minimum_panel_dimensions_on_tiny_terminals() {
        let geometry = compute_layout_geometry(
            10,
            5,
            LayoutOptions {
                has_error: false,
                left_width_ratio: DEFAULT_LEFT_WIDTH_RATIO,
            },
        );
        assert_eq!(geometry.panel_height, MIN_PANEL_HEIGHT);
        assert_eq!(geometry.left_width, MIN_PANEL_WIDTH);
    }
}
