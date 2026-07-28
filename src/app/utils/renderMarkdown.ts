import cliTruncate from "cli-truncate";
import emojiRegex from "emoji-regex";
import { Marked } from "marked";
import { markedTerminal } from "marked-terminal";
import supportsHyperlinks from "supports-hyperlinks";
import { wrapText } from "./wrapText.js";

// The installed ansi-width toolchain (string-width/slice-ansi) doesn't parse
// OSC 8 hyperlink escapes correctly, so clickable links can corrupt output
// when lines get reflowed/truncated. Force plain "text (url)" rendering.
supportsHyperlinks.stdout = false;
supportsHyperlinks.stderr = false;

const renderers = new Map<number, Marked>();

function getRenderer(width: number): Marked {
  let instance = renderers.get(width);
  if (!instance) {
    instance = new Marked();
    instance.use(
      markedTerminal({
        width,
        reflowText: true,
        showSectionPrefix: false,
        tab: 2,
      }),
    );
    renderers.set(width, instance);
  }
  return instance;
}

/**
 * Renders markdown (PR descriptions, comment bodies) to ANSI-styled lines
 * sized to a fixed column width, glow-style, so it can be dropped straight
 * into a fixed-height scrollable line list. Falls back to plain wrapped text
 * if parsing fails (e.g. malformed markdown).
 */
export function renderMarkdownToLines(markdown: string, width: number): string[] {
  const trimmed = markdown.trim();
  if (!trimmed) return [];

  try {
    const output = getRenderer(width).parse(trimmed, { async: false });
    // Terminal emoji rendering width is inconsistent (many terminals draw
    // them double-width while our tooling measures them as single-width),
    // which throws off fixed-width layout by a column and corrupts wrapping.
    // Strip them so every visible character has an unambiguous column width.
    const deEmojified = output.replace(emojiRegex(), "");
    // marked-terminal reflows paragraphs to `width` but doesn't constrain
    // wide elements (tables, unbroken links), so clip defensively to keep
    // every rendered line within the fixed-height, fixed-width panel.
    return deEmojified
      .replace(/\s+$/, "")
      .split("\n")
      .map((line) => cliTruncate(line, width));
  } catch {
    return wrapText(trimmed, width);
  }
}
