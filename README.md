# hoodrich

[Textualize's Rich](https://github.com/Textualize/rich) for [ratatui](https://ratatui.rs)

- **ANSI-only markdown styling.** Headings, emphasis, lists, quotes, tables and links use the
  16 standard terminal colours and modifiers, so your terminal theme drives the look.
- **Two modes.** `Concealed` hides markdown syntax (`###`, `**`, code fences, link urls) and
  shows bullets, task boxes, quote bars and rules as glyphs. `Raw` shows the source as written
  with the syntax dimmed.
- **Code blocks** are highlighted with [Dracula](https://draculatheme.com) colours on the Dracula
  background. Whole source files render the same way.
- **Diffs.** Give it the old and new text. Unchanged lines come back styled, added and removed
  lines come back plain so you can colour them yourself.
- **Clickable links.** `render_with_links` also says where each link's text landed (line and
  display columns) and its url, even when concealed mode hides it.
- **One line out per line in.** Every source line maps to exactly one output line in both modes,
  which keeps scrolling, cursors and diffs simple.
- **Never fails.** There is no `Result` in the API. Malformed markdown renders best effort.

## Usage

```toml
[dependencies]
hoodrich = "0.1"
```

```rust
use hoodrich::{Mode, Renderer};
use ratatui::widgets::Paragraph;

let renderer = Renderer::new(Mode::Concealed).with_width(area.width);

// Markdown
let text = renderer.render("# Title\n\nSome **bold** text and `code`.");
frame.render_widget(Paragraph::new(text), area);

// Markdown plus where its links are, to open the one under a mouse click
let rendered = renderer.render_with_links(source);
for link in &rendered.links {
    // link.line, link.columns (display columns) and link.url
}

// A source file
let text = renderer.render_code(source, "rs");

// A diff of two markdown documents
for line in renderer.render_diff(old, new) {
    match line.change {
        hoodrich::Change::Same => { /* line.line is styled */ }
        hoodrich::Change::Added | hoodrich::Change::Removed => { /* plain, colour it yourself */ }
    }
}
```

`Renderer` is cheap to create and clone. Syntax definitions load once per process.
Rendering keeps a small style record per source byte while it runs, so memory use is roughly
30x the input size. That is nothing for chat messages and docs, but worth knowing for
multi-megabyte files.

### Code backgrounds

ratatui's `Paragraph` only paints a line's background under its text. Set `with_width` to the
width you render into, and code lines are padded with spaces so their background fills the row.

### Language detection

`render_code` takes a language name, extension or file name (`"rust"`, `"rs"`, `"main.rs"`).
When it is empty or unknown, the first line is tried (shebangs), and otherwise the code renders
plain on the code background. Fenced code blocks use their info string the same way.

Highlighting uses syntect's bundled syntaxes. TOML, TypeScript and Dockerfile are not among
them and render plain.

## Theming

`Theme` has a public field per element. Start from `Theme::default()` and change what you need:

```rust
use hoodrich::{Renderer, Theme, ratatui::style::{Color, Style}};

let theme = Theme {
    inline_code: Style::new().fg(Color::Green),
    ..Theme::default()
};
let renderer = Renderer::default().with_theme(theme);
```

## Build performance

syntect uses the pure-Rust `fancy-regex` engine, which is very slow in unoptimised builds. Add
this to your `Cargo.toml` to keep debug builds responsive:

```toml
[profile.dev.package.syntect]
opt-level = 3
[profile.dev.package.fancy-regex]
opt-level = 3
[profile.dev.package.regex-automata]
opt-level = 3
[profile.dev.package.regex-syntax]
opt-level = 3
```

## ratatui version

hoodrich returns ratatui types, so it is tied to a ratatui minor version (currently 0.30).
`hoodrich::ratatui` re-exports the version it uses.

## Demo

```sh
cargo run --example demo            # renders examples/sample.md
cargo run --example demo README.md  # or any markdown file
```

`m` toggles concealed/raw, `v` cycles the markdown, diff and code views, `j`/`k` scroll.

## License

MIT
