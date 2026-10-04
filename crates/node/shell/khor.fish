# khor shell integration for fish (khor writes this file; edits are lost).
#
# khor starts fish with its own directory first in XDG_DATA_DIRS, so fish
# loads this from `fish/vendor_conf.d`, Kitty's way (kitty-shell-integration
# for fish); the variable is put back at once so programs started from this
# shell see the person's own. The person's files are not touched.
#
# The marks are khor.zsh's: OSC 133 A/B/C/D, OSC 633;E (the command line,
# escaped as VS Code escapes it) and OSC 7, wrapped in tmux's passthrough
# inside tmux. fish 4.0 and later write the OSC 133 marks themselves (and
# OSC 7, and the command line in C's `cmdline_url`, Kitty's extension);
# then only VS Code's command line mark is added here.
#
# Opt out: KHOR_SHELL_INTEGRATION=0 in the environment khor runs in.

if set -q KHOR_ORIG_XDG_DATA_DIRS
    if test -n "$KHOR_ORIG_XDG_DATA_DIRS"
        set -gx XDG_DATA_DIRS $KHOR_ORIG_XDG_DATA_DIRS
    else
        set -e XDG_DATA_DIRS
    end
    set -e KHOR_ORIG_XDG_DATA_DIRS
end

status is-interactive; or exit
set -q _khor_loaded; and exit
set -g _khor_loaded 1

function _khor_osc
    if set -q TMUX
        printf '\ePtmux;\e\e]633;P;KhorPane=%s\a\e\e]%s\a\e\\' "$TMUX_PANE" $argv[1]
    else
        printf '\e]%s\a' $argv[1]
    end
end

set -g _khor_native 0
# Inside tmux fish's own marks reach tmux, never khor: khor wraps its own.
if not set -q TMUX; and test (string split -- . $version)[1] -ge 4 2>/dev/null
    set -g _khor_native 1
end

function _khor_cwd
    set -l p (string replace -a -- % %25 $PWD)
    set p (string replace -a -- ' ' %20 $p)
    set p (string replace -a -- ';' %3B $p)
    _khor_osc "7;file://$hostname$p"
end

function _khor_preexec --on-event fish_preexec
    # `string collect`: a command substitution splits at newlines, which
    # would turn a multi-line command into one with spaces.
    set -l c (string replace -a -- '\\' '\\\\' "$argv[1]" | string collect)
    set c (string replace -a -- ';' '\\x3b' "$c" | string collect)
    set c (string replace -a -- \n '\\x0a' "$c" | string collect)
    set c (string replace -a -- \t '\\x09' "$c" | string collect)
    set c (string replace -a -- \e '\\x1b' "$c" | string collect)
    _khor_osc "633;E;$c"
    test $_khor_native = 1; or _khor_osc "133;C"
end

function _khor_postexec --on-event fish_postexec
    test $_khor_native = 1; or _khor_osc "133;D;$status"
end

function _khor_prompt --on-event fish_prompt
    test $_khor_native = 1; and return
    _khor_cwd
    _khor_osc "133;A"
    # The input starts after the prompt: wrap the prompt function once it
    # is the person's (config.fish and autoloaded functions come after
    # vendor_conf.d), as Kitty does at the first prompt.
    if not functions -q _khor_user_prompt; and functions -q fish_prompt
        functions -c fish_prompt _khor_user_prompt
        function fish_prompt
            _khor_user_prompt
            _khor_osc "133;B"
        end
    end
end
