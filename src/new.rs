//! `grove new <name>`.

use crate::{git, name, repo};
use anyhow::{Result, anyhow, bail};
use std::path::{Path, PathBuf};

pub fn run(cwd: &Path, root: &Path, input: &str) -> Result<PathBuf> {
    let name = name::normalize(input)?;
    let repo = repo::resolve(cwd, root)?;
    repo.ensure_dir(root)?;
    let path = repo.dir.canonicalize()?.join(&name.folder);

    if std::fs::symlink_metadata(&path).is_ok() {
        bail!("{} already exists", path.display());
    }

    let base = base_ref(&repo.clone)?;

    let has_ref = |r: &str| git::ok(&repo.clone, &["show-ref", "--verify", "--quiet", r]);
    if has_ref(&format!("refs/heads/{}", name.branch)) {
        bail!(
            "branch `{}` already exists in {}",
            name.branch,
            repo.clone.display()
        );
    }
    if has_ref(&format!("refs/remotes/origin/{}", name.branch)) {
        bail!(
            "branch `{}` already exists on origin (origin/{})",
            name.branch,
            name.branch
        );
    }

    git::run(
        &repo.clone,
        &[
            "worktree",
            "add",
            "--no-track",
            "-b",
            &name.branch,
            &path.to_string_lossy(),
            &base,
        ],
    )?;
    eprintln!("branch: {}", name.branch);
    eprintln!("folder: {}", name.folder);
    eprintln!("base:   {base}");
    Ok(path)
}

/// Fetch origin and return `origin/<default>`. Offline or without an origin,
/// warn and fall back to the local default branch. Never touches local branches.
fn base_ref(clone: &Path) -> Result<String> {
    if !git::ok(clone, &["remote", "get-url", "origin"]) {
        let local = local_default_branch(clone)?;
        eprintln!("warning: no `origin` remote; branching from local `{local}`");
        return Ok(local);
    }
    eprintln!("fetching origin…");
    if let Err(e) = git::run(clone, &["fetch", "origin"]) {
        let local = local_default_branch(clone)?;
        eprintln!("warning: could not fetch origin ({e:#}); branching from local `{local}`");
        return Ok(local);
    }
    let default = match origin_head(clone) {
        Some(b) => b,
        None => remote_head(clone).ok_or_else(|| {
            anyhow!("can't tell origin's default branch; try `git remote set-head origin --auto`")
        })?,
    };
    Ok(format!("origin/{default}"))
}

/// origin's default branch as recorded locally by `origin/HEAD`.
fn origin_head(clone: &Path) -> Option<String> {
    let r = git::run(
        clone,
        &[
            "symbolic-ref",
            "--quiet",
            "--short",
            "refs/remotes/origin/HEAD",
        ],
    )
    .ok()?;
    r.strip_prefix("origin/").map(str::to_string)
}

/// origin's default branch, asked over the network.
fn remote_head(clone: &Path) -> Option<String> {
    let out = git::run(clone, &["ls-remote", "--symref", "origin", "HEAD"]).ok()?;
    out.lines()
        .find_map(|l| l.strip_prefix("ref: refs/heads/"))
        .and_then(|l| l.split('\t').next())
        .map(str::to_string)
}

/// Best local guess at the default branch: whatever origin/HEAD names, else
/// `main`/`master`, else the clone's current branch.
fn local_default_branch(clone: &Path) -> Result<String> {
    let exists = |b: &str| {
        git::ok(
            clone,
            &[
                "show-ref",
                "--verify",
                "--quiet",
                &format!("refs/heads/{b}"),
            ],
        )
    };
    if let Some(b) = origin_head(clone).filter(|b| exists(b)) {
        return Ok(b);
    }
    for b in ["main", "master"] {
        if exists(b) {
            return Ok(b.to_string());
        }
    }
    git::run(clone, &["symbolic-ref", "--quiet", "--short", "HEAD"])
        .map_err(|_| anyhow!("can't find a local default branch to branch from"))
}
