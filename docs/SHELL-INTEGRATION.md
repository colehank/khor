# Shell integration and the command ledger

Roadmap phase 1 (`audit/TERM-ROADMAP.md`). The host knows where a hosted shell's commands begin and end, which directory it is in, and how each command ended. Faces use that for list rows, command blocks, notifications and history.

## What the shell says

Each hosted zsh, bash or fish loads a short script that writes these marks. They are the marks the field agreed on, as defined by FinalTerm and documented by iTerm2 ("Shell Integration"), Kitty (shell-integration), WezTerm, Ghostty, Windows Terminal ("Shell integration in the Windows Terminal") and VS Code (`terminal.integrated.shellIntegration`):

| Mark | Meaning |
|---|---|
| `OSC 133 ; A` | The prompt starts. |
| `OSC 133 ; B` | The input starts, where the prompt ends. |
| `OSC 133 ; C` | The command runs. It may carry Kitty's `cmdline=` or fish 4's `cmdline_url=`. |
| `OSC 133 ; D ; n` | The command finished with status `n`. |
| `OSC 633 ; E ; line` | VS Code's command line. `\\` stands for a backslash; `\xNN` stands for `;` and for control characters. |
| `OSC 7 ; file://host/path` | The working directory. |
| `OSC 0 / 2` | The title, when a program sets one. |

The scripts are in `crates/node/shell/`. The host writes them under `<khor home>/.khor/shell/` and rewrites them only when they change (`crate::integration`).

| Shell | How it loads, without touching the person's files | After |
|---|---|---|
| zsh | `ZDOTDIR` points at khor's directory. Its `.zshenv` first restores the person's `ZDOTDIR` and sources their `.zshenv`. | Kitty |
| bash | `bash --init-file khor.bash`, which reads `~/.bashrc` first, as bash would. | VS Code |
| fish | khor's directory goes first in `XDG_DATA_DIRS` (`fish/vendor_conf.d`). The variable is restored at once. | Kitty |
| PowerShell 7 | `khor.ps1` wraps `prompt` and PSReadLine's Enter. It is written but not wired. | VS Code |
| cmd | `khor.cmd` is a `PROMPT` with A, B, D and OSC 7. cmd has no hook before a command runs. It is written but not wired. | Windows Terminal |

**When the hooks go in.** In zsh and fish they install at the first prompt, after the person's own startup files, so they come last. zsh re-adds the input mark to `PS1` at every prompt, because themes rebuild `PS1`.

**fish 4.** fish 4.0 and later write OSC 133 and OSC 7 themselves. There, only `633;E` is added.

**bash: how commands are caught.**
- With bash-preexec loaded, the command is caught through bash-preexec's arrays.
- Otherwise it is caught by a DEBUG trap. The DEBUG trap is installed only if none is set, because macOS's bash 3.2 has no `PS0`.
- A line that is only a subshell, such as `(exit 3)`, is not seen by a DEBUG trap.

**Shells that already have an integration.** Such a shell is left alone. khor recognises one by the hook functions that VS Code's, iTerm2's, Kitty's, WezTerm's, Ghostty's or Warp's scripts define in the shell. It never relies on environment variables, because anything started from that terminal inherits them.

**Turning it off.** Set `KHOR_SHELL_INTEGRATION=0` in the environment khor runs in.

## Terminal runtime

Every new khor PTY declares `TERM=xterm-256color` and `COLORTERM=truecolor`.
It removes inherited color overrides such as `NO_COLOR`: those describe the
launcher, not the new interactive terminal. A person's shell startup files or
an explicit `env NO_COLOR=1 command` still control that command. Existing
processes and existing tmux servers keep their original environment.

Codex commands in ordinary terminals keep their original PATH resolution and
arguments. New PTYs remove this store's retired `.khor/codex-bin` PATH entry;
no launcher is installed and no daemon, title or hook flags are injected.
The `legacy_codex` entry exists only for already-open shells carrying the old
alias: it forwards the original command to real Codex, without changing its
arguments, cwd, stdio or tmux settings. Losing `KHOR_CODEX_SHIM_DIR` cannot make
that compatibility entry fall through to khor GUI/server startup.

Mouse policy belongs to the terminal connection: grouped tmux attach enables
`mouse` for its khor session. Codex 0.162.1 checks that option on TUI entry;
turning it on after Codex starts does not reactivate native mouse capture.
An existing client started with mouse off needs a user-driven exit/resume.
The Codex command is never wrapped to change tmux options.

The workspace uses the current shell or current window's active pane directory.
`Meta.place` remains the birthplace. Default workspace views follow `cd` and
pane changes; opening a particular file or directory pins that view until the
workspace seat is pressed again.

