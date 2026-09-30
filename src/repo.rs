//! Registered repos. A repo is registered when `<root>/<id>/@repo` resolves to
//! its original clone; that folder is the whole registry.

use crate::{git, name};
use anyhow::{Context, Result, anyhow, bail};
use std::path::{Path, PathBuf};

pub const REPO_LINK: &str = "@repo";

pub struct Repo {
    /// The original clone (never modified by grove, apart from `git fetch`).
    pub clone: PathBuf,
    /// The repo's id: its directory name under the grove root.
    pub name: String,
    /// `<root>/<name>`.
    pub dir: PathBuf,
}

/// A `<root>/<id>/@repo` entry; `clone` is `None` when the link dangles.
pub struct Entry {
    pub name: String,
    pub dir: PathBuf,
    pub link_target: PathBuf,
    pub clone: Option<PathBuf>,
}

impl Entry {
    pub fn repo(&self) -> Option<Repo> {
        Some(Repo {
            clone: self.clone.clone()?,
            name: self.name.clone(),
            dir: self.dir.clone(),
        })
    }
}

/// Every registered repo under `root`, sorted by id.
pub fn entries(root: &Path) -> Result<Vec<Entry>> {
    let Ok(read) = std::fs::read_dir(root) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for dirent in read {
        let dir = dirent?.path();
        let link = dir.join(REPO_LINK);
        let Ok(link_target) = std::fs::read_link(&link) else {
            continue;
        };
        out.push(Entry {
            name: dir.file_name().unwrap().to_string_lossy().into_owned(),
            clone: link.canonicalize().ok(),
            link_target,
            dir,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// The registered repo whose `@repo` points at `clone`, if any.
pub fn lookup(root: &Path, clone: &Path) -> Result<Option<Repo>> {
    let clone = clone.canonicalize()?;
    Ok(entries(root)?
        .iter()
        .find(|e| e.clone.as_deref() == Some(clone.as_path()))
        .and_then(Entry::repo))
}

/// `--repo <id>`.
pub fn by_name(root: &Path, id: &str) -> Result<Repo> {
    let all = entries(root)?;
    let Some(entry) = all.iter().find(|e| e.name == id) else {
        if all.is_empty() {
            bail!("no repo `{id}`; none are registered yet (run `grove repo add` in a clone)");
        }
        let names: Vec<&str> = all.iter().map(|e| e.name.as_str()).collect();
        bail!("no repo `{id}`; registered: {}", names.join(", "));
    };
    entry.repo().ok_or_else(|| {
        anyhow!(
            "repo `{id}` points at {}, which is missing",
            entry.link_target.display()
        )
    })
}

/// The registered repo for `cwd`: (1) any git checkout → its original clone,
/// (2) somewhere under `<root>/<id>/` → that folder. Errors if unregistered.
pub fn resolve(cwd: &Path, root: &Path) -> Result<Repo> {
    match current(cwd, root)? {
        Current::Registered(repo) => Ok(repo),
        Current::Unregistered(clone) => bail!("{}", not_registered(&clone)),
        Current::Outside => bail!(
            "not in a registered repo; use `--repo <name>`, or run `grove repo add` in a clone"
        ),
    }
}

pub enum Current {
    Registered(Repo),
    /// Inside a clone (or one of its worktrees) that isn't registered.
    Unregistered(PathBuf),
    Outside,
}

pub fn current(cwd: &Path, root: &Path) -> Result<Current> {
    if let Some(clone) = clone_from_git(cwd) {
        return Ok(match lookup(root, &clone)? {
            Some(repo) => Current::Registered(repo),
            None => Current::Unregistered(clone),
        });
    }
    Ok(from_root_folder(cwd, root).map_or(Current::Outside, Current::Registered))
}

pub fn not_registered(clone: &Path) -> String {
    format!(
        "{} isn't set up with grove yet. Run: grove repo add",
        basename(clone)
    )
}

fn basename(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
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

/// `cwd` under `<root>/<id>/` (e.g. the repo folder itself) → that repo.
fn from_root_folder(cwd: &Path, root: &Path) -> Option<Repo> {
    let (cwd, root) = (cwd.canonicalize().ok()?, root.canonicalize().ok()?);
    let id = cwd.strip_prefix(&root).ok()?.components().next()?;
    let dir = root.join(id);
    Some(Repo {
        clone: dir.join(REPO_LINK).canonicalize().ok()?,
        name: id.as_os_str().to_string_lossy().into_owned(),
        dir,
    })
}

/// `grove repo add [path] [--name <id>]`. Returns the repo and whether it was
/// newly registered.
pub fn add(cwd: &Path, root: &Path, path: Option<&Path>, id: Option<&str>) -> Result<(Repo, bool)> {
    let target = path.map_or_else(|| cwd.to_path_buf(), |p| cwd.join(p));
    if !target.is_dir() {
        bail!("{} is not a directory", target.display());
    }
    let clone = match clone_from_git(&target) {
        Some(c) => c,
        None => from_root_folder(&target, root)
            .map(|r| r.clone)
            .ok_or_else(|| anyhow!("{} is not inside a git clone", target.display()))?,
    };
    let wanted = id
        .map(|i| name::normalize(i).map(|n| n.folder))
        .transpose()?;

    if let Some(existing) = lookup(root, &clone)? {
        if let Some(w) = wanted.filter(|w| *w != existing.name) {
            bail!(
                "{} is already registered as `{}`; rename it with `grove repo mv {w}`",
                clone.display(),
                existing.name
            );
        }
        return Ok((existing, false));
    }

    let id = match wanted {
        Some(w) => w,
        None => name::normalize(&basename(&clone))?.folder,
    };
    let dir = root.join(&id);
    let link = dir.join(REPO_LINK);
    if std::fs::symlink_metadata(&link).is_ok() {
        let owner = link
            .canonicalize()
            .unwrap_or_else(|_| std::fs::read_link(&link).unwrap_or_default());
        bail!(
            "`{id}` is taken by {}. Pick another: grove repo add --name {}",
            owner.display(),
            suggest(&clone, &id)
        );
    }
    if dir.exists() {
        bail!(
            "{} exists but isn't a grove repo folder; move it aside or pick another --name",
            dir.display()
        );
    }

    std::fs::create_dir_all(root).with_context(|| format!("creating {}", root.display()))?;
    let marker = root.join(".metadata_never_index");
    if !marker.exists() {
        std::fs::write(&marker, "")?;
    }
    std::fs::create_dir(&dir).with_context(|| format!("creating {}", dir.display()))?;
    std::os::unix::fs::symlink(&clone, &link)
        .with_context(|| format!("creating {}", link.display()))?;
    Ok((
        Repo {
            clone,
            name: id,
            dir,
        },
        true,
    ))
}

/// `<parent>-<id>`, e.g. `oss-api` for `~/oss/api`.
fn suggest(clone: &Path, id: &str) -> String {
    clone
        .parent()
        .and_then(|p| name::normalize(&format!("{}-{id}", basename(p))).ok())
        .map_or_else(|| format!("{id}-2"), |n| n.folder)
}
