//! `cot self update`: replaces this binary with the latest GitHub release.

use std::error::Error;
use std::fs::{self, DirBuilder};
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::version;

const REPO: &str = "atbuy/cot";

type Version = (u32, u32, u32);

/// Reads `v1.2.3` or `1.2.3`.
fn parse_version(text: &str) -> Option<Version> {
    let mut parts = text.strip_prefix('v').unwrap_or(text).split('.').map(|part| part.parse().ok());
    let version = (parts.next()??, parts.next()??, parts.next()??);
    parts.next().is_none().then_some(version)
}

/// Whether `latest` should replace `current`. A current version that can't be
/// read, such as a prerelease, is replaced.
fn is_newer(latest: &str, current: &str) -> Option<bool> {
    let latest = parse_version(latest)?;
    Some(parse_version(current).is_none_or(|current| latest > current))
}

fn os_release_field<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines()
        .filter_map(|line| line.split_once('='))
        .find(|(name, _)| *name == key)
        .map(|(_, value)| value.trim_matches(|c| c == '"' || c == '\''))
}

/// The release archive for this system. GTK is linked dynamically, so each
/// distribution release gets its own build.
fn asset_name(os_release: &str, arch: &str) -> Option<String> {
    let distro = match os_release_field(os_release, "ID")? {
        "arch" => "arch".to_string(),
        "ubuntu" => format!("ubuntu-{}", os_release_field(os_release, "VERSION_ID")?),
        _ => return None,
    };
    Some(format!("cot-{distro}-{arch}.tar.gz"))
}

fn checksum_for<'a>(checksums: &'a str, file: &str) -> Option<&'a str> {
    checksums.lines().find_map(|line| {
        let (hash, name) = line.split_once(char::is_whitespace)?;
        (name.trim_start().trim_start_matches('*') == file).then_some(hash)
    })
}

fn tag_from_release_json(json: &[u8]) -> Result<String, Box<dyn Error>> {
    let release: serde_json::Value = serde_json::from_slice(json)?;
    let tag = release["tag_name"].as_str().ok_or("the release has no tag")?;
    Ok(tag.to_string())
}

fn output_of(command: &mut Command, what: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let output = command
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("could not run {what}: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("{what} failed: {}", stderr.trim()).into());
    }
    Ok(output.stdout)
}

fn curl() -> Command {
    let mut curl = Command::new("curl");
    curl.args(["-fsSL", "--proto", "=https", "--tlsv1.2"]);
    curl
}

fn download(url: &str, destination: &Path) -> Result<(), Box<dyn Error>> {
    output_of(curl().arg("-o").arg(destination).arg(url), &format!("downloading {url}"))?;
    Ok(())
}

/// A private temporary folder, removed when dropped.
struct WorkDir(PathBuf);

impl WorkDir {
    fn new() -> std::io::Result<Self> {
        let path = std::env::temp_dir().join(format!("cot-update-{}", std::process::id()));
        DirBuilder::new().mode(0o700).create(&path)?;
        Ok(WorkDir(path))
    }
}

impl Drop for WorkDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn verify_checksum(file: &Path, name: &str, checksums: &str) -> Result<(), Box<dyn Error>> {
    let expected = checksum_for(checksums, name).ok_or_else(|| format!("no checksum published for {name}"))?;
    let output = String::from_utf8(output_of(Command::new("sha256sum").arg(file), "sha256sum")?)?;
    let actual = output.split_whitespace().next().unwrap_or_default();
    if actual != expected {
        return Err(format!("checksum mismatch for {name}, not installing it").into());
    }
    Ok(())
}

/// Swaps the binary in one step, from a copy beside it so the rename is atomic.
fn replace_binary(new: &Path, target: &Path) -> Result<(), Box<dyn Error>> {
    let folder = target.parent().ok_or("the binary has no parent folder")?;
    let staged = folder.join(format!(".cot-update-{}", std::process::id()));
    let result = fs::copy(new, &staged)
        .and_then(|_| fs::set_permissions(&staged, fs::Permissions::from_mode(0o755)))
        .and_then(|()| fs::rename(&staged, target));
    if let Err(e) = result {
        let _ = fs::remove_file(&staged);
        return Err(format!("could not replace {}: {e}", target.display()).into());
    }
    Ok(())
}

