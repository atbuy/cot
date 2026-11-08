//! The built-in themes and the colors in use, as 0xRRGGBB. The colors are
//! Hot Coral unless the config file says otherwise, and change while a theme
//! is previewed or chosen in the settings panel.

use std::cell::Cell;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    pub selection: u32,
    pub surface: u32,
    pub text: u32,
    pub annotation: u32,
}

const fn palette(selection: u32, surface: u32, text: u32, annotation: u32) -> Palette {
    Palette { selection, surface, text, annotation }
}

const HOT_CORAL: Palette = palette(0xFF4D6D, 0x1C1216, 0xFFECEF, 0x4DD8FF);

pub const DEFAULT_THEME: &str = "hot-coral";

pub const THEMES: [(&str, Palette); 5] = [
    (DEFAULT_THEME, HOT_CORAL),
    ("signal-amber", palette(0xFFB000, 0x17140F, 0xF5EDE0, 0xFF5C39)),
    ("electric-mint", palette(0x2EF2B3, 0x0E1A1A, 0xE6FFF7, 0xFF4F8B)),
    ("ultraviolet", palette(0xB388FF, 0x17122B, 0xF0EBFF, 0xFFD166)),
    ("graphite-mono", palette(0xFFFFFF, 0x1A1A1A, 0xF2F2F2, 0xFF3B30)),
];

impl Default for Palette {
    fn default() -> Self {
        HOT_CORAL
    }
}

pub fn theme(name: &str) -> Option<Palette> {
    THEMES.iter().find(|(theme, _)| *theme == name).map(|(_, palette)| *palette)
}

/// The built-in spelling of a theme's name, along with its colors.
pub fn named(name: &str) -> Option<(&'static str, Palette)> {
    THEMES.iter().find(|(theme, _)| *theme == name).copied()
}

/// `hot-coral` as it reads in the settings panel: `Hot Coral`.
pub fn label(name: &str) -> String {
    let words = name.split('-').map(|word| {
        let mut letters = word.chars();
        letters.next().map_or_else(String::new, |first| first.to_uppercase().chain(letters).collect())
    });
    words.collect::<Vec<_>>().join(" ")
}

pub fn theme_names() -> String {
    THEMES.map(|(name, _)| name).join(", ")
}

/// Colors from `[colors]` in the config file, which win over any theme.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Overrides {
    pub selection: Option<u32>,
    pub surface: Option<u32>,
    pub text: Option<u32>,
    pub annotation: Option<u32>,
}

impl Overrides {
    pub fn apply(&self, theme: Palette) -> Palette {
        Palette {
            selection: self.selection.unwrap_or(theme.selection),
            surface: self.surface.unwrap_or(theme.surface),
            text: self.text.unwrap_or(theme.text),
            annotation: self.annotation.unwrap_or(theme.annotation),
        }
    }
}

// All drawing happens on the GTK thread.
thread_local! {
    static ACTIVE: Cell<Palette> = const { Cell::new(HOT_CORAL) };
}

pub fn set(palette: Palette) {
    ACTIVE.set(palette);
}

pub fn active() -> Palette {
    ACTIVE.get()
}

/// Reads a color written as `#RRGGBB`.
pub fn parse_hex(text: &str) -> Option<u32> {
    let digits = text.strip_prefix('#')?;
    if digits.len() != 6 || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    u32::from_str_radix(digits, 16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_theme_is_found_by_name_and_distinct() {
        for (name, palette) in THEMES {
            assert_eq!(theme(name), Some(palette));
            assert!(theme_names().contains(name));
            assert_eq!(THEMES.iter().filter(|(_, other)| *other == palette).count(), 1);
        }
        assert_eq!(theme("sparkle"), None);
    }

    #[test]
    fn labels_read_as_words() {
        assert_eq!(label("hot-coral"), "Hot Coral");
        assert_eq!(label("ultraviolet"), "Ultraviolet");
    }

    #[test]
    fn overrides_replace_only_what_they_name() {
        let overrides = Overrides { text: Some(1), ..Overrides::default() };
        assert_eq!(overrides.apply(HOT_CORAL), Palette { text: 1, ..HOT_CORAL });
    }

    #[test]
    fn the_default_is_hot_coral() {
        assert_eq!(theme("hot-coral"), Some(Palette::default()));
    }

    #[test]
    fn reads_hex_colors_in_either_case() {
        assert_eq!(parse_hex("#FF4D6D"), Some(0xFF4D6D));
        assert_eq!(parse_hex("#ff4d6d"), Some(0xFF4D6D));
    }

    #[test]
    fn rejects_anything_else() {
        for bad in ["FF4D6D", "#FFF", "#FF4D6D00", "#GG4D6D", "#+F4D6D", "red", "", "#ÿÿÿ"] {
            assert_eq!(parse_hex(bad), None, "{bad}");
        }
    }
}
