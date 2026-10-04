//! Syntax highlighting with syntect and a Dracula theme, shared by markdown code blocks and
//! [`crate::Renderer::render_code`].

pub mod dracula;

use std::{fmt, ops::Range, sync::OnceLock};

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use syntect::{
    easy::HighlightLines,
    highlighting::{self as hl, FontStyle},
    parsing::{SyntaxReference, SyntaxSet},
};
use unicode_width::UnicodeWidthStr;

use crate::{source, theme::Theme};

/// Byte range within a line and the token style for it (foreground and modifiers only).
pub type Token = (Range<usize>, Style);

pub struct Highlighter {
    syntaxes: SyntaxSet,
    theme: hl::Theme,
}

impl fmt::Debug for Highlighter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Highlighter").finish_non_exhaustive()
    }
}

impl Highlighter {
    /// Loading the bundled syntaxes takes a few milliseconds, so every renderer shares one.
    pub fn shared() -> &'static Highlighter {
        static SHARED: OnceLock<Highlighter> = OnceLock::new();
        SHARED.get_or_init(|| Highlighter {
            syntaxes: SyntaxSet::load_defaults_newlines(),
            theme: dracula::theme(),
        })
    }

    /// Resolves a fenced-code info string (`rust`, `rust,ignore`, `{.python}`), a language name,
    /// an extension or a file name. With `sniff`, falls back to the first line (shebangs, `<?php`).
    fn syntax(&self, lang: &str, first_line: Option<&str>) -> Option<&SyntaxReference> {
        let token = lang
            .split(|c: char| c.is_whitespace() || c == ',')
            .next()
            .unwrap_or("")
            .trim_matches(|c| c == '{' || c == '}' || c == '.');

        let by_token = (!token.is_empty())
            .then(|| self.syntaxes.find_syntax_by_token(token))
            .flatten();
        let by_extension = || {
            let (_, extension) = token.rsplit_once('.')?;
            self.syntaxes.find_syntax_by_token(extension)
        };
        let by_first_line = || self.syntaxes.find_syntax_by_first_line(first_line?);

        by_token.or_else(by_extension).or_else(by_first_line)
    }

    /// Highlights `lines` (each including its line terminator, if any) as one block, so multi-line
    /// constructs like block comments carry over. Unknown languages yield no tokens.
    pub fn highlight(&self, lang: &str, lines: &[&str], sniff: bool) -> Vec<Vec<Token>> {
        let first_line = sniff.then(|| lines.first().copied()).flatten();
        let Some(syntax) = self.syntax(lang, first_line) else {
            return vec![Vec::new(); lines.len()];
        };

        let mut highlighter = HighlightLines::new(syntax, &self.theme);
        let mut failed = false;
        lines
            .iter()
            .map(|line| {
                if failed {
                    return Vec::new();
                }
                match highlighter.highlight_line(line, &self.syntaxes) {
                    Ok(regions) => self.tokens(&regions),
                    // fancy-regex can give up on pathological lines (backtrack limit); the rest
                    // of the block renders plain rather than with a desynchronised parse state.
                    Err(_) => {
                        failed = true;
                        Vec::new()
                    }
                }
            })
            .collect()
    }

    fn tokens(&self, regions: &[(hl::Style, &str)]) -> Vec<Token> {
        let mut offset = 0;
        regions
            .iter()
            .map(|(style, text)| {
                let range = offset..offset + text.len();
                offset = range.end;
                (range, self.style(*style))
            })
            .collect()
    }

    fn style(&self, style: hl::Style) -> Style {
        let mut out = Style::new().fg(rgb(style.foreground));
        if Some(style.background) != self.theme.settings.background {
            out = out.bg(rgb(style.background));
        }
        let modifiers = [
            (FontStyle::BOLD, Modifier::BOLD),
            (FontStyle::ITALIC, Modifier::ITALIC),
            (FontStyle::UNDERLINE, Modifier::UNDERLINED),
        ];
        for (font, modifier) in modifiers {
            if style.font_style.contains(font) {
                out = out.add_modifier(modifier);
            }
        }
        out
    }
}

