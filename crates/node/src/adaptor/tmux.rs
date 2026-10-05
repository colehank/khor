//! tmux: the container, not a vendor.
//!
//! Every other module here reads one agent's files. This one reads a
//! **terminal multiplexer**, and the difference is the whole design:
//! claude and codex each own the sessions they report, while a tmux
//! session owns nothing — it holds whatever the user started in it,
//! which is often a session somebody else in this directory already
//! listed.
//!
//! # Why this is not an `Adaptor`
//!
//! It has no [`super::Adaptor`] impl and no trait of its own. The seam
//! rule this repo follows is to cut one at the **second** real
//! implementation, never at the first (`Adaptor` itself waited for
//! codex; [`crate::KindSurface`] waited for the third kind). There is
//! one multiplexer. A second — screen, zellij — is what would show which
//! parts of this are general, and until then a trait would be a guess
//! wearing ceremony.
//!
//! It also would not fit: [`super::Adaptor::sweep`] answers "which of my
//! sessions are alive", and this one cannot answer alone. Its rows are
//! only rows once nothing else claims what is inside them, so it hands
//! back [`Multiplexed`] — a candidate plus the pids it holds — and
//! `crate::live::LiveKind::rows` decides.
//!
//! # Calling the user's tmux is not a dependency on tmux
//!
//! Khor asks nothing of the user (docs/KHOR.md 零依赖). Running their
//! `tmux` binary does not break that: **no tmux, no rows, no error, no
//! count** — the list is exactly what it would have been. The rule being
//! kept is "khor never requires an install"; the command-line interface
//! of a program the user already chose to run is the same kind of public
//! surface as the files claude already chose to write, and this module
//! reads it in the same posture.
//!
//! **Where the binary is looked for is part of that promise.** A GUI
//! launched from the Finder inherits a PATH without `/opt/homebrew/bin`,
//! so PATH alone would make the app list fewer sessions than the
//! terminal does, with nothing on screen to say why. See [`CANDIDATES`].
//!
//! # One row is one tmux session
//!
//! Panes and windows are not rows (decided 2026-08-16; the terminal
//! screen batch owns pane detail). A session with three panes is one
//! line, and its word is the busiest thing in it.
//!
//! # What tmux can and cannot say
//!
//! Two of the six words, and it is worth naming the four it cannot:
//!
//! - **忙碌 / 空闲** — from the foreground command, below.
//! - **完成 and 待批 are unreachable**: tmux has no notion of a turn or
//!   of an approval, so there is nothing in it that could be either. Not
//!   a gap to fill later — a shell has no turns.
//! - **中断 is unreachable**: an error is a thing the program inside
//!   knows, and tmux does not read it.
//! - **失败 is unreachable, and this is the sharp one.** Every other
//!   source here can spell 失败 because a file outlived the process that
//!   wrote it ([`super`] module head, the missing-ending rule). tmux
//!   keeps no such file: when a session ends, it is gone from
//!   `list-panes` on the very next sweep and the row simply stops
//!   existing. **There is no crash to record because there is nothing
//!   left to record it with**, and inventing one from the row's
//!   disappearance would put 失败 on every session the user closed on
//!   purpose.
//!
//! ## 忙碌 vs 空闲, and the two kinds of pane
//!
//! `#{pane_current_command}` is the command of the pane tty's foreground
//! process group — the same fact `crate::host::host_main` polls with
//! `process_group_leader()` for the shells khor hosts itself, so this is
//! that judgment ported, not a new one.
//!
//! Reading it needs to know what the pane started as, and tmux answers
//! that too:
//!
//! - `pane_start_command` **non-empty** — `tmux new-session 'sleep 300'`
//!   — means the pane *is* that command. It is running (its pid is in
//!   the process table, or there would be no word at all): 忙碌.
//! - `pane_start_command` **empty** means the pane runs the user's login
//!   shell. Then the foreground command equals the shell's own name
//!   exactly while there is a prompt: 空闲, and anything else is 忙碌.
//!
//! Without that split the first case reads 空闲 while its command runs,
//! because the pane's own process is trivially its own foreground —
//! measured, not reasoned: `tmux new-session -d 'sleep 300'` reports
//! `pane_pid=18849, pane_current_command=sleep`, and `ps` says 18849 is
//! the `sleep`.
//!
//! A pane whose pid is not in the process table yields **no word**
//! rather than a guess; a session where no pane yields one is counted,
//! never shown (see [`Listing::unmapped`]).
//!
//! # Measured on this machine, 2026-08-16
//!
//! 9 tmux sessions on the user's default server, 817 processes, debug
//! build (`cargo test -p khor-node --test cost -- --ignored --nocapture
//! --test-threads=1`):
//!
//! - one `tmux list-panes -a`: **6.8 ms** median (6.4 min, 8.9 max),
//!   which is **0.14% of a core** at the app's five-second poll. It is
//!   the only cost in the session list that is a subprocess rather than
//!   a syscall, and it is roughly what the whole process-table snapshot
//!   costs beside it (5.1 ms) — so asking tmux doubles the cheap part
//!   of a list that already costs 13.6 ms, and no cache is warranted for
//!   the same reason `Procs::snapshot` has none.
//! - of the 9, **8 hold something already listed and are dropped**: six
//!   claude sessions, one codex, and one claude that had been resumed
//!   into a second process (see [`super::Sighting::pids`] — that last
//!   one is why the field is a list). The ninth is a bare shell and is
//!   the one row this module adds on this machine.
//!
//! That ratio is the argument for the whole duplicate rule: without it
//! this machine's list would grow eight rows that are all second copies
//! of rows already on it.

use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use khor_core::{SessionId, State};

use super::{Found, Multiplexed, Proc, Procs, Sighting};

/// This source's name, which is also the category its rows carry —
/// **and those are deliberately not the same word.**
///
/// `tmux` is who recognised the session; `shell` is what the session is.
/// Everywhere else in this directory those coincide, because a vendor's
/// row belongs to that vendor. Here the row belongs to the user: they
/// opened a shell, and tmux is the furniture it sits in. A category of
/// "tmux" would file "my own shell" under the name of a program the
/// person may not think of as owning anything.
pub const VENDOR: &str = "tmux";

/// Where the binary might be, tried in order until one starts.
///
/// PATH first, because that is the user's own answer. Then the two
/// places package managers put it: khor's list must not be shorter in
/// the app than in the terminal, and the app inherits the Finder's PATH,
/// which has neither. `KHOR_TMUX` overrides the lot — that is how the
/// tests point at a binary and how an odd install is fixed without a
/// release.
const CANDIDATES: [&str; 4] = [
    "tmux",
    "/opt/homebrew/bin/tmux",
    "/usr/local/bin/tmux",
    "/usr/bin/tmux",
];

/// How long one `list-panes` may take before khor walks away from it.
///
/// A wedged tmux server is a thing that happens, and the session list is
/// polled every 5 seconds by the app — one stuck call would freeze the
/// list instead of showing it without tmux rows. Generous against the
/// 6 ms this takes when it works, small against the poll it must not
/// eat.
const CALL_DEADLINE: Duration = Duration::from_millis(1000);

/// One line of the format below. Session fields repeat per pane; that is
/// tmux's shape, and it is cheaper than a second call to join against.
///
/// **Separated by the unit separator (`\x1f`), not `|`.** The line
/// carries two fields a person chooses freely — a session name and a
/// directory — and with `|` between them each one's `|` was the
/// other's problem: the name went last so its pipes could take the
/// rest of the line, and the path in front of it could then spill a
/// piece of itself into the name. A byte neither of them can hold
/// makes the order a matter of taste rather than a trade, and the
/// parser a plain split. (`SEP` is what tmux prints for it: format
/// strings pass ordinary bytes through untouched.)
///
/// The start command is still asked for as a `1`/`0` rather than as
/// text: it is a whole command line, and only its emptiness is being
/// asked about.
const SEP: char = '\x1f';
const FORMAT: &str = "#{session_id}\x1f#{session_created}\x1f#{session_activity}\
\x1f#{pane_pid}\x1f#{?pane_start_command,1,0}\x1f#{pane_current_command}\
\x1f#{pane_current_path}\x1f#{pane_id}\x1f#{session_name}";

/// What one look at a multiplexer found.
#[derive(Debug, Default, Clone)]
pub struct Listing {
    pub rows: Vec<Multiplexed>,
    /// Sessions tmux named that khor could not describe: a line in a
    /// shape this version does not parse, or a session whose every pane
    /// has left the process table.
    ///
    /// **Not the same thing as "no tmux".** A machine without tmux, or
    /// with tmux and no server, reports zero rows and zero here, because
    /// zero rows is then the truth. This counts only the case where tmux
    /// answered and khor could not read the answer — which is the
    /// "适配器过时" signal the CLI prints, said about a program instead
    /// of about a file.
    pub unmapped: usize,
}

/// One tmux server.
pub struct Tmux {
    /// `-L <name>`. `None` is the user's default server, which is the
    /// only one production ever touches; tests name their own so that
    /// they never see, and can never disturb, the user's sessions.
    socket: Option<String>,
    /// Where to look for the binary, in order. Resolved once at
    /// construction rather than per call: `KHOR_TMUX` read inside
    /// [`Tmux::sweep`] would make every test that sets it a hazard for
    /// every test running beside it.
    binaries: Vec<String>,
}

impl Tmux {
    /// The server `tmux` talks to with no arguments — the user's.
    pub fn default_server() -> Tmux {
        Tmux {
            socket: None,
            binaries: Tmux::binaries(),
        }
    }

    /// A server of one's own (`tmux -L <name>`).
    pub fn on_socket(name: &str) -> Tmux {
        Tmux {
            socket: Some(name.to_owned()),
            binaries: Tmux::binaries(),
        }
    }

    /// This one binary and no fallbacks. How the "there is no tmux on
    /// this machine" path is exercised without uninstalling tmux — it is
    /// the same path, [`Tmux::spawn`] finding nothing to start.
    pub fn at_binary(path: &str) -> Tmux {
        Tmux {
            socket: None,
            binaries: vec![path.to_owned()],
        }
    }

