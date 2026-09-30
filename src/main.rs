mod config;
mod env;
mod git;
mod init;
mod name;
mod nav;
mod new;
mod picker;
mod repo;
mod self_update;
mod tui;
mod worktrees;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::Path;
use std::process::ExitCode;

/// Create and jump between git worktrees.
///
/// Commands that navigate print only a path on stdout; everything else goes to
/// stderr. Add `eval "$(grove init zsh)"` to your shell rc so grove can `cd` for you.
#[derive(Parser)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Create a new branch + worktree from origin's default branch, then run its setup
    New {
        name: String,
        /// Skip this repo's `@env/setup` script
        #[arg(long)]
        no_env: bool,
    },
    /// Jump to an existing worktree by name
    Cd { name: String },
    /// List this repo's grove worktrees, most recently active first
    Ls {
        /// Print one worktree path per line (for scripts and agents)
        #[arg(long)]
        paths: bool,
    },
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
enum EnvCmd {
    /// Run setup again in the current worktree
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
    match cli.command {
        Some(Cmd::New { name, no_env }) => {
            let path = new::run(&cwd, &root, &name, !no_env)?;
            emit_path(&path);
        }
        Some(Cmd::Cd { name }) => {
            let repo = repo::resolve(&cwd, &root)?;
            emit_path(&nav::cd(&repo, &name)?);
        }
        Some(Cmd::Ls { paths }) => nav::ls(&repo::resolve(&cwd, &root)?, &cwd, paths)?,
        Some(Cmd::Env { command }) => {
            let repo = repo::resolve(&cwd, &root)?;
            match command {
                Some(EnvCmd::Setup) => env::rerun(&repo, &cwd)?,
                Some(EnvCmd::Edit) => env::edit(&repo, &root)?,
                None => env::status(&repo)?,
            }
        }
        Some(Cmd::Init { .. } | Cmd::SelfCmd(_)) => unreachable!("handled above"),
        None => {
            let repo = repo::resolve(&cwd, &root)?;
            if let Some(path) = nav::pick(&repo)? {
                emit_path(&path);
            }
        }
    }
    Ok(())
}

/// The shell wrapper captures stdout and `cd`s to it; without it, nudge the user.
fn emit_path(path: &Path) {
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
}
