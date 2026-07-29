//! Markdown-to-terminal rendering with structured Ratatui styles.

use std::iter::Peekable;

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::style::{Color, Modifier, Style};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// A rendered terminal line expressed as text and Ratatui style pairs.
///
/// Keeping style metadata separate from text avoids embedding ANSI escapes in
/// copied content and makes width calculations deterministic.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StyledLine(pub Vec<(String, Style)>);

impl StyledLine {
    /// Returns the line's text without terminal styling.
    pub fn plain_text(&self) -> String {
        self.0.iter().map(|(t, _)| t.as_str()).collect()
    }

    /// Returns the rendered terminal column width of the line.
    pub fn width(&self) -> usize {
        UnicodeWidthStr::width(self.plain_text().as_str())
    }
}

#[derive(Debug, Clone)]
struct Run {
    text: String,
    style: Style,
}

#[derive(Debug, Clone)]
enum Block {
    Heading(u8, Vec<Run>),
    Paragraph(Vec<Run>),
    List {
        ordered: bool,
        start: u64,
        items: Vec<Vec<Block>>,
    },
    BlockQuote(Vec<Block>),
    CodeBlock(Vec<String>),
    Table {
        headers: Vec<Vec<Run>>,
        rows: Vec<Vec<Vec<Run>>>,
    },
    Rule,
}

/// Renders Markdown into styled lines constrained to a fixed terminal width.
///
/// A panic in the renderer falls back to plain wrapped text so malformed or
/// unexpected Markdown cannot terminate the TUI.
pub fn render_markdown_to_lines(markdown: &str, width: usize) -> Vec<StyledLine> {
    let trimmed = markdown.trim();
    if trimmed.is_empty() {
        return vec![];
    }
    let width = width.max(1);

    let result = std::panic::catch_unwind(|| render_inner(trimmed, width));
    match result {
        Ok(lines) => lines,
        Err(_) => plain_wrap(trimmed, width),
    }
}

fn render_inner(trimmed: &str, width: usize) -> Vec<StyledLine> {
    let options = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let parser = Parser::new_ext(trimmed, options);
    let mut iter = parser.peekable();
    let mut ctx = InlineCtx::default();
    let blocks = parse_blocks_until(&mut iter, &mut ctx, None);
    let mut lines = render_blocks(&blocks, width, 0);
    for line in lines.iter_mut() {
        clip_line_to_width(line, width);
    }
    lines
}

fn plain_wrap(text: &str, width: usize) -> Vec<StyledLine> {
    crate::app::wrap_text::wrap_text(text, width)
        .into_iter()
        .map(|line| StyledLine(vec![(line, Style::default())]))
        .collect()
}

#[derive(Default)]
struct InlineCtx {
    style_stack: Vec<Style>,
    link_urls: Vec<String>,
    in_image: bool,
    image_has_text: bool,
}

impl InlineCtx {
    fn effective_style(&self) -> Style {
        self.style_stack
            .iter()
            .fold(Style::default(), |acc, s| acc.patch(*s))
    }
}

/// Common GitHub-style `:shortcode:` emoji names used by CI/PR bots (Snyk,
/// GitHub Actions, Dependabot, CodeQL, etc). Only these are recognized, so
/// unrelated colon-delimited text (times, ratios, code) is left untouched.
const KNOWN_EMOJI_SHORTCODES: &[&str] = &[
    "white_check_mark",
    "heavy_check_mark",
    "x",
    "negative_squared_cross_mark",
    "warning",
    "exclamation",
    "heavy_exclamation_mark",
    "grey_exclamation",
    "question",
    "grey_question",
    "rocket",
    "tada",
    "sparkles",
    "bug",
    "memo",
    "pencil2",
    "computer",
    "gear",
    "wrench",
    "hammer",
    "construction",
    "fire",
    "boom",
    "chart_with_upwards_trend",
    "chart_with_downwards_trend",
    "lock",
    "unlock",
    "closed_lock_with_key",
    "key",
    "shield",
    "no_entry",
    "no_entry_sign",
    "red_circle",
    "green_circle",
    "yellow_circle",
    "large_blue_circle",
    "white_circle",
    "black_circle",
    "checkered_flag",
    "hourglass",
    "hourglass_flowing_sand",
    "arrow_up",
    "arrow_down",
    "arrow_right",
    "information_source",
    "bulb",
    "package",
    "robot",
    "zap",
    "thumbsup",
    "thumbsdown",
    "eyes",
    "100",
    "raised_hands",
    "clap",
];