fn rgb(colour: hl::Color) -> Color {
    Color::Rgb(colour.r, colour.g, colour.b)
}

/// Gives a code line its background across the whole row.
///
/// `Paragraph` only paints a `Line`'s style under its graphemes, and empty lines get no
/// background at all, so the row is padded with spaces up to `width`. Without a width an empty
/// line still gets one space so the block does not have holes.
pub fn fill_background<'a>(mut line: Line<'a>, theme: &Theme, width: Option<u16>) -> Line<'a> {
    line.style = theme.code_block;
    let used = line
        .spans
        .iter()
        .map(|span| span.content.width())
        .sum::<usize>();
    let pad = match width {
        Some(width) => usize::from(width).saturating_sub(used),
        None => usize::from(used == 0),
    };
    if pad > 0 {
        line.spans
            .push(Span::styled(" ".repeat(pad), theme.code_block));
    }
    line
}

/// Renders a whole source file: one `Line` per line, every line on the code background.
pub fn render<'a>(
    source: &'a str,
    lang: &str,
    theme: &Theme,
    width: Option<u16>,
    highlighter: &Highlighter,
) -> Vec<Line<'a>> {
    let lines: Vec<&str> = source::split_inclusive(source).collect();
    let tokens = highlighter.highlight(lang, &lines, true);

    lines
        .iter()
        .zip(tokens)
        .map(|(line, tokens)| {
            let content = source::trim_terminator(line);
            let spans: Vec<Span> = if tokens.is_empty() {
                source::visible_spans(content, theme.code_block).collect()
            } else {
                tokens
                    .into_iter()
                    .filter_map(|(range, style)| {
                        let end = range.end.min(content.len());
                        content
                            .get(range.start..end)
                            .map(|text| source::visible_spans(text, theme.code_block.patch(style)))
                    })
                    .flatten()
                    .collect()
            };
            fill_background(Line::from(spans), theme, width)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens_for(lang: &str, line: &str) -> Vec<Token> {
        Highlighter::shared()
            .highlight(lang, &[line], false)
            .remove(0)
    }

    #[test]
    fn resolves_info_strings_names_extensions_and_files() {
        let h = Highlighter::shared();
        for lang in [
            "rust",
            "rs",
            "Rust",
            "rust,ignore",
            "rust title=x",
            "{.rust}",
            "main.rs",
        ] {
            assert_eq!(
                h.syntax(lang, None).map(|s| s.name.as_str()),
                Some("Rust"),
                "{lang}"
            );
        }
        assert!(h.syntax("zzz", None).is_none());
        assert!(h.syntax("", None).is_none());
        assert_eq!(
            h.syntax("", Some("#!/usr/bin/env python3\n"))
                .map(|s| s.name.as_str()),
            Some("Python")
        );
    }

    #[test]
    fn keywords_are_dracula_pink() {
        let line = "if ready { go() }\n";
        let tokens = tokens_for("rust", line);
        let (range, style) = &tokens[0];
        assert_eq!(&line[range.clone()], "if");
        assert_eq!(style.fg, Some(Color::Rgb(0xff, 0x79, 0xc6)));
        assert_eq!(style.bg, None);
    }

    #[test]
    fn unknown_language_has_no_tokens() {
        assert!(tokens_for("zzz", "whatever\n").is_empty());
    }

    #[test]
    fn padding_counts_columns_and_saturates() {
        let theme = Theme::default();
        let line = fill_background(Line::from("日本"), &theme, Some(6));
        assert_eq!(line.width(), 6);
        let line = fill_background(Line::from("much too long"), &theme, Some(4));
        assert_eq!(line.width(), 13);
        let line = fill_background(Line::default(), &theme, None);
        assert_eq!(line.width(), 1);
    }
}
