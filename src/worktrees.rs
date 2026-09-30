//! Grove's own worktrees for a repo: those registered with git that live directly
//! under `<root>/<repo>/`. Other tools' worktrees and `@`-entries are ignored.

use crate::git;
use crate::repo::Repo;
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Worktree {
    /// Folder name under `<root>/<repo>/`.
    pub name: String,
    pub path: PathBuf,
    /// `None` for a detached HEAD.
    pub branch: Option<String>,
    /// `git worktree lock`ed.
    pub locked: bool,
}

pub fn list(repo: &Repo) -> Result<Vec<Worktree>> {
    let Ok(dir) = repo.dir.canonicalize() else {
        return Ok(Vec::new());
    };
    let porcelain = git::run(&repo.clone, &["worktree", "list", "--porcelain"])?;
    let mut out = Vec::new();
    for block in porcelain.split("\n\n") {
        let mut path = None;
        let mut branch = None;
        let mut locked = false;
        for line in block.lines() {
            if line == "locked" || line.starts_with("locked ") {
                locked = true;
            }
            if let Some(p) = line.strip_prefix("worktree ") {
                path = Some(PathBuf::from(p));
            } else if let Some(b) = line.strip_prefix("branch ") {
                branch = Some(b.strip_prefix("refs/heads/").unwrap_or(b).to_string());
            }
        }
        let Some(path) = path.and_then(|p| p.canonicalize().ok()) else {
            continue; // missing on disk; pruning is out of scope for v1
        };
        if path.parent() != Some(dir.as_path()) {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if name.starts_with('@') {
            continue;
        }
        out.push(Worktree {
            name,
            path,
            branch,
            locked,
        });
    }
    Ok(out)
}

/// The worktree containing `cwd`, if any.
pub fn current<'a>(worktrees: &'a [Worktree], cwd: &Path) -> Option<&'a Worktree> {
    let cwd = cwd.canonicalize().ok()?;
    worktrees.iter().find(|w| cwd.starts_with(&w.path))
}

pub struct Details {
    pub created: Option<SystemTime>,
    pub last_active: Option<SystemTime>,
    /// Entries in `git status --porcelain` (modified, staged and untracked).
    pub changed: usize,
}

impl Details {
    /// `clean` or `N changed`.
    pub fn status(&self) -> String {
        match self.changed {
            0 => "clean".into(),
            n => format!("{n} changed"),
        }
    }
}

impl Worktree {
    pub fn branch_label(&self) -> &str {
        self.branch.as_deref().unwrap_or("(detached)")
    }

    pub fn details(&self) -> Result<Details> {
        let changed_paths = self.changed_paths()?;
        let last_commit = git::run(&self.path, &["log", "-1", "--format=%ct"])
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .map(|s| UNIX_EPOCH + std::time::Duration::from_secs(s));
        // Only the files git reports — never walk the tree (node_modules etc).
        let last_active = changed_paths
            .iter()
            .filter_map(|p| {
                std::fs::symlink_metadata(self.path.join(p))
                    .ok()?
                    .modified()
                    .ok()
            })
            .chain(last_commit)
            .max();
        Ok(Details {
            created: created(&self.path),
            last_active,
            changed: changed_paths.len(),
        })
    }

    fn changed_paths(&self) -> Result<Vec<String>> {
        let out = git::output(&self.path, &["status", "--porcelain", "-z"])?;
        if !out.status.success() {
            anyhow::bail!(
                "git status failed in {}: {}",
                self.path.display(),
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        let text = String::from_utf8_lossy(&out.stdout);
        let mut fields = text.split('\0').filter(|f| !f.is_empty());
        let mut paths = Vec::new();
        while let Some(entry) = fields.next() {
            let (xy, path) = entry.split_at(entry.len().min(3));
            // Renames/copies carry the original path as an extra field.
            if xy.contains('R') || xy.contains('C') {
                fields.next();
            }
            paths.push(path.to_string());
        }
        Ok(paths)
    }
}

/// Folder birth time where the filesystem has one, else when git wrote the
/// worktree's `.git` file (which only `git worktree repair` rewrites).
fn created(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path)
        .and_then(|m| m.created())
        .or_else(|_| std::fs::metadata(path.join(".git")).and_then(|m| m.modified()))
        .ok()
}

/// The repo's original clone, dressed as a worktree for the global views.
pub fn clone_entry(repo: &Repo) -> Worktree {
    Worktree {
        name: crate::repo::REPO_LINK.to_string(),
        path: repo.clone.clone(),
        branch: git::run(&repo.clone, &["symbolic-ref", "--quiet", "--short", "HEAD"]).ok(),
        locked: false,
    }
}
