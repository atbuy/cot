//! User preferences, kept in `~/.config/cot/config.toml`.

use std::fs;
use std::io;
use std::path::PathBuf;

/// What happens when the mouse is released after drawing a new selection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AfterSelect {
    #[default]
    Edit,
    Instant,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub after_select: AfterSelect,
    /// Whether to show the magnifier while drawing or resizing a selection.
    pub magnifier: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            after_select: AfterSelect::default(),
            magnifier: true,
        }
    }
}

impl Settings {
    fn parse(text: &str) -> Settings {
        let mut settings = Settings::default();
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else { continue };
            match (key.trim(), value.trim().trim_matches('"')) {
                ("after_select", "instant") => settings.after_select = AfterSelect::Instant,
                ("magnifier", "false") => settings.magnifier = false,
                _ => {}
            }
        }
        settings
    }

    fn serialize(&self) -> String {
        let after_select = match self.after_select {
            AfterSelect::Edit => "edit",
            AfterSelect::Instant => "instant",
        };
        format!("after_select = \"{after_select}\"\nmagnifier = {}\n", self.magnifier)
    }

    fn path() -> Option<PathBuf> {
        let config = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|dir| dir.is_absolute())
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
        Some(config.join("cot").join("config.toml"))
    }

    /// Reads the saved settings. A missing or unreadable file means defaults.
    pub fn load() -> Settings {
        Settings::path()
            .and_then(|path| fs::read_to_string(path).ok())
            .map_or_else(Settings::default, |text| Settings::parse(&text))
    }

    pub fn save(&self) -> io::Result<()> {
        let path = Settings::path().ok_or(io::ErrorKind::NotFound)?;
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(path, self.serialize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_edit() {
        assert_eq!(Settings::parse("").after_select, AfterSelect::Edit);
        assert_eq!(Settings::parse("nonsense\nafter_select = \"what\"").after_select, AfterSelect::Edit);
    }

    #[test]
    fn the_magnifier_is_on_unless_turned_off() {
        assert!(Settings::parse("after_select = \"edit\"\n").magnifier);
        assert!(!Settings::parse("magnifier = false\n").magnifier);
    }

    #[test]
    fn round_trips() {
        for after_select in [AfterSelect::Edit, AfterSelect::Instant] {
            for magnifier in [true, false] {
                let settings = Settings { after_select, magnifier };
                assert_eq!(Settings::parse(&settings.serialize()), settings);
            }
        }
    }
}