pub fn self_update(force: bool) -> Result<(), Box<dyn Error>> {
    if !version::IS_RELEASE && !force {
        return Err("this copy was built from source. Run `git pull && make install`, \
                    or use --force to replace it with the latest release"
            .into());
    }

    let api = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let tag = tag_from_release_json(&output_of(curl().arg(&api), "checking for the latest release")?)?;
    let newer = is_newer(&tag, version::VERSION).ok_or_else(|| format!("could not read release tag {tag}"))?;
    if !newer && !force {
        println!("cot {} is up to date", version::VERSION);
        return Ok(());
    }

    let os_release = fs::read_to_string("/etc/os-release").unwrap_or_default();
    let asset = asset_name(&os_release, std::env::consts::ARCH)
        .ok_or("there is no prebuilt release for this system, so build from source instead")?;
    let target = std::env::current_exe()?.canonicalize()?;

    println!("Updating cot {} to {tag}...", version::VERSION);
    let work = WorkDir::new()?;
    let base = format!("https://github.com/{REPO}/releases/download/{tag}");
    let archive = work.0.join(&asset);
    let checksums = work.0.join("checksums.txt");
    download(&format!("{base}/{asset}"), &archive)?;
    download(&format!("{base}/checksums.txt"), &checksums)?;
    verify_checksum(&archive, &asset, &fs::read_to_string(&checksums)?)?;

    output_of(Command::new("tar").arg("-xzf").arg(&archive).arg("-C").arg(&work.0), "tar")?;
    let new = work.0.join("cot");
    // Catches a build this system can't run, such as a missing library.
    output_of(Command::new(&new).arg("version"), "the new version")?;
    replace_binary(&new, &target)?;
    println!("Updated to {tag}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_versions_with_or_without_the_v() {
        assert_eq!(parse_version("v1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_version("0.10.0"), Some((0, 10, 0)));
    }

    #[test]
    fn rejects_other_version_shapes() {
        for bad in ["", "v1.2", "1.2.3.4", "v1.2.3-rc1", "latest"] {
            assert_eq!(parse_version(bad), None, "{bad}");
        }
    }

    #[test]
    fn compares_numerically() {
        assert_eq!(is_newer("v0.10.0", "v0.9.0"), Some(true));
        assert_eq!(is_newer("v1.0.0", "v1.0.0"), Some(false));
        assert_eq!(is_newer("v0.9.0", "v1.0.0"), Some(false));
    }

    #[test]
    fn a_prerelease_build_can_be_updated() {
        assert_eq!(is_newer("v1.0.0", "v1.0.0-rc1"), Some(true));
    }

    #[test]
    fn an_unreadable_release_tag_is_an_error() {
        assert_eq!(is_newer("nightly", "v1.0.0"), None);
    }

    #[test]
    fn picks_the_archive_for_the_distribution() {
        assert_eq!(asset_name("NAME=\"Arch Linux\"\nID=arch\n", "x86_64").unwrap(), "cot-arch-x86_64.tar.gz");
        assert_eq!(
            asset_name("ID=ubuntu\nID_LIKE=debian\nVERSION_ID=\"26.04\"\n", "x86_64").unwrap(),
            "cot-ubuntu-26.04-x86_64.tar.gz"
        );
    }

    #[test]
    fn other_systems_have_no_archive() {
        assert_eq!(asset_name("ID=fedora\nVERSION_ID=43\n", "x86_64"), None);
        assert_eq!(asset_name("ID=ubuntu\n", "x86_64"), None);
        assert_eq!(asset_name("", "x86_64"), None);
    }

    #[test]
    fn finds_a_checksum_by_exact_file_name() {
        let list = "aaa  cot-arch-x86_64.tar.gz\nbbb *cot-ubuntu-26.04-x86_64.tar.gz\n";
        assert_eq!(checksum_for(list, "cot-arch-x86_64.tar.gz"), Some("aaa"));
        assert_eq!(checksum_for(list, "cot-ubuntu-26.04-x86_64.tar.gz"), Some("bbb"));
        assert_eq!(checksum_for(list, "x86_64.tar.gz"), None);
    }

    #[test]
    fn reads_the_tag_from_the_release_response() {
        assert_eq!(tag_from_release_json(br#"{"tag_name": "v1.2.3", "draft": false}"#).unwrap(), "v1.2.3");
        assert!(tag_from_release_json(br#"{"message": "Not Found"}"#).is_err());
    }
}
