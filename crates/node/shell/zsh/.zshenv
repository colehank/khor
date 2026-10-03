# khor shell integration, the zsh shim (khor writes this file; edits are lost).
#
# khor started this zsh with ZDOTDIR pointing here, so this is the first
# startup file it reads. Kitty's approach (shell-integration/zsh/.zshenv):
# put the person's ZDOTDIR back at once, so every later startup file
# (.zprofile, .zshrc, .zlogin) is theirs, read from where they keep it;
# source their own .zshenv; and, in an interactive shell, load khor's hooks.
# Nothing in the person's files is touched.
if [[ -n "${KHOR_ORIG_ZDOTDIR+set}" ]]; then
  if [[ -n "$KHOR_ORIG_ZDOTDIR" ]]; then
    builtin export ZDOTDIR="$KHOR_ORIG_ZDOTDIR"
  else
    builtin unset ZDOTDIR
  fi
  builtin unset KHOR_ORIG_ZDOTDIR
fi
if [[ -f "${ZDOTDIR:-$HOME}/.zshenv" ]]; then
  builtin source "${ZDOTDIR:-$HOME}/.zshenv"
fi
if [[ -o interactive && -n "$KHOR_SHELL_DIR" && -f "$KHOR_SHELL_DIR/khor.zsh" ]]; then
  builtin source "$KHOR_SHELL_DIR/khor.zsh"
fi
builtin unset KHOR_SHELL_DIR
