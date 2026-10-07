//! Turns painted cells back into one `Line` per source line.

use std::ops::Range;

use ratatui::{
    style::Style,
    text::{Line, Span},
};
use unicode_width::UnicodeWidthStr;

use super::{Kind, Link, Painter};
use crate::{Mode, code};

pub fn build<'a>(
    painter: &Painter<'a, '_>,
    mode: Mode,
    width: Option<u16>,
) -> (Vec<Line<'a>>, Vec<Link>) {
    let mut links = Vec::new();
    let lines = painter
        .line_ranges
        .iter()
        .enumerate()
        .map(|(index, bounds)| {
            let mut runs = Runs::new(index);
            let line = build_line(painter, bounds.clone(), mode, &mut runs)
                // A cut off a char boundary would be a bug in the painter; show the line plainly
                // rather than panic.
                .unwrap_or_else(|| Line::raw(painter.src.get(bounds.clone()).unwrap_or("")));
            let in_code = painter
                .code_blocks
                .iter()
                .any(|block| bounds.start < block.end && block.start <= bounds.end);
            runs.finish(painter, &mut links);
            if in_code {
                code::fill_background(line, painter.theme, width)
            } else {
                line
            }
        })
        .collect();
    (lines, links)
}

/// The links on one output line, collected while its text is emitted.
struct Runs {
    line: usize,
    /// Display width emitted so far.
    col: usize,
    /// Link index and columns of the run being extended.
    open: Option<(usize, Range<usize>)>,
    done: Vec<(usize, Range<usize>)>,
}

impl Runs {
    fn new(line: usize) -> Self {
        Self {
            line,
            col: 0,
            open: None,
            done: Vec::new(),
        }
    }

    /// `text` was emitted for source byte `i`. Whitespace never starts or ends a run, so the
    /// indentation of a link's continuation line and its trailing spaces are left out.
    fn emit(&mut self, link: Option<usize>, text: &str, leading: bool) {
        let start = self.col;
        self.col += text.width();
        if leading || text.trim().is_empty() {
            if link.is_none() || leading {
                self.close();
            }
            return;
        }
        match (&mut self.open, link) {
            (Some((open, columns)), Some(link)) if *open == link => columns.end = self.col,
            _ => {
                self.close();
                self.open = link.map(|link| (link, start..self.col));
            }
        }
    }

    fn close(&mut self) {
        if let Some(run) = self.open.take() {
            self.done.push(run);
        }
    }

    fn finish(mut self, painter: &Painter, links: &mut Vec<Link>) {
        self.close();
        links.extend(self.done.into_iter().map(|(link, columns)| Link {
            line: self.line,
            columns,
            url: painter.links[link].1.clone(),
        }));
    }
}

enum Out {
    Source(Style),
    Glyph(usize, Style),
    Hide,
}

fn build_line<'a>(
    painter: &Painter<'a, '_>,
    bounds: Range<usize>,
    mode: Mode,
    runs: &mut Runs,
) -> Option<Line<'a>> {
    let src = painter.src;
    let mut spans: Vec<Span<'a>> = Vec::new();
    let mut run: Option<(Range<usize>, Style)> = None;
    let mut leading = true;

    let flush = |run: &mut Option<(Range<usize>, Style)>, spans: &mut Vec<Span<'a>>| {
        if let Some((range, style)) = run.take() {
            spans.push(Span::styled(src.get(range)?, style));
        }
        Some(())
    };

    for i in bounds {
        let cell = painter.cells[i];
        let byte = src.as_bytes()[i];
        let whitespace = matches!(byte, b' ' | b'\t');
        leading &= whitespace;

        let out = match (mode, cell.kind) {
            // A lone `\r`, NUL or escape would reach the terminal raw.
            _ if byte.is_ascii_control() && byte != b'\t' => Out::Hide,
            (Mode::Raw, Kind::Marker) => Out::Source(cell.style.patch(painter.theme.marker)),
            (Mode::Raw, _) | (Mode::Concealed, Kind::Show) => Out::Source(cell.style),
            // Indentation is kept so nested lists and code in list items stay aligned.
            (Mode::Concealed, Kind::Marker) if leading => Out::Source(cell.style),
            (Mode::Concealed, Kind::Marker | Kind::Skip) => Out::Hide,
            (Mode::Concealed, Kind::Replace(index)) => Out::Glyph(index, cell.style),
        };

        match &out {
            // A multi-byte char counts once, at its first byte.
            Out::Source(_) => {
                if let Some(c) = src.get(i..).and_then(|rest| rest.chars().next()) {
                    let mut buf = [0; 4];
                    runs.emit(painter.link_at(i), c.encode_utf8(&mut buf), leading);
                }
            }
            Out::Glyph(index, _) => {
                let glyph = painter.glyphs.get(*index).map_or("", |g| g.as_ref());
                runs.emit(painter.link_at(i), glyph, leading);
            }
            Out::Hide => {}
        }

        match out {
            Out::Source(style) => match &mut run {
                Some((range, run_style)) if *run_style == style && range.end == i => {
                    range.end = i + 1
                }
                _ => {
                    flush(&mut run, &mut spans)?;
                    run = Some((i..i + 1, style));
                }
            },
            Out::Glyph(index, style) => {
                flush(&mut run, &mut spans)?;
                let glyph = painter.glyphs.get(index).cloned().unwrap_or_default();
                spans.push(Span::styled(glyph, style));
            }
            Out::Hide => flush(&mut run, &mut spans)?,
        }
    }
    flush(&mut run, &mut spans)?;
    Some(Line::from(spans))
}
