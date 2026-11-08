//! User preferences, kept in `~/.config/cot/config.toml`.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use toml_edit::{DocumentMut, Item, Value};

use crate::palette::{self, Palette};

/// What happens when the mouse is released after drawing a new selection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AfterSelect {
    #[default]
    Edit,
    Instant,
}

impl AfterSelect {
    fn as_str(self) -> &'static str {
        match self {
            AfterSelect::Edit => "edit",
            AfterSelect::Instant => "instant",
        }
    }
}

/// The preferences that can be changed from the toolbar.
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

/// Everything read from the config file.
#[derive(Debug, PartialEq, Eq)]
pub struct Config {
    pub settings: Settings,
    pub palette: Palette,
    /// `None` only if no folder is configured and `HOME` is not set.
    pub save_folder: Option<PathBuf>,
}

fn config_path() -> Option<PathBuf> {
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|dir| dir.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(config.join("cot").join("config.toml"))
}

fn default_save_folder(home: Option<&Path>) -> Option<PathBuf> {
    Some(home?.join("Pictures").join("Screenshots"))
}

/// Resolves a configured folder: absolute, or starting with `~/`.
fn expand_folder(text: &str, home: Option<&Path>) -> Option<PathBuf> {
    let path = match text.strip_prefix('~') {
        Some("") => home?.to_path_buf(),
        Some(rest) => home?.join(rest.strip_prefix('/')?),
        None => PathBuf::from(text),
    };
    path.is_absolute().then_some(path)
}

impl Config {
    /// Reads the config. Whatever is invalid keeps its default and is reported
    /// in the returned warnings, so a typo can't stop a screenshot.
    fn parse(text: &str, home: Option<&Path>) -> (Config, Vec<String>) {
        let mut config = Config {
            settings: Settings::default(),
            palette: Palette::default(),
            save_folder: default_save_folder(home),
        };
        let mut warnings = Vec::new();
        let doc = match text.parse::<DocumentMut>() {
            Ok(doc) => doc,
            Err(e) => {
                warnings.push(format!("the file is not valid TOML, using defaults: {e}"));
                return (config, warnings);
            }
        };

        for (key, item) in doc.iter() {
            let mut invalid = |expected: &str| warnings.push(format!("{key} must be {expected}"));
            match key {
                "after_select" => match item.as_str() {
                    Some("edit") => config.settings.after_select = AfterSelect::Edit,
                    Some("instant") => config.settings.after_select = AfterSelect::Instant,
                    _ => invalid("\"edit\" or \"instant\""),
                },
                "magnifier" => match item.as_bool() {
                    Some(on) => config.settings.magnifier = on,
                    None => invalid("true or false"),
                },
                "save_folder" => match item.as_str().and_then(|text| expand_folder(text, home)) {
                    Some(folder) => config.save_folder = Some(folder),
                    None => invalid("an absolute path or one starting with ~/"),
                },
                "colors" => read_colors(item, &mut config.palette, &mut warnings),
                _ => warnings.push(format!("unknown setting {key}")),
            }
        }
        (config, warnings)
    }

    pub fn load() -> Config {
        let text = config_path().and_then(|path| fs::read_to_string(path).ok()).unwrap_or_default();
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let (config, warnings) = Config::parse(&text, home.as_deref());
        for warning in warnings {
            eprintln!("cot: config: {warning}");
        }
        config
    }
}

fn read_colors(item: &Item, palette: &mut Palette, warnings: &mut Vec<String>) {
    let Some(table) = item.as_table_like() else {
        warnings.push("colors must be a table".into());
        return;
    };
    for (key, value) in table.iter() {
        let slot = match key {
            "selection" => &mut palette.selection,
            "surface" => &mut palette.surface,
            "text" => &mut palette.text,
            "annotation" => &mut palette.annotation,
            _ => {
                warnings.push(format!("unknown color {key}"));
                continue;
            }
        };
        match value.as_str().and_then(palette::parse_hex) {
            Some(color) => *slot = color,
            None => warnings.push(format!("colors.{key} must look like \"#RRGGBB\"")),
        }
    }
}

/// Changes a top-level value, leaving any comment on its line in place.
fn set_value(doc: &mut DocumentMut, key: &str, new: impl Into<Value>) {
    let mut new = new.into();
    match doc.get_mut(key).and_then(Item::as_value_mut) {
        Some(existing) => {
            *new.decor_mut() = existing.decor().clone();
            *existing = new;
        }
        None => doc[key] = Item::Value(new),
    }
}

impl Settings {
    /// Applies these settings to the text of a config file, keeping everything
    /// else in it as written.
    fn merged_into(&self, text: &str) -> Result<String, toml_edit::TomlError> {
        let mut doc = text.parse::<DocumentMut>()?;
        set_value(&mut doc, "after_select", self.after_select.as_str());
        set_value(&mut doc, "magnifier", self.magnifier);
        Ok(doc.to_string())
    }

