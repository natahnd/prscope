/// Greedily word-wraps text to a fixed column width, preserving explicit
/// newlines (paragraph breaks) as blank lines. Used as the defensive
/// fallback if markdown rendering ever panics.
pub fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let safe_width = width.max(1);
    let mut lines: Vec<String> = Vec::new();

    for paragraph in text.split('\n') {
        let words: Vec<&str> = paragraph.split_whitespace().collect();
        let mut current = String::new();

        for raw_word in words {
            let mut word = raw_word;
            while word.chars().count() > safe_width {
                if !current.is_empty() {
                    lines.push(std::mem::take(&mut current));
                }
                let (head, tail) = split_at_chars(word, safe_width);
                lines.push(head.to_string());
                word = tail;
            }
            if word.is_empty() {
                continue;
            }
            let candidate = if current.is_empty() {
                word.to_string()
            } else {
                format!("{current} {word}")
            };
            if candidate.chars().count() > safe_width {
                lines.push(std::mem::take(&mut current));
                current = word.to_string();
            } else {
                current = candidate;
            }
        }

        lines.push(current);
    }

    lines
}

fn split_at_chars(s: &str, n: usize) -> (&str, &str) {
    match s.char_indices().nth(n) {
        Some((idx, _)) => s.split_at(idx),
        None => (s, ""),
    }
}

pub fn strip_html(content: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for ch in content.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_at_word_boundaries() {
        assert_eq!(wrap_text("one two three", 7), vec!["one two", "three"]);
    }

    #[test]
    fn hard_breaks_overlong_words() {
        assert_eq!(wrap_text("abcdefgh", 3), vec!["abc", "def", "gh"]);
    }

    #[test]
    fn strips_html_tags() {
        assert_eq!(strip_html("<p>Hello <b>world</b></p>"), "Hello world");
    }
}
