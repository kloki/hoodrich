//! Line diffs where unchanged lines keep their styling and changed lines are left plain, so the
//! caller can colour additions and removals however it likes.

use ratatui::text::Line;
use similar::{ChangeTag, TextDiff};

use crate::source;

/// How a line differs between the old and the new text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// Present in both; styled.
    Same,
    /// Only in the new text; unstyled.
    Added,
    /// Only in the old text; unstyled.
    Removed,
}

/// One line of a diff.
#[derive(Debug, Clone, PartialEq)]
pub struct DiffLine<'a> {
    pub change: Change,
    pub line: Line<'a>,
}

/// `styled_new` must hold one line per line of `new`, as every renderer in this crate returns.
pub fn merge<'a>(old: &'a str, new: &'a str, styled_new: Vec<Line<'a>>) -> Vec<DiffLine<'a>> {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();
    let mut styled: Vec<Option<Line<'a>>> = styled_new.into_iter().map(Some).collect();
    let plain = |lines: &[&'a str], index: Option<usize>| {
        let text = index.and_then(|i| lines.get(i)).copied().unwrap_or("");
        Line::from(source::visible_spans(text, Default::default()).collect::<Vec<_>>())
    };

    // Diffing our own line slices keeps change indices aligned with `styled_new`. `from_lines`
    // would also split on a lone `\r` and treat a missing final newline as a change.
    TextDiff::configure()
        .diff_slices(&old_lines, &new_lines)
        .iter_all_changes()
        .map(|change| match change.tag() {
            ChangeTag::Equal => DiffLine {
                change: Change::Same,
                line: change
                    .new_index()
                    .and_then(|i| styled.get_mut(i))
                    .and_then(Option::take)
                    .unwrap_or_else(|| plain(&new_lines, change.new_index())),
            },
            ChangeTag::Insert => DiffLine {
                change: Change::Added,
                line: plain(&new_lines, change.new_index()),
            },
            ChangeTag::Delete => DiffLine {
                change: Change::Removed,
                line: plain(&old_lines, change.old_index()),
            },
        })
        .collect()
}
