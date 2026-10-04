use ratatui::style::{Color, Modifier, Style};

use crate::code::dracula;

/// Styles for every markdown element.
///
/// Markdown elements only use the 16 ANSI colours and modifiers so the terminal theme drives the
/// look. Code blocks are the exception: they use Dracula RGB colours on a Dracula background.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    /// H1 to H6.
    pub headings: [Style; 6],
    pub emphasis: Style,
    pub strong: Style,
    pub strikethrough: Style,
    pub inline_code: Style,
    pub link: Style,
    pub image: Style,
    /// Text inside a block quote.
    pub blockquote: Style,
    /// The `│` that replaces `>` in concealed mode.
    pub blockquote_bar: Style,
    /// Bullets (`•`) and ordered list numbers.
    pub list_marker: Style,
    /// `☐` / `☑`.
    pub task_marker: Style,
    /// Horizontal rules and setext heading underlines.
    pub rule: Style,
    /// Table pipes and the delimiter row.
    pub table_border: Style,
    pub html: Style,
    pub footnote: Style,
    /// Markdown syntax (`#`, `**`, fences, link urls) as shown in raw mode.
    pub marker: Style,
    /// Base style of fenced/indented code blocks and [`crate::Renderer::render_code`].
    pub code_block: Style,
}

impl Default for Theme {
    fn default() -> Self {
        let bold = Style::new().add_modifier(Modifier::BOLD);
        Self {
            headings: [
                bold.fg(Color::Magenta),
                bold.fg(Color::Blue),
                bold.fg(Color::Cyan),
                bold,
                bold,
                bold,
            ],
            emphasis: Style::new().add_modifier(Modifier::ITALIC),
            strong: bold,
            strikethrough: Style::new().add_modifier(Modifier::CROSSED_OUT),
            inline_code: Style::new().fg(Color::Yellow),
            link: Style::new()
                .fg(Color::Blue)
                .add_modifier(Modifier::UNDERLINED),
            image: Style::new().fg(Color::Blue).add_modifier(Modifier::ITALIC),
            blockquote: Style::new().add_modifier(Modifier::ITALIC),
            blockquote_bar: Style::new().fg(Color::DarkGray),
            list_marker: Style::new().fg(Color::Cyan),
            task_marker: Style::new().fg(Color::Cyan),
            rule: Style::new().fg(Color::DarkGray),
            table_border: Style::new().fg(Color::DarkGray),
            html: Style::new().fg(Color::DarkGray),
            footnote: Style::new().fg(Color::Cyan),
            marker: Style::new().fg(Color::DarkGray),
            code_block: Style::new().fg(dracula::FOREGROUND).bg(dracula::BACKGROUND),
        }
    }
}
