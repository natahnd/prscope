export interface LayoutGeometry {
  /** Absolute terminal row where the list/detail panel row begins. */
  reservedTop: number;
  panelHeight: number;
  leftWidth: number;
  rightWidth: number;
}

export const STATUS_BAR_HEIGHT = 3;
export const HEADER_HEIGHT = 3;
export const ERROR_LINE_HEIGHT = 2;
export const MIN_PANEL_WIDTH = 20;
export const MIN_PANEL_HEIGHT = 5;
export const MIN_LEFT_WIDTH_RATIO = 0.15;
export const MAX_LEFT_WIDTH_RATIO = 0.7;
export const DEFAULT_LEFT_WIDTH_RATIO = 0.35;

/**
 * Computes the shared panel layout used both to render PrReviewLayout and to
 * hit-test mouse clicks in App — kept in one place so the two can never
 * drift out of sync. The search box lives inside the PR list panel's own
 * header (see prListRows.ts), not here, so it doesn't affect this geometry.
 */
export function computeLayoutGeometry(
  terminalSize: { columns: number; rows: number },
  options: { hasError: boolean; leftWidthRatio: number },
): LayoutGeometry {
  const reservedTop = HEADER_HEIGHT + (options.hasError ? ERROR_LINE_HEIGHT : 0);
  const panelHeight = Math.max(
    MIN_PANEL_HEIGHT,
    terminalSize.rows - reservedTop - STATUS_BAR_HEIGHT,
  );
  const leftWidth = Math.max(
    MIN_PANEL_WIDTH,
    Math.floor(terminalSize.columns * options.leftWidthRatio),
  );
  const rightWidth = Math.max(MIN_PANEL_WIDTH, terminalSize.columns - leftWidth);
  return { reservedTop, panelHeight, leftWidth, rightWidth };
}