    /// The command a khor host runs to show this server's session
    /// `target` without disturbing whoever is already looking at it:
    /// a **grouped** session (`new-session -t`) gets its own client and
    /// its own size, so attaching from the app never squeezes the user's
    /// real tmux client to the smaller window — plain `attach` does
    /// exactly that. `destroy-unattached` cleans the grouped session up
    /// the moment our client detaches (the host dying detaches it), so
    /// nothing lingers in the user's session list.
    ///
    /// Runs through the same binary and socket the sweep used — an
    /// attach against a different server than the one that listed the
    /// session would "succeed" against the wrong world.
    /// `window` narrows the grouped client to one window — the one an
    /// agent actually sits in. Current-window is per-session inside a
    /// group, so selecting it here never moves the user's own client.
    /// (Pane selection and zoom are window-level flags the user's client
    /// shares, so neither is touched — on the ledger.)
    ///
    /// # The wheel: `mouse on`, on khor's own grouped session only
    ///
    /// The owner, 2026-10-01: 「滚轮翻 tmux 历史这，自动打开吧」. A tmux
    /// client sits in the alternate screen, and there a terminal's wheel
    /// is ↑ ↓ (Terminal.app does the same) — on a claude or a shell in
    /// the pane that walks input history, never tmux's. With `mouse` on,
    /// tmux asks the outer terminal for SGR mouse reports (`ESC [ ? 1006
    /// h`), xterm sends the wheel as one, and tmux enters copy mode and
    /// scrolls (measured, scroll_position 27; and in `tests/tmux_mouse.rs`).
    ///
    /// **Set on the grouped session this command creates, not on the
    /// user's.** `mouse` is a session option and a grouped session has
    /// options of its own (it inherits the global ones, never the
    /// target's), and tmux reads it off the *client's* session: so this
    /// turns the mouse on for khor's client and nobody else's. Measured
    /// on a private server: after the attach the grouped session reads
    /// `mouse on`, the target session and an unrelated one still read
    /// `off`, and `-g` still reads `off`. Nothing the user owns is
    /// written — no `-g`, no config file, no other session — so there is
    /// nothing to set back when khor leaves: `destroy-unattached` takes
    /// the grouped session, and its option with it, the moment khor's
    /// client detaches.
    ///
    /// What the user's own client *does* share is the pane: a wheel in
    /// khor puts the pane in copy mode, and a client of theirs looking
    /// at the same window sees it there too (`q` leaves; `-e` leaves on
    /// its own once scrolled back to the bottom).
    ///
    /// **Last in the chain, and `-q`.** A command that fails ends the
    /// rest of its chain; last, it can only cost itself, never the
    /// attach or `destroy-unattached`. `-q` keeps a tmux older than 2.1
    /// (no `mouse` option) quiet rather than flashing an error on the
    /// person's status line; [`Tmux::knows_mouse`] is where that case
    /// leaves its trace.
    ///
    /// # Which pane is in front, and shells that mark (item 21)
    ///
    /// Two more session options, on khor's grouped session alone like
    /// `mouse`:
    ///
    /// - `set-titles on` with `set-titles-string` `#{pane_id}
    ///   #{pane_title}`: tmux writes the title to khor's client whenever
    ///   the active pane or window changes, so the host's ledger knows
    ///   which pane is in front (`commands::Ledger::tmux`). The person's
    ///   own clients keep their own titles: both are session options.
    /// - `default-command`, when `shim` is given ([`Tmux::shell_shim`]):
    ///   a window or pane opened from khor's client starts its shell the
    ///   way khor starts its own, marking its commands; one opened from
    ///   the person's own client does not, since `default-command` is
    ///   read off the session the pane is made in (tmux(1), spawn).
    /// - `@khor_row`, a user option naming the row, so the session can
    ///   be found again to move between prompts ([`Tmux::jump_prompt`]).
    pub fn grouped_attach_argv(&self, row: &str, target: &str, window: Option<&str>, shim: Option<&str>) -> Vec<String> {
        let mut argv = vec![
            self.binaries
                .first()
                .cloned()
                .unwrap_or_else(|| "tmux".into()),
        ];
        if let Some(sock) = &self.socket {
            argv.push("-L".into());
            argv.push(sock.clone());
        }
        for a in ["new-session", "-t", target, ";"] {
            argv.push(a.into());
        }
        if let Some(w) = window {
            for a in ["select-window", "-t", w, ";"] {
                argv.push(a.into());
            }
        }
        for a in [
            "set-option",
            "destroy-unattached",
            "on",
            ";",
            // **Marks pass through** (roadmap 1.1): a shell in the pane
            // that wraps its command marks in tmux's passthrough reaches
            // khor's host only with this on. A window option, so it is
            // the one thing written on the window the person shares with
            // khor's grouped session; `-q` for a tmux older than 3.3.
            "set-option",
            "-w",
            "-q",
            "allow-passthrough",
            "on",
            ";",
            "set-option",
            "-q",
            "set-titles",
            "on",
            ";",
            "set-option",
            "-q",
            "set-titles-string",
            "#{pane_id} #{pane_title}",
            ";",
            "set-option",
            "-q",
            "@khor_row",
            row,
            ";",
        ] {
            argv.push(a.into());
        }
        if let Some(shim) = shim {
            for a in ["set-option", "-q", "default-command", shim, ";"] {
                argv.push(a.into());
            }
        }
        for a in ["set-option", "-q", "mouse", "on"] {
            argv.push(a.into());
        }
        argv
    }

    /// **Moves between shell prompts in the pane khor's client has in
    /// front** (item 21): tmux's own copy mode, `previous-prompt` and
    /// `next-prompt` (tmux 3.4 and later), which find the prompts by the
    /// unwrapped `OSC 133 ; A` khor's scripts give tmux. The face has no
    /// history there to scroll: tmux holds the screen. Past the last
    /// prompt, copy mode is left, which is the bottom. The session is
    /// the one whose `@khor_row` is `row` ([`Tmux::grouped_attach_argv`]).
    pub fn jump_prompt(&self, row: &str, up: bool) -> Result<(), String> {
        let id = self.session_of_row(row)?;
        let step = if up { "previous-prompt" } else { "next-prompt" };
        let at = |args: &[&str]| self.ask(args).ok_or_else(|| khor_catalog::msg::tmux_session_gone(row));
        let place = ["display-message", "-p", "-t", &id, "#{pane_in_mode} #{scroll_position} #{copy_cursor_y}"];
        let before = at(&place)?;
        at(&["copy-mode", "-t", &id, ";", "send-keys", "-X", "-t", &id, step])?;
        // Down from the last prompt goes nowhere: that is the bottom.
        if !up && before.starts_with('1') && at(&place)? == before {
            at(&["send-keys", "-X", "-t", &id, "cancel"])?;
        }
        Ok(())
    }

    /// **The front pane's history, for the phone's local scroll** (item
    /// 31; `audit/term-tmux-history.md`). khor's tmux client draws on the
    /// outer terminal's alternate screen, which keeps no history, and
    /// without it tmux redraws rather than scrolls whenever the outer
    /// terminal drains slowly — so the face's own scrollback never holds
    /// the pane's lines. They come from tmux instead: `capture-pane -e -J`
    /// (colours, wrapped lines joined), read from khor's grouped session
    /// (the one whose `@khor_row` is `row`), whose active pane is the one
    /// in front of khor's client.
    ///
    /// `since` is the [`HistoryAnchor`] the previous answer gave, or
    /// `None` for everything. The answer's `lines` are what tmux added to
    /// the history after it — or, when the anchor is gone (trimmed past
    /// [`HISTORY_CAP`], `clear-history`, another pane in front), all of
    /// it, with `refill` set. History only ever grows at its end, so the
    /// face appends; the screen below it is sent whole every time.
    ///
    /// Each read is one tmux command list, so the history, the screen
    /// and the numbers describing them are of one moment: tmux parses no
    /// pane output between the commands of a list.
    pub fn history(&self, row: &str, since: Option<&HistoryAnchor>) -> Result<PaneHistory, String> {
        let id = self.session_of_row(row)?;
        // A first read looks near the end for the anchor; one with no
        // anchor to look for takes everything at once.
        let mut anchor = since.filter(|a| !a.tail.is_empty());
        let mut window = if anchor.is_some() { HISTORY_FIRST_WINDOW } else { HISTORY_CAP };
        loop {
            let read = self
                .read_history(&id, window, anchor.is_some())
                .ok_or_else(|| khor_catalog::msg::tmux_session_gone(row))?;
            // Everything there is, or as much as is sent. The window is in
            // rows, as `history_size` is; the lines come joined (`-J`), so a
            // narrow pane's are fewer than the rows they fill (item 33:
            // counting lines took 100 of them for the whole history).
            let whole = window >= HISTORY_CAP || window >= read.size;
            match anchor.map(|a| read.place(a)) {
                Some(Place::At(end)) => return Ok(read.answer(Some(end))),
                Some(Place::Gone) => {
                    // The whole history in one more read, for the refill.
                    anchor = None;
                    if whole {
                        return Ok(read.answer(None));
                    }
                    window = HISTORY_CAP;
                    continue;
                }
                Some(Place::Further) | None => {}
            }
            if whole {
                return Ok(read.answer(None));
            }
            window = (window * 8).min(HISTORY_CAP);
        }
    }

    fn session_of_row(&self, row: &str) -> Result<String, String> {
        let sessions = self
            .ask(&["list-sessions", "-F", "#{session_id}\t#{@khor_row}"])
            .ok_or_else(|| khor_catalog::msg::tmux_session_gone(row))?;
        sessions
            .lines()
            .find_map(|l| l.split_once('\t').filter(|(_, r)| *r == row).map(|(id, _)| id.to_owned()))
            .ok_or_else(|| khor_catalog::msg::tmux_session_gone(row))
    }

    /// The newest `window` history rows of `session`'s front pane, wrapped
    /// lines joined, and its screen, in one command list; with `rows`, the
    /// same history rows unjoined as well, which place an anchor
    /// ([`HistoryRead::place`]).
    fn read_history(&self, session: &str, window: usize, rows: bool) -> Option<HistoryRead> {
        // A line no capture can hold: tmux prints the unit separator as
        // itself only under `-u`, which `spawn` passes.
        let split = "\x1fkhor-screen\x1f";
        let unjoined = "\x1fkhor-rows\x1f";
        let start = format!("-{window}");
        let mut args = vec![
            "display-message", "-p", "-t", session, "#{pane_id} #{alternate_on} #{history_size} #{pane_top} #{pane_left} #{mouse_any_flag} #{status} #{status-position}", ";",
            "capture-pane", "-p", "-e", "-J", "-S", &start, "-E", "-1", "-t", session, ";",
        ];
        if rows {
            args.extend(["display-message", "-p", unjoined, ";", "capture-pane", "-p", "-S", &start, "-E", "-1", "-t", session, ";"]);
        }
        args.extend(["display-message", "-p", split, ";", "capture-pane", "-p", "-e", "-t", session]);
        let out = self.ask_long(&args)?;
        let (head, rest) = out.split_once('\n')?;
        let mut words = head.split(' ');
        let pane = words.next()?.to_owned();
        let alternate = words.next()? == "1";
        let size: usize = words.next()?.trim().parse().ok()?;
        // Where the pane sits on khor's client: its place in the window,
        // under the status line when that is at the top (`status` is
        // `off`, `on` or a number of lines).
        let pane_top: u16 = words.next()?.parse().ok()?;
        let left: u16 = words.next()?.parse().ok()?;
        let mouse = words.next()? == "1";
        let status = match words.next()? {
            "off" => 0,
            "on" => 1,
            n => n.parse().unwrap_or(1),
        };
        let top = pane_top + if words.next()?.trim() == "top" { status } else { 0 };
        let (history, screen) = rest.split_once(&format!("{split}\n"))?;
        let (history, unjoined) = match rows {
            true => history.split_once(&format!("{unjoined}\n"))?,
            false => (history, ""),
        };
        // With no history, `-E -1` names a line that is not there and tmux
        // answers with the screen's first: nothing of it is history.
        let lines = |text: &str| -> Vec<String> { if size == 0 { Vec::new() } else { text.lines().map(str::to_owned).collect() } };
        let (history, rows) = (lines(history), lines(unjoined));
        let screen = screen.lines().map(str::to_owned).collect();
        Some(HistoryRead { pane, alternate, mouse, size, top, left, history, rows, screen })
    }

