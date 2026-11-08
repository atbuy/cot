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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Settings {
    pub after_select: AfterSelect,
}

impl Settings {
    fn parse(text: &str) -> Settings {
        let mut settings = Settings::default();
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else { continue };
            if key.trim() == "after_select" && value.trim().trim_matches('"') == "instant" {
                settings.after_select = AfterSelect::Instant;
            }
        }
        settings
    }

    fn serialize(&self) -> String {
        let after_select = match self.after_select {
            AfterSelect::Edit => "edit",
            AfterSelect::Instant => "instant",
        };
        format!("after_select = \"{after_select}\"\n")
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
    fn round_trips() {
        for after_select in [AfterSelect::Edit, AfterSelect::Instant] {
            let settings = Settings { after_select };
            assert_eq!(Settings::parse(&settings.serialize()), settings);
        }
    }
}
