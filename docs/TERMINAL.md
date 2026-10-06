# The terminal

How a session's terminal works end to end, as built through the terminal roadmap (`audit/TERM-ROADMAP.md`, phases 1–5) and the terminal audit's fixes (`audit/term.md`). Read this before changing anything in `crates/node/src/{host,modes,pictures,commands,answers}.rs`, `apps/gui/core/src/term.rs` or `apps/gui/src/views/TerminalPane.tsx`.

The shell integration scripts and the command ledger have their own page: [SHELL-INTEGRATION.md](SHELL-INTEGRATION.md).

## The path of a byte

```
program ── PTY ── host (one process per session)
                   ├─ ring: 1 MiB of text, the replay        crates/node/src/host.rs  (Ring)
                   ├─ pictures: 4 MiB, beside the ring       crates/node/src/pictures.rs
                   ├─ modes: what a trimmed head had set     crates/node/src/modes.rs
                   ├─ command ledger                         crates/node/src/commands.rs
                   └─ answer ledger                          crates/node/src/answers.rs
                        │  loopback TCP, or a far node's wire
                   app / bridge: one attachment per session  apps/gui/core/src/term.rs  (feed, 2 MiB)
                        │  term_wait (pushed) / term_stream
                   face: xterm.js 6 in the pane              apps/gui/src/views/TerminalPane.tsx
```

**Host.** The host owns the PTY and outlives every face. It keeps three things for a face that attaches later:
- the last `RING` bytes of output;
- the DEC modes the trimmed head had set, replayed first as a preamble (`Modes`);
- the pictures drawn among those bytes, spliced back at their stream positions.

A trim never stops inside an escape sequence; it runs on to the sequence's end. So a replay never opens on half a picture or half an OSC.

**Feed.** The app keeps a per-attachment feed (`FEED_CAP`, 2 MiB) that a face pours on arrival and then tails.
- It has the same trim rule and the same `Modes` head.
- A face that falls off its head is handed the screen as it stands, not a splice (T-12).
- Pictures stay inline in the feed, because the feed's cursor counts raw bytes.

**Face.** The pane writes what arrives:
- **Whole frames.** Output is held until the stream has been quiet for 8 ms. While a redraw is open (`?25l` not yet shown again, or `?2026h` not yet closed) it is held until the redraw closes, never past 60 ms (`framesWhole`).
- **Replays.** A replay (attach, a reset beat, a re-join, a catch-up after a long gap) is written with xterm's answers muted (below).

## Answers to a program's questions

A program asks its terminal things in its output (DA1/DA2, DSR, OSC 10/11/12/4 colours, DECRQSS, DECRQM, `CSI ? u`), and xterm answers through `onData`.

**Mute during replay.** While replay bytes are being parsed, the face drops whatever has the shape of an answer. It never drops a key. The shapes are in `lib/term-replies.ts`, and `scripts/check-replies.mjs` tells answers from keys.

**Answer ledger.** The host lets an answer through only while a matching question is pending and younger than 2 s. The first face to answer wins, so two faces never answer twice.

**Fallback.** If no face answers within 150 ms, the host answers the fixed questions itself: DA1, DA2, DSR 5, and colours from the last face answer it saw. That covers a hidden or suspended phone. It never answers the kitty flags question: those flags are the faces'.

## The semantic layer

**Marks.** The shell's OSC 133/633/7 marks are read twice:
- by the host's ledger, into `Session::doing`, `commands.json` and `history.jsonl` (see SHELL-INTEGRATION.md);
- by the face (`lib/term-marks.ts`), into xterm markers that ride the scrollback.

**Blocks** are drawn from those markers (`drawBlocks`): a hairline where each prompt starts, and an exit stripe once the command has ended. A right-click, or a long press on a prompt, opens the block's actions: copy output, copy command, run again, send to an agent, and 结构化查看 when the output is JSON or a table (`lib/term-peek.ts`, `views/TermPeek.tsx`). ⌘↑/⌘↓ and the key sheet's chevrons jump between prompts.

**Command history** (`Request::History`, `views/TermHistory.tsx`) is searched on the machine that ran the commands; only the hits cross the wire. Open it with ⌘⇧; on the Mac, or 历史命令 in the key sheet.

**Paths and URLs** (`lib/term-links.ts`). A path that exists on the session's machine becomes a link: an absolute path, `~/x`, a relative path resolved against the block's cwd, `file:12:3`, or a Python traceback line. URLs too. ⌘-click under a mouse, a tap under a finger. They open khor's own file view, the files page or a browsing tab that goes out through that machine.