    /// [`Tmux::ask`] for an answer bigger than a pipe's buffer: a
    /// capture of [`HISTORY_CAP`] lines is ~400 KB, which tmux could not
    /// write while nobody read, so the reading runs beside the waiting.
    fn ask_long(&self, args: &[&str]) -> Option<String> {
        let mut child = self.spawn(args)?;
        let mut stdout = child.stdout.take()?;
        let reader = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            stdout.read_to_end(&mut bytes).ok().map(|_| bytes)
        });
        let deadline = Instant::now() + HISTORY_DEADLINE;
        let ok = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status.success(),
                Ok(None) if Instant::now() >= deadline => {
                    let _ = child.kill();
                    let _ = child.wait();
                    break false;
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(5)),
                Err(_) => break false,
            }
        };
        let bytes = reader.join().ok()??;
        ok.then(|| String::from_utf8_lossy(&bytes).into_owned())
    }

    /// **The `default-command` for khor's grouped session** (item 21): the
    /// person's own shell (`default-shell`, as a login shell, which is what
    /// tmux starts when `default-command` is empty), started the way a
    /// shell khor hosts is (`crate::integration::plan`), with the
    /// variables it carries over read from the tmux server's environment,
    /// which is what that shell would have seen. `None` — the option left
    /// alone — when the person set a `default-command` of their own (a
    /// `reattach-to-user-namespace` wrapper is the classic one), when
    /// khor has no way to start that shell marking (bash: its init file
    /// would cost the login shell), when the integration is turned off,
    /// or when a path will not quote. Quoted for `sh`, `zsh`, `bash` and
    /// `fish` alike, since tmux runs it with `default-shell -c`.
    pub fn shell_shim(&self, scripts: &std::path::Path) -> Option<String> {
        let own = self.ask(&["show-options", "-gv", "default-command"])?;
        if !own.trim().is_empty() {
            return None;
        }
        let shell = self.ask(&["show-options", "-gv", "default-shell"])?.trim().to_owned();
        let server_env = |k: &str| {
            self.ask(&["show-environment", "-g", k])
                .and_then(|l| l.trim().strip_prefix(&format!("{k}=")).map(str::to_owned))
        };
        let plan = crate::integration::plan(scripts, &[shell.clone(), "-l".into()], |k| {
            if k == "KHOR_SHELL_INTEGRATION" {
                std::env::var(k).ok()
            } else {
                server_env(k)
            }
        })?;
        let quote = |s: &str| (!s.contains(['\'', '\\'])).then(|| format!("'{s}'"));
        let mut out = String::from("exec env");
        for (k, v) in &plan.env {
            out.push_str(&format!(" {k}={}", quote(v)?));
        }
        out.push_str(&format!(" {}", quote(&shell)?));
        for a in &plan.args {
            out.push_str(&format!(" {}", quote(a)?));
        }
        Some(out)
    }

    /// Whether this server has a `mouse` option at all — tmux 2.1 and
    /// later. Read-only (`show-options -gv`), asked once per attach so
    /// that the one case where [`Tmux::grouped_attach_argv`]'s
    /// `set-option -q mouse on` silently does nothing leaves a line in
    /// the log. `false` also for a server that will not answer; the
    /// attach that follows then fails on its own, with its own words.
    pub fn knows_mouse(&self) -> bool {
        self.ask(&["show-options", "-gv", "mouse"]).is_some()
    }

    /// The window of session `target` whose panes hold any of `wanted`,
    /// asked of the live server at attach time — the sweep flattens
    /// panes away, and this is the one moment pane detail matters.
    /// `None` when tmux will not answer or no window qualifies; the
    /// caller then attaches without narrowing, which shows the session's
    /// own current window — not wrong, just wider.
    pub fn window_holding(&self, target: &str, wanted: &[u32], procs: &Procs) -> Option<String> {
        let text = self.ask(&[
            "list-panes",
            "-s",
            "-t",
            target,
            "-F",
            "#{window_id}|#{pane_pid}",
        ])?;
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            let Some((window, pid)) = line.split_once('|') else {
                continue;
            };
            let Ok(pid) = pid.trim().parse::<u32>() else {
                continue;
            };
            if procs.held_by(&[pid]).iter().any(|p| wanted.contains(p)) {
                return Some(window.to_owned());
            }
        }
        None
    }

    fn binaries() -> Vec<String> {
        match std::env::var("KHOR_TMUX") {
            Ok(p) if !p.is_empty() => vec![p],
            _ => CANDIDATES.iter().map(|s| (*s).to_owned()).collect(),
        }
    }

    /// Every session on this server, each with the pids it holds.
    pub fn sweep(&self, procs: &Procs) -> Listing {
        let mut listing = Listing::default();
        // No tmux, no server, or a server that would not answer in time:
        // zero rows and nothing counted. Khor never asked for tmux.
        let Some(text) = self.list_panes() else {
            return listing;
        };
        // Panes arrive grouped by nothing in particular, so the session
        // is assembled by id and the order of first sight is kept —
        // tmux lists in its own order and khor has no better one.
        let mut order: Vec<String> = Vec::new();
        let mut sessions: std::collections::BTreeMap<String, Gathering> =
            std::collections::BTreeMap::new();
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            let Some(pane) = Pane::parse(line) else {
                // A line in a shape this version does not understand.
                // Counted against the session it names — except that its
                // name is exactly what did not parse, so it is counted
                // on its own.
                listing.unmapped += 1;
                continue;
            };
            let entry = sessions.entry(pane.session_id.clone()).or_insert_with(|| {
                order.push(pane.session_id.clone());
                Gathering::new(&pane)
            });
            entry.absorb(&pane, procs);
        }
        for id in order {
            let Some(g) = sessions.remove(&id) else {
                continue;
            };
            match g.word {
                Some(word) => listing.rows.push(g.into_row(word, procs)),
                // Every pane gone from the process table: tmux still
                // names the session, khor cannot say a thing about it.
                None => listing.unmapped += 1,
            }
        }
        listing
    }

    /// The raw `list-panes` output, or `None` for every way of not
    /// getting one — binary absent, no server, non-zero exit, a wedged
    /// call. They are one outcome here because they have one meaning:
    /// khor has no tmux rows to add.
    fn list_panes(&self) -> Option<String> {
        self.ask(&["list-panes", "-a", "-F", FORMAT])
    }

    /// One tmux question, same binary resolution and the same deadline
    /// as the sweep — a wedged server must not freeze whoever asked.
    fn ask(&self, args: &[&str]) -> Option<String> {
        let mut child = self.spawn(args)?;
        let deadline = Instant::now() + CALL_DEADLINE;
        loop {
            match child.try_wait() {
                Ok(Some(status)) if status.success() => break,
                Ok(Some(_)) => return None,
                Ok(None) if Instant::now() >= deadline => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(5)),
                Err(_) => return None,
            }
        }
        // Read after the exit rather than during it, which is safe only
        // because the answer is small: one line per pane, ~55 bytes.
        // Beyond a pipe buffer (~1100 panes) tmux would block writing,
        // never exit, and be killed at the deadline — no tmux rows on a
        // machine with a thousand panes, which is a boundary worth
        // writing down and not worth a reader thread.
        let mut out = String::new();
        child.stdout.take()?.read_to_string(&mut out).ok()?;
        Some(out)
    }

    /// Starts a tmux command at the first candidate binary that exists.
    ///
    /// Only a missing binary moves on to the next one. A tmux that
    /// started and failed has answered — "no server running" is an
    /// answer — and trying `/usr/bin/tmux` after it would be asking a
    /// different program the same question.
    fn spawn(&self, args: &[&str]) -> Option<std::process::Child> {
        for binary in &self.binaries {
            let mut c = Command::new(binary);
            // **`-u`: tmux, assume UTF-8.** The unit separator between
            // this adaptor's fields is a control byte, and a tmux that
            // decided it was not in a UTF-8 locale printed it as `_`
            // (measured 2026-09-01 under launchd's environment: `$4_`
            // `_mandala`, every line unreadable, ten sessions counted
            // as a vendor khor could not read). The locale below is the
            // ordinary cure and is kept; this flag is the one tmux
            // itself offers, and it holds when the locale probe has
            // nothing to say — in a bare environment it said nothing,
            // and the parse only came back once the flag went in.
            c.arg("-u");

            // **The user's locale, or tmux answers in a dialect.** Under

            // launchd this process has no `LANG` at all, and a tmux started

            // without one renders every byte outside ASCII printables as `_`

            // — measured 2026-09-01: the unit separator this adaptor puts

            // between fields came back as `_`, every line failed to parse, and

            // the sweep that had just been deployed counted ten sessions it

            // could not read while the same binary from a terminal read all

            // ten. The same disease `host.rs` and `gui_host.rs` already treat

            // the same way (`Node::user_locale`).

            for (name, value) in crate::Node::user_locale() {
                c.env(name, value);
            }
            if let Some(sock) = &self.socket {
                c.arg("-L").arg(sock);
            }
            c.args(args)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::null());
            match c.spawn() {
                Ok(child) => return Some(child),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(_) => return None,
            }
        }
        None
    }
}

/// One pane, as tmux describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Pane {
    session_id: String,
    session_name: String,
    created_secs: i64,
    activity_secs: i64,
    pid: u32,
    /// tmux was given a command to run in this pane, rather than the
    /// user's shell.
    started_with_command: bool,
    current_command: String,
    /// Where this pane is standing right now (`pane_current_path`).
    current_path: String,
    /// tmux's own name for it (`%3`), the one khor's marks carry.
    pane_id: String,
}

impl Pane {
    fn parse(line: &str) -> Option<Pane> {
        let f: Vec<&str> = line.split(SEP).collect();
        let [id, created, activity, pid, started, current, path, pane_id, name] = f[..] else {
            return None;
        };
        Some(Pane {
            session_id: id.to_owned(),
            session_name: name.to_owned(),
            current_path: path.to_owned(),
            created_secs: created.parse().ok()?,
            activity_secs: activity.parse().ok()?,
            pid: pid.parse().ok()?,
            started_with_command: started == "1",
            current_command: current.to_owned(),
            pane_id: pane_id.to_owned(),
        })
    }
}

/// A session under construction, as its panes arrive.
struct Gathering {
    session_id: String,
    title: String,
    created_secs: i64,
    activity_secs: i64,
    pane_pids: Vec<u32>,
    /// The busiest word any pane has given so far. `None` while no pane
    /// has given one.
    word: Option<State>,
    cwd: String,
    /// The first pane running something other than its shell.
    running: Option<String>,
    /// Each pane's own reading.
    panes: Vec<super::PaneReading>,
}

impl Gathering {
    fn new(first: &Pane) -> Gathering {
        Gathering {
            session_id: first.session_id.clone(),
            title: first.session_name.clone(),
            created_secs: first.created_secs,
            activity_secs: first.activity_secs,
            pane_pids: Vec::new(),
            word: None,
            // **The first pane's**, which is where the session was
            // made. Later panes may have wandered, and it does not
            // matter: the row's place is folded up to a project root,
            // so a `cd` inside the checkout lands in the same workspace
            // (`khor_node::place`).
            cwd: first.current_path.clone(),
            running: None,
            panes: Vec::new(),
        }
    }

