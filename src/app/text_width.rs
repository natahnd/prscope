use ratatui::text::{Line, Span};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub fn display_width(text: &str) -> usize {
    UnicodeWidthStr::width(text)
}

pub fn truncate_to_width(text: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    if display_width(text) <= width {
        return text.to_string();
    }
    if width == 1 {
        return "…".to_string();
    }
    let mut out = String::new();
    let mut used = 0usize;
    for ch in text.chars() {
        let cw = UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + cw > width.saturating_sub(1) {
            break;
        }
        used += cw;
        out.push(ch);
    }
    out.push('…');
    out
}

pub fn clip_line_to_width(line: Line<'_>, max_width: usize) -> Line<'_> {
    let current: usize = line
        .spans
        .iter()
        .map(|span| display_width(span.content.as_ref()))
        .sum();
    if current <= max_width {
        return line;
    }
    if max_width == 0 {
        return Line::from("");
    }

    let mut remaining = max_width.saturating_sub(1);
    let mut clipped = Vec::new();

    'outer: for span in line.spans {
        let mut buf = String::new();
        for ch in span.content.chars() {
            let cw = UnicodeWidthChar::width(ch).unwrap_or(0);
            if cw > remaining {
                if !buf.is_empty() {
                    clipped.push(Span::styled(buf, span.style));
                }
                clipped.push(Span::styled("…", span.style));
                break 'outer;
            }
            remaining -= cw;
            buf.push(ch);
        }
        if !buf.is_empty() {
            clipped.push(Span::styled(buf, span.style));
        }
    }

    if clipped.is_empty() {
        Line::from(Span::raw("…"))
    } else {
        Line::from(clipped)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_respects_display_width() {
        assert_eq!(truncate_to_width("hello", 10), "hello");
        assert_eq!(truncate_to_width("hello world", 8), "hello w…");
    }

    #[test]
    fn clip_line_preserves_styles_and_ellipsis() {
        use ratatui::style::{Color, Style};

        let line = Line::from(vec![
            Span::styled("abcdef", Style::default().fg(Color::Red)),
            Span::styled("ghijkl", Style::default().fg(Color::Blue)),
        ]);
        let clipped = clip_line_to_width(line, 8);
        assert_eq!(clipped.width(), 8);
        assert!(clipped
            .spans
            .iter()
            .any(|span| span.content.as_ref().contains('…')));
    }
}
