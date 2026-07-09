use anyhow::{bail, Result};

pub fn wrapper(shell: &str) -> Result<&'static str> {
    match shell {
        "zsh" => Ok(ZSH),
        "bash" => Ok(BASH),
        "fish" => bail!("fish isn't supported yet — zsh and bash are (fish is planned)"),
        other => bail!("unknown shell '{other}' — expected zsh or bash"),
    }
}

// Flags and empty input pass through so `howto --last`, `--help`, etc. behave
// identically wrapped or unwrapped. Queries run captured: stdout carries only
// the command, which lands on the prompt buffer.
const ZSH: &str = r#"# howto shell integration (zsh)
# Install: add  eval "$(howto --init zsh)"  to ~/.zshrc
howto() {
  case "$1" in
    ""|-*) command howto "$@"; return $? ;;
  esac
  local __howto_cmd
  __howto_cmd="$(__HOWTO_WRAP=1 command howto "$@")" || return $?
  [[ -n "$__howto_cmd" ]] && print -z -- "$__howto_cmd"
}
"#;

// bash has no `print -z`; `read -e -i` pre-fills an editable line instead.
// Enter runs it (recorded in history), Ctrl-C discards it.
const BASH: &str = r#"# howto shell integration (bash)
# Install: add  eval "$(howto --init bash)"  to ~/.bashrc
howto() {
  case "$1" in
    ""|-*) command howto "$@"; return $? ;;
  esac
  local __howto_cmd __howto_edited
  __howto_cmd="$(__HOWTO_WRAP=1 command howto "$@")" || return $?
  [[ -z "$__howto_cmd" ]] && return 0
  IFS= read -r -e -p "$ " -i "$__howto_cmd" __howto_edited || return 0
  if [[ -n "$__howto_edited" ]]; then
    history -s "$__howto_edited"
    eval "$__howto_edited"
  fi
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_shells_have_wrappers() {
        assert!(wrapper("zsh").unwrap().contains("print -z"));
        assert!(wrapper("bash").unwrap().contains("read -r -e"));
        assert!(wrapper("fish").is_err());
        assert!(wrapper("powershell").is_err());
    }
}