    fn absorb(&mut self, pane: &Pane, procs: &Procs) {
        self.pane_pids.push(pane.pid);
        self.activity_secs = self.activity_secs.max(pane.activity_secs);
        // 忙碌 wins: a session with one pane at a prompt and one running
        // a build is a session with something running in it. Reducing
        // the other way would let the idle pane hide the work.
        let word = word_of(pane, procs);
        if word == Some(State::Busy) && self.running.is_none() && !pane.current_command.is_empty() {
            self.running = Some(pane.current_command.clone());
        }
        self.panes.push(super::PaneReading {
            id: pane.pane_id.clone(),
            running: (word == Some(State::Busy) && !pane.current_command.is_empty()).then(|| pane.current_command.clone()),
            cwd: (!pane.current_path.is_empty()).then(|| pane.current_path.clone()),
        });
        match word {
            Some(State::Busy) => self.word = Some(State::Busy),
            Some(w) if self.word.is_none() => self.word = Some(w),
            _ => {}
        }
    }

    fn into_row(self, word: State, procs: &Procs) -> Multiplexed {
        Multiplexed {
            found: Found {
                category: khor_core::category::SHELL,
                kind: khor_core::kind::SHELL,
                sighting: Sighting {
                    vendor_session_id: leaf_of(&self.session_id, self.created_secs),
                    title: self.title,
                    word,
                    // tmux's own clock for the session: when it last saw
                    // anything happen in it. Not the instant the word
                    // flipped — tmux does not record that — but the
                    // closest thing it has, and the only one that does
                    // not come from khor's own clock.
                    at_ms: self.activity_secs.saturating_mul(1000),
                    // A multiplexer session is not a process, so it
                    // accounts for none. What it *holds* is the answer
                    // to the question these pids would have been asked,
                    // and that is carried below.
                    pids: Vec::new(),
                    // Empty is not a directory; tmux answers with one
                    // for a session whose panes are all gone.
                    cwd: (!self.cwd.is_empty()).then_some(self.cwd),
                },
            },
            running: self.running.clone(),
            panes: self.panes,
            holds: procs.held_by(&self.pane_pids),
        }
    }
}

/// The word one pane is worth. `None` = tmux named a pane whose process
/// khor cannot find, so there is nothing to read a word off.
fn word_of(pane: &Pane, procs: &Procs) -> Option<State> {
    let p: &Proc = procs.get(pane.pid)?;
    if pane.started_with_command {
        return Some(State::Busy);
    }
    Some(if same_program(&pane.current_command, &p.name) {
        State::Idle
    } else {
        State::Busy
    })
}

/// Whether two names are the same program.
///
/// A login shell wears a leading `-` by convention and two programs
/// disagree about whether to show it: `ps` prints `-zsh` for every pane
/// on this machine, and if the process table ever answered that way
/// while tmux said `zsh`, **every shell pane would read 忙碌 forever**.
///
/// **Not what this machine does, and that is worth writing down rather
/// than implying.** Measured 2026-08-16 across all nine panes on the
/// user's server: sysinfo reports the executable name, `zsh`, with no
/// dash, matching tmux exactly. So this normalisation guards a
/// convention that exists in the neighbourhood rather than a case
/// observed here — kept because it costs one call and the failure it
/// would prevent is silent and total.
fn same_program(a: &str, b: &str) -> bool {
    a.trim_start_matches('-') == b.trim_start_matches('-')
}

/// The stable part of a tmux session's identity.
///
/// **Not the name.** A tmux session can be renamed and usually is, while
/// a pin (`khor_sync::pins`) is a fact about the session itself — keyed
/// by a name it would follow the name to whatever session took it.
///
/// `$120` alone would not do either: tmux numbers sessions from zero
/// again every time its server restarts, so a pin left on `$0` would
/// land on an unrelated session weeks later. The creation instant closes
/// that, costs one field khor already asks for, and never changes for a
/// session that lives.
fn leaf_of(session_id: &str, created_secs: i64) -> String {
    format!("{created_secs}-{}", session_id.trim_start_matches('$'))
}

/// Whether a khor session leaf is one [`leaf_of`] minted — the one place
/// that may take the format apart (a second parser is how two readers
/// drift). `khor open`'s leaves are pure hex with no dash
/// (`link::fresh_leaf`), so the two mints cannot collide.
pub fn is_tmux_leaf(leaf: &str) -> bool {
    tmux_target_of(leaf).is_some()
}

/// The tmux target (`$<id>`) back out of a leaf, or `None` for a leaf
/// some other mint produced.
pub fn tmux_target_of(leaf: &str) -> Option<String> {
    let (created, id) = leaf.split_once('-')?;
    if created.is_empty() || id.is_empty() {
        return None;
    }
    if !created.bytes().all(|b| b.is_ascii_digit()) || !id.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some(format!("${id}"))
}

/// The khor session id for a tmux session.
///
/// Deliberately **not** [`super::id_for`], which mints `tui/…`: that
/// function exists so a vendor's hook and its files agree on one string,
/// and tmux has no hook to agree with. A tmux session is a shell
/// (`khor_core::kind::SHELL`), which is also what stops `khor close`
/// from thinking it started it.
pub fn id_of(session_id: &str, created_secs: i64) -> SessionId {
    SessionId(format!(
        "{}/{}",
        khor_core::kind::SHELL,
        crate::live::clean_leaf(&leaf_of(session_id, created_secs))
    ))
}

/// A script that answers `list-panes` like tmux would, for tests that
/// need a multiplexer whose contents they chose.
///
/// Lives outside the test module because the rule it exists to test —
/// a session holding a listed process is not a second row — is decided
/// in `crate::live`, not here.
#[cfg(test)]
pub(crate) fn fake_tmux(tag: &str, output: &str) -> crate::testdir::TempBin {
    let dir = crate::testdir::TempTree::fresh(&format!("faketmux-{tag}"));
    let path = dir.join("tmux");
    std::fs::write(
        &path,
        format!("#!/bin/sh\ncat <<'KHOREOF'\n{output}KHOREOF\n"),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let path = path.to_string_lossy().to_string();
    crate::testdir::TempBin::new(dir, path)
}

/// The most history lines one read sends: the owner's phone keeps 5 000
/// to 10 000 lines of its own, and tmux's `history-limit` (2 000 unless
/// set) bounds it before this does.
pub const HISTORY_CAP: usize = 10_000;
/// The newest rows read first, looking for the anchor; widened eight
/// times over until it is found or the cap is reached.
const HISTORY_FIRST_WINDOW: usize = 200;
/// A capture of the cap took 34 ms (measured 2026-10-05); the deadline
/// is for a wedged server, as [`CALL_DEADLINE`] is.
const HISTORY_DEADLINE: Duration = Duration::from_secs(3);
/// Lines an anchor carries: what proves the copy ends where the rows
/// say. A program that repeats itself (Claude Code's frames scroll into
/// the history again and again) needs more than a line or two for that.
const ANCHOR_LINES: usize = 8;
/// The most bytes of lines one answer carries: under the wire's frame
/// (`proto::MAX_FRAME`, 512 KiB) with room for the envelope.
const HISTORY_BYTES: usize = 384 * 1024;
/// The most bytes of colours an answer leads with ([`escapes`]).
const LEAD_BYTES: usize = 4 * 1024;

/// Where the face's copy of a pane's history ends (item 31): handed back
/// by [`Tmux::history`] and given to it again, opaque to the face.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HistoryAnchor {
    /// The pane the history is of (`%3`).
    pub pane: String,
    /// tmux's `history_size` then, in rows: the history grows at its end,
    /// so the copy ends this many rows from its start until tmux takes
    /// rows away (the pane grew and pulled its newest rows back onto the
    /// screen, `history-limit` dropped a tenth of the oldest, or
    /// `clear-history`).
    pub size: usize,
    /// The plain text of the history's last lines, oldest first.
    pub tail: Vec<String>,
}

/// One read of the front pane's history ([`Tmux::history`]).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PaneHistory {
    pub pane: String,
    /// The pane is on its own alternate screen (vim, less, htop): its
    /// history is not what it shows, and the program scrolls itself.
    pub alternate: bool,
    /// `lines` are the whole history (or as much of its end as one answer
    /// carries), not what follows the anchor: the face clears its copy
    /// first.
    pub refill: bool,
    /// The colours in force where the face's copy ends, so `lines` draw
    /// as tmux would: `capture-pane -e` writes each line's escapes
    /// against the line before it, not from scratch. Empty on a refill.
    pub lead: String,
    /// History lines, oldest first, with their escapes.
    pub lines: Vec<String>,
    /// The pane's screen, top to bottom, with its escapes.
    pub screen: Vec<String>,
    pub anchor: HistoryAnchor,
    /// Rows above the pane on khor's client: its place in the window,
    /// and tmux's status line when that is at the top. Tail-appended.
    #[serde(default)]
    pub top: u16,
    /// Columns left of the pane (a split). Tail-appended.
    #[serde(default)]
    pub left: u16,
    /// The pane's program has taken the mouse (`#{mouse_any_flag}`):
    /// Claude Code does, and then tmux hands it the wheel, and what tmux
    /// keeps as history is its frames drawn over and over, not a
    /// transcript. The program scrolls itself, as on its alternate screen
    /// (item 36). Tail-appended.
    #[serde(default)]
    pub mouse: bool,
}

struct HistoryRead {
    pane: String,
    alternate: bool,
    mouse: bool,
    size: usize,
    top: u16,
    left: u16,
    /// The window's history, wrapped lines joined.
    history: Vec<String>,
    /// The same window's rows, unjoined and plain; read only to place an
    /// anchor.
    rows: Vec<String>,
    screen: Vec<String>,
}

/// Where a read puts the face's copy ([`HistoryRead::place`]).
#[derive(Debug, PartialEq, Eq)]
enum Place {
    /// The copy ends at this line of the read's history.
    At(usize),
    /// Past the read's window: look further back.
    Further,
    /// tmux took rows away since: only the whole history is right.
    Gone,
}

impl HistoryRead {
    /// **Where the face's copy ends** (item 33). By count, not by text: the
    /// copy ended `a.size` rows into the history, so what followed it is
    /// the newest `size - a.size` rows, which [`row_counts`] turns into
    /// joined lines; the lines just before them must then read as the
    /// anchor's. Searching for the text instead found the newest place a
    /// repeated run of lines stood (Claude Code's frames, `yes`), and the
    /// copy silently missed everything after the anchor. When tmux has
    /// taken rows away (the history is shorter than the copy, or the lines
    /// at the count do not match), the copy is redone whole.
    fn place(&self, a: &HistoryAnchor) -> Place {
        if a.pane != self.pane {
            return Place::Gone;
        }
        let Some(new) = self.size.checked_sub(a.size) else { return Place::Gone };
        let Some(counts) = row_counts(&self.history, &self.rows) else { return Place::Gone };
        // The window holds the history's first row, so its first line is
        // whole; otherwise that line may be a wrapped line's end.
        let all = self.rows.len() >= self.size;
        let short = if all { Place::Gone } else { Place::Further };
        // Joined lines that make up the newest `new` rows.
        let mut rows = 0;
        let mut k = 0;
        while rows < new {
            let Some(i) = counts.len().checked_sub(k + 1) else { return short };
            rows += counts[i];
            k += 1;
        }
        if rows != new {
            return Place::Gone;
        }
        let n = a.tail.len();
        let Some(end) = self.history.len().checked_sub(k + 1).filter(|&e| e + 1 >= n && (all || e >= n)) else { return short };
        let held = self.history[end + 1 - n..=end].iter().map(|l| plain(l));
        if held.eq(a.tail.iter().cloned()) { Place::At(end) } else { Place::Gone }
    }

