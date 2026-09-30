//! Where grove keeps worktrees. Precedence: `GROVE_ROOT` > config `root` > `~/.grove`.

use anyhow::{Context, Result, bail};
use std::env;
use std::path::PathBuf;

pub fn home() -> Result<PathBuf> {
    match env::var_os("HOME") {
        Some(h) if !h.is_empty() => Ok(PathBuf::from(h)),
        _ => bail!("$HOME is not set"),
    }
}

/// `$XDG_CONFIG_HOME/grove/config.toml`, else `~/.config/grove/config.toml`
/// (deliberately not macOS's `~/Library/Application Support`).
fn config_path() -> Result<PathBuf> {
    let base = match env::var_os("XDG_CONFIG_HOME") {
        Some(x) if !x.is_empty() => PathBuf::from(x),
        _ => home()?.join(".config"),
    };
    Ok(base.join("grove").join("config.toml"))
}

fn expand_tilde(s: &str) -> Result<PathBuf> {
    if s == "~" {
        return home();
    }
    if let Some(rest) = s.strip_prefix("~/") {
        return Ok(home()?.join(rest));
    }
    Ok(PathBuf::from(s))
}

pub fn root() -> Result<PathBuf> {
    if let Some(r) = env::var_os("GROVE_ROOT")
        && !r.is_empty()
    {
        return expand_tilde(&r.to_string_lossy());
    }
    let path = config_path()?;
    if path.exists() {
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let table: toml::Table = text
            .parse()
            .with_context(|| format!("parsing {}", path.display()))?;
        if let Some(v) = table.get("root") {
            let Some(s) = v.as_str() else {
                bail!("{}: `root` must be a string", path.display());
            };
            return expand_tilde(s);
        }
    }
    Ok(home()?.join(".grove"))
}
