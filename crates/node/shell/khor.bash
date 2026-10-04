# khor shell integration for bash (khor writes this file; edits are lost).
#
# khor starts an interactive bash as `bash --init-file <this file>`, VS
# Code's way (shellIntegration-bash.sh): bash then reads this instead of
# ~/.bashrc, so this reads ~/.bashrc first, exactly as bash would have, and
# then adds the marks. The person's files are not touched.
#
# The marks are khor.zsh's: OSC 133 A/B/C/D, OSC 633;E (the command line,
# escaped as VS Code escapes it) and OSC 7, wrapped in tmux's passthrough
# inside tmux. The command is caught by bash-preexec's arrays when that is
# loaded, else by a DEBUG trap of our own when none is set (bash 3.2, the
# one macOS ships, has no PS0); a DEBUG trap someone else set is left
# alone, and then only the prompt marks are made.
#
# Opt out: KHOR_SHELL_INTEGRATION=0 in the environment khor runs in.

if [ -r "$HOME/.bashrc" ]; then
  . "$HOME/.bashrc"
fi

if [ -z "$_khor_loaded" ] && [ -n "$BASH_VERSION" ]; then
_khor_loaded=1
_khor_ran=0
_khor_armed=0

_khor_osc() {
  if [ -n "$TMUX" ]; then
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
  _khor_e=$s
}

_khor_cwd() {
  local p=$PWD
  p=${p//%/%25}
  p=${p// /%20}
  p=${p//;/%3B}
  _khor_osc "7;file://${HOSTNAME}${p}"
}

_khor_prompt_start() {
  local s=$?
  if [ "$_khor_ran" = 1 ]; then
    _khor_osc "133;D;$s"
    _khor_ran=0
  fi
  _khor_cwd
  _khor_osc "133;A"
  return $s
}

_khor_prompt_end() {
  local s=$?
  case "$PS1" in
    *"133;B"*) ;;
    *) PS1="$PS1\[$(_khor_osc '133;B')\]" ;;
  esac
  # tmux's own prompt mark, in the prompt itself (see khor.zsh).
  if [ -n "$TMUX" ]; then
    case "$PS1" in
      *"]133;A"*) ;;
      *) PS1="\[$(builtin printf '\e]133;A\a')\]$PS1" ;;
    esac
  fi
  _khor_armed=1
  return $s
}

# $1: the command line (bash-preexec hands it over whole), or with $2 set
# the DEBUG trap's $BASH_COMMAND, only its first simple command — then the
# history's last entry is the whole line.
_khor_preexec() {
  [ "$_khor_armed" = 1 ] || return 0
  _khor_armed=0
  # An empty line runs no command: the trap's first sight is the prompt's.
  [ "$1" = "_khor_prompt_start" ] && return 0
  local c=$1
  if [ -n "$2" ]; then
    c=$(HISTTIMEFORMAT= builtin history 1)
    if [[ $c =~ ^\ *[0-9]+\*?\ +(.*)$ ]]; then
      c=${BASH_REMATCH[1]}
    else
      c=$1
    fi
  fi
  _khor_escape "$c"
  _khor_osc "633;E;$_khor_e"
  _khor_osc "133;C"
  [ -n "$TMUX" ] && builtin printf '\e]133;C\a'
  _khor_ran=1
}

# Another integration is judged by the functions it defines in this shell,
# never by environment variables, which anything started from that
# terminal inherits.
_khor_other() {
  declare -F __vsc_prompt_cmd >/dev/null || declare -F iterm2_preexec_invoke_cmd >/dev/null \
    || declare -F _ksi_prompt_command >/dev/null || declare -F __wezterm_semantic_precmd >/dev/null \
    || declare -F __ghostty_precmd >/dev/null || declare -F _warp_precmd >/dev/null
}

if ! _khor_other; then
  case "$(declare -p PROMPT_COMMAND 2>/dev/null)" in
    "declare -a"*) PROMPT_COMMAND=(_khor_prompt_start "${PROMPT_COMMAND[@]}" _khor_prompt_end) ;;
    *) PROMPT_COMMAND="_khor_prompt_start${PROMPT_COMMAND:+; $PROMPT_COMMAND}; _khor_prompt_end" ;;
  esac
  if declare -F __bp_preexec_invoke_exec >/dev/null; then
    preexec_functions+=(_khor_preexec)
  elif [ -z "$(trap -p DEBUG)" ]; then
    trap '_khor_preexec "$BASH_COMMAND" trap' DEBUG
  fi
fi
fi