    /// The answer, given where in `history` the anchor ended (`None`:
    /// everything, a refill).
    fn answer(self, found: Option<usize>) -> PaneHistory {
        let tail = self.history[self.history.len().saturating_sub(ANCHOR_LINES)..]
            .iter()
            .map(|l| plain(l))
            .collect();
        let anchor = HistoryAnchor { pane: self.pane.clone(), size: self.size, tail };
        let start = found.map_or(0, |end| end + 1);
        // Under one frame (`proto::MAX_FRAME`): the oldest lines go, and
        // with a gap left behind the face's copy, it starts again.
        let mut first = start;
        let mut bytes: usize = self.screen.iter().chain(&self.history[start..]).map(|l| l.len() + 1).sum();
        while bytes > HISTORY_BYTES && first < self.history.len() {
            bytes -= self.history[first].len() + 1;
            first += 1;
        }
        let refill = found.is_none() || first > start;
        let lead = escapes(&self.history[..first]);
        let lines = self.history[first..].to_vec();
        PaneHistory {
            pane: self.pane,
            alternate: self.alternate,
            refill,
            lead,
            lines,
            screen: self.screen,
            anchor,
            top: self.top,
            left: self.left,
            mouse: self.mouse,
        }
    }
}

/// How many rows each of `lines` (a capture with wrapped lines joined)
/// takes in `rows` (the same capture unjoined), told apart by their ink:
/// a wrapped line's rows hold its characters and nothing else, so the
/// non-blank characters add up. `None` when they do not.
fn row_counts(lines: &[String], rows: &[String]) -> Option<Vec<usize>> {
    let ink = |s: &str| plain(s).chars().filter(|c| !c.is_whitespace()).count();
    let mut out = Vec::with_capacity(lines.len());
    let mut r = 0;
    for line in lines {
        let want = ink(line);
        let mut got = 0;
        let mut n = 0;
        loop {
            got += ink(rows.get(r)?);
            r += 1;
            n += 1;
            if got >= want {
                break;
            }
        }
        if got != want {
            return None;
        }
        out.push(n);
    }
    (r == rows.len()).then_some(out)
}