/// Strips recognized GitHub-style `:shortcode:` emoji markers.
///
/// Emoji presentation width varies between terminals, so removing known
/// decorative markers keeps wrapping and table alignment predictable.
fn strip_emoji_shortcodes(text: &str) -> String {
    if !text.contains(':') {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == ':' {
            if let Some(end) = find_shortcode_end(&chars, i + 1) {
                let name: String = chars[i + 1..end].iter().collect();
                if KNOWN_EMOJI_SHORTCODES.contains(&name.as_str()) {
                    i = end + 1;
                    continue;
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// Given the index just after an opening `:`, returns the index of the
/// closing `:` if the run of characters in between looks like a shortcode
/// (letters, digits, `_`, `+`, `-`, non-empty).
fn find_shortcode_end(chars: &[char], start: usize) -> Option<usize> {
    let mut j = start;
    while j < chars.len() {
        let c = chars[j];
        if c == ':' {
            return if j > start { Some(j) } else { None };
        }
        if !(c.is_ascii_alphanumeric() || c == '_' || c == '+' || c == '-') {
            return None;
        }
        j += 1;
    }
    None
}

fn heading_level_to_u8(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn parse_blocks_until<'a, I: Iterator<Item = Event<'a>>>(
    iter: &mut Peekable<I>,
    ctx: &mut InlineCtx,
    end: Option<TagEnd>,
) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut pending: Vec<Run> = Vec::new();

    while let Some(event) = iter.next() {
        if let Event::End(e) = &event {
            if Some(*e) == end {
                if !pending.is_empty() {
                    blocks.push(Block::Paragraph(std::mem::take(&mut pending)));
                }
                break;
            }
        }
        match event {
            Event::Start(Tag::Paragraph) => {
                let runs = parse_inline_until(iter, ctx, TagEnd::Paragraph);
                blocks.push(Block::Paragraph(runs));
            }
            Event::Start(Tag::Heading { level, .. }) => {
                if !pending.is_empty() {
                    blocks.push(Block::Paragraph(std::mem::take(&mut pending)));
                }
                let runs = parse_inline_until(iter, ctx, TagEnd::Heading(level));
                blocks.push(Block::Heading(heading_level_to_u8(level), runs));
            }
            Event::Start(Tag::List(start)) => {
                if !pending.is_empty() {
                    blocks.push(Block::Paragraph(std::mem::take(&mut pending)));
                }
                blocks.push(parse_list(iter, ctx, start));
            }
            Event::Start(Tag::BlockQuote(_)) => {
                if !pending.is_empty() {
                    blocks.push(Block::Paragraph(std::mem::take(&mut pending)));
                }
                let inner = parse_blocks_until(iter, ctx, Some(TagEnd::BlockQuote(None)));
                blocks.push(Block::BlockQuote(inner));
            }
            Event::Start(Tag::CodeBlock(_)) => {
                if !pending.is_empty() {
                    blocks.push(Block::Paragraph(std::mem::take(&mut pending)));
                }
                blocks.push(parse_code_block(iter));
            }
            Event::Start(Tag::Table(aligns)) => {
                if !pending.is_empty() {
                    blocks.push(Block::Paragraph(std::mem::take(&mut pending)));
                }
                blocks.push(parse_table(iter, ctx, aligns.len()));
            }
            Event::Rule => {
                if !pending.is_empty() {
                    blocks.push(Block::Paragraph(std::mem::take(&mut pending)));
                }
                blocks.push(Block::Rule);
            }
            other => handle_inline_event(other, ctx, &mut pending),
        }
    }

    blocks
}

fn parse_inline_until<'a, I: Iterator<Item = Event<'a>>>(
    iter: &mut Peekable<I>,
    ctx: &mut InlineCtx,
    end: TagEnd,
) -> Vec<Run> {
    let mut runs = Vec::new();
    while let Some(event) = iter.next() {
        if let Event::End(e) = &event {
            if *e == end {
                break;
            }
        }
        handle_inline_event(event, ctx, &mut runs);
    }
    runs
}

fn handle_inline_event<'a>(event: Event<'a>, ctx: &mut InlineCtx, pending: &mut Vec<Run>) {
    match event {
        Event::Start(Tag::Emphasis) => ctx
            .style_stack
            .push(Style::default().add_modifier(Modifier::ITALIC)),
        Event::End(TagEnd::Emphasis) => {
            ctx.style_stack.pop();
        }
        Event::Start(Tag::Strong) => ctx
            .style_stack
            .push(Style::default().add_modifier(Modifier::BOLD)),
        Event::End(TagEnd::Strong) => {
            ctx.style_stack.pop();
        }
        Event::Start(Tag::Strikethrough) => ctx
            .style_stack
            .push(Style::default().add_modifier(Modifier::CROSSED_OUT)),
        Event::End(TagEnd::Strikethrough) => {
            ctx.style_stack.pop();
        }
        Event::Start(Tag::Link { dest_url, .. }) => {
            ctx.link_urls.push(dest_url.to_string());
            ctx.style_stack
                .push(Style::default().fg(Color::Blue).add_modifier(Modifier::UNDERLINED));
        }
        Event::End(TagEnd::Link) => {
            ctx.style_stack.pop();
            if let Some(url) = ctx.link_urls.pop() {
                pending.push(Run {
                    text: format!(" ({url})"),
                    style: Style::default().add_modifier(Modifier::DIM),
                });
            }
        }
        Event::Start(Tag::Image { dest_url, .. }) => {
            ctx.link_urls.push(dest_url.to_string());
            ctx.style_stack.push(Style::default().fg(Color::Magenta));
            ctx.in_image = true;
            ctx.image_has_text = false;
        }
        Event::End(TagEnd::Image) => {
            ctx.style_stack.pop();
            ctx.in_image = false;
            let had_text = std::mem::take(&mut ctx.image_has_text);
            if let Some(url) = ctx.link_urls.pop() {
                // Badge/decorative images (no alt text) are dropped rather than
                // showing the raw URL, which is mostly noise in a terminal and
                // was blowing out table column widths for icon-only headers.
                if had_text {
                    pending.push(Run {
                        text: format!(" ({url})"),
                        style: Style::default().add_modifier(Modifier::DIM),
                    });
                }
            }
        }
        Event::Text(t) => {
            let text = strip_emoji_shortcodes(&t);
            if text.is_empty() {
                return;
            }
            if ctx.in_image {
                ctx.image_has_text = true;
            }
            pending.push(Run {
                text,
                style: ctx.effective_style(),
            });
        }
        Event::Code(t) => pending.push(Run {
            text: t.to_string(),
            style: Style::default().fg(Color::Yellow),
        }),
        Event::SoftBreak | Event::HardBreak => pending.push(Run {
            text: " ".to_string(),
            style: Style::default(),
        }),
        Event::TaskListMarker(checked) => pending.push(Run {
            text: if checked { "[x] ".to_string() } else { "[ ] ".to_string() },
            style: Style::default(),
        }),
        _ => {}
    }
}

fn parse_list<'a, I: Iterator<Item = Event<'a>>>(
    iter: &mut Peekable<I>,
    ctx: &mut InlineCtx,
    start: Option<u64>,
) -> Block {
    let ordered = start.is_some();
    let start_num = start.unwrap_or(1);
    let mut items = Vec::new();
    loop {
        match iter.next() {
            Some(Event::Start(Tag::Item)) => {
                let item_blocks = parse_blocks_until(iter, ctx, Some(TagEnd::Item));
                items.push(item_blocks);
            }
            Some(Event::End(TagEnd::List(_))) | None => break,
            _ => {}
        }
    }
    Block::List {
        ordered,
        start: start_num,
        items,
    }
}

fn parse_code_block<'a, I: Iterator<Item = Event<'a>>>(iter: &mut Peekable<I>) -> Block {
    let mut text = String::new();
    loop {
        match iter.next() {
            Some(Event::Text(t)) => text.push_str(&t),
            Some(Event::End(TagEnd::CodeBlock)) | None => break,
            _ => {}
        }
    }
    let lines: Vec<String> = text
        .trim_end_matches('\n')
        .split('\n')
        .map(|s| s.to_string())
        .collect();
    Block::CodeBlock(lines)
}

fn parse_table<'a, I: Iterator<Item = Event<'a>>>(
    iter: &mut Peekable<I>,
    ctx: &mut InlineCtx,
    _ncols: usize,
) -> Block {
    let mut headers: Vec<Vec<Run>> = Vec::new();
    let mut rows: Vec<Vec<Vec<Run>>> = Vec::new();
    let mut current_row: Vec<Vec<Run>> = Vec::new();

    loop {
        match iter.next() {
            Some(Event::Start(Tag::TableHead)) => {}
            Some(Event::End(TagEnd::TableHead)) => {
                headers = std::mem::take(&mut current_row);
            }
            Some(Event::Start(Tag::TableRow)) => {
                current_row = Vec::new();
            }
            Some(Event::End(TagEnd::TableRow)) => {
                rows.push(std::mem::take(&mut current_row));
            }
            Some(Event::Start(Tag::TableCell)) => {
                let cell_runs = parse_inline_until(iter, ctx, TagEnd::TableCell);
                current_row.push(cell_runs);
            }
            Some(Event::End(TagEnd::Table)) | None => break,
            _ => {}
        }
    }

    Block::Table { headers, rows }
}

fn normalize_whitespace(chars: Vec<(char, Style)>) -> Vec<(char, Style)> {
    let mut out = Vec::new();
    let mut last_was_space = false;
    for (ch, style) in chars {
        if ch.is_whitespace() {
            if !last_was_space {
                out.push((' ', style));
            }
            last_was_space = true;
        } else {
            out.push((ch, style));
            last_was_space = false;
        }
    }
    while out.first().map(|(c, _)| *c == ' ').unwrap_or(false) {
        out.remove(0);
    }
    while out.last().map(|(c, _)| *c == ' ').unwrap_or(false) {
        out.pop();
    }
    out
}

fn flatten_runs(runs: &[Run]) -> Vec<(char, Style)> {
    let mut out = Vec::new();
    for run in runs {
        for ch in run.text.chars() {
            out.push((ch, run.style));
        }
    }
    out
}

fn flatten_plain(runs: &[Run]) -> String {
    normalize_whitespace(flatten_runs(runs))
        .into_iter()
        .map(|(c, _)| c)
        .collect()
}

fn split_words(chars: &[(char, Style)]) -> Vec<Vec<(char, Style)>> {
    chars
        .split(|(c, _)| *c == ' ')
        .filter(|w| !w.is_empty())
        .map(|w| w.to_vec())
        .collect()
}

fn char_width(c: char) -> usize {
    UnicodeWidthChar::width(c).unwrap_or(0)
}

fn word_width(word: &[(char, Style)]) -> usize {
    word.iter().map(|(c, _)| char_width(*c)).sum()
}

fn coalesce_line(line_chars: Vec<(char, Style)>) -> Vec<(String, Style)> {
    let mut out: Vec<(String, Style)> = Vec::new();
    for (ch, style) in line_chars {
        if let Some(last) = out.last_mut() {
            if last.1 == style {
                last.0.push(ch);
                continue;
            }
        }
        out.push((ch.to_string(), style));
    }
    out
}

fn flush_line(
    current: &mut Vec<(char, Style)>,
    lines: &mut Vec<StyledLine>,
    first_line: &mut bool,
    indent: usize,
    first_prefix: Option<(&str, Style)>,
) {
    let mut spans: Vec<(String, Style)> = Vec::new();
    if *first_line {
        if let Some((p, s)) = first_prefix {
            spans.push((p.to_string(), s));
            let pad = indent.saturating_sub(UnicodeWidthStr::width(p));
            if pad > 0 {
                spans.push((" ".repeat(pad), Style::default()));
            }
        } else if indent > 0 {
            spans.push((" ".repeat(indent), Style::default()));
        }
    } else if indent > 0 {
        spans.push((" ".repeat(indent), Style::default()));
    }
    spans.extend(coalesce_line(std::mem::take(current)));
    lines.push(StyledLine(spans));
    *first_line = false;
}

fn wrap_words(
    words: Vec<Vec<(char, Style)>>,
    width: usize,
    indent: usize,
    first_prefix: Option<(&str, Style)>,
) -> Vec<StyledLine> {
    let avail = width.saturating_sub(indent).max(1);
    let mut lines = Vec::new();
    let mut current: Vec<(char, Style)> = Vec::new();
    let mut current_width = 0usize;
    let mut first_line = true;

    for word in words {
        let w = word_width(&word);
        if current_width > 0 && current_width + 1 + w > avail {
            flush_line(&mut current, &mut lines, &mut first_line, indent, first_prefix);
            current_width = 0;
        }
        if w > avail {
            for ch in word {
                if current_width >= avail {
                    flush_line(&mut current, &mut lines, &mut first_line, indent, first_prefix);
                    current_width = 0;
                }
                current_width += char_width(ch.0);
                current.push(ch);
            }
            continue;
        }
        if current_width > 0 {
            current.push((' ', Style::default()));
            current_width += 1;
        }
        current.extend(word);
        current_width += w;
    }

    if !current.is_empty() || lines.is_empty() {
        flush_line(&mut current, &mut lines, &mut first_line, indent, first_prefix);
    }

    lines
}

fn wrap_runs(runs: &[Run], width: usize, indent: usize) -> Vec<StyledLine> {
    let flat = normalize_whitespace(flatten_runs(runs));
    let words = split_words(&flat);
    wrap_words(words, width, indent, None)
}

fn render_list_item(
    item_blocks: &[Block],
    width: usize,
    item_indent: usize,
    marker: &str,
) -> Vec<StyledLine> {
    let mut lines = render_blocks(item_blocks, width, item_indent);
    let marker_style = Style::default();
    match lines.first_mut() {
        Some(first) if !first.0.is_empty() => {
            let first_span = &mut first.0[0];
            if first_span.0.chars().all(|c| c == ' ')
                && UnicodeWidthStr::width(first_span.0.as_str()) == item_indent
            {
                let pad = item_indent.saturating_sub(UnicodeWidthStr::width(marker));
                first_span.0 = format!("{marker}{}", " ".repeat(pad));
                first_span.1 = marker_style;
            } else {
                first.0.insert(0, (marker.to_string(), marker_style));
            }
        }
        Some(first) => {
            first.0.push((marker.to_string(), marker_style));
        }
        None => {
            lines.push(StyledLine(vec![(marker.to_string(), marker_style)]));
        }
    }
    lines
}

fn dim_style() -> Style {
    Style::default().add_modifier(Modifier::DIM)
}

fn truncate_to_width(text: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    if UnicodeWidthStr::width(text) <= width {
        return text.to_string();
    }
    let mut out = String::new();
    let mut w = 0usize;
    for ch in text.chars() {
        let cw = char_width(ch);
        if w + cw > width.saturating_sub(1) {
            break;
        }
        w += cw;
        out.push(ch);
    }
    out.push('…');
    out
}

fn clip_line_to_width(line: &mut StyledLine, width: usize) {
    if line.width() <= width {
        return;
    }
    let mut remaining = width.saturating_sub(1);
    let mut new_spans: Vec<(String, Style)> = Vec::new();
    'outer: for (text, style) in line.0.drain(..) {
        let mut buf = String::new();
        for ch in text.chars() {
            let cw = char_width(ch);
            if remaining < cw {
                if !buf.is_empty() {
                    new_spans.push((buf, style));
                }
                new_spans.push(("…".to_string(), style));
                break 'outer;
            }
            remaining -= cw;
            buf.push(ch);
        }
        if !buf.is_empty() {
            new_spans.push((buf, style));
        }
    }
    line.0 = new_spans;
}

fn border_line(left: char, mid: char, right: char, widths: &[usize]) -> String {
    let mut s = String::new();
    s.push(left);
    for (i, w) in widths.iter().enumerate() {
        s.push_str(&"─".repeat(w + 2));
        s.push(if i + 1 == widths.len() { right } else { mid });
    }
    s
}

fn render_table_row(cells: &[String], widths: &[usize], is_header: bool) -> StyledLine {
    let mut spans = vec![("│".to_string(), dim_style())];
    for (i, w) in widths.iter().enumerate() {
        let cell = cells.get(i).cloned().unwrap_or_default();
        let truncated = truncate_to_width(&cell, *w);
        let pad = w.saturating_sub(UnicodeWidthStr::width(truncated.as_str()));
        let padded = format!(" {}{} ", truncated, " ".repeat(pad));
        let style = if is_header {
            Style::default().add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        spans.push((padded, style));
        spans.push(("│".to_string(), dim_style()));
    }
    StyledLine(spans)
}

fn render_table(headers: &[Vec<Run>], rows: &[Vec<Vec<Run>>], width: usize) -> Vec<StyledLine> {
    let ncols = headers
        .len()
        .max(rows.iter().map(|r| r.len()).max().unwrap_or(0));
    if ncols == 0 {
        return vec![];
    }

    let header_text: Vec<String> = (0..ncols)
        .map(|i| headers.get(i).map(|c| flatten_plain(c)).unwrap_or_default())
        .collect();
    let row_texts: Vec<Vec<String>> = rows
        .iter()
        .map(|row| {
            (0..ncols)
                .map(|i| row.get(i).map(|c| flatten_plain(c)).unwrap_or_default())
                .collect()
        })
        .collect();

    let mut col_widths: Vec<usize> = (0..ncols)
        .map(|i| {
            let mut w = UnicodeWidthStr::width(header_text[i].as_str());
            for row in &row_texts {
                w = w.max(UnicodeWidthStr::width(row[i].as_str()));
            }
            w.clamp(3, 40)
        })
        .collect();

    let overhead = ncols * 3 + 1;
    let natural_total: usize = col_widths.iter().sum();
    if natural_total + overhead > width && width > overhead {
        let available = (width - overhead).max(ncols * 3);
        for w in col_widths.iter_mut() {
            let scaled = ((*w as f64 / natural_total as f64) * available as f64).floor() as usize;
            *w = scaled.max(3);
        }
    }

    let mut lines = Vec::new();
    lines.push(StyledLine(vec![(
        border_line('┌', '┬', '┐', &col_widths),
        dim_style(),
    )]));
    lines.push(render_table_row(&header_text, &col_widths, true));
    lines.push(StyledLine(vec![(
        border_line('├', '┼', '┤', &col_widths),
        dim_style(),
    )]));
    if row_texts.is_empty() {
        lines.push(StyledLine(vec![(
            border_line('└', '┴', '┘', &col_widths),
            dim_style(),
        )]));
    } else {
        for (i, row) in row_texts.iter().enumerate() {
            lines.push(render_table_row(row, &col_widths, false));
            if i + 1 == row_texts.len() {
                lines.push(StyledLine(vec![(
                    border_line('└', '┴', '┘', &col_widths),
                    dim_style(),
                )]));
            }
        }
    }
    lines
}

fn render_blocks(blocks: &[Block], width: usize, indent: usize) -> Vec<StyledLine> {
    let mut out: Vec<StyledLine> = Vec::new();
    for (i, block) in blocks.iter().enumerate() {
        if i > 0 {
            out.push(StyledLine::default());
        }
        match block {
            Block::Heading(level, runs) => {
                let style = if *level == 1 {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
                } else {
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                };
                let flat = normalize_whitespace(flatten_runs(runs));
                let words = split_words(&flat);
                let styled_words: Vec<Vec<(char, Style)>> = words
                    .into_iter()
                    .map(|w| w.into_iter().map(|(c, _)| (c, style)).collect())
                    .collect();
                out.extend(wrap_words(styled_words, width, indent, None));
            }
            Block::Paragraph(runs) => {
                out.extend(wrap_runs(runs, width, indent));
            }
            Block::List { ordered, start, items } => {
                for (idx, item_blocks) in items.iter().enumerate() {
                    let marker = if *ordered {
                        format!("{}. ", start + idx as u64)
                    } else {
                        "• ".to_string()
                    };
                    let item_indent = indent + UnicodeWidthStr::width(marker.as_str());
                    out.extend(render_list_item(item_blocks, width, item_indent, &marker));
                }
            }
            Block::BlockQuote(inner) => {
                let inner_lines = render_blocks(inner, width.saturating_sub(2), 0);
                for line in inner_lines {
                    let mut spans = vec![("│ ".to_string(), dim_style())];
                    spans.extend(line.0);
                    out.push(StyledLine(spans));
                }
            }
            Block::CodeBlock(code_lines) => {
                for line in code_lines {
                    let truncated = truncate_to_width(line, width.saturating_sub(indent));
                    let mut spans = Vec::new();
                    if indent > 0 {
                        spans.push((" ".repeat(indent), Style::default()));
                    }
                    spans.push((truncated, Style::default().fg(Color::Yellow)));
                    out.push(StyledLine(spans));
                }
            }
            Block::Table { headers, rows } => {
                out.extend(render_table(headers, rows, width.saturating_sub(indent)));
            }
            Block::Rule => {
                out.push(StyledLine(vec![(
                    "─".repeat(width.saturating_sub(indent)),
                    dim_style(),
                )]));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(lines: &[StyledLine]) -> Vec<String> {
        lines.iter().map(|l| l.plain_text()).collect()
    }

    #[test]
    fn renders_plain_paragraph() {
        let lines = render_markdown_to_lines("Hello world", 80);
        assert_eq!(plain(&lines), vec!["Hello world"]);
    }

    #[test]
    fn wraps_long_paragraph_to_width() {
        let lines = render_markdown_to_lines("one two three four five", 11);
        assert_eq!(plain(&lines), vec!["one two", "three four", "five"]);
    }

    #[test]
    fn renders_heading_bold_cyan() {
        let lines = render_markdown_to_lines("# Title", 80);
        assert_eq!(plain(&lines), vec!["Title"]);
        assert!(lines[0].0[0].1.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn renders_bullet_list() {
        let lines = render_markdown_to_lines("- one\n- two", 80);
        assert_eq!(plain(&lines), vec!["• one", "• two"]);
    }

    #[test]
    fn renders_ordered_list() {
        let lines = render_markdown_to_lines("1. one\n2. two", 80);
        assert_eq!(plain(&lines), vec!["1. one", "2. two"]);
    }

    #[test]
    fn renders_bold_and_italic_spans() {
        let lines = render_markdown_to_lines("**bold** and *italic*", 80);
        assert_eq!(plain(&lines), vec!["bold and italic"]);
        let bold_span = &lines[0].0[0];
        assert_eq!(bold_span.0, "bold");
        assert!(bold_span.1.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn renders_link_as_text_and_url() {
        let lines = render_markdown_to_lines("[click here](https://example.com)", 80);
        assert_eq!(plain(&lines), vec!["click here (https://example.com)"]);
    }

    #[test]
    fn renders_table() {
        let lines = render_markdown_to_lines("| a | b |\n|---|---|\n| 1 | 2 |", 80);
        let text = plain(&lines).join("\n");
        assert!(text.contains('┌'));
        assert!(text.contains('a'));
        assert!(text.contains('1'));
    }

    #[test]
    fn empty_markdown_renders_no_lines() {
        assert_eq!(render_markdown_to_lines("   ", 80).len(), 0);
    }
}
