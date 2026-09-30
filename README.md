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

```bash
cargo install --path .
```

Or straight from GitHub:

```bash
cargo install --git https://github.com/genesisdayrit/grove
```

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

## Development

```bash
cargo test
```

The suite is mostly integration tests: each builds a bare "origin" plus a clone in a temp dir and runs the real binary against it, including through real `zsh`/`bash` with the shell integration loaded.
