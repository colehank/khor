// **What a terminal says back when a program asks it something** — and how
// to tell one from a key (2026-10-03, the owner's phone: 「11;rgb:0000/0000/
// 00001;2c1$r2 q…」 at a zsh prompt, `command not found: 2c1`).
//
// A program asks the terminal questions in its output: the background
// colour (OSC 11 `?`), what kind of terminal it is (DA1 `ESC [ c`, DA2), the
// cursor style (DECRQSS `ESC P $ q SP q`), whether a mode is set (DECRQM
// `ESC [ ? n $ p`), where the cursor is (DSR 6 / CPR). xterm parses those
// and answers through `onData`, the same road keys take. Answering is
// right while the program is waiting. It is wrong when the question is
// history: replaying a host's ring writes yesterday's `yazi` into xterm,
// xterm answers it, and the answer lands in a shell that asked nothing.
//
// The field's own answer (xterm.js maintainer, discussions #4869): replay
// into the terminal *before* wiring it to the pty, so replayed questions
// answer into nothing. khor's pane is wired before its first write (keys
// typed while the screen loads must still go), so it does the equivalent
// in one place: while replay bytes are being parsed, `onData` drops what
// has the shape of an answer (`isReply`), and lets every key through.
// tmux never meets this: its reattach redraws from its own grid, it does
// not replay the bytes that asked.
//
// The shapes are xterm's own replies (src/common/InputHandler.ts in
// xterm.js 6: `sendDeviceAttributesPrimary`/`Secondary`,
// `deviceStatus`, `requestMode`, the OSC colour report, DECRQSS in
// `DcsHandler`), and the forms in xterm's ctlseqs: none of them is a key
// a keyboard sends, with one overlap noted below.

const ESC = "\x1b";

/** One reply, anchored at the start of what is left. */
const REPLY = new RegExp(
  "^(?:" +
    [
      // DA1 / DA2 / DA3 answers: CSI ? … c, CSI > … c, (DA3 is DCS below).
      `${ESC}\\[[?>][0-9;]*c`,
      // DECRPM: CSI ? Ps ; Pm $ y, CSI Ps ; Pm $ y.
      `${ESC}\\[\\??[0-9;]*\\$y`,
      // DSR status (CSI 0 n), and the private forms (CSI ? … n).
      `${ESC}\\[\\??[0-9;]*n`,
      // CPR / DECXCPR: CSI row ; col R, CSI ? row ; col ; page R. ⚠️ ⌃F3
      // (CSI 1 ; 5 R) has this shape; only ever dropped during a replay.
      `${ESC}\\[\\??[0-9]+;[0-9]+(?:;[0-9]+)?R`,
      // Window and text-area reports (CSI … t), where a program may ask.
      `${ESC}\\[[0-9;]+t`,
      // OSC replies: ESC ] … BEL or ST (colours, clipboard reads).
      `${ESC}\\][^\\x07\\x1b]*(?:\\x07|${ESC}\\\\)`,
      // DCS replies: ESC P … ST (DECRQSS, XTVERSION, DA3, XTGETTCAP).
      `${ESC}P[^\\x1b]*${ESC}\\\\`,
    ].join("|") +
    ")",
);

/** Whether `data`, as xterm hands it to `onData`, is nothing but one or
    more replies to terminal queries — never true of anything a key, a
    paste or a mouse report produces (bar the ⌃F3 noted above). */
export function isReply(data: string): boolean {
  if (!data.startsWith(ESC)) return false;
  let rest = data;
  while (rest.length > 0) {
    const m = REPLY.exec(rest);
    if (!m) return false;
    rest = rest.slice(m[0].length);
  }
  return true;
}

/** A focus report (`ESC [ I`, `ESC [ O`, DECSET 1004): an event, not an
    answer, but just as stale when a replayed program turned the mode on
    and the pane takes focus while the replay is parsed. */
export function isFocusReport(data: string): boolean {
  return data === `${ESC}[I` || data === `${ESC}[O`;
}
