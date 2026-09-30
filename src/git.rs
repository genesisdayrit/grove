//! Thin wrapper over the `git` CLI. We shell out everywhere so the user's
//! credential helpers / SSH agents just work.

use anyhow::{Result, bail};
use std::path::Path;
use std::process::{Command, Output};

/// Run git in `dir`, returning the raw output (never inherits our stdout).
pub fn output(dir: &Path, args: &[&str]) -> Result<Output> {
    Ok(Command::new("git").current_dir(dir).args(args).output()?)
}

/// Run git in `dir`; on success return trimmed stdout, otherwise an error carrying git's stderr.
pub fn run(dir: &Path, args: &[&str]) -> Result<String> {
    let out = output(dir, args)?;
    if !out.status.success() {
        bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// True if `refname` (e.g. `refs/heads/main`) exists.
pub fn has_ref(dir: &Path, refname: &str) -> bool {
    ok(dir, &["show-ref", "--verify", "--quiet", refname])
}

/// True if git exits successfully.
pub fn ok(dir: &Path, args: &[&str]) -> bool {
    output(dir, args)
        .map(|o| o.status.success())
        .unwrap_or(false)
}
