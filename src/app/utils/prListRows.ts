import type { NormalizedPullRequest } from "../../api/types.js";

export type VisualRow =
  | { type: "header"; key: string; repoName: string }
  | { type: "spacer"; key: string }
  | { type: "title"; key: string; pr: NormalizedPullRequest; prIndex: number }
  | { type: "meta"; key: string; pr: NormalizedPullRequest; prIndex: number };

// Rows above the scrollable list within the panel: border-top + "Pull
// Requests (N)" line + "Filters: ..." line (+1 more while the inline search
// box is showing). Shared by rendering (PrListPanel) and mouse hit-testing
// (App) so they can never drift out of sync.
export const LIST_CHROME_ROWS = 3;

export function listChromeRows(searchMode: boolean): number {
  return LIST_CHROME_ROWS + (searchMode ? 1 : 0);
}

export function buildRows(pullRequests: NormalizedPullRequest[]): VisualRow[] {
  const rows: VisualRow[] = [];
  let lastRepo: string | null = null;

  pullRequests.forEach((pr, index) => {
    if (pr.repoName !== lastRepo) {
      if (rows.length > 0) {
        rows.push({ type: "spacer", key: `sp-${pr.repositoryId}-${pr.id}` });
      }
      rows.push({
        type: "header",
        key: `h-${pr.repositoryId}-${pr.repoName}`,
        repoName: pr.repoName,
      });
      lastRepo = pr.repoName;
    } else {
      rows.push({ type: "spacer", key: `sp-${pr.repositoryId}-${pr.id}` });
    }
    rows.push({ type: "title", key: `t-${pr.repositoryId}-${pr.id}`, pr, prIndex: index });
    rows.push({ type: "meta", key: `m-${pr.repositoryId}-${pr.id}`, pr, prIndex: index });
  });

  return rows;
}

export function computeScrollStart(
  selectedRowIndex: number,
  total: number,
  visibleRows: number,
): number {
  if (total <= visibleRows) return 0;
  const start = selectedRowIndex - Math.floor(visibleRows / 2);
  return Math.max(0, Math.min(start, total - visibleRows));
}

export function getVisiblePrRows(
  pullRequests: NormalizedPullRequest[],
  selectedIndex: number,
  panelHeight: number,
  searchMode = false,
): { rows: VisualRow[]; visible: VisualRow[]; start: number; availableRows: number } {
  const rows = buildRows(pullRequests);
  const availableRows = Math.max(1, panelHeight - (listChromeRows(searchMode) + 1));
  const selectedRowIndex = rows.findIndex(
    (row) => row.type === "title" && row.prIndex === selectedIndex,
  );
  const start = computeScrollStart(Math.max(0, selectedRowIndex), rows.length, availableRows);
  const visible = rows.slice(start, start + availableRows);
  return { rows, visible, start, availableRows };
}

/**
 * Resolves a mouse click at `localRow` (0 = the panel's own top border row)
 * to the pull request index it landed on, or null if it hit chrome/spacer.
 */
export function resolveListClickRow(
  pullRequests: NormalizedPullRequest[],
  selectedIndex: number,
  panelHeight: number,
  localRow: number,
  searchMode = false,
): number | null {
  const { rows, start } = getVisiblePrRows(pullRequests, selectedIndex, panelHeight, searchMode);
  const contentRow = localRow - listChromeRows(searchMode);
  if (contentRow < 0) return null;
  const row = rows[start + contentRow];
  if (!row) return null;
  if (row.type === "title" || row.type === "meta") return row.prIndex;
  return null;
}
