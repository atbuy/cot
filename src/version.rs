//! What this binary is. Release builds get these from the release pipeline.

/// The release tag, or the crate version for a build from source.
pub const VERSION: &str = match option_env!("COT_VERSION") {
    Some(version) => version,
    None => env!("CARGO_PKG_VERSION"),
};

pub const IS_RELEASE: bool = option_env!("COT_VERSION").is_some();

const COMMIT: Option<&str> = option_env!("COT_COMMIT");

pub fn describe() -> String {
    match (IS_RELEASE, COMMIT) {
        (true, Some(commit)) => format!("cot {VERSION}\ncommit {commit}"),
        (true, None) => format!("cot {VERSION}"),
        (false, _) => format!("cot {VERSION} (built from source)"),
    }
}
