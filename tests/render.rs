use hoodrich::{
    Change, Mode, Renderer,
    ratatui::{
        Terminal,
        backend::TestBackend,
        style::{Color, Modifier},
        widgets::Paragraph,
    },
};

const SAMPLE: &str = include_str!("../examples/sample.md");
const DRACULA_BG: Color = Color::Rgb(0x28, 0x2a, 0x36);

#[test]
fn one_line_per_source_line_in_both_modes() {
    for mode in [Mode::Concealed, Mode::Raw] {
        let text = Renderer::new(mode).render(SAMPLE);
        assert_eq!(text.lines.len(), SAMPLE.lines().count(), "{mode:?}");
    }
}

#[test]
fn rendering_with_links_draws_the_same_text() {
    for mode in [Mode::Concealed, Mode::Raw] {
        let renderer = Renderer::new(mode);
        let rendered = renderer.render_with_links(SAMPLE);
        assert_eq!(rendered.text, renderer.render(SAMPLE), "{mode:?}");
    }
}

#[test]
fn raw_mode_round_trips_the_source() {
    let text = Renderer::new(Mode::Raw).render(SAMPLE);
    let rendered: Vec<String> = text.lines.iter().map(ToString::to_string).collect();
    let source: Vec<&str> = SAMPLE.lines().collect();
    assert_eq!(rendered, source);
}

#[test]
fn code_background_fills_the_row_through_paragraph() {
    let width = 40;
    let source = "text\n\n```rust\nfn main() {}\n\n```\n";
    let text = Renderer::new(Mode::Concealed)
        .with_width(width)
        .render(source);
    let mut terminal = Terminal::new(TestBackend::new(width, 6)).expect("test backend");
    terminal
        .draw(|frame| frame.render_widget(Paragraph::new(text), frame.area()))
        .expect("draw");
    let buffer = terminal.backend().buffer();

    for y in 2..6 {
        for x in 0..width {
            assert_eq!(buffer[(x, y)].bg, DRACULA_BG, "cell {x},{y}");
        }
    }
    assert_ne!(buffer[(0, 0)].bg, DRACULA_BG);
}

#[test]
fn blank_code_lines_keep_a_background_cell_without_width() {
    let text = Renderer::new(Mode::Concealed).render("```\n\n```");
    for line in &text.lines {
        assert!(line.width() >= 1);
        assert!(line.spans.iter().all(|s| s.style.bg == Some(DRACULA_BG)));
    }
}

#[test]
fn diff_styles_unchanged_lines_and_leaves_changes_plain() {
    let old = "# Title\n\n- keep **this**\n- drop me\n";
    let new = "# Title\n\n- keep **this**\n- added\n";
    let diff = Renderer::new(Mode::Concealed).render_diff(old, new);

    let changes: Vec<Change> = diff.iter().map(|l| l.change).collect();
    assert_eq!(
        changes,
        [
            Change::Same,
            Change::Same,
            Change::Same,
            Change::Removed,
            Change::Added
        ]
    );
    assert_eq!(diff[0].line.to_string(), "Title");
    assert_eq!(diff[0].line.spans[0].style.fg, Some(Color::Magenta));
    assert_eq!(diff[2].line.to_string(), "• keep this");
    for line in &diff[3..] {
        assert_eq!(line.line.spans.len(), 1);
        assert_eq!(line.line.spans[0].style, Default::default());
        assert_eq!(line.line.style, Default::default());
    }
    assert_eq!(diff[3].line.to_string(), "- drop me");
    assert_eq!(diff[4].line.to_string(), "- added");
}

#[test]
fn diff_of_identical_and_empty_documents() {
    let renderer = Renderer::default();
    assert!(renderer.render_diff("", "").is_empty());
    let same = renderer.render_diff(SAMPLE, SAMPLE);
    assert_eq!(same.len(), SAMPLE.lines().count());
    assert!(same.iter().all(|l| l.change == Change::Same));
    let added = renderer.render_diff("", "a\nb");
    assert!(added.iter().all(|l| l.change == Change::Added));
}

#[test]
fn render_code_highlights_a_file() {
    let source = "// hi\nfn main() {\n    if true { return; }\n}\n";
    let text = Renderer::default().render_code(source, "rs");
    assert_eq!(text.lines.len(), 4);
    let comment = &text.lines[0].spans[0];
    assert_eq!(comment.style.fg, Some(Color::Rgb(0x62, 0x72, 0xa4)));
    let keyword = text.lines[2]
        .spans
        .iter()
        .find(|s| s.content == "if")
        .expect("if span");
    assert_eq!(keyword.style.fg, Some(Color::Rgb(0xff, 0x79, 0xc6)));
    assert!(text.lines.iter().all(|l| l.style.bg == Some(DRACULA_BG)));
}

#[test]
fn render_code_falls_back_to_plain_and_sniffs_shebangs() {
    let renderer = Renderer::default();
    let plain = renderer.render_code("whatever here\n", "zzz");
    assert_eq!(
        plain.lines[0].spans[0].style.fg,
        Some(Color::Rgb(0xf8, 0xf8, 0xf2))
    );
    assert!(renderer.render_code("", "rs").lines.is_empty());

    let script = renderer.render_code("#!/usr/bin/env python3\ndef f(): pass\n", "");
    let def = script.lines[1]
        .spans
        .iter()
        .find(|s| s.content == "def")
        .expect("def span");
    assert_ne!(def.style.fg, Some(Color::Rgb(0xf8, 0xf8, 0xf2)));
}

#[test]
fn code_diff_highlights_unchanged_lines() {
    let old = "fn a() {}\nfn b() {}\n";
    let new = "fn a() {}\nfn c() {}\n";
    let diff = Renderer::default().render_code_diff(old, new, "rust");
    assert_eq!(diff[0].change, Change::Same);
    assert!(diff[0].line.spans.len() > 1);
    assert!(
        diff[0].line.spans[0]
            .style
            .add_modifier
            .contains(Modifier::ITALIC)
    );
    assert_eq!(diff[1].change, Change::Removed);
    assert_eq!(diff[1].line.spans[0].style, Default::default());
    assert_eq!(diff[2].change, Change::Added);
}

#[test]
fn free_function_matches_renderer() {
    assert_eq!(
        hoodrich::render(SAMPLE, Mode::Concealed),
        Renderer::new(Mode::Concealed).render(SAMPLE)
    );
}

#[test]
fn diff_ignores_missing_final_newline_and_lone_carriage_returns() {
    let renderer = Renderer::default();
    let diff = renderer.render_diff("a\nb", "a\nb\nc\n");
    let changes: Vec<Change> = diff.iter().map(|l| l.change).collect();
    assert_eq!(changes, [Change::Same, Change::Same, Change::Added]);

    let diff = renderer.render_diff("x\ra **b**", "x\ra **b**");
    assert_eq!(diff.len(), 1);
    assert_eq!(diff[0].line.to_string(), "xa b");
}
