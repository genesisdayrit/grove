# grove

Create and jump between git worktrees from the terminal.

```
grove repo add              # register the clone you're in with grove (once per repo)
grove worktree add feat/auth  # fetch origin, branch `feat/auth` off origin's default branch, cd into it
grove wt add feat/auth      # same, shorter
grove                       # pick a worktree interactively (↑/↓ or j/k, type to filter, enter, esc/q)
grove cd feat-auth          # jump straight to a worktree
grove ls                    # table of this repo's worktrees, most recently active first
grove ls --paths            # one path per line, for scripts and agents
grove -a                    # pick from every repo's worktrees, wherever you are
grove -r myapp cd feat-auth # act on another repo without leaving this one
grove repo ls               # every registered repo
grove env edit              # write this repo's private setup script (deps, .env, …)
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
cargo install --git https://github.com/genesisdayrit/grove --tag v0.1.2
```

Swap in the latest tag from the [releases page](https://github.com/genesisdayrit/grove/releases).

Then add the shell integration to your rc file, so grove can `cd` for you:

```bash
# ~/.zshrc
eval "$(grove init zsh)"

# ~/.bashrc
eval "$(grove init bash)"
```

Without it, `grove`, `grove cd` and `grove worktree add` still work but only print the path. See [Shell integration](#shell-integration) for how it works.

## Repos

Register a clone once with `grove repo add` (run inside it, or pass its path). That creates `~/.grove/<repo>/` with an `@repo` link back to the clone, and nothing else: no worktree, no branch. The folder *is* the registration; there is no other registry file.

```bash
grove repo add                      # the clone you're in (or any of its worktrees)
grove repo add ~/code/api           # a clone somewhere else
grove repo add --name oss-api       # pick the name yourself
grove repo add --env                # and then write its setup script
grove repo ls                       # name, clone, worktree count, setup, last active
grove repo ls --names               # one name per line
grove repo mv api2                  # rename it
```

- **Names.** A repo's name defaults to its clone's folder name, lowercased like worktree names. If another clone already has that name (say `~/work/api` and `~/oss/api`), `repo add` fails and suggests `--name oss-api`. Adding a repo twice is harmless.
- **Registration comes first.** `grove worktree add` and `grove env edit`/`setup` refuse to run in a clone that isn't registered and tell you to run `grove repo add`. Nothing is created in `~/.grove` by accident.
- **From anywhere.** `-r/--repo <name>` points any command at a registered repo, wherever you are: `grove -r api wt add fix-login`.
- **Everything at once.** Outside a registered repo, `grove` and `grove ls` cover every repo, labelled `<repo>/<worktree>`, plus each repo's clone as `<repo>/@repo`. Inside one, add `-a/--all` to get the same view.
- **Renaming.** `grove repo mv <new>` renames the repo's folder and runs `git worktree repair` so git follows. Branches, the clone and your setup script are untouched. It refuses while any worktree is locked. If your shell is inside the folder, the shell integration moves you along; other open shells and editors keep the old path.
- **Removing** isn't a command yet. Remove the worktrees with `git worktree remove <path>` (from the clone), then delete `~/.grove/<repo>`. Your setup script in `@env/` goes with it, so copy it out first if you want it.

## How it works

- **Layout.** Worktrees live in `~/.grove/<repo>/<name>`. Each repo folder has an `@repo` symlink back to the original clone, so from any worktree `../@repo` is the clone and `../<name>` is a sibling. `~/.grove/.metadata_never_index` keeps Spotlight out.
- **Base.** `grove worktree add` runs `git fetch origin` and branches from `origin/<default>` (detected via `origin/HEAD`). It never checks out or moves anything in your original clone. Offline or with no `origin`, it warns and falls back to the local default branch.
- **Names.** Lowercased; spaces and other characters become `-`. `/` is kept in the branch but flattened in the folder: `Feat/Auth` → branch `feat/auth`, folder `feat-auth`. Names starting with `@` are reserved.
- **Collisions fail.** An existing folder, local branch, or `origin` branch with the same name is an error; grove never reuses or auto-suffixes.
- **Scope.** Only grove's own worktrees are listed. Worktrees made by other tools are ignored.
- **Output.** Progress and errors go to stderr. The picker draws on `/dev/tty`.

`grove ls` columns: `*` marks the worktree you're in; STATUS is `clean` or `N changed` (lines of `git status --porcelain`); LAST ACTIVE is the later of the last commit and the newest file git reports as changed. The all-repos view adds a REPO column.

## Shell integration

A program can't change its parent shell's directory, so `eval "$(grove init zsh)"` defines a small `grove` function that does it:

1. It creates an empty temp file and runs the real `grove` with `GROVE_CD_FILE` set to that file. Output goes straight to your terminal; nothing is captured.
2. Commands that take you somewhere write the destination to that file: the picker, `cd`, `worktree add`, and `repo mv` when you're inside the moved folder.
3. If grove succeeds and the file names a directory, the function `cd`s there. The temp file is always removed.

The function never inspects the arguments, so it works however you order flags (`grove -r api cd x`).

Without the function, `GROVE_CD_FILE` is unset and grove prints the destination on stdout instead, plus a hint to set up the integration. That's the form to use from scripts and agents: `cd "$(grove worktree add foo)"`.

After upgrading grove, open a new shell to pick up the latest function. Until then, an older function still handles `grove cd` and the picker; `grove worktree add` prints the path instead of moving you.

## Setup scripts

A new worktree only has tracked files: no `node_modules`, no `.env`. Give a repo a setup script and run it with `grove worktree add <name> --env`, or later with `grove env setup`. Plain `grove worktree add` never runs it.

The script is yours alone. It lives at `~/.grove/<repo>/@env/setup`, next to the worktrees, and never in the repo, so teammates with different setups are unaffected.

```bash
grove env edit          # open it in $EDITOR (created from a template the first time)
grove env               # where it is and whether it's ready
grove env setup         # run it in the current worktree
grove wt add foo --env  # create a worktree and run setup in it
```

To write it without an editor (handy for agents), pipe it in:

```bash
grove env edit <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
cp "$GROVE_REPO_PATH/.env" .env
pnpm install
EOF
```

It runs with the new worktree as its working directory and these variables:

| Variable | Value |
| --- | --- |
| `GROVE_REPO_PATH` | the original clone (copy `.env` files from here) |
| `GROVE_WORKTREE_PATH` | the new worktree |
| `GROVE_WORKTREE_NAME` | its folder name, e.g. `feat-auth` |
| `GROVE_BRANCH` | its branch, e.g. `feat/auth` |

Its output goes to stderr and it gets no stdin, so it can't stall waiting for input. If it fails during `grove worktree add --env`, grove warns, keeps the worktree and still takes you there; fix the problem and run `grove env setup`.

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
