mod config;
mod env;
mod git;
mod init;
mod name;
mod nav;
mod picker;
mod repo;
mod repo_cmd;
mod self_update;
mod tui;
mod worktree;
mod worktrees;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use nav::Scope;
use repo::Current;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Create and jump between git worktrees.
///
/// Register a clone with `grove repo add`, then `grove worktree add <name>`.
/// Add `eval "$(grove init zsh)"` to your shell rc so grove can `cd` for you.
#[derive(Parser)]
#[command(version)]
struct Cli {
    /// Act on this registered repo instead of the one you're in (see `grove repo ls`)
    #[arg(short, long, global = true, value_name = "NAME")]
    repo: Option<String>,
    /// `grove` and `grove ls` only: cover every registered repo
    #[arg(short, long, global = true)]
    all: bool,
    #[command(subcommand)]
    command: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Create and manage worktrees
    #[command(alias = "wt", subcommand)]
    Worktree(WorktreeCmd),
    /// Jump to an existing worktree by name
    Cd { name: String },
    /// List worktrees (this repo's, or every repo's outside one), most recently active first
    Ls {
        /// Print one path per line (for scripts and agents)
        #[arg(long)]
        paths: bool,
    },
    /// Register repos with grove, list and rename them
    #[command(subcommand)]
    Repo(RepoCmd),
    /// Show or manage this repo's private setup script (`@env/setup`)
    Env {
        #[command(subcommand)]
        command: Option<EnvCmd>,
    },
    /// Print the shell function that lets grove cd (add `eval "$(grove init zsh)"` to your rc)
    Init { shell: init::Shell },
    /// Manage grove itself
    #[command(name = "self", subcommand)]
    SelfCmd(SelfCmd),
}

#[derive(Subcommand)]
enum WorktreeCmd {
    /// Create a new branch + worktree from origin's default branch
    Add {
        name: String,
        /// Also run this repo's setup script (`grove env`) in the new worktree
        #[arg(long)]
        env: bool,
    },
}

#[derive(Subcommand)]
enum RepoCmd {
    /// Register a clone with grove (default: the one you're in)
    Add {
        path: Option<PathBuf>,
        /// Id to register it under (default: the clone's folder name)
        #[arg(long)]
        name: Option<String>,
        /// Then open its setup script in $EDITOR (`grove env edit`)
        #[arg(long)]
        env: bool,
    },
    /// List registered repos
    Ls {
        /// Print one repo name per line
        #[arg(long)]
        names: bool,
    },
    /// Rename a repo (moves its folder and all its worktrees)
    Mv { name: String },
}

#[derive(Subcommand)]
enum EnvCmd {
    /// Run setup in the current worktree
    Setup,
    /// Open the setup script in $EDITOR (creating it from a template), or install piped input
    Edit,
}

#[derive(Subcommand)]
enum SelfCmd {
    /// Update grove to the latest release
    Update {
        /// Only report whether a newer release exists
        #[arg(long)]
        check: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("grove: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<()> {
    match &cli.command {
        Some(Cmd::Init { shell }) => {
            print!("{}", init::script(*shell));
            return Ok(());
        }
        Some(Cmd::SelfCmd(SelfCmd::Update { check })) => return self_update::run(*check),
        _ => {}
    }
    let root = config::root()?;
    let cwd = std::env::current_dir()?;
    let listing = matches!(cli.command, None | Some(Cmd::Ls { .. }));
    if cli.all && !listing {
        bail!("--all only applies to `grove` and `grove ls`");
    }
    if cli.all && cli.repo.is_some() {
        bail!("use either --all or --repo, not both");
    }
    // The repo this command acts on: --repo, else the one we're in.
    let target = || match &cli.repo {
        Some(id) => repo::by_name(&root, id),
        None => repo::resolve(&cwd, &root),
    };
    match cli.command {
        Some(Cmd::Worktree(WorktreeCmd::Add { name, env })) => {
            let path = worktree::add(&target()?, &name, env)?;
            emit_path(&path)?;
        }
        Some(Cmd::Cd { name }) => emit_path(&nav::cd(&target()?, &name)?)?,
        Some(Cmd::Ls { paths }) => nav::ls(&scope(&cli, &cwd, &root)?, &cwd, paths)?,
        Some(Cmd::Repo(RepoCmd::Add { path, name, env })) => {
            if cli.repo.is_some() {
                bail!("`grove repo add` takes a path, not --repo");
            }
            let (repo, added) = repo::add(&cwd, &root, path.as_deref(), name.as_deref())?;
            if added {
                eprintln!("registered {} → {}", repo.name, repo.dir.display());
                if !env {
                    eprintln!("next: grove worktree add <name>");
                }
            } else {
                eprintln!("{} already registered", repo.name);
            }
            if env {
                env::edit(&repo)?;
            }
        }
        Some(Cmd::Repo(RepoCmd::Ls { names })) => repo_cmd::ls(&root, &cwd, names)?,
        Some(Cmd::Repo(RepoCmd::Mv { name })) => {
            if let Some(path) = repo_cmd::mv(&target()?, &root, &cwd, &name)? {
                emit_path(&path)?;
            }
        }
        Some(Cmd::Env { command }) => match command {
            Some(EnvCmd::Setup) => {
                if cli.repo.is_some() {
                    bail!("`grove env setup` runs in the worktree you're in; drop --repo");
                }
                env::rerun(&target()?, &cwd)?
            }
            Some(EnvCmd::Edit) => env::edit(&target()?)?,
            None => env::status(&target()?)?,
        },
        Some(Cmd::Init { .. } | Cmd::SelfCmd(_)) => unreachable!("handled above"),
        None => {
            if let Some(path) = nav::pick(&scope(&cli, &cwd, &root)?)? {
                emit_path(&path)?;
            }
        }
    }
    Ok(())
}

/// `grove`/`grove ls`: --repo, --all, the repo we're in, else every repo.
fn scope(cli: &Cli, cwd: &Path, root: &Path) -> Result<Scope> {
    if let Some(id) = &cli.repo {
        return Ok(Scope::One(repo::by_name(root, id)?));
    }
    if !cli.all {
        match repo::current(cwd, root)? {
            Current::Registered(r) => return Ok(Scope::One(r)),
            Current::Unregistered(clone) => {
                eprintln!("note: {}; showing all repos", repo::not_registered(&clone));
            }
            Current::Outside => {}
        }
    }
    let repos = repo::entries(root)?
        .iter()
        .filter_map(|e| e.repo())
        .collect();
    Ok(Scope::All(repos))
}

/// Send the user to `path`. Under the shell wrapper (`grove init`), write it to
/// `$GROVE_CD_FILE` for the wrapper to `cd` to; otherwise print it on stdout.
fn emit_path(path: &Path) -> Result<()> {
    if let Some(file) = std::env::var_os("GROVE_CD_FILE").filter(|f| !f.is_empty()) {
        std::fs::write(&file, path.as_os_str().as_encoded_bytes())
            .with_context(|| format!("writing {}", Path::new(&file).display()))?;
        return Ok(());
    }
    println!("{}", path.display());
    if std::env::var_os("GROVE_SHELL").is_none() {
        let shell = std::env::var("SHELL").unwrap_or_default();
        let shell = if shell.ends_with("/bash") {
            "bash"
        } else {
            "zsh"
        };
        eprintln!(
            "hint: add `eval \"$(grove init {shell})\"` to your shell rc to cd automatically"
        );
    }
    Ok(())
}