**The glyph atlas** (`lib/term-atlas.ts`, item 24). xterm's WebGL addon shares one atlas among every terminal with the same font settings, and 0.19.0 has defects there (xterm.js #6014; a clear on one terminal garbles the others). The face never clears it from one pane alone. When the page's fonts finish loading, every atlas is cleared first and every terminal redrawn after. A clear also asks every sharer to rebuild, which is upstream fix #6055's effect, put on the class at run time. A lost context drops the pane to the DOM renderer, and WebGL comes back on the next showing. A second loss within a minute keeps the pane on the DOM renderer and logs it. Gate: `npm run atlas`.

**Pictures.** `@xterm/addon-image` draws sixel and iTerm2 inline images. The face's store is 64 MB on a Mac and 32 MB on a phone. A tap or ⌘-click opens a picture whole.

**Progress and notices.** `OSC 9;4` progress becomes `Doing.progress`, a line on the row and the tab. `OSC 9` and `OSC 777` notifications follow the finished-command path (`lib/finished.ts`).

**Bell.** The pane's edge flashes. On a phone there is also a light haptic (`bell_haptic`).

**Kitty keyboard protocol** (`lib/term-kitty.ts`, one module so it can be dropped when xterm 6.1 ships). A flag stack is kept per screen, pushed, popped, set and queried as the spec says. Keys are encoded by the five enhancement flags. The phone key row (Esc, Tab, ⇧Tab, symbols, Ctrl/Alt latches) asks the same module. The host's `Modes` keeps the same stacks for a trimmed replay.

**Through tmux** (item 21). A tmux row's terminal is khor's own grouped tmux client, and tmux owns that screen, so the face's marks stop at the alternate screen. What still works, and how:
- **The ledger reads every visible pane.** Inside tmux, khor's scripts wrap each mark in tmux's passthrough, led by `OSC 633 ; P ; KhorPane=%n`. tmux forwards a pane's passthrough while the pane is visible (`allow-passthrough on`, which each script sets on its own pane), so the ledger keeps a context per pane (`commands::Ledger::tmux`). khor's grouped session sets `set-titles-string` to `#{pane_id} #{pane_title}`; the title names the pane in front, and the row reads that pane. History lines carry `pane`. A khor shell in which the person typed `tmux` reads the same wrapped marks: from the first `KhorPane` mark its own context is kept apart from each pane's, so its `tmux` runs until tmux exits and each pane's commands reach the history with their pane.
- **Lost out of sight.** A pane in a window not on screen sends nothing. A command that ends there keeps its start and text; its end time is the moment the pane's next command starts, and its exit status is unknown. A command that both starts and ends out of sight is not recorded.
- **Prompt jumps are tmux's.** The scripts also give tmux the unwrapped `133;A` (inside the prompt) and `133;C`, which tmux 3.4 and later keep for copy mode's `previous-prompt` / `next-prompt`. ⌘↑/⌘↓ and the key sheet's chevrons on a tmux row ask the row's machine (`Act` "prompt-up" / "prompt-down") to run those on khor's grouped session, found by its `@khor_row` option. ⌘↓ past the last prompt leaves copy mode.
- **Command history** on a tmux row takes 这个目录 and "at a prompt" from the row's reading of the front pane.
- **Which shells mark.** A pane opened from khor's client starts its shell through khor's `default-command` (set on khor's grouped session only, and only when the person has no `default-command` of their own; zsh and fish). For every other shell in tmux, one line in `~/.zshrc`, shown with a copy button in 设置 → 终端:
  `[[ -n $TMUX && -r ~/.khor/shell/khor.zsh ]] && source ~/.khor/shell/khor.zsh`
- **Shift+Enter and other modified keys** (item 39). khor's client is attached with `-T extkeys` (tmux 3.2 and later), so tmux asks the face for xterm's modifyOtherKeys (`CSI > 4 ; 2 m`) without any `terminal-features` line; the face sends Shift+Enter as `CSI 27 ; 2 ; 13 ~` (`lib/term-kitty.ts`), and tmux hands it to a program that asked for extended keys (Claude Code does). The one line still the person's: `set -s extended-keys on`, a server option khor does not write. Without it tmux sends every pane a bare CR, under iTerm2 too.
- **Not here:** blocks drawn over a tmux pane. They need tmux's control mode (`-CC`, iTerm2's way): one xterm per pane, each with its own scrollback. Sized in `audit/term-tmux-marks.md`.

