//! Turning user input into a branch name and a folder name.

use anyhow::{Result, bail};

#[derive(Debug, PartialEq, Eq)]
pub struct Name {
    /// Git branch; may contain `/` (e.g. `feat/auth`).
    pub branch: String,
    /// Folder under `<root>/<repo>/`; `/` flattened to `-` (e.g. `feat-auth`).
    pub folder: String,
}

/// Lowercase; anything outside `[a-z0-9_/-]` becomes `-`; dashes collapse;
/// empty `/` segments and edge dashes are dropped.
pub fn normalize(input: &str) -> Result<Name> {
    let input = input.trim();
    if input.starts_with('@') {
        bail!("names starting with `@` are reserved for grove (got `{input}`)");
    }
    let mapped: String = input
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'a'..='z' | '0'..='9' | '_' | '/' => c,
            _ => '-',
        })
        .collect();
    let segments: Vec<String> = mapped
        .split('/')
        .map(|seg| {
            seg.split('-')
                .filter(|p| !p.is_empty())
                .collect::<Vec<_>>()
                .join("-")
        })
        .filter(|seg| !seg.is_empty())
        .collect();
    if segments.is_empty() {
        bail!("`{input}` is not a usable name");
    }
    Ok(Name {
        branch: segments.join("/"),
        folder: segments.join("-"),
    })
}
