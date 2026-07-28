import stripAnsi from "strip-ansi";

export interface SelectionPoint {
  /** Absolute index into the full (unscrolled) content lines array. */
  row: number;
  col: number;
}

export interface SelectionRange {
  anchor: SelectionPoint;
  focus: SelectionPoint;
}

export interface NormalizedSelection {
  startRow: number;
  startCol: number;
  endRow: number;
  endCol: number;
}

export function normalizeSelection({ anchor, focus }: SelectionRange): NormalizedSelection {
  const anchorFirst = anchor.row < focus.row || (anchor.row === focus.row && anchor.col <= focus.col);
  const [start, end] = anchorFirst ? [anchor, focus] : [focus, anchor];
  return { startRow: start.row, startCol: start.col, endRow: end.row, endCol: end.col };
}

/**
 * Resolves the highlight column range for one content row, or null if the
 * row isn't part of the selection. Full-width (`Number.MAX_SAFE_INTEGER`) for
 * rows strictly between the selection's start/end rows, matching how normal
 * multi-line text selection extends across whole intermediate lines.
 */
export function rowHighlightRange(
  rowIndex: number,
  selection: NormalizedSelection | null,
): { start: number; end: number } | null {
  if (!selection) return null;
  if (rowIndex < selection.startRow || rowIndex > selection.endRow) return null;
  const start = rowIndex === selection.startRow ? selection.startCol : 0;
  const end = rowIndex === selection.endRow ? selection.endCol : Number.MAX_SAFE_INTEGER;
  return { start, end };
}

function trimTrailingWhitespace(line: string): string {
  return line.replace(/[ \t]+$/g, "");
}

/** Slices selected text out of content lines, trimming trailing padding per row. */
export function extractSelectedText(
  lines: Array<{ text: string }>,
  selection: NormalizedSelection,
): string {
  const out: string[] = [];
  for (let row = selection.startRow; row <= selection.endRow; row += 1) {
    const line = lines[row];
    if (!line) continue;
    const start = row === selection.startRow ? selection.startCol : 0;
    const end = row === selection.endRow ? selection.endCol : line.text.length - 1;
    const clampedStart = Math.max(0, Math.min(start, line.text.length));
    const clampedEnd = Math.min(end, line.text.length - 1);
    const slice = clampedEnd < clampedStart ? "" : line.text.slice(clampedStart, clampedEnd + 1);
    out.push(trimTrailingWhitespace(stripAnsi(slice)));
  }
  return out.join("\n").replace(/\n+$/g, "");
}
