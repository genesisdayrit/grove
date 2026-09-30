# Changelog

All notable changes to grove are listed here. Each release's section becomes its GitHub Release notes.

## Unreleased

### Added

- Per-repo setup scripts: `grove new` runs `~/.grove/<repo>/@env/setup` in each new worktree to install dependencies, copy `.env` files and so on. The script is private to you and never committed. Manage it with `grove env`, `grove env edit` and `grove env setup`; skip it with `grove new --no-env`.

## [0.1.0] - 2026-09-30

### Added

- `grove new`, `grove cd`, `grove ls` and the interactive picker for creating and jumping between git worktrees.
- `grove init zsh|bash` shell integration so grove can `cd` for you.
- `grove self update` to install the latest release, and `grove self update --check` to see whether one exists.
- Prebuilt binaries for macOS and Linux (Apple Silicon, Intel, x86_64, arm64), a shell installer, and a Homebrew formula in `genesisdayrit/tap`.
