//! Splitting source text into lines with the same rules as [`str::lines`], so every renderer
//! returns exactly one `Line` per source line.

use ratatui::{style::Style, text::Span};

/// Lines including their `\n`. A trailing newline does not start an extra line.
pub fn split_inclusive(source: &str) -> impl Iterator<Item = &str> {
    source.split_inclusive('\n')
}

/// Byte ranges of each line, excluding the terminator (`\n` or `\r\n`).
pub fn ranges(source: &str) -> Vec<std::ops::Range<usize>> {
    let mut start = 0;
    split_inclusive(source)
        .map(|line| {
            let range = start..start + trim_terminator(line).len();
            start += line.len();
            range
        })
        .collect()
}

/// Splits `text` into spans around control characters (a lone `\r`, NUL, escape), which would
/// otherwise reach the terminal raw. Tabs are kept.
pub fn visible_spans(text: &str, style: Style) -> impl Iterator<Item = Span<'_>> {
    text.split(is_control)
        .filter(|part| !part.is_empty())
        .map(move |part| Span::styled(part, style))
}

pub fn is_control(c: char) -> bool {
    c.is_control() && c != '\t'
}

pub fn trim_terminator(line: &str) -> &str {
    let line = line.strip_suffix('\n').unwrap_or(line);
    line.strip_suffix('\r').unwrap_or(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_str_lines() {
        for source in ["", "a", "a\n", "a\nb", "a\r\nb\r\n", "\n\n", "a\rb\n"] {
            let ours: Vec<&str> = ranges(source).into_iter().map(|r| &source[r]).collect();
            let std: Vec<&str> = source.lines().collect();
            assert_eq!(ours, std, "{source:?}");
        }
    }
}
