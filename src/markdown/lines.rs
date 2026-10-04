//! Turns painted cells back into one `Line` per source line.

use std::ops::Range;

use ratatui::{
    style::Style,
    text::{Line, Span},
};

use super::{Kind, Painter};
use crate::{Mode, code};

pub fn build<'a>(painter: &Painter<'a, '_>, mode: Mode, width: Option<u16>) -> Vec<Line<'a>> {
    painter
        .line_ranges
        .iter()
        .map(|bounds| {
            let line = build_line(painter, bounds.clone(), mode)
                // A cut off a char boundary would be a bug in the painter; show the line plainly
                // rather than panic.
                .unwrap_or_else(|| Line::raw(painter.src.get(bounds.clone()).unwrap_or("")));
            let in_code = painter
                .code_blocks
                .iter()
                .any(|block| bounds.start < block.end && block.start <= bounds.end);
            if in_code {
                code::fill_background(line, painter.theme, width)
            } else {
                line
            }
        })
        .collect()
}

enum Out {
    Source(Style),
    Glyph(usize, Style),
    Hide,
}

fn build_line<'a>(painter: &Painter<'a, '_>, bounds: Range<usize>, mode: Mode) -> Option<Line<'a>> {
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