**Prediction** (`lib/term-predict.ts`). This is mosh's model, as VS Code's TypeAheadAddon carries it onto xterm.js.
- A printable key at the end of the line is drawn underlined at once.
- Guesses are taken back before output is written, then confirmed against the screen after it.
- It is on only while this session's key-to-echo time is over 100 ms, and off again below 70 ms.
- In a run of typing nothing is drawn until the first echo has come back, so a password prompt never shows.

## Input, focus and the keyboard

- **Focus follows the page.** The terminal's field has the focus whenever its page is open (`lib/soft-keyboard.ts` `holdFocus`). Under a finger it is held with `inputmode="none"`, so a hardware keyboard types and no soft keyboard rises by itself.
- **Raising the soft keyboard.** Three modes (`lib/term-tap.ts`, 外观 → 终端 → 键盘唤醒):
  - 自动 (default): a tap on the line being typed raises it, and a tap elsewhere lowers it;
  - 点击屏幕: any tap raises it;
  - 点击按钮: only the deck's keyboard switch raises it.
  - A fast downward swipe lowers it in every mode.
- **The deck API**, for the session screen's other controls: `terminalKeyboard.raise() / lower() / up() / mode()` (`lib/soft-keyboard.ts`). The ux lane's deck uses it: its downward drag calls `lower()`, and its switch is drawn only when `mode()` is 点击按钮.
- **Key row** (`views/TermKeys.tsx`, `lib/term-keys.ts`):
  - a person-chosen row of caps;
  - the cursor joystick, second from the right (`views/CursorPad.tsx`);
  - … last: a sheet with every key, the prompt jumps and 历史命令, plus the row editor.
  - A key a hardware keyboard repeats (⌫, Del, the moves, Tab and ⇧Tab, the characters) repeats while held, on the cursor stick's numbers (`lib/key-repeat`): 400 ms, then 6 a second, 12 after a second, a haptic tick each time under 「摇杆震动」. A tap still sends at the click, so a finger scrolling the row sends nothing. Esc, the latches, paste and the function keys never repeat; latches hold for the whole repeat, as the stick's do.
- **Phone composition.** The text being composed shows in a fixed strip at the pane's foot, not at the cursor.
- **iOS's delete key repeats** (item 35, `lib/term-soft-delete.ts`). Under the soft keyboard the delete key's keydown is left to the browser, xterm's hidden field always holds a run of no-break spaces for it to delete, and each edit the field reports goes out as the key: DEL (or the kitty spelling), ⌃W for a word, ⌃U for a line. xterm used to answer the keydown with `preventDefault`, and on an empty field iOS has nothing to repeat, so a held delete took one character. The placeholder never reaches the pty, the clipboard or a selection; a hardware keyboard and a composition (pinyin) keep xterm's own road. Gate: `npm run softdel`, which replays the simulator's recorded events.
- **Pastes.** A multi-line paste asks first. An `OSC 52` clipboard read is asked about once per terminal.
- **Gestures.** A two-finger tap is Esc. A pinch sets the terminal's type size. A sideways swipe steps to the next tab, stopping at the ends, as the deck pill's swipe does. The left 24 pt stay the page's edge swipe.
- **Mac keys.** ⌥←/→ send `\eb`/`\ef`. ⌘←/→/⌫ send `^A`/`^E`/`^U`. ⌘K clears. ⌘R rejoins.

## Scrolling and size

- **Native scroll proxy.** A transparent element scrolls natively (`scrollsNatively`). Its height is the buffer's lines × row height, and its position drives xterm's viewport. The alternate screen and mouse-tracking programs keep wheel reports.
- **First fit.** It waits for a settled box (`settled`, capped at 300 ms). The pane's first second is traced (`__termTrace`, and `face_note` to the app log).
- **What a device saw** (item 33). On attach the pane notes the modes the replay left (buffer, mouse tracking), the grid and xterm's measured cell; the first touch after it notes which scroller took it (native, the tmux history view, or the wheel to the program, with how many wheels became mouse reports) or what a tap or press did (`lib/term-first-touch.ts`). `face_note` keeps these lines, with the first-second summary, in `.khor/face-notes.log` (64 KiB at most); `khor doctor`'s 终端 row, and the 自检 page on a phone, say the newest entry.
- **A new shell starts at this device's terminal size.** `open_on` takes `cols`/`rows`; the face sends them from `lib/term-size-hint.ts`. The first prompt is written for the width it will be shown at, so zsh's `%` mark (PROMPT_SP) does not wrap into its own line.
- **Scrollback** is a setting. The host's ring holds about 13 000–26 000 lines at 42–80 bytes a line.

