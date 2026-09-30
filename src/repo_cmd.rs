//! `grove repo ls` and `grove repo mv`.

use crate::nav::{ago, print_table};
use crate::repo::{self, Repo};
use crate::{env, git, name, worktrees};
use anyhow::{Result, bail};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub fn ls(root: &Path, cwd: &Path, names_only: bool) -> Result<()> {
    let entries = repo::entries(root)?;
    if names_only {
        for e in &entries {
            println!("{}", e.name);
        }
        return Ok(());
    }
    if entries.is_empty() {
        eprintln!("no repos registered yet; run `grove repo add` in a clone");
        return Ok(());
    }
    let here = match repo::current(cwd, root)? {
        repo::Current::Registered(r) => Some(r.name),
        _ => None,
    };
    let home = crate::config::home().ok();
    let mut rows = Vec::new();
    for e in &entries {
        let (count, last_active, setup) = match e.repo() {
            Some(r) => {
                let wts = worktrees::list(&r)?;
                let last = wts
                    .iter()
                    .map(|w| w.details().map(|d| d.last_active))
                    .collect::<Result<Vec<_>>>()?
                    .into_iter()
                    .flatten()
                    .max();
                (wts.len().to_string(), last, setup_state(&r))
            }
            None => ("-".into(), None, "-"),
        };
        let clone = tilde(&e.link_target, home.as_deref());
        let clone = if e.clone.is_some() {
            clone
        } else {
            format!("{clone} (missing)")
        };
        rows.push((e, clone, count, last_active, setup));
    }
    rows.sort_by(|a, b| b.3.cmp(&a.3).then_with(|| a.0.name.cmp(&b.0.name)));

    let now = SystemTime::now();
    let mut table = vec![
        [" ", "NAME", "CLONE", "WORKTREES", "SETUP", "LAST ACTIVE"]
            .map(String::from)
            .to_vec(),
    ];
    for (e, clone, count, last, setup) in rows {
        let mark = if here.as_deref() == Some(e.name.as_str()) {
            "*"
        } else {
            " "
        };
        table.push(vec![
            mark.into(),
            e.name.clone(),
            clone,
            count,
            setup.into(),
            ago(now, last),
        ]);
    }
    print_table(&table);
    Ok(())
}

fn setup_state(repo: &Repo) -> &'static str {
    match std::fs::metadata(env::setup_script(repo)) {
        Err(_) => "no",
        Ok(m) if m.permissions().mode() & 0o111 == 0 => "not executable",
        Ok(_) => "yes",
    }
}

fn tilde(path: &Path, home: Option<&Path>) -> String {
    match home.and_then(|h| path.strip_prefix(h).ok()) {
        Some(rest) => format!("~/{}", rest.display()),
        None => path.display().to_string(),
    }
}

/// `grove repo mv <new>`: rename `<root>/<old>` and repair git's worktree links.
/// Returns where the caller should now be, if `cwd` was inside the moved folder.
pub fn mv(repo: &Repo, root: &Path, cwd: &Path, input: &str) -> Result<Option<PathBuf>> {
    let new = name::normalize(input)?.folder;
    if new == repo.name {
        bail!("{} is already called `{new}`", repo.clone.display());
    }
    let to = root.join(&new);
    if std::fs::symlink_metadata(&to).is_ok() {
        bail!("{} already exists; pick another name", to.display());
    }
    let from = repo.dir.canonicalize()?;
    let wts = worktrees::list(repo)?;
    let locked: Vec<&str> = wts
        .iter()
        .filter(|w| w.locked)
        .map(|w| w.name.as_str())
        .collect();
    if !locked.is_empty() {
        bail!(
            "locked worktrees can't be moved: {} (unlock with `git worktree unlock`)",
            locked.join(", ")
        );
    }
    let inside = cwd
        .canonicalize()
        .ok()
        .and_then(|c| c.strip_prefix(&from).ok().map(Path::to_path_buf));

    std::fs::rename(&from, &to)?;
    let to = to.canonicalize()?;
    let moved: Vec<PathBuf> = wts.iter().map(|w| to.join(&w.name)).collect();
    if let Err(e) = repair(&repo.clone, &moved) {
        // Put everything back where git expects it.
        let restored = std::fs::rename(&to, &from).is_ok();
        let _ = repair(
            &repo.clone,
            &wts.iter().map(|w| w.path.clone()).collect::<Vec<_>>(),
        );
        if restored {
            bail!("{e:#}; nothing was moved");
        }
        bail!(
            "{e:#}; the folder is now {} (run `git worktree repair` there)",
            to.display()
        );
    }
    eprintln!("renamed {} → {new}", repo.name);
    Ok(inside.map(|rel| to.join(rel)))
}

fn repair(clone: &Path, worktrees: &[PathBuf]) -> Result<()> {
    if worktrees.is_empty() {
        return Ok(());
    }
    let mut args = vec!["worktree".to_string(), "repair".to_string()];
    args.extend(worktrees.iter().map(|p| p.to_string_lossy().into_owned()));
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    git::run(clone, &args)?;
    Ok(())
}
