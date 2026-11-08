mod backend;
mod editor;
mod geometry;
mod image;
mod magnifier;
mod overlay;
mod palette;
mod render;
mod selection;
mod settings;
mod toolbar;
mod update;
mod version;
mod windows;
mod workspace;
mod x11;

use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::{self, ErrorKind, Write};
use std::os::linux::net::SocketAddrExt;
use std::os::unix::net::{SocketAddr, UnixListener};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::rc::Rc;

use clap::{Parser, Subcommand};
use gtk::glib;

use crate::backend::Backend;
use crate::image::Image;
use crate::settings::Config;
use crate::workspace::Action;

/// Freezes the screen, then lets you pick what to keep.
///
/// Drag to select a region, click a window to select it, or Ctrl+click for a
/// whole monitor. Adjust and annotate it from the toolbar, then press Enter or
/// Ctrl+C to copy it to the clipboard, or Ctrl+S to save it to
/// ~/Pictures/Screenshots. With nothing selected, these take all monitors.
/// Esc or right-click cancels. Colors and the save folder can be set in
/// ~/.config/cot/config.toml.
#[derive(Parser)]
#[command(version = version::VERSION, args_conflicts_with_subcommands = true)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Skip selection and take all monitors
    #[arg(long)]
    full: bool,

    /// Include the mouse cursor
    #[arg(short, long)]
    cursor: bool,

    /// Write the PNG to this path instead of the clipboard or screenshots folder
    #[arg(short, long, value_name = "PATH")]
    output: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Print version information
    Version,

    /// Manage this installation of cot
    #[command(name = "self", subcommand)]
    Manage(SelfCommand),
}

#[derive(Subcommand)]
enum SelfCommand {
    /// Update to the latest release
    Update {
        /// Install the latest release even if it isn't newer, or over a build from source
        #[arg(long)]
        force: bool,
    },
}

/// Holds an abstract socket for the life of the process, so that a second
/// press of the keybind doesn't capture the first run's overlay.
fn acquire_instance_lock() -> io::Result<Option<UnixListener>> {
    let scope = std::env::var("XDG_RUNTIME_DIR").unwrap_or_default();
    let addr = SocketAddr::from_abstract_name(format!("cot:{scope}"))?;
    match UnixListener::bind_addr(&addr) {
        Ok(listener) => Ok(Some(listener)),
        Err(e) if e.kind() == ErrorKind::AddrInUse => Ok(None),
        Err(e) => Err(e),
    }
}

fn copy_to_clipboard(png: &[u8], backend: Backend) -> Result<(), Box<dyn Error>> {
    // The tool stays behind to serve the clipboard, so it must not inherit our
    // stdout: whoever is reading that pipe would never see it close.
    let (program, args) = backend.clipboard_command();
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("could not run {program}: {e}"))?;
    child.stdin.take().ok_or("the clipboard tool has no stdin")?.write_all(png)?;
    let status = child.wait()?;
    if !status.success() {
        return Err(format!("{program} failed: {status}").into());
    }
    Ok(())
}

/// Writes a new file named after `stem`, adding a number if that name is taken.
fn write_new_file(dir: &Path, stem: &str, bytes: &[u8]) -> io::Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let mut attempt = 0;
    loop {
        let name = match attempt {
            0 => format!("{stem}.png"),
            n => format!("{stem}-{n}.png"),
        };
        let path = dir.join(name);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                file.write_all(bytes)?;
                return Ok(path);
            }
            Err(e) if e.kind() == ErrorKind::AlreadyExists => attempt += 1,
            Err(e) => return Err(e),
        }
    }
}

fn save_screenshot(png: &[u8], folder: Option<&Path>) -> Result<(), Box<dyn Error>> {
    let folder = folder.ok_or("HOME is not set, so set save_folder in the config file")?;
    let stamp = glib::DateTime::now_local()?.format("%Y-%m-%d_%H-%M-%S")?;
    write_new_file(folder, &format!("cot-{stamp}"), png)
        .map_err(|e| format!("could not save to {}: {e}", folder.display()))?;
    Ok(())
}

fn run(args: Args) -> Result<(), Box<dyn Error>> {
    match args.command {
        Some(Commands::Version) => {
            println!("{}", version::describe());
            return Ok(());
        }
        Some(Commands::Manage(SelfCommand::Update { force })) => return update::self_update(force),
        None => {}
    }

    let Some(_lock) = acquire_instance_lock()? else {
        return Ok(());
    };

    let backend = Backend::detect()?;
    let frame = Rc::new(Image::capture(backend, args.cursor)?);
    let config = Config::load();
    palette::set(config.palette);
    let mut action = Action::Copy;
    let result = if args.full {
        frame
    } else {
        let Some((capture, chosen)) = overlay::select_region(frame.clone(), config.settings, backend)? else {
            return Ok(());
        };
        action = chosen;
        let mut cropped = frame.crop(&capture.region).ok_or("the selected region is empty")?;
        render::flatten(&mut cropped, &capture, &frame)?;
        Rc::new(cropped)
    };

    let png = result.to_png()?;
    match (args.output, action) {
        (Some(path), _) => {
            fs::write(&path, png).map_err(|e| format!("could not write {}: {e}", path.display()))?
        }
        (None, Action::Copy) => copy_to_clipboard(&png, backend)?,
        (None, Action::Save) => save_screenshot(&png, config.save_folder.as_deref())?,
    }
    Ok(())
}

fn main() -> ExitCode {
    match run(Args::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("cot: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder for one test, removed when the test ends.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            TempDir(std::env::temp_dir().join(format!("cot-test-{}-{name}", std::process::id())))
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn creates_missing_folders() {
        let temp = TempDir::new("create");
        let dir = temp.0.join("Pictures").join("Screenshots");
        let path = write_new_file(&dir, "shot", b"one").unwrap();
        assert_eq!(path, dir.join("shot.png"));
        assert_eq!(fs::read(path).unwrap(), b"one");
    }

    #[test]
    fn never_overwrites_an_existing_file() {
        let temp = TempDir::new("collide");
        let first = write_new_file(&temp.0, "shot", b"one").unwrap();
        let second = write_new_file(&temp.0, "shot", b"two").unwrap();
        let third = write_new_file(&temp.0, "shot", b"three").unwrap();
        assert_eq!(second, temp.0.join("shot-1.png"));
        assert_eq!(third, temp.0.join("shot-2.png"));
        assert_eq!(fs::read(first).unwrap(), b"one");
    }
}
