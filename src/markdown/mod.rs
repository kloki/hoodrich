//! Markdown to styled lines.
//!
//! pulldown-cmark reports a byte range for every event. Each source byte gets a [`Cell`] saying
//! whether it is content or markdown syntax and how to style it: a container's whole range is
//! painted as syntax first, then its children paint their content over it. Whatever is still
//! syntax at the end (`### `, `**`, fences, `](url)`, `> ` on continuation lines) is hidden in
//! concealed mode and dimmed in raw mode. Because the output is built from the source bytes,
//! every source line maps to exactly one output line.

mod lines;

use std::{borrow::Cow, ops::Range};

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use ratatui::{style::Style, text::Line};

use crate::{Mode, code::Highlighter, source, theme::Theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// Markdown syntax: hidden in concealed mode, dimmed in raw mode.
    Marker,
    /// Content.
    Show,
    /// Hidden in concealed mode because an earlier `Replace` cell stands in for it.
    Skip,
    /// Concealed mode shows `Painter::glyphs[n]` instead of the source byte.
    Replace(usize),
}

#[derive(Debug, Clone, Copy)]
struct Cell {
    kind: Kind,
    style: Style,
}

pub fn render<'a>(
    source: &'a str,
    mode: Mode,
    theme: &Theme,
    width: Option<u16>,
    highlighter: &Highlighter,
) -> Vec<Line<'a>> {
    let mut painter = Painter::new(source, theme, highlighter);
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES;
    for (event, range) in Parser::new_ext(source, options).into_offset_iter() {
        painter.event(event, range);
    }
    lines::build(&painter, mode, width)
}

struct CodeBlock {
    lang: String,
    style: Style,
    lines: Vec<Range<usize>>,
}

struct Painter<'a, 't> {
    src: &'a str,
    theme: &'t Theme,
    highlighter: &'t Highlighter,
    line_ranges: Vec<Range<usize>>,
    cells: Vec<Cell>,
    glyphs: Vec<Cow<'a, str>>,
    code_blocks: Vec<Range<usize>>,
    styles: Vec<Style>,
    /// One entry per open list: whether it is ordered.
    lists: Vec<bool>,
    quote_depth: usize,
    code: Option<CodeBlock>,
    /// The `•` of the current list item and the space after it, hidden again for task items.
    bullet: Option<Range<usize>>,
}

impl<'a, 't> Painter<'a, 't> {
    fn new(src: &'a str, theme: &'t Theme, highlighter: &'t Highlighter) -> Self {
        Self {
            src,
            theme,
            highlighter,
            line_ranges: source::ranges(src),
            cells: vec![
                Cell {
                    kind: Kind::Show,
                    style: Style::new(),
                };
                src.len()
            ],
            glyphs: Vec::new(),
            code_blocks: Vec::new(),
            styles: Vec::new(),
            lists: Vec::new(),
            quote_depth: 0,
            code: None,
            bullet: None,
        }
    }

    fn top(&self) -> Style {
        self.styles.last().copied().unwrap_or_default()
    }

    fn byte(&self, i: usize) -> u8 {
        self.src.as_bytes().get(i).copied().unwrap_or(0)
    }

    fn paint(&mut self, range: Range<usize>, kind: Kind, style: Style) {
        let end = range.end.min(self.cells.len());
        let start = range.start.min(end);
        for cell in &mut self.cells[start..end] {
            *cell = Cell { kind, style };
        }
    }

    fn replace(&mut self, range: Range<usize>, glyph: impl Into<Cow<'a, str>>, style: Style) {
        if range.start >= range.end.min(self.cells.len()) {
            return;
        }
        let index = self.glyphs.len();
        self.glyphs.push(glyph.into());
        self.paint(range.clone(), Kind::Skip, style);
        self.cells[range.start].kind = Kind::Replace(index);
    }

