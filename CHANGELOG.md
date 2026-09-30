# Changelog

All notable changes to grove are listed here. Each release's section becomes its GitHub Release notes.

## Unreleased

## [0.1.2] - 2026-09-30

### Changed

- **Breaking:** `grove new <name>` is now `grove worktree add <name>` (or `grove wt add <name>`). `grove new` is gone.
- **Breaking:** a repo must be registered with `grove repo add` before `grove worktree add` or `grove env edit`/`setup` will work in it. Repos you've already used grove in are registered already.
- The shell integration now hands grove a temp file (`GROVE_CD_FILE`) to write its destination to, instead of capturing stdout. Open a new shell after upgrading.

### Added

- `grove repo add [path] [--name <name>] [--env]` registers a clone without creating a worktree. Pick your own name with `--name` when two clones share a folder name.
- `grove repo ls` lists registered repos, and `grove repo mv <new>` renames one, moving its worktrees with it.
- `-r/--repo <name>` points any command at a registered repo from anywhere.
- Outside a registered repo, `grove` and `grove ls` cover every repo (with each clone listed as `<repo>/@repo`); `-a/--all` does the same from inside one.

## [0.1.1] - 2026-09-30

### Added

- Per-repo setup scripts: `grove new <name> --env` runs `~/.grove/<repo>/@env/setup` in the new worktree to install dependencies, copy `.env` files and so on. The script is private to you and never committed. Manage it with `grove env` and `grove env edit`, and run it in an existing worktree with `grove env setup`.

## [0.1.0] - 2026-09-30

### Added

- `grove new`, `grove cd`, `grove ls` and the interactive picker for creating and jumping between git worktrees.
- `grove init zsh|bash` shell integration so grove can `cd` for you.
- `grove self update` to install the latest release, and `grove self update --check` to see whether one exists.
- Prebuilt binaries for macOS and Linux (Apple Silicon, Intel, x86_64, arm64), a shell installer, and a Homebrew formula in `genesisdayrit/tap`.
