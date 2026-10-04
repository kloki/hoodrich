//! Rendering must never panic and must always return one line per source line.

use hoodrich::{Change, Mode, Renderer};

fn check(source: &str) {
    for mode in [Mode::Concealed, Mode::Raw] {
        for renderer in [Renderer::new(mode), Renderer::new(mode).with_width(20)] {
            let text = renderer.render(source);
            assert_eq!(
                text.lines.len(),
                source.lines().count(),
                "{mode:?}: {source:?}"
            );
            let diff = renderer.render_diff("# other\n", source);
            let new_lines = diff.iter().filter(|l| l.change != Change::Removed).count();
            assert_eq!(new_lines, source.lines().count(), "diff: {source:?}");
            let same = renderer.render_diff(source, source);
            assert!(same.iter().all(|l| l.change == Change::Same));
            assert_eq!(same.len(), source.lines().count());
            let code = renderer.render_code(source, "rust");
            assert_eq!(code.lines.len(), source.lines().count());
        }
    }
}

const CASES: &[&str] = &[
    "",
    "\n",
    "\r\n\r\n",
    "#",
    "######",
    "####### seven",
    "**unclosed bold",
    "*a **b* c**",
    "***",
    "`unclosed code",
    "``",
    "```",
    "```rust\nfn unclosed() {",
    "~~~\n```\n~~~",
    "[link](",
    "[link](url",
    "](",
    "![",
    "![alt](",
    "<http://unclosed",
    "<div>\nunclosed html",
    "&amp",
    "&#xFFFFFFFF;",
    "&#0;",
    "> ",
    ">",
    "- ",
    "-",
    "1.",
    "99999999999. big",
    "- [",
    "- [x",
    "- [x]",
    "| a |\n|",
    "| a | b |\n|---|\n| c |",
    "|\n|-|\n|",
    "[^]: empty",
    "[^1]:",
    "x[^missing]",
    "\\",
    "a\\",
    "\t\tcode\twith\ttabs",
    "    \n    \n",
    "Setext\n===\n---\n===",
    "\u{0}\u{1}\u{7f}",
    "\u{feff}# bom",
    "\r",
    "a\rb\rc",
    "# 日本語 **太字** `コード`",
    "🎉🎉 *🎉* `🎉` [🎉](🎉)",
    "> 🎉 quote\n> - 日本\n>   ```\n>   🎉\n>   ```",
    "| 🎉 | 日本 |\n|---|---|\n| **é** | `ü` |",
    "e\u{301}\u{301}\u{301} combining *marks*",
];

#[test]
fn malformed_inputs() {
    for case in CASES {
        check(case);
    }
}

#[test]
fn deep_nesting() {
    check(&"> ".repeat(200));
    check(&format!("{}x", "> ".repeat(50)));
    let list: String = (0..60)
        .map(|i| format!("{}- item\n", "  ".repeat(i)))
        .collect();
    check(&list);
    check(&"[".repeat(500));
    check(&"*".repeat(1000));
}

#[test]
fn large_input() {
    for source in ["*".repeat(1 << 20), "a **b** `c` [d](e)\n".repeat(20_000)] {
        for mode in [Mode::Concealed, Mode::Raw] {
            let text = Renderer::new(mode).render(&source);
            assert_eq!(text.lines.len(), source.lines().count());
        }
    }
}

/// Deterministic pseudo-random documents built from markdown-heavy fragments.
#[test]
fn random_documents() {
    const FRAGMENTS: &[&str] = &[
        "#", " ", "\n", "\r\n", "*", "_", "`", "```", "~", "[", "]", "(", ")", "<", ">", "!", "|",
        "-", "+", "1.", "\\", "&amp;", "&", ";", "\t", "x", "日", "🎉", "é", "[ ]", "[x]", ":",
        "=", "^", "\"", "'", "\u{0}", "    ", "rust", "{", "}",
    ];
    let mut state: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for _ in 0..2_000 {
        let len = (next() % 80) as usize;
        let doc: String = (0..len)
            .map(|_| FRAGMENTS[(next() % FRAGMENTS.len() as u64) as usize])
            .collect();
        check(&doc);
    }
}
