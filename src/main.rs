mod editor;
mod geometry;
mod image;
mod overlay;
mod palette;
mod render;
mod selection;
mod settings;
mod toolbar;
mod workspace;

use std::error::Error;
use std::io::{self, ErrorKind, Write};
use std::os::linux::net::SocketAddrExt;
use std::os::unix::net::{SocketAddr, UnixListener};
use std::path::PathBuf;
use std::process::{Command, ExitCode, Stdio};
use std::rc::Rc;

use clap::Parser;

use crate::image::Image;

/// Freezes the screen, then lets you pick what to keep.
///
/// Drag to select a region, or click to select a whole monitor. Adjust it and
/// annotate it from the toolbar, then press Enter to copy it to the clipboard.
/// With nothing selected, Enter takes all monitors. Esc or right-click cancels.
#[derive(Parser)]
#[command(version)]
struct Args {
    /// Skip selection and take all monitors
    #[arg(long)]
    full: bool,

    /// Include the mouse cursor
    #[arg(short, long)]
    cursor: bool,

    /// Write a PNG to this path instead of the clipboard
    #[arg(short, long, value_name = "PATH")]
    output: Option<PathBuf>,
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

fn copy_to_clipboard(png: &[u8]) -> Result<(), Box<dyn Error>> {
    // wl-copy stays behind to serve the clipboard, so it must not inherit our
    // stdout: whoever is reading that pipe would never see it close.
    let mut child = Command::new("wl-copy")
        .args(["--type", "image/png"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("could not run wl-copy: {e}"))?;
    child.stdin.take().ok_or("wl-copy has no stdin")?.write_all(png)?;
    let status = child.wait()?;
    if !status.success() {
        return Err(format!("wl-copy failed: {status}").into());
    }
    Ok(())
}

fn run(args: Args) -> Result<(), Box<dyn Error>> {
    let Some(_lock) = acquire_instance_lock()? else {
        return Ok(());
    };

    let frame = Rc::new(Image::capture(args.cursor)?);
    let result = if args.full {
        frame
    } else {
        let Some(capture) = overlay::select_region(frame.clone())? else {
            return Ok(());
        };
        let mut cropped = frame.crop(&capture.region).ok_or("the selected region is empty")?;
        render::flatten(&mut cropped, &capture, &frame)?;
        Rc::new(cropped)
    };

    let png = result.to_png()?;
    match args.output {
        Some(path) => std::fs::write(&path, png)
            .map_err(|e| format!("could not write {}: {e}", path.display()))?,
        None => copy_to_clipboard(&png)?,
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