    pub fn save(&self) -> io::Result<()> {
        let path = config_path().ok_or(io::ErrorKind::NotFound)?;
        let existing = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(e),
        };
        // A broken file is left for the user to fix rather than replaced.
        let merged = self.merged_into(&existing).map_err(io::Error::other)?;
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        // Written in place, as the file may be a symlink into a dotfiles repo.
        fs::write(path, merged)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn home() -> Option<&'static Path> {
        Some(Path::new("/home/me"))
    }

    fn parse(text: &str) -> (Config, Vec<String>) {
        Config::parse(text, home())
    }

    #[test]
    fn an_empty_file_means_defaults() {
        let (config, warnings) = parse("");
        assert!(warnings.is_empty());
        assert_eq!(config.settings, Settings::default());
        assert!(config.settings.magnifier);
        assert_eq!(config.settings.after_select, AfterSelect::Edit);
        assert_eq!(config.palette, Palette::default());
        assert_eq!(config.save_folder, Some(PathBuf::from("/home/me/Pictures/Screenshots")));
    }

    #[test]
    fn reads_every_setting() {
        let (config, warnings) = parse(
            r##"
            after_select = "instant"  # copy straight away
            magnifier = false
            save_folder = "~/shots"

            [colors]
            selection = "#112233"
            surface = "#445566"
            text = "#778899"
            annotation = "#aabbcc"
            "##,
        );
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(config.settings, Settings { after_select: AfterSelect::Instant, magnifier: false });
        assert_eq!(config.save_folder, Some(PathBuf::from("/home/me/shots")));
        assert_eq!(
            config.palette,
            Palette { selection: 0x112233, surface: 0x445566, text: 0x778899, annotation: 0xAABBCC }
        );
    }

    #[test]
    fn colors_left_out_stay_at_their_defaults() {
        let (config, warnings) = parse("[colors]\nselection = \"#112233\"\n");
        assert!(warnings.is_empty());
        assert_eq!(config.palette, Palette { selection: 0x112233, ..Palette::default() });
    }

    #[test]
    fn a_bad_value_warns_and_keeps_the_default() {
        for text in [
            "after_select = \"sometimes\"",
            "after_select = 3",
            "magnifier = \"no\"",
            "save_folder = \"relative/path\"",
            "save_folder = \"~other/shots\"",
            "save_folder = 5",
            "colors = \"red\"",
            "[colors]\nselection = \"red\"",
            "[colors]\nsparkle = \"#112233\"",
            "typo = true",
        ] {
            let (config, warnings) = parse(text);
            assert_eq!(warnings.len(), 1, "{text}: {warnings:?}");
            assert_eq!(config.settings, Settings::default(), "{text}");
            assert_eq!(config.palette, Palette::default(), "{text}");
            assert_eq!(config.save_folder, default_save_folder(home()), "{text}");
        }
    }

    #[test]
    fn one_bad_value_does_not_affect_the_others() {
        let (config, warnings) = parse("magnifier = \"no\"\nafter_select = \"instant\"\n");
        assert_eq!(warnings.len(), 1);
        assert_eq!(config.settings.after_select, AfterSelect::Instant);
    }

    #[test]
    fn a_broken_file_warns_and_uses_defaults() {
        let (config, warnings) = parse("this is not toml");
        assert_eq!(warnings.len(), 1);
        assert_eq!(config.settings, Settings::default());
    }

    #[test]
    fn folders_can_be_absolute_or_under_home() {
        assert_eq!(expand_folder("/mnt/shots", home()), Some(PathBuf::from("/mnt/shots")));
        assert_eq!(expand_folder("~", home()), Some(PathBuf::from("/home/me")));
        assert_eq!(expand_folder("~/a/b", home()), Some(PathBuf::from("/home/me/a/b")));
        assert_eq!(expand_folder("~/a", None), None);
        assert_eq!(expand_folder("", home()), None);
    }

    #[test]
    fn no_home_and_no_folder_means_no_save_folder() {
        assert_eq!(Config::parse("", None).0.save_folder, None);
        let (config, _) = Config::parse("save_folder = \"/mnt/shots\"", None);
        assert_eq!(config.save_folder, Some(PathBuf::from("/mnt/shots")));
    }

    #[test]
    fn saved_settings_round_trip() {
        for after_select in [AfterSelect::Edit, AfterSelect::Instant] {
            for magnifier in [true, false] {
                let settings = Settings { after_select, magnifier };
                let text = settings.merged_into("").unwrap();
                assert_eq!(parse(&text).0.settings, settings);
            }
        }
    }

    #[test]
    fn saving_keeps_comments_and_other_settings() {
        let original = "# my screenshots\nmagnifier = true # keep this on\nsave_folder = \"~/shots\"\n\n[colors]\n# brand red\nselection = \"#112233\"\n";
        let settings = Settings { after_select: AfterSelect::Instant, magnifier: false };
        let merged = settings.merged_into(original).unwrap();
        assert!(merged.contains("# my screenshots"));
        assert!(merged.contains("magnifier = false # keep this on"), "{merged}");
        assert!(merged.contains("save_folder = \"~/shots\""));
        assert!(merged.contains("# brand red"));
        let (config, warnings) = parse(&merged);
        assert!(warnings.is_empty(), "{warnings:?}: {merged}");
        assert_eq!(config.settings, settings);
        assert_eq!(config.palette.selection, 0x112233);
    }

    #[test]
    fn saving_refuses_to_touch_a_broken_file() {
        assert!(Settings::default().merged_into("this is not toml").is_err());
    }
}
