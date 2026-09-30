//! Per-repo environments: a private `<root>/<repo>/@env/setup` script that
//! prepares each new worktree (deps, `.env`, …). Never lives in the repo itself.

use crate::repo::Repo;
use crate::worktrees;
use anyhow::{Context, Result, bail};
use std::io::{IsTerminal, Read};
use std::os::fd::AsFd;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub const ENV_DIR: &str = "@env";

pub fn setup_script(repo: &Repo) -> PathBuf {
    repo.dir.join(ENV_DIR).join("setup")
}

/// Run `@env/setup` in `worktree`, if there is one. Its stdout goes to our
/// stderr so stdout stays reserved for the path the shell wrapper captures.
/// Errors if the script can't start or exits non-zero.
pub fn setup(repo: &Repo, worktree: &Path, name: &str, branch: &str) -> Result<()> {
    let script = setup_script(repo);
    if !script.exists() {
        return Ok(());
    }
    let stderr = std::io::stderr().as_fd().try_clone_to_owned()?;
    let status = Command::new(&script)
        .current_dir(worktree)
        .env("GROVE_REPO_PATH", &repo.clone)
        .env("GROVE_WORKTREE_PATH", worktree)
        .env("GROVE_WORKTREE_NAME", name)
        .env("GROVE_BRANCH", branch)
        .stdin(Stdio::null())
        .stdout(stderr)
        .status()
        .with_context(|| format!("setup failed: can't run {}", script.display()))?;
    match status.code() {
        Some(0) => Ok(()),
        Some(code) => bail!("setup failed (exit {code})"),
        None => bail!("setup failed (killed by a signal)"),
    }
}

/// `grove env setup`: run setup again for the grove worktree containing `cwd`.
pub fn rerun(repo: &Repo, cwd: &Path) -> Result<()> {
    let script = setup_script(repo);
    if !script.exists() {
        bail!(
            "no setup script at {}; create one with `grove env edit`",
            script.display()
        );
    }
    let all = worktrees::list(repo)?;
    let Some(wt) = worktrees::current(&all, cwd) else {
        bail!("not in a grove worktree; setup only runs in worktrees made by `grove new`");
    };
    setup(repo, &wt.path, &wt.name, wt.branch.as_deref().unwrap_or(""))
}

const TEMPLATE: &str = r#"#!/usr/bin/env bash
# grove setup for __REPO__. Runs in each new worktree (the working directory).
# Available: $GROVE_REPO_PATH $GROVE_WORKTREE_PATH $GROVE_WORKTREE_NAME $GROVE_BRANCH
# This file is private to you; it is never committed to the repo.
set -euo pipefail

# cp "$GROVE_REPO_PATH/.env" .env
# pnpm install
"#;

/// `grove env edit`: with piped input, install it as the setup script;
/// otherwise open the script (created from a template if missing) in `$EDITOR`.
pub fn edit(repo: &Repo, root: &Path) -> Result<()> {
    repo.ensure_dir(root)?;
    let script = setup_script(repo);
    std::fs::create_dir_all(script.parent().unwrap())?;

    let stdin = std::io::stdin();
    if !stdin.is_terminal() {
        let mut body = String::new();
        stdin.lock().read_to_string(&mut body)?;
        if body.trim().is_empty() {
            bail!("nothing on stdin; pipe a script in, or run from a terminal to open $EDITOR");
        }
        std::fs::write(&script, body)?;
        make_executable(&script)?;
        eprintln!("wrote {}", script.display());
        return Ok(());
    }

    if !script.exists() {
        std::fs::write(&script, TEMPLATE.replace("__REPO__", &repo.name))?;
    }
    make_executable(&script)?;
    let editor = std::env::var("EDITOR")
        .ok()
        .filter(|e| !e.trim().is_empty())
        .unwrap_or_else(|| "vi".to_string());
    // Through sh so `EDITOR="code --wait"` works.
    let status = Command::new("sh")
        .args(["-c", &format!("{editor} \"$1\""), "sh"])
        .arg(&script)
        .status()
        .with_context(|| format!("running editor `{editor}`"))?;
    if !status.success() {
        bail!("editor `{editor}` exited with {status}");
    }
    Ok(())
}

fn make_executable(path: &Path) -> Result<()> {
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(perms.mode() | 0o755);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}

/// `grove env`: where the setup script lives and whether `grove new` will run it.
pub fn status(repo: &Repo) -> Result<()> {
    let script = setup_script(repo);
    println!("setup: {}", script.display());
    match std::fs::metadata(&script) {
        Err(_) => println!("not set up; create it with `grove env edit`"),
        Ok(m) if m.permissions().mode() & 0o111 == 0 => {
            println!("not executable; fix with `chmod +x` or `grove env edit`")
        }
        Ok(_) => println!("runs on `grove new` (skip with --no-env)"),
    }
    Ok(())
}
