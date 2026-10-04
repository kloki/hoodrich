//! Interactive preview.
//!
//! `cargo run --example demo [file]`, then: `m` toggle concealed/raw, `v` cycle
//! markdown / diff / code view, `j`/`k` scroll, `q` quit.

use std::{fs, io};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use hoodrich::{Change, DiffLine, Mode, Renderer};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Paragraph},
};

const SAMPLE: &str = include_str!("sample.md");
const CODE: &str = include_str!("../src/code/mod.rs");

#[derive(Clone, Copy)]
enum View {
    Markdown,
    Diff,
    Code,
}

struct App {
    source: String,
    previous: String,
    mode: Mode,
    view: View,
    scroll: u16,
}

fn main() -> io::Result<()> {
    let source = match std::env::args().nth(1) {
        Some(path) => fs::read_to_string(path)?,
        None => SAMPLE.to_owned(),
    };
    let previous = source
        .replacen("**bold**", "bold", 1)
        .replacen("2. two\n", "", 1)
        .replacen("# hoodrich", "# hoodrich (old)", 1);
    let app = App {
        source,
        previous,
        mode: Mode::Concealed,
        view: View::Markdown,
        scroll: 0,
    };
    let terminal = ratatui::init();
    let result = run(terminal, app);
    ratatui::restore();
    result
}

fn run(mut terminal: DefaultTerminal, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|frame| draw(frame, &app))?;
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
            KeyCode::Char('m') => {
                app.mode = match app.mode {
                    Mode::Concealed => Mode::Raw,
                    Mode::Raw => Mode::Concealed,
                }
            }
            KeyCode::Char('v') => {
                app.view = match app.view {
                    View::Markdown => View::Diff,
                    View::Diff => View::Code,
                    View::Code => View::Markdown,
                };
                app.scroll = 0;
            }
            KeyCode::Char('j') | KeyCode::Down => app.scroll = app.scroll.saturating_add(1),
            KeyCode::Char('k') | KeyCode::Up => app.scroll = app.scroll.saturating_sub(1),
            KeyCode::PageDown | KeyCode::Char(' ') => app.scroll = app.scroll.saturating_add(20),
            KeyCode::PageUp => app.scroll = app.scroll.saturating_sub(20),
            _ => {}
        }
    }
}

fn draw(frame: &mut Frame, app: &App) {
    let [body, help] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(frame.area());
    let block = Block::bordered().title(match app.view {
        View::Markdown => " markdown ",
        View::Diff => " diff ",
        View::Code => " src/code/mod.rs ",
    });
    let inner = block.inner(body);
    let renderer = Renderer::new(app.mode).with_width(inner.width);

    let text = match app.view {
        View::Markdown => renderer.render(&app.source),
        View::Diff => diff_text(renderer.render_diff(&app.previous, &app.source)),
        View::Code => renderer.render_code(CODE, "rs"),
    };
    frame.render_widget(
        Paragraph::new(text).block(block).scroll((app.scroll, 0)),
        body,
    );

    let mode = match app.mode {
        Mode::Concealed => "concealed",
        Mode::Raw => "raw",
    };
    let help_line = Line::from(vec![
        Span::raw(" m ").bold(),
        Span::raw(format!("mode: {mode}  ")),
        Span::raw("v ").bold(),
        Span::raw("view  "),
        Span::raw("j/k ").bold(),
        Span::raw("scroll  "),
        Span::raw("q ").bold(),
        Span::raw("quit"),
    ])
    .dark_gray();
    frame.render_widget(help_line, help);
}

/// How a consumer like nth might present a diff: a gutter plus colour on changed lines.
fn diff_text(lines: Vec<DiffLine<'_>>) -> Text<'_> {
    lines
        .into_iter()
        .map(|DiffLine { change, line }| {
            let (gutter, colour) = match change {
                Change::Same => ("  ", None),
                Change::Added => ("+ ", Some(Color::Green)),
                Change::Removed => ("- ", Some(Color::Red)),
            };
            let style = colour.map_or_else(Style::new, |c| Style::new().fg(c));
            let mut spans = vec![Span::styled(gutter, style)];
            spans.extend(line.spans.into_iter().map(|s| s.patch_style(style)));
            Line::from(spans).style(line.style)
        })
        .collect()
}