## Settings and where they live

All per device, in the face's `localStorage`. The real store after the redesign's sign-off is the khor home; see `lib/term-font.ts`.

| Setting | 外观 → 终端 row | Key | Default |
|---|---|---|---|
| Terminal font | 字体 | `khor.term-font` | the bundled mono |
| Terminal type size | 终端字号 (slider) | `khor.type-size` (`term`) | 14 |
| Cursor shape / blink | 光标样式, 光标闪烁 | `khor.term-cursor` | block, no blink (a program may change the shape, never the blink) |
| Scrollback | 历史行数 | `khor.term-history` | 5000 on a phone, 10 000 on a Mac |
| Notify when a command finishes | 命令完成时通知 | `khor.finished-after` | after 30 s |
| Bell | 响铃 | `khor.term-bell` | on |
| Predict typing | 慢网预测回显 | `khor.term-predict` | on |
| Keyboard on tap (phone only) | 键盘唤醒 | `khor.term-wake` | 自动 |
| Key row (phone only) | 终端键行… | `khor.term-key-row` | Esc, Ctrl, Tab, ⇧Tab, 粘贴, / - \| : |
| New shell size | — (remembered) | `khor.term-size-hint` | the last fitted size, else estimated |

The node's own setting is `khor config set session-form chat|term`, which picks the form a new agent session opens in.

## What a Windows (ConPTY) lane must keep

SHELL-INTEGRATION.md's ConPTY section covers the marks: create the pseudoconsole in passthrough mode, use OSC rather than DCS, keep `633;E`, and never rely on a re-sent screen. Beyond it, everything above assumes these facts about the PTY's output:

1. **The bytes are the program's own, in order.**
   - The ring, the frame coalescer (`?2026`, `?25l/h`) and the kitty and mode trackers all read the program's sequences.
   - A ConPTY that re-renders from its grid would drop the program's mode sets: kitty flags, bracketed paste, mouse modes.
   - It would also break the picture store (sixel and OSC 1337 must pass through).
   - Passthrough is required, not optional.
2. **Questions reach the faces.**
   - ConPTY answers some queries itself (DA1, DSR 6/CPR) when not in passthrough.
   - The answer ledger then sees a question no face should answer, and the answers arrive twice.
   - In passthrough, keep the host's fallback answers as they are.
3. **Keys go in as VT.**
   - The faces send VT key sequences: xterm's, the key row's, kitty's when asked.
   - ConPTY's input side must accept them as written.
   - Win32 input mode (`?9001`), if a program asks for it, needs its own encoder in the pane's key path, as kitty's has.
   - Do not translate kitty sequences into Win32 input records.
4. **Size before the first prompt.** Start the pseudoconsole at the size `open_on` was given (`term-size-hint`); PowerShell's prompt is written at that width as zsh's is.
5. **Integration scripts.** `khor.ps1` and `khor.cmd` are written but not wired. Wire them as SHELL-INTEGRATION.md says; cmd has no hook before a command runs, so its blocks have no C mark.
6. **Paths.** `lib/term-links.ts` recognises POSIX paths only. `C:\…` and `\\server\…` need their own candidate pattern, and a listing that answers on Windows.
7. **Environment** (T-26). Set `TERM_PROGRAM=khor`, `TERM_PROGRAM_VERSION` and `COLORTERM=truecolor`. Strip the launching terminal's `WT_SESSION`, `TERM_SESSION_ID` and their kind, as the Unix host strips `ITERM_*` and `TMUX*`.

## Tests

| Where | What |
|---|---|
| `cargo test -p khor-node` | Modes (incl. kitty stacks), pictures, commands (ledger, history search), answers (ledger incl. kitty), proto round-trips |
| `crates/node/tests/command_ledger.rs`, `history_wire.rs`, `host_replay.rs` | Recorded shell streams, history over a real wire, replay |
| `cargo test -p khor-gui-core` | The feed's trim and reset rules; `command_tables` keeps the bridge and the Tauri command list in step |
| `npm test` (apps/gui) | check-replies (answers vs keys), check-peek (what is data) and the rest |