    fn event(&mut self, event: Event<'a>, range: Range<usize>) {
        match event {
            Event::Start(tag) => self.start(tag, range),
            Event::End(tag) => self.end(tag, range),
            Event::Text(text) => self.text(&text, range),
            Event::Code(_) => self.inline_code(range),
            Event::Html(_) | Event::InlineHtml(_) => {
                self.paint(range, Kind::Show, self.top().patch(self.theme.html))
            }
            Event::FootnoteReference(_) => {
                self.paint(range, Kind::Show, self.top().patch(self.theme.footnote))
            }
            Event::InlineMath(_) | Event::DisplayMath(_) => {
                self.paint(range, Kind::Show, self.top().patch(self.theme.inline_code))
            }
            // Breaks cover trailing spaces or the `\` of a hard break plus the newline.
            Event::SoftBreak | Event::HardBreak => self.paint(range, Kind::Marker, self.top()),
            Event::Rule => self.rule(range),
            Event::TaskListMarker(done) => self.task_marker(done, range),
        }
    }

    fn tag_style(&self, tag: &Tag) -> Style {
        let theme = self.theme;
        match tag {
            Tag::Heading { level, .. } => theme.headings[*level as usize - 1],
            Tag::BlockQuote(_) => theme.blockquote,
            Tag::CodeBlock(_) => theme.code_block,
            Tag::HtmlBlock => theme.html,
            Tag::TableHead => theme.strong,
            Tag::Emphasis => theme.emphasis,
            Tag::Strong => theme.strong,
            Tag::Strikethrough => theme.strikethrough,
            Tag::Link { .. } => theme.link,
            Tag::Image { .. } => theme.image,
            _ => Style::new(),
        }
    }

