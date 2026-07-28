/**
 * Greedily word-wraps text to a fixed column width, preserving explicit
 * newlines (paragraph breaks) as blank lines. Used to pre-compute lines for
 * scrollable panels so a fixed-height Box never needs to grow/shrink to fit
 * its content.
 */
export function wrapText(text: string, width: number): string[] {
  const safeWidth = Math.max(1, width);
  const lines: string[] = [];

  for (const paragraph of text.split("\n")) {
    const words = paragraph.split(/\s+/).filter(Boolean);
    let current = "";

    for (const rawWord of words) {
      let word = rawWord;
      while (word.length > safeWidth) {
        if (current) {
          lines.push(current);
          current = "";
        }
        lines.push(word.slice(0, safeWidth));
        word = word.slice(safeWidth);
      }
      if (word.length === 0) continue;

      const candidate = current ? `${current} ${word}` : word;
      if (candidate.length > safeWidth) {
        lines.push(current);
        current = word;
      } else {
        current = candidate;
      }
    }

    lines.push(current);
  }

  return lines;
}

export function stripHtml(content: string): string {
  return content.replace(/<[^>]*>/g, "").trim();
}