/// A captured line without its escapes: CSI sequences and OSC strings
/// (tmux writes hyperlinks as OSC 8).
fn plain(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\x1b' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('[') => {
                for c in chars.by_ref() {
                    if ('\x40'..='\x7e').contains(&c) {
                        break;
                    }
                }
            }
            Some(']') => {
                while let Some(c) = chars.next() {
                    if c == '\x07' {
                        break;
                    }
                    if c == '\x1b' {
                        chars.next();
                        break;
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// Only the SGR escapes of `lines`, in order, from the last reset on and
/// none twice running: replayed from a reset, they leave the colours where
/// the last of the lines left them. Past [`LEAD_BYTES`] only the newest
/// are kept — a run that long without a reset is a program that sets
/// every attribute again as it goes.
fn escapes(lines: &[String]) -> String {
    let mut out = String::new();
    let mut last = "";
    for line in lines {
        let mut rest = line.as_str();
        while let Some(i) = rest.find("\x1b[") {
            let tail = &rest[i + 2..];
            let Some(j) = tail.find(|c: char| ('\x40'..='\x7e').contains(&c)) else { break };
            if tail.as_bytes()[j] == b'm' {
                let sgr = &rest[i..i + 2 + j + 1];
                if sgr == "\x1b[0m" || sgr == "\x1b[m" || sgr.starts_with("\x1b[0;") {
                    out.clear();
                }
                if sgr != last {
                    out.push_str(sgr);
                    last = sgr;
                }
            }
            rest = &tail[j + 1..];
        }
    }
    if out.len() > LEAD_BYTES {
        let cut = out[out.len() - LEAD_BYTES..].find("\x1b[").map_or(out.len(), |k| out.len() - LEAD_BYTES + k);
        out.drain(..cut);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proc_named(name: &str, ppid: Option<u32>) -> Proc {
        Proc {
            name: name.into(),
            started_ms: 0,
            cwd: None,
            ppid,
        }
    }

    /// A test server of khor's own. Named after the process so two test
    /// binaries never meet, and **never without `-L`**: the user's own
    /// sessions are read-only scenery for this suite, and a missing
    /// socket argument here would kill them.
    struct Server {
        socket: String,
    }

    impl Server {
        fn start(tag: &str) -> Server {
            Server {
                socket: format!("khor-{tag}-{}", std::process::id()),
            }
        }

        fn tmux(&self, args: &[&str]) -> Option<String> {
            let out = Command::new("tmux")
                .arg("-L")
                .arg(&self.socket)
                .args(args)
                .stdin(Stdio::null())
                .output()
                .ok()?;
            String::from_utf8(out.stdout).ok()
        }

        fn at(&self) -> Tmux {
            Tmux::on_socket(&self.socket)
        }

        /// Blocks until tmux reports what the test set up, or gives up.
        ///
        /// **These tests had fixed sleeps first, and one lost a race.**
        /// On 2026-08-16 a full suite run that happened to overlap a
        /// compile saw `send-keys "sleep 300"` fail to become a running
        /// `sleep` inside 900 ms; the pane read 空闲 and the assertion
        /// failed — once, then passed eleven runs in a row, which is
        /// exactly the shape of failure this repo refuses to write off
        /// as "flaky, just re-run". A test that is only true on an idle
        /// machine is not testing khor.
        ///
        /// Waits on the same `list-panes` khor itself reads, so what it
        /// waits for is what the assertion will see.
        fn until(&self, ready: impl Fn(&str) -> bool) {
            let deadline = Instant::now() + Duration::from_secs(10);
            loop {
                let seen = self
                    .tmux(&[
                        "list-panes",
                        "-a",
                        "-F",
                        "#{session_name}|#{pane_current_command}",
                    ])
                    .unwrap_or_default();
                if ready(&seen) || Instant::now() >= deadline {
                    return;
                }
                std::thread::sleep(Duration::from_millis(25));
            }
        }

        /// Waits for one session's foreground command to be `want`.
        fn until_running(&self, session: &str, want: &str) {
            let line = format!("{session}|{want}");
            self.until(|seen| seen.lines().any(|l| l == line));
        }

        /// Waits for a session to exist at all. Deliberately not "waits
        /// for a `zsh` prompt": the shell here is whatever the machine
        /// running the suite uses, and naming one would make these tests
        /// quietly time out on a bash machine rather than fail on it.
        fn until_present(&self, session: &str) {
            let prefix = format!("{session}|");
            self.until(|seen| seen.lines().any(|l| l.starts_with(&prefix)));
        }

        /// Waits for a session to settle **at a prompt**.
        ///
        /// # The other half of the rule the batch above only did once
        ///
        /// `until_running` was added because "send keys, sleep 900ms,
        /// assert 忙碌" measured how busy the machine was rather than
        /// what khor read. **The idle side had the same hole and kept
        /// it**: `new-session -d` returns the moment tmux has the
        /// session, and a shell that is still running its startup files
        /// has a foreground command that is *not* the shell — which is
        /// [`word_of`]'s definition of 忙碌. So "a prompt is 空闲" was
        /// true only on a machine quiet enough for the shell to have
        /// finished. Observed once in a full-suite run 2026-08-17
        /// (`Some(Busy)` where 空闲 was expected) and green on five
        /// straight runs of the test alone, which is the signature.
        ///
        /// # Why sampling alone was not enough — the settled look-alike
        ///
        /// This wait first shipped as the sampling loop below on its
        /// own, and the same red came back with it compiled in (later
        /// the same day, same two values). Startup files run *external*
        /// commands with the shell back in the foreground **between**
        /// them, and a sample landing in such a gap is indistinguishable
        /// from a prompt; the sweep a few milliseconds later meets the
        /// next startup command and reads 忙碌. Traced live on this
        /// machine's real rc: the sampled condition held from 0.2s in,
        /// while the buffered keys below were not consumed until 0.9s.
        ///
        /// So the sample now has a precondition that is evidence rather
        /// than a look: keys typed at a pane sit in the pty's buffer,
        /// and an interactive shell reads them only once its startup
        /// files have finished — so the sentinel's product appearing on
        /// disk *is* "startup is over", by definition. The sampling
        /// loop still runs after it: the sentinel command itself, and
        /// any prompt hook that follows it, must also leave the
        /// foreground.
        ///
        /// Two boundaries, written down rather than implied. A startup
        /// file that reads stdin eats the sentinel (observed while
        /// building the red control for this fix: a `read -t` swallowed
        /// the buffered line), leaving only the deadline. And a prompt
        /// hook running external commands *after* the sentinel re-opens
        /// a far smaller gap of the same shape — never observed, and
        /// the first place to look if this test ever reds at these two
        /// values again.
        ///
        /// # Why it cannot be spelled as "wait for the shell"
        ///
        /// The same reason [`Server::until_present`] gives: the shell is
        /// whatever the machine running the suite uses. So this waits on
        /// **khor's own rule** instead — the pane's foreground command
        /// *is* the pane's own process ([`same_program`]) — which is also
        /// what makes it the right thing to wait for: it is the very
        /// comparison the assertion is about to make, read from the same
        /// `list-panes`.
        ///
        /// A pane khor cannot find a process for is not settled either:
        /// [`word_of`] answers `None` there, and a row with no word is
        /// not the row this waits for.
        fn until_at_a_prompt(&self, session: &str) {
            let marker =
                std::env::temp_dir().join(format!("khor-prompt-{}-{session}", std::process::id()));
            let _ = std::fs::remove_file(&marker);
            self.tmux(&[
                "send-keys",
                "-t",
                session,
                &format!("touch '{}'", marker.display()),
                "Enter",
            ]);
            let rc_deadline = Instant::now() + Duration::from_secs(10);
            while !marker.exists() && Instant::now() < rc_deadline {
                std::thread::sleep(Duration::from_millis(25));
            }
            let _ = std::fs::remove_file(&marker);
            let prefix = format!("{session}|");
            let deadline = Instant::now() + Duration::from_secs(10);
            loop {
                let seen = self
                    .tmux(&[
                        "list-panes",
                        "-a",
                        "-F",
                        "#{session_name}|#{pane_pid}|#{pane_current_command}",
                    ])
                    .unwrap_or_default();
                let procs = Procs::snapshot();
                let mut panes = seen.lines().filter(|l| l.starts_with(&prefix)).peekable();
                // `all` over nothing is true, so the session has to be
                // there before its panes can be settled.
                let settled = panes.peek().is_some()
                    && panes.all(|l| {
                        let mut parts = l.split('|').skip(1);
                        let Some(pid) = parts.next().and_then(|p| p.parse::<u32>().ok()) else {
                            return false;
                        };
                        let Some(command) = parts.next() else {
                            return false;
                        };
                        procs
                            .get(pid)
                            .is_some_and(|p| same_program(command, &p.name))
                    });
                if settled || Instant::now() >= deadline {
                    return;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }

    impl Drop for Server {
        fn drop(&mut self) {
            self.tmux(&["kill-server"]);
            // The socket file survives `kill-server`, and one per test
            // run accumulates in /tmp forever. Same discipline as
            // closing a dev server's port: what a test starts, it takes
            // away.
            let dir = std::env::var("TMUX_TMPDIR").unwrap_or_else(|_| "/tmp".into());
            #[cfg(unix)]
            let uid = unsafe { libc::getuid() };
            #[cfg(not(unix))]
            let uid = 0;
            let _ = std::fs::remove_file(
                std::path::Path::new(&dir)
                    .join(format!("tmux-{uid}"))
                    .join(&self.socket),
            );
        }
    }

    fn have_tmux() -> bool {
        Command::new("tmux")
            .arg("-V")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
    }

    /// **The promise, stated as a test**: a machine with no tmux gets no
    /// rows, no count, and no error. Written against a binary name that
    /// cannot exist rather than by uninstalling tmux, which is the same
    /// code path — [`Tmux::spawn`] returning `None`.
    #[test]
    fn no_tmux_is_no_rows_and_no_complaint() {
        let listing = Tmux::at_binary("/nonexistent/khor-test/tmux").sweep(&Procs::default());
        assert_eq!(listing.rows.len(), 0);
        assert_eq!(
            listing.unmapped, 0,
            "khor never asked for tmux; not having it is not khor being out of date"
        );
    }

    /// A tmux that runs and says "no server": also nothing, also not a
    /// complaint. Separated from the test above because they fail
    /// differently — one never starts, one exits non-zero — and one
    /// covering both would pass while the other path was broken.
    #[test]
    fn a_server_that_is_not_running_is_no_rows_and_no_complaint() {
        if !have_tmux() {
            return;
        }
        let quiet = Tmux::on_socket(&format!("khor-absent-{}", std::process::id()));
        let listing = quiet.sweep(&Procs::default());
        assert_eq!(listing.rows.len(), 0);
        assert_eq!(listing.unmapped, 0);
    }

    /// The attach turns the mouse on as the chain's last command, for the
    /// session the chain itself created: no `-t` (which would name
    /// somebody's session), no `-g` (which would name everybody's), and
    /// `-q`. The live half — which session ends up `on` — is
    /// `tests/tmux_mouse.rs`, through the whole attach.
    #[test]
    fn the_attach_turns_the_mouse_on_last_and_only_for_its_own_session() {
        for window in [None, Some("@3")] {
            let argv = Tmux::on_socket("s").grouped_attach_argv("shell/1-1", "$1", window, Some("exec zsh -l"));
            let tail: Vec<&str> = argv[argv.len() - 5..].iter().map(String::as_str).collect();
            assert_eq!(tail, [";", "set-option", "-q", "mouse", "on"], "{argv:?}");
            let mouse = argv.iter().position(|a| a == "mouse").unwrap();
            let own = argv[..mouse].iter().rposition(|a| a == ";").unwrap();
            assert!(
                !argv[own..].iter().any(|a| a == "-g" || a == "-t"),
                "{argv:?}"
            );
            // The title names the pane in front; nothing here is global.
            let titles = argv.iter().position(|a| a == "set-titles-string").unwrap();
            assert_eq!(argv[titles + 1], "#{pane_id} #{pane_title}");
            assert!(argv.iter().any(|a| a == "default-command"), "{argv:?}");
            assert!(!argv.iter().any(|a| a == "-g"), "{argv:?}");
        }
    }

    /// The question behind the one log line: a tmux that has the option
    /// answers, a missing binary does not.
    #[test]
    fn a_real_tmux_knows_the_mouse_option_and_a_missing_one_does_not() {
        assert!(!Tmux::at_binary("/nonexistent/khor-test/tmux").knows_mouse());
        if !have_tmux() {
            return;
        }
        let s = Server::start("knows");
        s.tmux(&["-f", "/dev/null", "new-session", "-d", "-s", "one"]);
        s.until_present("one");
        assert!(s.at().knows_mouse());
    }

    /// **Prompt to prompt through tmux** (item 21), on a server of the
    /// test's own: the session is found by its `@khor_row`, up walks back
    /// over the lines a shell marked with `OSC 133 ; A`, down walks
    /// forward, and down from the last leaves copy mode.
    #[test]
    fn prompts_are_walked_in_tmux_copy_mode_for_khors_session() {
        if !have_tmux() {
            return;
        }
        let s = Server::start("jump");
        s.tmux(&["-f", "/dev/null", "new-session", "-d", "-s", "one", "-x", "80", "-y", "20", "/bin/sh"]);
        s.until_present("one");
        s.tmux(&["set-option", "-t", "one", "@khor_row", "shell/9-9"]);
        s.tmux(&["send-keys", "-t", "one", "printf '\\033]133;A\\033\\\\p1\\n\\033]133;A\\033\\\\p2\\n'", "Enter"]);
        let deadline = Instant::now() + Duration::from_secs(10);
        while !s.tmux(&["capture-pane", "-p", "-t", "one"]).unwrap_or_default().lines().any(|l| l == "p2") {
            assert!(Instant::now() < deadline, "the marks were never printed");
            std::thread::sleep(Duration::from_millis(20));
        }
        let at = || s.tmux(&["display-message", "-p", "-t", "one", "#{pane_in_mode} #{copy_cursor_line}"]).unwrap_or_default().trim().to_owned();
        let t = s.at();
        assert!(t.jump_prompt("shell/0-0", true).is_err(), "no session is that row's");
        t.jump_prompt("shell/9-9", true).unwrap();
        assert_eq!(at(), "1 p2");
        t.jump_prompt("shell/9-9", true).unwrap();
        assert_eq!(at(), "1 p1");
        t.jump_prompt("shell/9-9", false).unwrap();
        assert_eq!(at(), "1 p2");
        t.jump_prompt("shell/9-9", false).unwrap();
        assert!(at().starts_with('0'), "{}", at());
    }

    /// Waits until the pane `target` shows `line` on its screen.
    fn until_shown(s: &Server, target: &str, line: &str) {
        let deadline = Instant::now() + Duration::from_secs(20);
        while !s.tmux(&["capture-pane", "-p", "-t", target]).unwrap_or_default().lines().any(|l| l == line) {
            assert!(Instant::now() < deadline, "{line} was never shown");
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn numbers(lines: &[String]) -> Vec<u32> {
        lines.iter().filter_map(|l| plain(l).trim().parse().ok()).collect()
    }

    /// **Item 31, the host's half**: the front pane's history from before
    /// anyone asked, then only what followed — never a line twice, never
    /// a line missed — and the whole again when the anchor is gone or
    /// another pane is in front.
    #[test]
    fn history_is_read_whole_then_from_the_anchor_on() {
        if !have_tmux() {
            return;
        }
        let s = Server::start("hist");
        s.tmux(&["-f", "/dev/null", "new-session", "-d", "-s", "one", "-x", "60", "-y", "20", "/bin/sh"]);
        s.until_present("one");
        s.tmux(&["set-option", "-t", "one", "@khor_row", "shell/9-9"]);
        s.tmux(&["send-keys", "-t", "one", "clear; seq 1 300", "Enter"]);
        until_shown(&s, "one", "300");
        let t = s.at();
        assert!(t.history("shell/0-0", None).is_err(), "no session is that row's");

        let first = t.history("shell/9-9", None).unwrap();
        assert!(first.refill && !first.alternate, "{first:?}");
        assert!(first.pane.starts_with('%'), "{}", first.pane);
        let mut seen = numbers(&first.lines);
        seen.extend(numbers(&first.screen));
        assert_eq!(seen, (1..=300).collect::<Vec<_>>(), "the history and the screen are the pane, once");

        s.tmux(&["send-keys", "-t", "one", "seq 301 400", "Enter"]);
        until_shown(&s, "one", "400");
        let next = t.history("shell/9-9", Some(&first.anchor)).unwrap();
        assert!(!next.refill, "{next:?}");
        let mut held = numbers(&first.lines);
        held.extend(numbers(&next.lines));
        held.extend(numbers(&next.screen));
        assert_eq!(held, (1..=400).collect::<Vec<_>>(), "appended exactly what followed");

        // Nothing new: nothing sent.
        let still = t.history("shell/9-9", Some(&next.anchor)).unwrap();
        assert!(!still.refill && still.lines.is_empty(), "{still:?}");

        // The anchor gone: everything again.
        s.tmux(&["clear-history", "-t", "one"]);
        s.tmux(&["send-keys", "-t", "one", "seq 401 450", "Enter"]);
        until_shown(&s, "one", "450");
        let cleared = t.history("shell/9-9", Some(&still.anchor)).unwrap();
        assert!(cleared.refill, "{cleared:?}");

        // Another window in front: its history, whole.
        s.tmux(&["new-window", "-t", "one", "/bin/sh"]);
        s.tmux(&["send-keys", "-t", "one", "printf '\\033[?1049h'", "Enter"]);
        let deadline = Instant::now() + Duration::from_secs(10);
        let other = loop {
            let h = t.history("shell/9-9", Some(&cleared.anchor)).unwrap();
            if h.alternate || Instant::now() > deadline {
                break h;
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        assert_ne!(other.pane, first.pane);
        assert!(other.refill && other.alternate, "{other:?}");
    }

    /// tmux's own history of `target`, wrapped lines joined, as plain text.
    fn tmux_history(s: &Server, target: &str) -> Vec<String> {
        s.tmux(&["capture-pane", "-p", "-J", "-S", "-", "-E", "-1", "-t", target])
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    fn plains(lines: &[String]) -> Vec<String> {
        lines.iter().map(|l| plain(l)).collect()
    }

    /// The face's copy is tmux's history, line for line; says where not.
    fn same(copy: &[String], tmux: &[String], what: &str) {
        let at = copy.iter().zip(tmux).position(|(a, b)| a != b).unwrap_or(copy.len().min(tmux.len()));
        assert!(
            copy.len() == tmux.len() && at == copy.len(),
            "{what}: the copy has {} lines, tmux {}; first difference at {at}: {:?} / {:?}",
            copy.len(),
            tmux.len(),
            copy.get(at),
            tmux.get(at),
        );
    }

    /// **A narrow pane's wrapped lines** (item 33, the owner's phone at 44
    /// columns): tmux counts its history in rows, a read sends lines with
    /// their wraps joined. More rows than the first look-back window, and
    /// lines that repeat (Claude Code's frames do), must still come out as
    /// exactly what followed the copy: measured before the fix, the read
    /// came back as a "refill" of the last 100 lines, and the face dropped
    /// its 10 000.
    #[test]
    fn wrapped_and_repeated_lines_are_appended_exactly() {
        if !have_tmux() {
            return;
        }
        let s = Server::start("histwrap");
        s.tmux(&["-f", "/dev/null", "new-session", "-d", "-s", "one", "-x", "20", "-y", "10", "/bin/sh"]);
        s.until_present("one");
        s.tmux(&["set-option", "-t", "one", "@khor_row", "shell/9-9"]);
        let x = "x".repeat(30);
        s.tmux(&["send-keys", "-t", "one", &format!("clear; yes {x} | head -n 300; echo d1"), "Enter"]);
        until_shown(&s, "one", "d1");
        let t = s.at();
        let first = t.history("shell/9-9", None).unwrap();
        same(&plains(&first.lines), &tmux_history(&s, "one"), "whole");
        // 150 more of the same line: 300 rows, past the first window.
        s.tmux(&["send-keys", "-t", "one", &format!("yes {x} | head -n 150; echo d2"), "Enter"]);
        until_shown(&s, "one", "d2");
        let next = t.history("shell/9-9", Some(&first.anchor)).unwrap();
        assert!(!next.refill, "{} lines, refill", next.lines.len());
        let mut held = plains(&first.lines);
        held.extend(plains(&next.lines));
        same(&held, &tmux_history(&s, "one"), "appended");
    }

    /// **The pane grew** (the phone's keyboard went down, or another face
    /// took the size back): tmux pulls the newest history rows back onto
    /// the screen, so the copy's last lines are no longer history. The
    /// answer is the whole history again, never a part of it.
    #[test]
    fn a_pane_that_grew_is_read_whole_again() {
        if !have_tmux() {
            return;
        }
        let s = Server::start("histgrow");
        s.tmux(&["-f", "/dev/null", "new-session", "-d", "-s", "one", "-x", "20", "-y", "10", "/bin/sh"]);
        s.until_present("one");
        s.tmux(&["set-option", "-t", "one", "@khor_row", "shell/9-9"]);
        s.tmux(&["send-keys", "-t", "one", &format!("clear; yes {} | head -n 300; echo d1", "y".repeat(30)), "Enter"]);
        until_shown(&s, "one", "d1");
        let t = s.at();
        let first = t.history("shell/9-9", None).unwrap();
        s.tmux(&["resize-window", "-t", "one", "-y", "30"]);
        let deadline = Instant::now() + Duration::from_secs(10);
        while s.tmux(&["display-message", "-p", "-t", "one", "#{pane_height}"]).unwrap_or_default().trim() != "30" {
            assert!(Instant::now() < deadline, "the window never grew");
            std::thread::sleep(Duration::from_millis(20));
        }
        let size = |h: &str| h.trim().parse::<usize>().unwrap();
        let now = size(&s.tmux(&["display-message", "-p", "-t", "one", "#{history_size}"]).unwrap());
        assert!(now < first.anchor.size, "tmux pulled rows back: {now} < {}", first.anchor.size);
        let next = t.history("shell/9-9", Some(&first.anchor)).unwrap();
        assert!(next.refill, "{} lines, not a refill", next.lines.len());
        same(&plains(&next.lines), &tmux_history(&s, "one"), "refilled");
    }

    /// Where the pane sits on khor's client comes with every read: under
    /// a status line at the top, and right of a split.
    #[test]
    fn a_read_says_where_the_pane_sits() {
        if !have_tmux() {
            return;
        }
        let s = Server::start("histplace");
        s.tmux(&["-f", "/dev/null", "new-session", "-d", "-s", "one", "-x", "80", "-y", "20", "/bin/sh"]);
        s.until_present("one");
        s.tmux(&["set-option", "-t", "one", "@khor_row", "shell/9-9"]);
        let t = s.at();
        let bottom = t.history("shell/9-9", None).unwrap();
        assert_eq!((bottom.top, bottom.left), (0, 0), "status at the bottom: the pane starts at the top");
        s.tmux(&["set-option", "-t", "one", "status-position", "top"]);
        let top = t.history("shell/9-9", None).unwrap();
        assert_eq!((top.top, top.left), (1, 0), "under a status line at the top");
        s.tmux(&["split-window", "-h", "-t", "one", "/bin/sh"]);
        let right = t.history("shell/9-9", None).unwrap();
        assert!(right.left > 30 && right.top == 1, "{} {}", right.left, right.top);
    }

    /// **A program that takes the mouse** (item 36: Claude Code does) is
    /// said so with every read, and so is one that lets it go.
    #[test]
    fn a_read_says_when_the_program_holds_the_mouse() {
        if !have_tmux() {
            return;
        }
        let s = Server::start("histmouse");
        s.tmux(&["-f", "/dev/null", "new-session", "-d", "-s", "one", "-x", "60", "-y", "20", "/bin/sh"]);
        s.until_present("one");
        s.tmux(&["set-option", "-t", "one", "@khor_row", "shell/9-9"]);
        let t = s.at();
        assert!(!t.history("shell/9-9", None).unwrap().mouse);
        let held = |want: bool| {
            let deadline = Instant::now() + Duration::from_secs(10);
            loop {
                let h = t.history("shell/9-9", None).unwrap();
                if h.mouse == want {
                    return;
                }
                assert!(Instant::now() < deadline, "the mouse flag never became {want}");
                std::thread::sleep(Duration::from_millis(20));
            }
        };
        s.tmux(&["send-keys", "-t", "one", "printf '\\033[?1002h\\033[?1006h'; echo m1", "Enter"]);
        until_shown(&s, "one", "m1");
        held(true);
        s.tmux(&["send-keys", "-t", "one", "printf '\\033[?1002l\\033[?1006l'; echo m2", "Enter"]);
        until_shown(&s, "one", "m2");
        held(false);
    }

    /// A history bigger than a pipe's buffer comes back, capped, and the
    /// colours where the face's copy ends lead what follows.
    #[test]
    fn a_long_coloured_history_is_capped_and_its_colours_carried() {
        if !have_tmux() {
            return;
        }
        let s = Server::start("histlong");
        s.tmux(&["-f", "/dev/null", "new-session", "-d", "-s", "one", "-x", "60", "-y", "20", "/bin/sh"]);
        s.until_present("one");
        s.tmux(&["set-option", "-t", "one", "history-limit", "20000"]);
        s.tmux(&["set-option", "-t", "one", "@khor_row", "shell/9-9"]);
        s.tmux(&["send-keys", "-t", "one", "clear; seq 1 12000; printf '\\033[31m'; seq 12001 12050; printf '\\033[0m'", "Enter"]);
        until_shown(&s, "one", "12050");
        let t = s.at();
        let first = t.history("shell/9-9", None).unwrap();
        assert_eq!(first.lines.len(), HISTORY_CAP, "capped");
        // The anchor ends inside the red run; what follows starts red.
        s.tmux(&["send-keys", "-t", "one", "seq 12051 12100", "Enter"]);
        until_shown(&s, "one", "12100");
        let next = t.history("shell/9-9", Some(&first.anchor)).unwrap();
        assert!(!next.refill, "{next:?}");
        let n = numbers(&next.lines);
        let held_last = *numbers(&first.lines).last().unwrap();
        assert_eq!(n.first().copied(), Some(held_last + 1), "{n:?}");
        assert!((12001..12050).contains(&held_last), "the anchor is inside the red run: {held_last}");
        assert!(next.lead.ends_with("\x1b[31m"), "{:?}", next.lead);
    }

    /// An answer bigger than a frame keeps its newest lines, in the
    /// colours the dropped ones left, and says the copy starts again.
    #[test]
    fn an_answer_over_the_frame_cap_keeps_its_end_as_a_refill() {
        let line = format!("\x1b[32m{}", "x".repeat(1000));
        let read = HistoryRead {
            pane: "%1".into(),
            alternate: false,
            mouse: false,
            size: 1000,
            top: 0,
            left: 0,
            history: (0..1000).map(|_| line.clone()).collect(),
            rows: vec![],
            screen: vec!["$ ".into()],
        };
        let small = HistoryRead { history: read.history[..10].to_vec(), ..read.clone_for_test() };
        let whole = small.answer(Some(4));
        assert!(!whole.refill && whole.lines.len() == 5 && whole.lead == "\x1b[32m", "{whole:?}");
        let capped = read.answer(Some(4));
        assert!(capped.refill, "a gap behind the copy");
        let bytes: usize = capped.lines.iter().map(|l| l.len() + 1).sum();
        assert!(bytes <= HISTORY_BYTES && capped.lines.len() > 300, "{}", capped.lines.len());
        assert_eq!(capped.lead, "\x1b[32m", "the colours the dropped lines left");
    }

    impl HistoryRead {
        fn clone_for_test(&self) -> HistoryRead {
            HistoryRead {
                pane: self.pane.clone(),
                alternate: self.alternate,
                mouse: self.mouse,
                size: self.size,
                top: self.top,
                left: self.left,
                history: self.history.clone(),
                rows: self.rows.clone(),
                screen: self.screen.clone(),
            }
        }
    }

    #[test]
    fn plain_text_and_the_colours_in_force() {
        assert_eq!(plain("\x1b[31mred\x1b[0m \x1b]8;;http://x\x1b\\link\x1b]8;;\x07"), "red link");
        let lines = ["\x1b[1mbold\x1b[0m".to_owned(), "\x1b[32mgreen".to_owned(), "still".to_owned()];
        assert_eq!(escapes(&lines), "\x1b[0m\x1b[32m");
    }

    /// Joined lines and the rows they fill, told apart by ink; and the
    /// copy placed by count among lines that repeat.
    #[test]
    fn rows_place_the_copy_among_repeated_lines() {
        let v = |xs: &[&str]| xs.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        // "abcdef" wrapped at 4: two rows; a blank line is one.
        assert_eq!(row_counts(&v(&["abcdef", "", "x"]), &v(&["abcd", "ef", "", "x"])), Some(vec![2, 1, 1]));
        assert_eq!(row_counts(&v(&["abcdef"]), &v(&["abcd", "efg"])), None);
        let read = |history: &[&str], rows: &[&str], size: usize| HistoryRead {
            pane: "%1".into(),
            alternate: false,
            mouse: false,
            size,
            top: 0,
            left: 0,
            history: v(history),
            rows: v(rows),
            screen: vec![],
        };
        let anchor = |size, tail: &[&str]| HistoryAnchor { pane: "%1".into(), size, tail: v(tail) };
        // Five repeated lines, the copy held three: by text the newest pair
        // matches; by count it is the third line.
        let r = read(&["y", "y", "y", "y", "y"], &["y", "y", "y", "y", "y"], 5);
        assert_eq!(r.place(&anchor(3, &["y", "y"])), Place::At(2));
        assert_eq!(r.place(&anchor(5, &["y", "y"])), Place::At(4));
        // Wrapped: the last line is two rows, so two new rows are one line.
        let w = read(&["a", "b", "cc"], &["a", "b", "c", "c"], 4);
        assert_eq!(w.place(&anchor(2, &["a", "b"])), Place::At(1));
        // Rows taken away (the pane grew), the text not where the count
        // says, another pane: the whole history.
        assert_eq!(r.place(&anchor(6, &["y"])), Place::Gone);
        assert_eq!(w.place(&anchor(2, &["a", "z"])), Place::Gone);
        assert_eq!(r.place(&HistoryAnchor { pane: "%2".into(), ..anchor(3, &["y"]) }), Place::Gone);
        // A window that does not reach back to the copy's end: further.
        let part = read(&["y", "y"], &["y", "y"], 50);
        assert_eq!(part.place(&anchor(10, &["y"])), Place::Further);
    }

    /// The flagship, on a real server of khor's own: a shell sitting at
    /// a prompt is 空闲, a shell running something is 忙碌, and a pane
    /// tmux was handed a command is 忙碌 too.
    ///
    /// The process table is the real one here, because the pids are
    /// real — this is the one test that proves the parse, the join and
    /// the word all line up against the program as shipped.
    #[test]
    fn a_real_server_gives_the_two_words_it_can() {
        if !have_tmux() {
            return;
        }
        let s = Server::start("words");
        s.tmux(&["new-session", "-d", "-s", "idle"]);
        s.tmux(&["new-session", "-d", "-s", "cmd", "sleep 300"]);
        s.tmux(&["new-session", "-d", "-s", "working"]);
        s.until_present("working");
        s.tmux(&["send-keys", "-t", "working", "sleep 300", "Enter"]);
        s.until_running("working", "sleep");
        // …and the idle one has to have finished starting up, or "a
        // prompt is 空闲" is a claim about how quiet the machine is.
        s.until_at_a_prompt("idle");

        let procs = Procs::snapshot();
        let listing = s.at().sweep(&procs);
        let word = |name: &str| {
            listing
                .rows
                .iter()
                .find(|m| m.found.sighting.title == name)
                .map(|m| m.found.sighting.word)
        };
        assert_eq!(listing.unmapped, 0);
        assert_eq!(word("idle"), Some(State::Idle), "a prompt is 空闲");
        assert_eq!(
            word("working"),
            Some(State::Busy),
            "a command in a shell is 忙碌"
        );
        assert_eq!(
            word("cmd"),
            Some(State::Busy),
            "a pane tmux started on a command is that command, and it is running"
        );
        for m in &listing.rows {
            assert_eq!(m.found.category, khor_core::category::SHELL);
            assert_eq!(m.found.kind, khor_core::kind::SHELL);
        }
    }

    /// A session holds its panes **and everything under them** — which
    /// is the only reason the duplicate rule can work, because an agent
    /// is a grandchild of the pane, not the pane.
    #[test]
    fn a_session_holds_what_is_running_inside_it() {
        if !have_tmux() {
            return;
        }
        let s = Server::start("holds");
        s.tmux(&["new-session", "-d", "-s", "deep"]);
        s.until_present("deep");
        s.tmux(&["send-keys", "-t", "deep", "sleep 300", "Enter"]);
        s.until_running("deep", "sleep");

        let procs = Procs::snapshot();
        let listing = s.at().sweep(&procs);
        assert_eq!(listing.rows.len(), 1);
        let held = &listing.rows[0].holds;
        let pane_pid: u32 = s
            .tmux(&["list-panes", "-t", "deep", "-F", "#{pane_pid}"])
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        assert!(held.contains(&pane_pid), "the pane itself");
        assert!(
            held.iter()
                .any(|p| procs.get(*p).is_some_and(|q| q.name == "sleep")),
            "and the sleep the shell started, which is a child of the pane, not the pane"
        );
    }

    /// Two panes, one session, one row — and the busy one decides.
    #[test]
    fn a_session_with_two_panes_is_one_row_wearing_the_busier_word() {
        if !have_tmux() {
            return;
        }
        let s = Server::start("panes");
        s.tmux(&["new-session", "-d", "-s", "split"]);
        s.until_present("split");
        s.tmux(&["split-window", "-t", "split"]);
        s.until(|seen| seen.lines().filter(|l| l.starts_with("split|")).count() == 2);
        s.tmux(&["send-keys", "-t", "split.1", "sleep 300", "Enter"]);
        s.until(|seen| seen.lines().any(|l| l == "split|sleep"));

        let listing = s.at().sweep(&Procs::snapshot());
        assert_eq!(listing.rows.len(), 1, "panes are not rows");
        assert_eq!(
            listing.rows[0].found.sighting.word,
            State::Busy,
            "one pane at a prompt must not hide the one doing work"
        );
    }

    /// The word rule, against a table with no real processes in it, so
    /// that both branches are reachable on any machine.
    #[test]
    fn the_word_reads_the_foreground_and_refuses_when_there_is_none() {
        let pane = |pid: u32, started: bool, current: &str| Pane {
            session_id: "$1".into(),
            session_name: "s".into(),
            created_secs: 100,
            activity_secs: 100,
            pid,
            started_with_command: started,
            current_command: current.into(),
            current_path: "/somewhere".into(),
            pane_id: "%1".into(),
        };
        let procs = Procs::of([
            // Both spellings of the same shell. `zsh` is what sysinfo
            // actually reports here (measured, see `same_program`);
            // `-zsh` is the convention `ps` uses, and the two must not
            // read differently or a shell pane would sit at 忙碌 for the
            // life of the machine.
            (10, proc_named("zsh", Some(1))),
            (12, proc_named("-zsh", Some(1))),
            (11, proc_named("sleep", Some(1))),
        ]);
        assert_eq!(word_of(&pane(10, false, "zsh"), &procs), Some(State::Idle));
        assert_eq!(
            word_of(&pane(12, false, "zsh"), &procs),
            Some(State::Idle),
            "the login shell's dash is a convention, not a different program"
        );
        assert_eq!(
            word_of(&pane(10, false, "cargo"), &procs),
            Some(State::Busy)
        );
        assert_eq!(
            word_of(&pane(11, true, "sleep"), &procs),
            Some(State::Busy),
            "a pane that IS the command must not read as its own prompt"
        );
        assert_eq!(
            word_of(&pane(99, false, "zsh"), &procs),
            None,
            "no process, no word — never a guessed one"
        );
    }

    /// The four words tmux must never spell. A property assertion
    /// ("the word is one of the six") is blind to this; enumerating the
    /// forbidden ones is not.
    #[test]
    fn tmux_never_spells_a_word_it_cannot_know() {
        if !have_tmux() {
            return;
        }
        let s = Server::start("narrow");
        s.tmux(&["new-session", "-d", "-s", "one"]);
        s.tmux(&["new-session", "-d", "-s", "two", "sleep 300"]);
        s.until(|seen| {
            seen.lines().any(|l| l.starts_with("one|")) && seen.lines().any(|l| l == "two|sleep")
        });
        let listing = s.at().sweep(&Procs::snapshot());
        assert_eq!(listing.rows.len(), 2, "the control: it did find them");
        for m in &listing.rows {
            let w = m.found.sighting.word;
            assert!(
                matches!(w, State::Busy | State::Idle),
                "tmux has no turn, no approval, no error and no file that \
                 outlives a session — {w:?} would have to have been invented"
            );
        }
    }

    /// A session name may hold the separator, which is the whole reason
    /// it is the last field.
    #[test]
    fn the_name_takes_the_rest_of_the_line() {
        assert!(
            Pane::parse("$1\x1f100\x1f100\x1f10\x1f0\x1fzsh\x1f/tmp").is_none(),
            "a field short"
        );
        assert!(Pane::parse("$1\x1f100\x1f100\x1fnotapid\x1f0\x1fzsh\x1f/tmp\x1f%1\x1falpha").is_none());
        assert_eq!(
            Pane::parse("$1\x1f100\x1f100\x1f10\x1f0\x1fzsh\x1f/tmp\x1f%1\x1fhas|pipes|in|it")
                .unwrap()
                .session_name,
            "has|pipes|in|it"
        );
    }

    /// A tmux khor cannot read is counted, and the session beside it
    /// still becomes a row — the "prove the finder was working when it
    /// declined" rule, and the only way to see the count at all.
    ///
    /// Driven through a stand-in binary rather than a real tmux: the
    /// case being tested is a **future** tmux whose output khor does not
    /// understand, and no installed tmux can be asked to be one. What is
    /// under test is khor's side of that day, which is exactly the code
    /// this reaches.
    #[test]
    fn output_this_version_cannot_read_is_counted_and_stops_nothing() {
        let fake = fake_tmux(
            "fmt",
            "$1\x1f100\x1f100\x1f10\x1f0\x1fzsh\x1f/tmp\x1f%1\x1freadable\n\
             $2|from-a-later-tmux\n\
             $3\x1f200\x1f200\x1f11\x1f0\x1fzsh\x1f/tmp\x1f%1\x1falso-readable\n",
        );
        let procs = Procs::of([
            (10, proc_named("-zsh", Some(1))),
            (11, proc_named("-zsh", Some(1))),
        ]);
        let listing = Tmux::at_binary(&fake).sweep(&procs);
        let mut titles: Vec<&str> = listing
            .rows
            .iter()
            .map(|m| m.found.sighting.title.as_str())
            .collect();
        titles.sort_unstable();
        assert_eq!(
            titles,
            vec!["also-readable", "readable"],
            "the ones it understood are still rows"
        );
        assert_eq!(
            listing.unmapped, 1,
            "and the one it did not is counted, not dropped"
        );
    }

    /// A session whose panes have all left the process table: tmux still
    /// names it, khor cannot say a word about it, so it is counted
    /// rather than shown with a guessed one.
    #[test]
    fn a_session_with_nothing_running_in_it_is_counted_not_guessed_at() {
        let fake = fake_tmux(
            "gone",
            "$1\x1f100\x1f100\x1f424242\x1f0\x1fzsh\x1f/tmp\x1f%1\x1fghost\n",
        );
        let listing = Tmux::at_binary(&fake).sweep(&Procs::default());
        assert_eq!(listing.rows.len(), 0);
        assert_eq!(listing.unmapped, 1);

        // The control: the same line against a table that vouches for
        // the pane is an ordinary row, so the absence above came from
        // the missing process and not from the line being unreadable.
        let procs = Procs::of([(424242, proc_named("-zsh", Some(1)))]);
        let listing = Tmux::at_binary(&fake).sweep(&procs);
        assert_eq!(listing.rows.len(), 1);
        assert_eq!(listing.unmapped, 0);
    }

    /// The id is the session's, not the name's, and survives a rename.
    #[test]
    fn the_id_outlives_a_rename_and_a_server_that_starts_counting_again() {
        assert_eq!(id_of("$120", 1_786_245_338).0, "shell/1786245338-120");
        assert_eq!(
            id_of("$120", 1_786_245_338),
            id_of("$120", 1_786_245_338),
            "the same session is the same id however it is named now"
        );
        assert_ne!(
            id_of("$0", 1_786_245_338),
            id_of("$0", 1_786_300_000),
            "a fresh server counting from zero again is not the old session"
        );
    }

    /// The whole duplicate rule rests on parentage being in the
    /// snapshot at all. That sysinfo fills it in under
    /// `ProcessRefreshKind::nothing()` is an assumption, and an
    /// assumption load-bearing enough to be an assertion rather than a
    /// comment: without it `holds` would be only the pane pids, every
    /// agent would be missed, and every tmux session would show twice.
    #[test]
    fn the_process_table_knows_who_started_what() {
        let procs = Procs::snapshot();
        let me = std::process::id();
        assert!(
            procs.get(me).and_then(|p| p.ppid).is_some(),
            "this test process has a parent and the snapshot must know it"
        );
        assert!(
            procs.subtree(&[me]).contains(&me),
            "a subtree includes its own root, which is what lets a bare pane match"
        );
    }
}