    fn start(&mut self, tag: Tag<'a>, range: Range<usize>) {
        let style = self.top().patch(self.tag_style(&tag));
        self.styles.push(style);
        self.paint(range.clone(), Kind::Marker, style);

        match tag {
            Tag::Heading { .. } if self.byte(range.start) != b'#' => self.setext_underline(range),
            Tag::List(start) => self.lists.push(start.is_some()),
            Tag::Item => self.item_marker(range),
            Tag::BlockQuote(_) => self.quote_depth += 1,
            Tag::CodeBlock(kind) => {
                let lang = match kind {
                    CodeBlockKind::Fenced(info) => info.into_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                self.code = Some(CodeBlock {
                    lang,
                    style,
                    lines: Vec::new(),
                });
            }
            Tag::FootnoteDefinition(_) => self.footnote_label(range),
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd, range: Range<usize>) {
        self.styles.pop();
        match tag {
            TagEnd::List(_) => {
                self.lists.pop();
            }
            TagEnd::BlockQuote(_) => {
                self.quote_depth = self.quote_depth.saturating_sub(1);
                if self.quote_depth == 0 {
                    self.quote_bars(range);
                }
            }
            TagEnd::CodeBlock => self.highlight_code(range),
            TagEnd::Table => self.table_borders(range),
            _ => {}
        }
    }

    fn text(&mut self, text: &str, range: Range<usize>) {
        if let Some(code) = &mut self.code {
            // Synthesized indentation arrives as an empty range; the source already has it.
            if !range.is_empty() {
                code.lines.push(range.clone());
            }
            return;
        }
        match self.src.get(range.clone()) {
            // Entities (`&amp;`) and similar: the event carries the decoded text.
            Some(raw) if !range.is_empty() && raw != text => {
                self.replace(range, text.to_owned(), self.top())
            }
            _ => self.paint(range, Kind::Show, self.top()),
        }
    }

    /// The `Code` range includes the backticks; only the content is shown.
    fn inline_code(&mut self, range: Range<usize>) {
        let style = self.top().patch(self.theme.inline_code);
        let ticks = self.src.as_bytes()
            [range.start.min(self.src.len())..range.end.min(self.src.len())]
            .iter()
            .take_while(|&&b| b == b'`')
            .count();
        if ticks * 2 > range.len() {
            self.paint(range, Kind::Show, style);
            return;
        }
        let inner = range.start + ticks..range.end - ticks;
        self.paint(range, Kind::Marker, style);
        self.paint(inner, Kind::Show, style);
    }

    fn rule(&mut self, range: Range<usize>) {
        let style = self.top().patch(self.theme.rule);
        for i in range {
            if !matches!(self.byte(i), b'\n' | b'\r') {
                self.replace(i..i + 1, "─", style);
            }
        }
    }

    /// `Title\n=====`: the underline line becomes a rule.
    fn setext_underline(&mut self, range: Range<usize>) {
        let Some(body) = self.src.get(range.clone()) else {
            return;
        };
        let body = body.trim_end_matches(['\n', '\r']);
        let underline = range.start + body.rfind('\n').map_or(0, |i| i + 1);
        let style = self.top().patch(self.theme.rule);
        for i in underline..range.start + body.len() {
            if matches!(self.byte(i), b'=' | b'-') {
                self.replace(i..i + 1, "─", style);
            }
        }
    }

    /// `- ` becomes `• `; ordered numbers stay. The space after the marker stays visible.
    fn item_marker(&mut self, range: Range<usize>) {
        self.bullet = None;
        let end = range.end.min(self.src.len());
        let mut i = range.start;
        while i < end && matches!(self.byte(i), b' ' | b'\t') {
            i += 1;
        }
        let style = self.top().patch(self.theme.list_marker);
        if self.lists.last().copied().unwrap_or(false) {
            let number = i;
            while i < end && self.byte(i).is_ascii_digit() {
                i += 1;
            }
            if i < end && matches!(self.byte(i), b'.' | b')') {
                i += 1;
            }
            self.paint(number..i, Kind::Show, style);
        } else if matches!(self.byte(i), b'-' | b'*' | b'+') {
            self.replace(i..i + 1, "•", style);
            i += 1;
        } else {
            return;
        }
        let bullet = range.start..i;
        if i < end && matches!(self.byte(i), b' ' | b'\t') {
            self.paint(i..i + 1, Kind::Show, self.top());
            i += 1;
        }
        self.bullet = (!self.lists.last().copied().unwrap_or(false)).then_some(bullet.start..i);
    }

    /// `- [x] done` becomes `☑ done`: the box stands in for the bullet.
    fn task_marker(&mut self, done: bool, range: Range<usize>) {
        if let Some(bullet) = self.bullet.take() {
            for i in bullet {
                if !matches!(self.byte(i), b' ' | b'\t') || self.cells[i].kind == Kind::Show {
                    self.cells[i].kind = Kind::Skip;
                }
            }
        }
        let glyph = if done { "☑" } else { "☐" };
        let end = range.end;
        self.replace(range, glyph, self.top().patch(self.theme.task_marker));
        if matches!(self.byte(end), b' ' | b'\t')
            && self.cells.get(end).is_some_and(|c| c.kind == Kind::Marker)
        {
            self.paint(end..end + 1, Kind::Show, self.top());
        }
    }

    /// `[^note]:` stays visible so the definition keeps its label.
    fn footnote_label(&mut self, range: Range<usize>) {
        let Some(body) = self.src.get(range.clone()) else {
            return;
        };
        let first_line = body.split('\n').next().unwrap_or("");
        if let Some(end) = first_line.find("]:") {
            let style = self.top().patch(self.theme.footnote);
            let mut label_end = range.start + end + 2;
            if self.byte(label_end) == b' ' {
                label_end += 1;
            }
            self.paint(range.start..label_end, Kind::Show, style);
        }
    }

    /// Replaces the `>` prefixes of every line in a (possibly nested) block quote with `│`.
    /// Continuation-line prefixes are not covered by any event, so they are still markers.
    fn quote_bars(&mut self, range: Range<usize>) {
        let first = self.line_of(range.start);
        let last = self.line_of(range.end.saturating_sub(1).max(range.start));
        for line in first..=last {
            let Some(bounds) = self.line_ranges.get(line).cloned() else {
                break;
            };
            let mut i = bounds.start.max(range.start);
            while i < bounds.end && self.cells[i].kind == Kind::Marker {
                match self.byte(i) {
                    b'>' => {
                        let bar = self.top().patch(self.theme.blockquote_bar);
                        self.replace(i..i + 1, "│", bar);
                        let next = i + 1;
                        if next < bounds.end
                            && self.byte(next) == b' '
                            && self.cells[next].kind == Kind::Marker
                        {
                            self.paint(next..next + 1, Kind::Show, self.cells[next].style);
                            i = next;
                        }
                    }
                    b' ' | b'\t' => {}
                    _ => break,
                }
                i += 1;
            }
        }
    }

    /// Pipes and the delimiter row are syntax, but hiding them would wreck the column layout.
    fn table_borders(&mut self, range: Range<usize>) {
        let border = self.top().patch(self.theme.table_border);
        for i in range.start..range.end.min(self.src.len()) {
            if self.cells[i].kind != Kind::Marker {
                continue;
            }
            match self.byte(i) {
                b'|' => self.replace(i..i + 1, "│", border),
                b'-' | b':' => self.replace(i..i + 1, "─", border),
                b' ' | b'\t' => self.cells[i].kind = Kind::Show,
                // `**`, backticks etc. inside cells become padding so columns stay aligned.
                _ => {
                    let style = self.cells[i].style;
                    self.replace(i..i + 1, " ", style);
                }
            }
        }
    }

    fn highlight_code(&mut self, range: Range<usize>) {
        let Some(code) = self.code.take() else {
            return;
        };
        let lines: Vec<&str> = code
            .lines
            .iter()
            .map(|r| self.src.get(r.clone()).unwrap_or(""))
            .collect();
        let tokens = self.highlighter.highlight(&code.lang, &lines, false);
        for (line, tokens) in code.lines.iter().zip(tokens) {
            self.paint(line.clone(), Kind::Show, code.style);
            for (token, style) in tokens {
                let start = line.start + token.start;
                let end = (line.start + token.end).min(line.end);
                self.paint(start..end, Kind::Show, code.style.patch(style));
            }
        }
        self.code_blocks.push(range);
    }

    fn line_of(&self, pos: usize) -> usize {
        self.line_ranges.partition_point(|r| r.end < pos)
    }
}

#[cfg(test)]
mod tests {
    use ratatui::style::{Color, Modifier};

    use super::*;

    fn lines(source: &str, mode: Mode) -> Vec<Line<'_>> {
        render(source, mode, &Theme::default(), None, Highlighter::shared())
    }

    fn concealed(source: &str) -> Vec<String> {
        lines(source, Mode::Concealed)
            .iter()
            .map(Line::to_string)
            .collect()
    }

    fn raw(source: &str) -> Vec<String> {
        lines(source, Mode::Raw)
            .iter()
            .map(Line::to_string)
            .collect()
    }

    fn span_style<'a>(line: &Line<'a>, text: &str) -> Style {
        line.spans
            .iter()
            .find(|s| s.content == text)
            .unwrap_or_else(|| panic!("no span {text:?} in {line:?}"))
            .style
    }

