//! `grove init <shell>`: the wrapper that lets grove change the shell's directory.
//!
//! A child process can't `cd` its parent, so the function runs the binary, takes
//! the path it prints on stdout, and `cd`s there. Only navigating commands are
//! captured; everything else passes straight through.

use anyhow::{Result, bail};

const FUNCTION: &str = r#"grove() {
  case "${1-}" in
    ""|new|cd)
      local __grove_dest
      __grove_dest="$(GROVE_SHELL=__SHELL__ command grove "$@")" || return $?
      if [ -n "$__grove_dest" ] && [ -d "$__grove_dest" ]; then
        builtin cd -- "$__grove_dest"
      elif [ -n "$__grove_dest" ]; then
        printf '%s\n' "$__grove_dest"
      fi
      ;;
    *)
      command grove "$@"
      ;;
  esac
}
"#;

pub fn script(shell: &str) -> Result<String> {
    let rc = match shell {
        "zsh" => "~/.zshrc",
        "bash" => "~/.bashrc",
        other => bail!("unsupported shell `{other}` (supported: zsh, bash)"),
    };
    Ok(format!(
        "# grove shell integration. Add to {rc}:\n#   eval \"$(grove init {shell})\"\n{}",
        FUNCTION.replace("__SHELL__", shell)
    ))
}
