# grove

Create and jump between git worktrees from the terminal.

```
grove new feat/auth     # fetch origin, branch `feat/auth` off origin's default branch, cd into it
grove                   # pick a worktree interactively (↑/↓ or j/k, type to filter, enter, esc/q)
grove cd feat-auth      # jump straight to a worktree
grove ls                # table of this repo's worktrees, most recently active first
grove ls --paths        # one path per line, for scripts and agents
```

## Install

Pick one. Prebuilt binaries cover macOS (Apple Silicon and Intel) and Linux (x86_64 and arm64).

**Homebrew**

```bash
brew install genesisdayrit/tap/grove
```

**Shell installer** (no Rust or Homebrew needed; installs to `~/.local/bin`)

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/genesisdayrit/grove/releases/latest/download/grove-installer.sh | sh
```

**From source** (needs a Rust toolchain)

```bash
cargo install --git https://github.com/genesisdayrit/grove --tag v0.1.0
```

Swap in the latest tag from the [releases page](https://github.com/genesisdayrit/grove/releases).

Then add the shell integration to your rc file. A program can't change its parent shell's directory, so grove prints a path and this small function `cd`s to it:

```bash
# ~/.zshrc
eval "$(grove init zsh)"

# ~/.bashrc
eval "$(grove init bash)"
```

Without it, `grove new`/`grove cd`/`grove` still work but only print the path.

## How it works

- **Layout.** Worktrees live in `~/.grove/<repo>/<name>`, where `<repo>` is the basename of your clone. Each repo folder has an `@repo` symlink back to the original clone, so from any worktree `../@repo` is the clone and `../<name>` is a sibling. `~/.grove/.metadata_never_index` keeps Spotlight out.
- **Base.** `grove new` runs `git fetch origin` and branches from `origin/<default>` (detected via `origin/HEAD`). It never checks out or moves anything in your original clone. Offline or with no `origin`, it warns and falls back to the local default branch.
- **Names.** Lowercased; spaces and other characters become `-`. `/` is kept in the branch but flattened in the folder: `Feat/Auth` → branch `feat/auth`, folder `feat-auth`. Names starting with `@` are reserved.
- **Collisions fail.** An existing folder, local branch, or `origin` branch with the same name is an error; grove never reuses or auto-suffixes.
- **Scope.** Only grove's own worktrees are listed. Worktrees made by other tools are ignored.
- **Output.** Navigating commands print only the path on stdout; progress and errors go to stderr. The picker draws on `/dev/tty`.

`grove ls` columns: `*` marks the worktree you're in; STATUS is `clean` or `N changed` (lines of `git status --porcelain`); LAST ACTIVE is the later of the last commit and the newest file git reports as changed.

## Configuration

Optional. The root directory is chosen by, in order:

1. `GROVE_ROOT`
2. `root` in `$XDG_CONFIG_HOME/grove/config.toml` (default `~/.config/grove/config.toml`)
3. `~/.grove`

```toml
root = "~/worktrees"
```

## Updating

```bash
grove self update --check   # is there a newer release?
grove self update           # install it
```

`--check` works however you installed grove. `grove self update` replaces the binary itself only when it came from the shell installer; otherwise it tells you what to run:

| Installed with | Update with |
| --- | --- |
| Shell installer | `grove self update` |
| Homebrew | `brew upgrade grove` |
| `cargo install` | `cargo install --git https://github.com/genesisdayrit/grove --tag vX.Y.Z --force` |

Set `GITHUB_TOKEN` if you hit GitHub's API rate limit (60 unauthenticated requests an hour).

What changed in each release: [CHANGELOG.md](CHANGELOG.md) or the [releases page](https://github.com/genesisdayrit/grove/releases).

## Development

```bash
cargo test
```

The suite is mostly integration tests: each builds a bare "origin" plus a clone in a temp dir and runs the real binary against it, including through real `zsh`/`bash` with the shell integration loaded.

## Releasing

Releases are built by [cargo-dist](https://github.com/axodotdev/cargo-dist) (`.github/workflows/release.yml`) when a `v*` tag is pushed. It builds the binaries, creates the GitHub Release with the matching `CHANGELOG.md` section as notes, and pushes `Formula/grove.rb` to [`genesisdayrit/tap`](https://github.com/genesisdayrit/tap).

1. Make sure CI is green on `main`.
2. Bump `version` in `Cargo.toml` and run `cargo check` to refresh `Cargo.lock`.
3. In `CHANGELOG.md`, rename `## Unreleased` to `## [X.Y.Z] - YYYY-MM-DD` and start a fresh `## Unreleased` above it.
4. Commit, tag and push:

   ```bash
   git commit -am "Release vX.Y.Z"
   git tag vX.Y.Z
   git push origin main vX.Y.Z
   ```

The workflow fails if the tag doesn't match the `Cargo.toml` version. Publishing the formula needs a `HOMEBREW_TAP_TOKEN` repo secret: a fine-grained PAT with Contents read/write on `genesisdayrit/tap` only.

After changing `dist-workspace.toml`, run `dist generate` to regenerate the release workflow rather than editing it by hand.
