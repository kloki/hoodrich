//! Render markdown and source code as styled [`ratatui`] [`Text`].
//!
//! - Markdown is styled with the 16 ANSI colours only, so the terminal theme drives the look.
//! - Code blocks and whole source files are highlighted with Dracula colours on a Dracula
//!   background.
//! - Every source line becomes exactly one output line, in both [`Mode`]s.
//! - Rendering never fails: malformed markdown renders best effort.
//!
//! ```
//! use hoodrich::{Mode, Renderer};
//!
//! let renderer = Renderer::new(Mode::Concealed);
//! let text = renderer.render("# Title\n\nSome **bold** text.");
//! assert_eq!(text.lines.len(), 3);
//! assert_eq!(text.lines[0].to_string(), "Title");
//! ```

mod code;
mod diff;
mod markdown;
mod source;
mod theme;

pub use diff::{Change, DiffLine};
/// The ratatui version hoodrich's types come from; use it to avoid version mismatches.
pub use ratatui;
use ratatui::text::Text;
pub use theme::Theme;

use crate::code::Highlighter;

/// How markdown syntax is displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// Hide markdown syntax (`###`, `**`, code fences, link urls); show bullets, task boxes,
    /// quote bars and rules as glyphs.
    #[default]
    Concealed,
    /// Show the source as written, with markdown syntax dimmed.
    Raw,
}

/// Renders markdown and code. Cheap to create and clone; syntax definitions are loaded once per
/// process and shared.
#[derive(Debug, Clone)]
pub struct Renderer {
    mode: Mode,
    theme: Theme,
    width: Option<u16>,
    highlighter: &'static Highlighter,
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new(Mode::default())
    }
}

impl Renderer {
    pub fn new(mode: Mode) -> Self {
        Self {
            mode,
            theme: Theme::default(),
            width: None,
            highlighter: Highlighter::shared(),
        }
    }

    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    pub fn with_mode(mut self, mode: Mode) -> Self {
        self.mode = mode;
        self
    }

    /// Pads code lines with spaces to `width` columns so their background fills the row.
    /// ratatui's `Paragraph` only paints a line's background under its text.
    pub fn with_width(mut self, width: u16) -> Self {
        self.width = Some(width);
        self
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    /// Markdown to text, one line per line of `source` (as split by [`str::lines`]).
    pub fn render<'a>(&self, source: &'a str) -> Text<'a> {
        Text::from(self.markdown_lines(source))
    }

    /// Diff of two markdown documents. Unchanged lines are styled; added and removed lines are
    /// returned unstyled.
    pub fn render_diff<'a>(&self, old: &'a str, new: &'a str) -> Vec<DiffLine<'a>> {
        diff::merge(old, new, self.markdown_lines(new))
    }

    /// A whole source file, highlighted with Dracula colours on the code background.
    ///
    /// `lang` is a language name, extension or file name (`"rust"`, `"rs"`, `"main.rs"`). When it
    /// is empty or unknown the first line is tried (shebangs), then the code renders plain.
    pub fn render_code<'a>(&self, source: &'a str, lang: &str) -> Text<'a> {
        Text::from(self.code_lines(source, lang))
    }

    /// Diff of two versions of a source file. Unchanged lines are highlighted; added and removed
    /// lines are returned unstyled.
    pub fn render_code_diff<'a>(
        &self,
        old: &'a str,
        new: &'a str,
        lang: &str,
    ) -> Vec<DiffLine<'a>> {
        diff::merge(old, new, self.code_lines(new, lang))
    }

    fn markdown_lines<'a>(&self, source: &'a str) -> Vec<ratatui::text::Line<'a>> {
        markdown::render(source, self.mode, &self.theme, self.width, self.highlighter)
    }

    fn code_lines<'a>(&self, source: &'a str, lang: &str) -> Vec<ratatui::text::Line<'a>> {
        code::render(source, lang, &self.theme, self.width, self.highlighter)
    }
}

/// Shorthand for `Renderer::new(mode).render(source)`.
pub fn render(source: &str, mode: Mode) -> Text<'_> {
    Renderer::new(mode).render(source)
}
