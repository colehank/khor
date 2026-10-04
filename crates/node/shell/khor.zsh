# khor shell integration for zsh (khor writes this file; edits are lost).
#
# What it says, so khor's host knows where commands begin and end:
#   OSC 133 ; A        a prompt starts            (FinalTerm's marks, as
#   OSC 133 ; B        the input starts            iTerm2, Kitty, WezTerm,
#   OSC 133 ; C        the command runs            Ghostty and VS Code
#   OSC 133 ; D ; n    it finished with status n   emit them)
#   OSC 633 ; E ; cmd  the command line, escaped as VS Code's shell
#                      integration escapes it (\\ for \, \xNN for ; and
#                      control characters)
#   OSC 7 ; file://host/path   the working directory
# Inside tmux each mark is wrapped in tmux's passthrough (ESC P tmux; …),
# which tmux forwards while the pane is visible and its `allow-passthrough`
# is on (turned on for this pane at the first prompt). Each wrap opens with
#   OSC 633 ; P ; KhorPane=%n  the pane, $TMUX_PANE
# so khor tells apart the panes of a split window. The prompt and command
# marks also go to tmux itself, unwrapped: tmux 3.4 and later read them for
# copy mode's previous-prompt and next-prompt (tmux(1)); older ones drop
# them.
#
# Inside tmux, one line in ~/.zshrc loads this in every shell, not only
# those khor starts (Settings → Terminal shows it):
#   [[ -n $TMUX && -r ~/.khor/shell/khor.zsh ]] && source ~/.khor/shell/khor.zsh
#
# The hooks go in at the first prompt, after the person's .zshrc has run,
# so they come last and a theme that rebuilds PS1 cannot drop the input
# mark (Kitty and VS Code defer the same way). A shell that already has an
# integration emitting these marks (VS Code's, iTerm2's, Kitty's, WezTerm's,
# Ghostty's, Warp's) is left to it, so nothing is marked twice.
#
# Opt out: KHOR_SHELL_INTEGRATION=0 in the environment khor runs in.

[[ -n "$_khor_loaded" ]] && return
typeset -g _khor_loaded=1
typeset -gi _khor_ran=0

_khor_osc() {
  if [[ -n "$TMUX" ]]; then
    builtin printf '\ePtmux;\e\e]633;P;KhorPane=%s\a\e\e]%s\a\e\\' "$TMUX_PANE" "$1"
  else
    builtin printf '\e]%s\a' "$1"
  fi
}

_khor_escape() {
  local s=$1
  s=${s//\\/\\\\}
  s=${s//;/\\x3b}
  s=${s//$'\n'/\\x0a}
  s=${s//$'\r'/\\x0d}
  s=${s//$'\t'/\\x09}
  s=${s//$'\e'/\\x1b}
  s=${s//$'\a'/\\x07}
  REPLY=$s
}

_khor_cwd() {
  local p=$PWD
  p=${p//\%/%25}
  p=${p// /%20}
  p=${p//;/%3B}
  _khor_osc "7;file://${HOST}${p}"
}

_khor_precmd() {
  local s=$?
  if (( _khor_ran )); then
    _khor_osc "133;D;$s"
    _khor_ran=0
  fi
  _khor_cwd
  _khor_osc "133;A"
  # The input starts where the prompt ends; themes rebuild PS1 each time.
  [[ $PS1 == *"133;B"* ]] || PS1="$PS1%{$(_khor_osc '133;B')%}"
  # tmux's own prompt mark goes in the prompt itself: one printed here
  # lands on a line the prompt's drawing then clears, and tmux keeps the
  # mark on the line (Kitty's zsh integration marks inside PS1 too).
  [[ -n "$TMUX" && $PS1 != *"]133;A"* ]] && PS1="%{$(builtin printf '\e]133;A\a')%}$PS1"
}

_khor_preexec() {
  _khor_escape "$1"
  _khor_osc "633;E;$REPLY"
  _khor_osc "133;C"
  [[ -n "$TMUX" ]] && builtin printf '\e]133;C\a'
  _khor_ran=1
}

# Another integration is judged by the hook functions it defines in this
# shell, never by environment variables: those are inherited by anything
# started from that terminal, khor included.
_khor_other() {
  (( ${+functions[__vsc_precmd]} || ${+functions[iterm2_precmd]} || ${+functions[_ksi_precmd]} \
    || ${+functions[__wezterm_semantic_precmd]} || ${+functions[_ghostty_precmd]} \
    || ${+functions[_warp_precmd]} || ${+functions[warp_precmd]} ))
}

_khor_install() {
  precmd_functions=(${precmd_functions:#_khor_install})
  _khor_other && return
  builtin autoload -Uz add-zsh-hook
  add-zsh-hook precmd _khor_precmd
  add-zsh-hook preexec _khor_preexec
  # Marks pass through only with the pane's `allow-passthrough` on (tmux
  # 3.3; `-p` is 3.0): this pane's, nobody else's.
  [[ -n "$TMUX" && -n "$TMUX_PANE" ]] \
    && command tmux set-option -p -q -t "$TMUX_PANE" allow-passthrough on 2>/dev/null
  # This first prompt is already being drawn: mark it now.
  _khor_precmd
}

precmd_functions+=(_khor_install)
