//! The colors in use, as 0xRRGGBB. Hot Coral unless the config file says
//! otherwise, and fixed once the program has read it.

use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    pub selection: u32,
    pub surface: u32,
    pub text: u32,
    pub annotation: u32,
}

impl Default for Palette {
    fn default() -> Self {
        Palette {
            selection: 0xFF4D6D,
            surface: 0x1C1216,
            text: 0xFFECEF,
            annotation: 0x4DD8FF,
        }
    }
}

static ACTIVE: OnceLock<Palette> = OnceLock::new();

pub fn set(palette: Palette) {
    let _ = ACTIVE.set(palette);
}

pub fn active() -> Palette {
    ACTIVE.get().copied().unwrap_or_default()
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
