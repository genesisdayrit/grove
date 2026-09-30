//! Figuring out which original clone we're talking about, and its home under the grove root.

use crate::git;
use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};

pub const REPO_LINK: &str = "@repo";

pub struct Repo {
    /// The original clone (never modified by grove, apart from `git fetch`).
    pub clone: PathBuf,
    /// Basename of the clone; also the directory name under the grove root.
    pub name: String,
    /// `<root>/<name>`.
    pub dir: PathBuf,
}

/// Resolve the repo for `cwd`: (1) any git checkout → its original clone,
/// (2) somewhere under `<root>/<repo>/` → follow `@repo`, (3) error.
pub fn resolve(cwd: &Path, root: &Path) -> Result<Repo> {
    let clone = match clone_from_git(cwd) {
        Some(c) => c,
        None => clone_from_root(cwd, root)?,
    };
    let name = clone
        .file_name()
        .context("original clone has no directory name")?
        .to_string_lossy()
        .into_owned();
    let dir = root.join(&name);
    Ok(Repo { clone, name, dir })
}

fn clone_from_git(cwd: &Path) -> Option<PathBuf> {
    let common = git::run(
        cwd,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )
    .ok()
    // git < 2.31 has no --path-format; resolve the (possibly relative) path ourselves.
    .or_else(|| {
        git::run(cwd, &["rev-parse", "--git-common-dir"])
            .ok()
            .map(|p| cwd.join(p).to_string_lossy().into_owned())
    })?;
    let common = PathBuf::from(common).canonicalize().ok()?;
    if common.file_name().is_some_and(|n| n == ".git") {
        common.parent().map(Path::to_path_buf)
    } else {
        // A bare repo has no worktree of its own to call "the clone".
        None
    }
}

fn clone_from_root(cwd: &Path, root: &Path) -> Result<PathBuf> {
    let not_a_repo = || anyhow::anyhow!("not in a git repo (or a grove worktree folder)");
    let (Ok(cwd), Ok(root)) = (cwd.canonicalize(), root.canonicalize()) else {
        return Err(not_a_repo());
    };
    let rel = cwd.strip_prefix(&root).map_err(|_| not_a_repo())?;
    let repo = rel.components().next().ok_or_else(not_a_repo)?;
    let link = root.join(repo).join(REPO_LINK);
    link.canonicalize()
        .with_context(|| format!("{} is missing or dangling", link.display()))
}

impl Repo {
    /// Create `<root>/<repo>/` and its `@repo` link, or verify an existing link
    /// points at this clone (two clones with the same basename would collide).
    pub fn ensure_dir(&self, root: &Path) -> Result<()> {
        std::fs::create_dir_all(root).with_context(|| format!("creating {}", root.display()))?;
        let marker = root.join(".metadata_never_index");
        if !marker.exists() {
            std::fs::write(&marker, "")?;
        }
        std::fs::create_dir_all(&self.dir)
            .with_context(|| format!("creating {}", self.dir.display()))?;

        let link = self.dir.join(REPO_LINK);
        match std::fs::symlink_metadata(&link) {
            Err(_) => std::os::unix::fs::symlink(&self.clone, &link)
                .with_context(|| format!("creating {}", link.display()))?,
            Ok(_) => {
                let target = link.canonicalize().with_context(|| {
                    format!("{} is dangling; remove it and retry", link.display())
                })?;
                if target != self.clone.canonicalize()? {
                    bail!(
                        "{} already belongs to {} (another clone named `{}`), not {}",
                        self.dir.display(),
                        target.display(),
                        self.name,
                        self.clone.display()
                    );
                }
            }
        }
        Ok(())
    }
}
