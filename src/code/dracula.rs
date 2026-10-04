//! The Dracula palette (<https://draculatheme.com>) as a syntect theme, built in code so the crate
//! does not need syntect's theme loading features.

use std::str::FromStr;

use ratatui::style::Color;
use syntect::highlighting::{
    self as hl, FontStyle, ScopeSelectors, StyleModifier, ThemeItem, ThemeSettings,
};

pub const BACKGROUND: Color = Color::Rgb(0x28, 0x2a, 0x36);
pub const FOREGROUND: Color = Color::Rgb(0xf8, 0xf8, 0xf2);

const BG: hl::Color = rgb(0x28, 0x2a, 0x36);
const FG: hl::Color = rgb(0xf8, 0xf8, 0xf2);
const COMMENT: hl::Color = rgb(0x62, 0x72, 0xa4);
const CYAN: hl::Color = rgb(0x8b, 0xe9, 0xfd);
const GREEN: hl::Color = rgb(0x50, 0xfa, 0x7b);
const ORANGE: hl::Color = rgb(0xff, 0xb8, 0x6c);
const PINK: hl::Color = rgb(0xff, 0x79, 0xc6);
const PURPLE: hl::Color = rgb(0xbd, 0x93, 0xf9);
const YELLOW: hl::Color = rgb(0xf1, 0xfa, 0x8c);

const fn rgb(r: u8, g: u8, b: u8) -> hl::Color {
    hl::Color { r, g, b, a: 0xff }
}

/// Scope selectors and colours, following the official dracula/sublime tmTheme.
/// Later entries win over earlier ones when they match with equal specificity.
const SCOPES: &[(&str, hl::Color, Option<FontStyle>)] = &[
    ("comment", COMMENT, None),
    ("string", YELLOW, None),
    (
        "constant.numeric, constant.language, constant.character, constant.other, variable.language",
        PURPLE,
        None,
    ),
    (
        "keyword, storage, entity.name.tag, punctuation.accessor, punctuation.section.embedded, \
         punctuation.separator.namespace, meta.function.return-type, constant.character.escape, \
         markup.deleted",
        PINK,
        None,
    ),
    ("storage.type", CYAN, Some(FontStyle::ITALIC)),
    (
        "entity.name.class, entity.name.type, entity.other.inherited-class, support.function, \
         support.type, support.class, meta.path, support.other.namespace",
        CYAN,
        None,
    ),
    (
        "entity.name.function, variable.function, entity.other.attribute-name, markup.inserted",
        GREEN,
        None,
    ),
    ("variable.parameter", ORANGE, Some(FontStyle::ITALIC)),
    ("variable.other.readwrite.instance", ORANGE, None),
    ("markup.heading", PURPLE, Some(FontStyle::BOLD)),
    ("markup.bold", ORANGE, Some(FontStyle::BOLD)),
    ("markup.italic", YELLOW, Some(FontStyle::ITALIC)),
];

pub fn theme() -> hl::Theme {
    let mut scopes: Vec<ThemeItem> = SCOPES
        .iter()
        .map(|(selector, colour, font_style)| ThemeItem {
            scope: ScopeSelectors::from_str(selector)
                .expect("hard-coded Dracula scope selector is valid"),
            style: StyleModifier {
                foreground: Some(*colour),
                background: None,
                font_style: *font_style,
            },
        })
        .collect();
    scopes.push(ThemeItem {
        scope: ScopeSelectors::from_str("invalid")
            .expect("hard-coded Dracula scope selector is valid"),
        style: StyleModifier {
            foreground: Some(FG),
            background: Some(PINK),
            font_style: None,
        },
    });

    hl::Theme {
        name: Some("Dracula".into()),
        author: None,
        // Tokens no scope matches fall back to these; syntect's own fallback is black on white.
        settings: ThemeSettings {
            foreground: Some(FG),
            background: Some(BG),
            ..ThemeSettings::default()
        },
        scopes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_builds_with_all_selectors() {
        let theme = theme();
        assert_eq!(theme.scopes.len(), SCOPES.len() + 1);
        assert_eq!(theme.settings.background, Some(BG));
    }
}
