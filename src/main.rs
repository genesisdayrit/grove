mod config;
mod git;
mod init;
mod name;
mod nav;
mod new;
mod picker;
mod repo;
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
    /// Create a new branch + worktree from origin's default branch
    New { name: String },
    /// Jump to an existing worktree by name
    Cd { name: String },
    /// List this repo's grove worktrees, most recently active first
    Ls {
        /// Print one worktree path per line (for scripts and agents)
        #[arg(long)]
        paths: bool,
    },
    /// Print the shell function that lets grove cd (add `eval "$(grove init zsh)"` to your rc)
    Init { shell: String },
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
    if let Some(Cmd::Init { shell }) = &cli.command {
        print!("{}", init::script(shell)?);
        return Ok(());
    }
    let root = config::root()?;
    let cwd = std::env::current_dir()?;
    match cli.command {
        Some(Cmd::New { name }) => {
            let path = new::run(&cwd, &root, &name)?;
            emit_path(&path);
        }
        Some(Cmd::Cd { name }) => {
            let repo = repo::resolve(&cwd, &root)?;
            emit_path(&nav::cd(&repo, &name)?);
        }
        Some(Cmd::Ls { paths }) => nav::ls(&repo::resolve(&cwd, &root)?, &cwd, paths)?,
        Some(Cmd::Init { .. }) => unreachable!("handled above"),
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
        eprintln!("hint: add `eval \"$(grove init zsh)\"` to your shell rc to cd automatically");
    }
}