    #[test]
    fn headings_hide_hashes_and_keep_style() {
        let out = lines("# Title\n### Sub ###", Mode::Concealed);
        assert_eq!(out[0].to_string(), "Title");
        assert_eq!(out[1].to_string(), "Sub");
        let style = span_style(&out[0], "Title");
        assert_eq!(style.fg, Some(Color::Magenta));
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn raw_mode_shows_everything_with_dim_markers() {
        let source = "# A **b** [c](d)";
        assert_eq!(raw(source), [source]);
        let out = lines(source, Mode::Raw);
        assert_eq!(span_style(&out[0], "# ").fg, Some(Color::DarkGray));
        assert!(
            span_style(&out[0], "b")
                .add_modifier
                .contains(Modifier::BOLD)
        );
    }

    #[test]
    fn inline_styles() {
        let out = lines("*i* **b** ~~s~~ `c`", Mode::Concealed);
        assert_eq!(out[0].to_string(), "i b s c");
        assert!(
            span_style(&out[0], "i")
                .add_modifier
                .contains(Modifier::ITALIC)
        );
        assert!(
            span_style(&out[0], "s")
                .add_modifier
                .contains(Modifier::CROSSED_OUT)
        );
        assert_eq!(span_style(&out[0], "c").fg, Some(Color::Yellow));
    }

    #[test]
    fn inline_code_with_double_backticks() {
        assert_eq!(concealed("``a ` b``"), ["a ` b"]);
    }

    #[test]
    fn links_hide_the_url() {
        let out = lines("see [docs](https://x.y) now", Mode::Concealed);
        assert_eq!(out[0].to_string(), "see docs now");
        assert!(
            span_style(&out[0], "docs")
                .add_modifier
                .contains(Modifier::UNDERLINED)
        );
        assert_eq!(concealed("<https://x.y>"), ["https://x.y"]);
    }

    #[test]
    fn lists_and_tasks() {
        assert_eq!(
            concealed("- a\n  - b\n* c\n1. d\n10) e\n- [ ] f\n- [x] g"),
            ["• a", "  • b", "• c", "1. d", "10) e", "☐ f", "☑ g"]
        );
    }

    #[test]
    fn quotes_get_bars() {
        assert_eq!(
            concealed("> a\n> b\n>\n> > c\nlazy"),
            ["│ a", "│ b", "│", "│ │ c", "lazy"]
        );
    }

    #[test]
    fn quote_inside_list() {
        assert_eq!(concealed("- > a\n  > b"), ["• │ a", "  │ b"]);
    }

    #[test]
    fn rules_and_setext() {
        assert_eq!(
            concealed("a\n\n---\n\nB\n==="),
            ["a", "", "───", "", "B", "───"]
        );
    }

    #[test]
    fn entities_are_decoded_only_when_concealed() {
        assert_eq!(concealed("a &amp; b &copy;"), ["a & b ©"]);
        assert_eq!(raw("a &amp; b"), ["a &amp; b"]);
    }

    #[test]
    fn escapes_and_hard_breaks() {
        assert_eq!(concealed("\\*not em\\*"), ["*not em*"]);
        assert_eq!(concealed("one\\\ntwo"), ["one", "two"]);
        assert_eq!(concealed("one  \ntwo"), ["one", "two"]);
    }

    #[test]
    fn fenced_code_block_lines_are_blank_and_on_code_background() {
        let theme = Theme::default();
        let out = lines("```rust\nlet x = 1;\n```", Mode::Concealed);
        assert_eq!(out.len(), 3);
        assert_eq!(out[0].to_string().trim(), "");
        assert_eq!(out[1].to_string(), "let x = 1;");
        for line in &out {
            assert_eq!(line.style.bg, theme.code_block.bg);
            assert!(line.spans.iter().all(|s| s.style.bg == theme.code_block.bg));
        }
        assert_eq!(raw("```rust\nlet x = 1;\n```")[0], "```rust");
    }

    #[test]
    fn code_block_in_list_keeps_indentation() {
        assert_eq!(concealed("- a\n\n  ```\n  code\n  ```")[3], "  code");
    }

    #[test]
    fn unknown_language_renders_plain_on_background() {
        let out = lines("```nope\nhello\n```", Mode::Concealed);
        assert_eq!(out[1].to_string(), "hello");
        assert_eq!(out[1].spans[0].style, Theme::default().code_block);
    }

    #[test]
    fn tables_keep_alignment() {
        let out = concealed("| a | `b` |\n|---|:---:|\n| **c** | d |");
        assert_eq!(out, ["│ a │  b  │", "│───│─────│", "│   c   │ d │"]);
    }

    #[test]
    fn footnotes() {
        assert_eq!(
            concealed("x[^1]\n\n[^1]: note"),
            ["x[^1]", "", "[^1]: note"]
        );
    }

    #[test]
    fn crlf_and_unicode() {
        assert_eq!(
            concealed("# 日本\r\n**émoji 🎉**\r\n"),
            ["日本", "émoji 🎉"]
        );
        assert_eq!(concealed("`🎉`  *日*"), ["🎉  日"]);
        assert_eq!(concealed("```\n🎉 日本\n```")[1], "🎉 日本");
    }
}
