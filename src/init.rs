//! `grove init <shell>`: the wrapper that lets grove change the shell's directory.
//!
//! A child process can't `cd` its parent, so the function does it for grove:
//!
//! 1. It makes an empty temp file and runs `grove "$@"` with `GROVE_CD_FILE`
//!    pointing at it. grove's stdout and stderr go straight to the terminal.
//! 2. A command that wants to move you (`grove`, `cd`, `worktree add`, and
//!    `repo mv` from inside the moved folder) writes the destination to that
//!    file instead of printing it.
//! 3. When grove exits successfully and the file names a directory, the
//!    function `cd`s there. The file is always removed.
//!
//! The wrapper never looks at the arguments, so new commands and flags need no
//! change here. Without it (`GROVE_CD_FILE` unset), grove prints the path on
//! stdout. An old wrapper that captured stdout still works for `cd` and the picker.

const FUNCTION: &str = r#"grove() {
  local __grove_cd __grove_rc __grove_dest
  __grove_cd="$(mktemp "${TMPDIR:-/tmp}/grove-cd.XXXXXX")" || return
  GROVE_SHELL=__SHELL__ GROVE_CD_FILE="$__grove_cd" command grove "$@"
  __grove_rc=$?
  __grove_dest="$(cat -- "$__grove_cd" 2>/dev/null)"
  rm -f -- "$__grove_cd"
  if [ "$__grove_rc" -eq 0 ] && [ -n "$__grove_dest" ] && [ -d "$__grove_dest" ]; then
    builtin cd -- "$__grove_dest" || return
  fi
  return "$__grove_rc"
}
"#;

#[derive(Clone, Copy, clap::ValueEnum)]
pub enum Shell {
    Zsh,
    Bash,
}

pub fn script(shell: Shell) -> String {
    let (name, rc) = match shell {
        Shell::Zsh => ("zsh", "~/.zshrc"),
        Shell::Bash => ("bash", "~/.bashrc"),
    };
    format!(
        "# grove shell integration. Add to {rc}:\n#   eval \"$(grove init {name})\"\n{}",
        FUNCTION.replace("__SHELL__", name)
    )
}
