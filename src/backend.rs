//! Which display system this session runs on.

use std::ffi::OsStr;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    Wayland,
    X11,
}

impl Backend {
    pub fn detect() -> Result<Backend, String> {
        Self::from_env(std::env::var_os("WAYLAND_DISPLAY").as_deref(), std::env::var_os("DISPLAY").as_deref())
    }

    /// Wayland goes first, as `DISPLAY` is also set under XWayland. This is
    /// the same order GTK tries them in.
    fn from_env(wayland: Option<&OsStr>, x11: Option<&OsStr>) -> Result<Backend, String> {
        let set = |value: Option<&OsStr>| value.is_some_and(|v| !v.is_empty());
        if set(wayland) {
            Ok(Backend::Wayland)
        } else if set(x11) {
            Ok(Backend::X11)
        } else {
            Err("no display found: neither WAYLAND_DISPLAY nor DISPLAY is set".into())
        }
    }

    /// The program that keeps serving the clipboard after cot exits.
    pub fn clipboard_command(self) -> (&'static str, &'static [&'static str]) {
        match self {
            Backend::Wayland => ("wl-copy", &["--type", "image/png"]),
            Backend::X11 => ("xclip", &["-selection", "clipboard", "-target", "image/png", "-in"]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn detect(wayland: Option<&str>, x11: Option<&str>) -> Result<Backend, String> {
        Backend::from_env(wayland.map(OsStr::new), x11.map(OsStr::new))
    }

    #[test]
    fn wayland_wins_under_xwayland() {
        assert_eq!(detect(Some("wayland-1"), Some(":0")), Ok(Backend::Wayland));
    }

    #[test]
    fn display_alone_means_x11() {
        assert_eq!(detect(None, Some(":0")), Ok(Backend::X11));
    }

    #[test]
    fn empty_values_count_as_unset() {
        assert_eq!(detect(Some(""), Some(":1")), Ok(Backend::X11));
        assert!(detect(Some(""), Some("")).is_err());
    }

    #[test]
    fn no_display_is_an_error() {
        assert!(detect(None, None).is_err());
    }
}