Full-screen scrolling follows the inner program's capabilities: an application
that requests mouse events keeps receiving them; one on the alternate screen
without mouse support receives PageUp/PageDown. Ordinary shell history retains
its existing scrolling path. The paging gesture accumulates travel by viewport
and never sends Up/Down into a draft's input history. Native mouse capture is
the normal path for Claude and current Codex; paging is only the fallback for
programs that do not take the mouse. A continuous wheel/touch gesture refreshes
tmux capabilities once at entry, rather than rereading history after every
output frame and wheel notch.

## tmux

Inside tmux (`$TMUX` set), every mark is wrapped in tmux's passthrough (`ESC P tmux; … ESC \`, with the inner ESC doubled), led by `OSC 633 ; P ; KhorPane=$TMUX_PANE` in the same wrap so the ledger can tell the panes of a split window apart. tmux forwards a wrap only while the pane is visible and its `allow-passthrough` is on. The zsh script turns that on for its own pane at the first prompt (`tmux set-option -p -q allow-passthrough on`), and khor's grouped attach turns it on for the window it attaches to (`Tmux::grouped_attach_argv`). Passthrough lets a program in that pane write to the outer terminal past tmux; that is tmux's reason for leaving it off by default.

The prompt and command marks also go to tmux itself, unwrapped: `133;A` inside the prompt (a mark printed before the prompt lands on a line the prompt's drawing clears, and tmux keeps marks per line) and `133;C` before the command. tmux 3.4 and later use them for copy mode's `previous-prompt` and `next-prompt`; khor's ⌘↑/⌘↓ on a tmux row run those. Inside tmux, fish 4's own marks reach tmux and never khor, so the fish script writes its own wrapped ones there.

Which shells load the scripts:
- **A pane opened from khor's client.** khor's grouped session gets a `default-command` that starts the person's `default-shell` the way khor starts its own shells (`Tmux::shell_shim`). It is a session option, so panes the person opens from their own client are untouched. It is not set when they have a `default-command` of their own, or for bash (its init file would cost the login shell).
- **Every other shell in tmux**, with one line in `~/.zshrc` (设置 → 终端 shows it with a copy button):
  `[[ -n $TMUX && -r ~/.khor/shell/khor.zsh ]] && source ~/.khor/shell/khor.zsh`
  The guard keeps it to tmux and to machines where khor has written its scripts. The line is not safe for bash: `khor.bash` reads `~/.bashrc` first, so sourcing it from there would loop.
- A shell already running is never touched: there is no way into it but typing.

A tmux row khor is not attached to reads tmux's own `pane_current_command` and `pane_current_path`. An attached one whose front pane marks nothing shows no command.

## The ledger

The host feeds the same bytes it keeps for the ring to `crate::commands::Ledger`.

**Reading the marks:**
- Marks split across reads are still read.
- A doubled mark changes nothing.
- A passthrough that nothing in between unwrapped is opened.
- Without a named command line, the text typed between B and C is used instead, with backspaces applied.
- The alternate screen or the mouse taken means a full-screen program is running.

**What is kept:**

| Where | What |
|---|---|
| The host process | The running command and the last 50. A face that re-attaches finds them unchanged. |
| `commands.json`, beside the session | `{ doing, recent }`, written whole when a reader-visible fact changes. The node reads it into `Session::doing`. |
| `<khor home>/.khor/history.jsonl` | Every finished command with its session id, appended. The file is cut to its newest half past 4 MiB. |

**Command fields:** the text, the cwd, start and end time in ms, the exit status, and the offsets in the session's byte stream.

## ConPTY: what Windows must preserve

khor does not host on Windows yet. When it does, the PTY is ConPTY, and ConPTY is not transparent. Warp's write-up ("Bringing Warp to Windows") found three problems:
- It did not forward unrecognised DCS sequences.
- It sent escape sequences out of order relative to the text.
- It re-rendered from a cached grid.

VS Code's `633;E` exists partly because, under ConPTY, the command line could not be recovered from cursor positions.

For phase 1 to work there, the host must create the pseudoconsole in passthrough mode. In the Windows Terminal codebase this is the `PSEUDOCONSOLE_PASSTHROUGH_MODE` flag. In this mode the client's VT output reaches the host as the client wrote it, so marks keep their order relative to the output around them. `portable-pty` does not set that flag today, so khor's Windows host would have to set it itself.

Even in passthrough, these rules apply:
1. **Use OSC, never DCS, on Windows.** The tmux wrap is not used there, and nothing khor sends may rely on a DCS arriving.
2. **Keep `633;E`.** The command line must travel in the mark, not be read off the screen.
3. **Do not rely on the screen being re-sent.** The ledger never assumes a full repaint, and it does not need one.
4. **Use passthrough rather than re-rendering.** If passthrough is unavailable (an older Windows build), ConPTY may still re-order marks against output. Then the ledger's block offsets are approximate, and the command, status and cwd remain right.
