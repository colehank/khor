// A live terminal for a session khor hosts (docs/handoff 终端画屏) —
// drawn by xterm.js, fed bytes.
//
// # The pivot (2026-09-03), and what each side keeps
//
// This pane used to paint a cell grid: gui-core's vt100 parser consumed
// the pty bytes and answered `term_poll` with `TermScreen` runs, and
// ~700 lines here re-implemented what a terminal renderer does — cell
// metrics, a measured CJK width ratio, an IME textarea hack, a key→byte
// table, a three-way wheel judgment. Every one of those was a scar with
// a comment on it, and the user's verdict on the result was a screenshot
// beside iTerm2 (「这是你渲染的终端…这是iterm渲染的」) and 「我们是
// 全功能的终端」. Re-implementing a terminal is the wrong place to
// spend scars: xterm.js is the renderer VS Code ships, and it does all
// of it — grid metrics, wide chars (unicode), IME composition, every
// key/modifier encoding, mouse protocols, alt-screen wheel, selection.
//
// So the split is now:
//   - **node stays the authority.** The pty host is untouched; gui-core
//     keeps its vt100 parser (resize, the authoritative screen, and the
//     attach snapshot come from it) and *tees* the same bytes into a
//     feed (`term_attach` hands over everything so far — which began
//     with the host's ring, so history comes along; `term_stream` hands
//     over what arrived since). Remote sessions change nothing: the
//     host socket was already tunneled, the feed fills the same way.
//   - **this pane is a byte pipe.** Decode base64, `term.write()`. Keys
//     travel the other way whole: xterm's `onData` already speaks every
//     sequence a terminal sends (the `keyBytes` table this replaces had
//     them one at a time — shift+Tab arrived the day before the table
//     died), and `termKey` carries the bytes as before.
//
// What this file still owns, because xterm cannot: the open/attach
// lifecycle and its three faces (`why` — never opened; `gone` — the
// host ended; `dropWhy` — a drop that failed), the poll pacing (nobody
// looking, nobody paying), the tauri file-drop (paths land as a
// shell-quoted paste via `term_drop`, cross-machine aware), and the
// theme — xterm takes concrete colours, so the app's tokens are
// resolved to values here and re-resolved when the theme flips.
//
// And the four things a terminal *emulator* leaves to the window
// around it, each set to what iTerm2 does out of the box (the app's
// rule: defaults over settings, aligned to iTerm):
//   - **links** open on ⌘-click (`WebLinksAddon`), through the same
//     `open_link` the conversation pane uses — the whitelist of schemes
//     is khor's, not the addon's;
//   - **a selection is a copy** (`onSelectionChange` → clipboard), so
//     there is no ⌘C to remember and nothing to lose by clicking away;
//     where the program has the mouse (a tmux khor attached), the
//     selection is ⌥-drag (⇧-drag off a Mac) — `macOptionClickForcesSelection`;
//   - **⌘F is the terminal's own search** (`SearchAddon` + the bar in
//     this file), not the page's — a page find would search the DOM of
//     a webgl canvas and find nothing;
//   - **a bell is a flash** on the frame. ⚠️ 墓碑 (2026-10-02, the owner:
//     「不要显示水印」): the title a program sets (OSC 0/2) was shown in
//     the top right corner, muted on a wash — 「zgh@Mac-mini:/Volumes/…」
//     over the output. It is not drawn any more, and not kept.
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { Terminal, type IDisposable } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { UnicodeGraphemesAddon } from "@xterm/addon-unicode-graphemes";
import { SearchAddon } from "@xterm/addon-search";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { WebglAddon } from "@xterm/addon-webgl";
import "@xterm/xterm/css/xterm.css";

import { bellHaptic, faceNote, inApp, termAttach, termDrop, termKey, termLeave, termOpen, termResize, termStream, termWait } from "@/api";
import { GlassCircle, GlassItem } from "@/components/Glass";
import { IconArrow, IconChevronDown, IconChevronUp, IconClose, IconReload } from "@/components/icons";
import { RegionState, Waiting } from "@/components/RegionState";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { HIDDEN_MS, useHidden } from "@/hooks/use-hidden";
import { useKeysTo } from "@/hooks/use-keys-to";
import { useCoarse } from "@/hooks/use-coarse";
import { BarSeat } from "@/components/SessionBar";
import { onTermSize } from "@/lib/type-size";
import { onTermFont } from "@/lib/term-font";
import { onTermCursor, termCursor, type CursorShape } from "@/lib/term-cursor";
import { onTermHistory, termHistory } from "@/lib/term-history";
import { inInput, trackMarks, type Block, type Marks } from "@/lib/term-marks";
import { askNewSession, askOpenPath, askOpenUrl, askStepTab, handOff } from "@/lib/composer-handoff";
import { setTypeSize, TERM_MAX, TERM_MIN, typeSize } from "@/lib/type-size";
import { termBell } from "@/lib/term-bell";
import { pathLinks, pathsOn, urlAt } from "@/lib/term-links";
import { programOf } from "@/components/DoingLine";
import { termWake, useTermWake } from "@/lib/term-tap";
import { holdFocus, keyboardUp, lowerKeyboard, raiseKeyboard, terminalShown } from "@/lib/soft-keyboard";
import { KeyboardSwitch, TermKeys } from "@/views/TermKeys";
import { typed, type Mods } from "@/lib/term-keys";
import { isFocusReport, isReply } from "@/lib/term-replies";

/** The 16 ANSI colours, the one palette a terminal owns regardless of
    the app's theme. 0–7 normal, 8–15 bright. Handed to xterm whole —
    the cube (16–231) and the grey ramp it computes itself. */
const ANSI16 = [
  "#000000", "#cc0000", "#4e9a06", "#c4a000", "#3465a4", "#75507b", "#06989a", "#d3d7cf",
  "#555753", "#ef2929", "#8ae234", "#fce94f", "#729fcf", "#ad7fa8", "#34e2e2", "#eeeeec",
];

/** How often a visible terminal asks for new bytes. Fast: this is the
    one surface where a person is watching characters appear — and a
    keystroke asks again immediately (`pollNow`), so echo latency is
    the round trip, not this number. */
const LIVE_MS = 50;
/** How long one pushed wait is held open with nothing new (`termWait`).
    Short enough that a pane torn down mid-wait frees its question soon,
    and well under `CATCH_UP_MS`: a wait that ran its whole length is a
    quiet screen, not a catch-up. */
const WAIT_MS = 1000;
/** A beat this long after the last one is a catch-up: what it carries
    was asked while nobody was looking, and is replayed (`replay`). */
const CATCH_UP_MS = 2_000;
/** The most characters one `term_key` carries: 64 KiB once UTF-8 encoded
    (T-16). */
const CHUNK_CHARS = 16 * 1024;


/** How long the frame lights for a bell (BEL). iTerm2's visual bell is
    a flash; a flash that stays is a border, and a border says nothing. */
const BELL_MS = 160;

/** Where the search paints what it found: **the accent's tint**, thin
    for every match and strong under the one the search stands on (the
    audit fix plan F4.8; W18) — read off the theme when the search runs,
    since xterm takes colours, not variables. ⚠️ 墓碑: the terminal's own
    yellow, a third accent beside the olive and the pink. */
function found() {
  const brand = getComputedStyle(document.documentElement).getPropertyValue("--brand-strong").trim() || "#99bb36";
  const hex = /^#[0-9a-f]{6}$/i.test(brand) ? brand : "#99bb36";
  return {
    matchBackground: `${hex}44`,
    matchOverviewRuler: hex,
    activeMatchBackground: `${hex}aa`,
    activeMatchColorOverviewRuler: hex,
  };
}

/** The key that opens the search: ⌘F on a Mac. Elsewhere ⌃⇧F, because
    ⌃F is a byte a shell wants (readline's forward-char) and stealing
    it would be the app taking a key the program had. */
const IS_MAC = /Mac|iPhone|iPad/.test(navigator.userAgent);
/**
 * **A Mac's editing keys say what a Mac terminal says** (T-17, the
 * terminal audit: ⌥←/→ went out as `ESC [ 1 ; 3 D`/`C`, which zsh does not
 * bind; ⌘←/→ and ⌘⌫ sent nothing; ⌥⌫ a bare ESC). The bytes are iTerm2's
 * "Natural Text Editing" key preset (iterm2.com, Profiles › Keys): word
 * left and right are the meta forms `ESC b`/`ESC f` every readline and
 * zsh binds, ⌥⌫ is ^W (delete the word before), ⌘← ⌘→ are ^A/^E (line
 * start and end), ⌘⌫ is ^U (delete to line start).
 * `null` for any other key.
 */
function macEditing(ev: KeyboardEvent): string | null {
  if (!IS_MAC || ev.type !== "keydown" || ev.shiftKey || ev.ctrlKey) return null;
  if (ev.altKey && !ev.metaKey) {
    if (ev.key === "ArrowLeft") return "\x1bb";
    if (ev.key === "ArrowRight") return "\x1bf";
    if (ev.key === "Backspace") return "\x17";
  }
  if (ev.metaKey && !ev.altKey) {
    if (ev.key === "ArrowLeft") return "\x01";
    if (ev.key === "ArrowRight") return "\x05";
    if (ev.key === "Backspace") return "\x15";
  }
  return null;
}

/** ⌘K, which clears a Mac terminal (iTerm2, Terminal: Clear Buffer). */
function asksToClear(ev: KeyboardEvent): boolean {
  return IS_MAC && ev.type === "keydown" && ev.metaKey && !ev.ctrlKey && !ev.altKey && ev.key.toLowerCase() === "k";
}

function asksToFind(ev: KeyboardEvent): boolean {
  if (ev.key.toLowerCase() !== "f" || ev.altKey) return false;
  return IS_MAC ? ev.metaKey && !ev.ctrlKey : ev.ctrlKey && ev.shiftKey;
}

/** Where a link goes. In the app, through khor (`open_link`: the
    scheme whitelist is the same one the conversation pane's links go
    through). In a browser, the browser's own new tab — `open` on the
    serving machine would open it on the wrong screen when the face is
    being read from a phone. */
function follow(uri: string, machine: string | null | undefined) {
  // **In the app, a browsing tab through the terminal's own machine**
  // (roadmap 3.2): a dev server's `localhost:3000` printed on a far
  // machine is that machine's. A browser face keeps its own tab.
  if (inApp) askOpenUrl(machine ?? "", uri);
  else window.open(uri, "_blank", "noopener");
}

/** Base64 → bytes, the shape `term.write` takes. The api boundary is
    JSON strings (gui-core `to_json`), so the feed crosses encoded. */
function b64bytes(b64: string): Uint8Array {
  const raw = atob(b64);
  const out = new Uint8Array(raw.length);
  for (let i = 0; i < raw.length; i += 1) out[i] = raw.charCodeAt(i);
  return out;
}

/**
 * The app's theme, resolved to the concrete colours xterm takes.
 *
 * Read off the pane's own computed style rather than re-declared: the
 * tokens (`--card`/`--card-foreground` through `--term-bg`/`--term-fg`)
 * are the one place the theme lives, and a hex copied here would be the
 * second copy that drifts. Selection stays xterm's default — a alpha
 * wash that works on both grounds.
 */
function themeOf(el: HTMLElement) {
  const cs = getComputedStyle(el);
  const bg = cs.backgroundColor;
  const fg = cs.color;
  return {
    background: bg,
    foreground: fg,
    cursor: fg,
    cursorAccent: bg,
    black: ANSI16[0], red: ANSI16[1], green: ANSI16[2], yellow: ANSI16[3],
    blue: ANSI16[4], magenta: ANSI16[5], cyan: ANSI16[6], white: ANSI16[7],
    brightBlack: ANSI16[8], brightRed: ANSI16[9], brightGreen: ANSI16[10],
    brightYellow: ANSI16[11], brightBlue: ANSI16[12], brightMagenta: ANSI16[13],
    brightCyan: ANSI16[14], brightWhite: ANSI16[15],
  };
}

/** The pane's font, resolved the same way for the same reason — xterm
    measures its own cells from these, which is the whole of what
    replaced the hand-measured `wideFit` ratio.

    **No leading** (2026-10-03, the owner's option A): xterm's
    `lineHeight` multiplies the font's own cell height, which already
    carries the face's ascent and descent (about 1.2 × the size), not the
    size. Handing it the token's 20 / 14 made rows 24 at 14 on a phone,
    some 40% taller than Terminal.app's. The terminal takes 1 — a row is the font's
    height and follows the size by itself; `--text-term--line-height`
    is left to the pane's own text and 外观's sample line. */
function fontOf(el: HTMLElement): { family: string; size: number } {
  const cs = getComputedStyle(el);
  return { family: cs.fontFamily, size: Number.parseFloat(cs.fontSize) || 13 };
}

/** Keys that are only modifiers — their keydown is not a character. */
const MODIFIER_KEYS = new Set(["Shift", "Control", "Alt", "Meta", "CapsLock", "AltGraph"]);

/**
 * A character an input method commits **without composing** while a
 * modifier is held — 「(」「"」「!」 under 拼音 with Shift — reaches the
 * pty, once.
 *
 * The owner, 2026-10-01: 「(」「"」 这类符号要按两次才上屏. Traced in a
 * bare WKWebView with the real 拼音 input source (scratch `ime/`
 * harness), WebKit's order for Shift+9 is
 *
 *     keydown Shift · beforeinput/input insertText "(" · keydown 229 · keyup · keyup Shift
 *
 * — the text lands **before** the key's own keydown. xterm 6.0 then
 * drops it on both of its roads: `_inputEvent` takes an `insertText`
 * only when no keydown is outstanding, and the Shift keydown is one;
 * the 229 keydown's textarea diff snapshots the value *after* the input
 * already landed and finds nothing new. With Shift still held the second
 * press goes through (the first one's keyup cleared the flag) — the
 * "press twice". Chrome fires the 229 keydown first, so the diff works
 * there and no chromium run ever showed it. Same family upstream:
 * xtermjs/xterm.js#5887.
 *
 * So this takes exactly the case xterm skips: an `insertText` that is
 * not composing, arriving while the last keydown was a modifier alone
 * and its keyup has not come (xterm's `_keyDownSeen`, mirrored). Every
 * other `insertText` xterm already sends — through `_inputEvent`, or a
 * keypress it marked handled — and sending it here too would double it.
 * Answers the undo.
 */
function keepShiftedCommits(term: Terminal): () => void {
  const field = term.textarea;
  if (!field) return () => {};
  let held = false;
  let modifierOnly = false;
  const down = (e: KeyboardEvent) => {
    held = true;
    modifierOnly = MODIFIER_KEYS.has(e.key);
  };
  const up = () => {
    held = false;
  };
  const input = (e: Event) => {
    const ev = e as InputEvent;
    if (ev.inputType !== "insertText" || !ev.data || ev.isComposing) return;
    if (held && modifierOnly) term.input(ev.data, true);
  };
  field.addEventListener("keydown", down, true);
  field.addEventListener("keyup", up, true);
  field.addEventListener("input", input, true);
  return () => {
    field.removeEventListener("keydown", down, true);
    field.removeEventListener("keyup", up, true);
    field.removeEventListener("input", input, true);
  };
}

/**
 * **What an iOS keyboard commits outside a composition reaches the pty**
 * (2026-10-03, the owner with 拼音: 「。」 and 「，」 vanished, space did
 * nothing; English letters and pinyin words arrived).
 *
 * Traced in the simulator with the 简体拼音 keyboard: every key of an
 * input method's keyboard comes as `keydown` with keyCode 229, and a
 * character it commits without composing — space, 「。」, 「，」 — lands
 * as `insertText` **in a later task**:
 *
 *     keydown " " (229) · …next task… · beforeinput/input insertText " " · keyup
 *
 * xterm 6 drops it on both of its roads. Its 229 road diffs the textarea
 * one `setTimeout(0)` after the keydown, finds nothing yet, and sends
 * nothing; its `input` road takes an `insertText` only when no keydown is
 * outstanding, and the keyup comes after. A pinyin word is fine: it
 * arrives through `compositionend`, which xterm reads. A desktop input
 * method lands the text before the diff runs, so this never showed off a
 * phone.
 *
 * So this takes exactly that case: a 229 keydown outside a composition
 * whose diff found the field unchanged (this timer is queued after
 * xterm's own, so it runs after it), followed by an `insertText` before
 * the keyup. Every other commit xterm already sends, and sending it here
 * too would double it. Answers the undo.
 */
function keepLateCommits(term: Terminal): () => void {
  const field = term.textarea;
  if (!field) return () => {};
  let missed = false;
  let lifted = false;
  let timer = 0;
  const down = (e: KeyboardEvent) => {
    missed = false;
    lifted = false;
    window.clearTimeout(timer);
    if (e.keyCode !== 229 || e.isComposing) return;
    const before = field.value;
    timer = window.setTimeout(() => {
      missed = field.value === before;
    }, 0);
  };
  const up = () => {
    lifted = true;
  };
  const input = (e: Event) => {
    const ev = e as InputEvent;
    if (!missed || lifted || ev.inputType !== "insertText" || !ev.data || ev.isComposing) return;
    missed = false;
    term.input(ev.data, true);
  };
  field.addEventListener("keydown", down, true);
  field.addEventListener("keyup", up, true);
  field.addEventListener("input", input, true);
  return () => {
    window.clearTimeout(timer);
    field.removeEventListener("keydown", down, true);
    field.removeEventListener("keyup", up, true);
    field.removeEventListener("input", input, true);
  };
}

/**
 * **What is being composed stays on the screen** (2026-10-03). xterm draws
 * the pinyin being typed at the cursor (`.composition-view`, one line, no
 * wrap); near the right edge it ran past it, under the pane's clip, and
 * the letters being chosen from could not be read. It is moved left just
 * enough to end at the screen's edge — never past its left.
 */
function keepCompositionInView(term: Terminal): () => void {
  const view = term.element?.querySelector<HTMLElement>(".composition-view");
  const screen = term.element?.querySelector<HTMLElement>(".xterm-screen");
  if (!view || !screen) return () => {};
  const clamp = () => {
    if (!view.classList.contains("active")) return;
    const room = screen.clientWidth - view.offsetWidth;
    const left = Number.parseFloat(view.style.left) || 0;
    if (left > room) view.style.left = `${Math.max(0, room)}px`;
  };
  const watch = new MutationObserver(clamp);
  watch.observe(view, { attributes: true, attributeFilter: ["style", "class"], childList: true, characterData: true, subtree: true });
  return () => watch.disconnect();
}

/** A report xterm writes for the mouse (SGR `ESC [ <`, X10 `ESC [ M`) —
    not a key, so a latched Ctrl lets it through and keeps waiting. */
const MOUSE_REPORT = /^\x1b\[(<|M)/;

/** The buffer cell under a point, clamped to the screen. */
function cellAt(term: Terminal, x: number, y: number): { col: number; row: number } | null {
  const screen = term.element?.querySelector(".xterm-screen");
  if (!screen) return null;
  const r = screen.getBoundingClientRect();
  const col = Math.max(0, Math.min(term.cols - 1, Math.floor(((x - r.left) / r.width) * term.cols)));
  const inView = Math.max(0, Math.min(term.rows - 1, Math.floor(((y - r.top) / r.height) * term.rows)));
  return { col, row: term.buffer.active.viewportY + inView };
}

/** A downward swipe this long (pt) and this fast (pt/ms) puts the
    keyboard away. */
const DISMISS_PT = 60;
const DISMISS_SPEED = 0.5;

/** Travel past which a touch is a drag, not a tap. */
const TAP_SLOP = 10;

/**
 * **The terminal holds the focus; a tap raises the keyboard only where
 * the person asked for that** (2026-10-03; roadmap 2.2, `lib/term-tap`'s
 * three modes; the focus model, `lib/soft-keyboard`).
 *
 * The field is focused whenever the page is open (`holdFocus`), with no
 * keyboard under a finger. A tap — a touch that ends where it began — is
 * judged at its end, which is a gesture iOS will show a keyboard for:
 * `screen` raises it anywhere; `auto` raises it on the line being typed
 * (`inInput`, from the shell's marks) and puts it away anywhere else;
 * `button` leaves it to the bar's switch. xterm's own mousedown focuses a
 * field that is already focused, and a program that took the mouse still
 * gets the click as before. **If the focus leaves for nowhere** — the system put
 * the keyboard away, or a tap blurred the field — it comes back without a
 * keyboard, so a hardware keyboard keeps typing; a focus that went to
 * another field (the find bar, a sheet) is left there.
 */
function tapsAskFirst(
  term: Terminal,
  box: HTMLElement,
  marks: Marks,
  /** A tap on a cell, first: a link there opens (roadmap 3.2). */
  tapped: (row: number, col: number) => void,
): () => void {
  const field = term.textarea;
  if (!field) return () => {};
  let startX = 0;
  let startY = 0;
  let still = false;
  let downAt = 0;
  const began = (e: TouchEvent) => {
    still = e.touches.length === 1;
    downAt = performance.now();
    startX = e.touches[0]?.clientX ?? 0;
    startY = e.touches[0]?.clientY ?? 0;
    lastX = startX;
    lastY = startY;
  };
  let lastX = 0;
  let lastY = 0;
  const moved = (e: TouchEvent) => {
    const t = e.touches[0];
    if (!t) return;
    lastX = t.clientX;
    lastY = t.clientY;
    if (Math.hypot(t.clientX - startX, t.clientY - startY) > TAP_SLOP) still = false;
  };
  const lifted = () => {
    // **A downward swipe puts the keyboard away**, in every mode (the
    // owner's ruling with the ux lane's deck): iOS's own gesture for it in
    // a scrolling view (`keyboardDismissMode`). A swipe, not a slow drag
    // through history: far enough, fast enough, more down than sideways.
    const dy = lastY - startY;
    const took = performance.now() - downAt;
    if (!still && dy > DISMISS_PT && dy / took > DISMISS_SPEED && dy > Math.abs(lastX - startX) && keyboardUp(field)) {
      lowerKeyboard(field);
      still = false;
      return;
    }
    // A long press is a selection (`pressSelects`), not a tap.
    if (performance.now() - downAt >= LONG_PRESS_MS) still = false;
    if (still) {
      const wake = termWake();
      const at = cellAt(term, startX, startY);
      if (at) tapped(at.row, at.col);
      const here = wake === "screen" || (wake === "auto" && !!at && inInput(term, marks, at.row, at.col));
      if (here && !keyboardUp(field)) raiseKeyboard(field);
      else if (!here && wake === "auto" && keyboardUp(field)) lowerKeyboard(field);
    }
    still = false;
  };
  const left = (e: FocusEvent) => {
    // A mouse's page keeps its own focus rules (`useKeysTo`).
    if (e.relatedTarget || !window.matchMedia("(pointer: coarse)").matches) return;
    window.setTimeout(() => {
      const at = document.activeElement;
      if (!field.isConnected || (at && at !== document.body)) return;
      lowerKeyboard(field);
    }, 0);
  };
  box.addEventListener("touchstart", began, { passive: true });
  box.addEventListener("touchmove", moved, { passive: true });
  box.addEventListener("touchend", lifted);
  field.addEventListener("focusout", left);
  return () => {
    box.removeEventListener("touchstart", began);
    box.removeEventListener("touchmove", moved);
    box.removeEventListener("touchend", lifted);
    field.removeEventListener("focusout", left);
  };
}

/** A finger held this long without moving starts a selection. */
const LONG_PRESS_MS = 450;
/** …and this much travel before then makes it a drag instead. */
const PRESS_SLOP = 8;

/**
 * **A long press selects, a drag after it extends, the lift copies**
 * (T-20, the terminal audit: no touch selection at all, so a phone could
 * copy nothing a program did not offer). xterm 6 has no touch code
 * (xterm.js #5961 is open); iSH and Blink select with native handles.
 * This is the plainer half of that — the same gesture iOS uses for text
 * (press and hold on a word, drag to extend) — drawn in xterm's own
 * selection, so the copy-on-select already wired keeps working, and the
 * lift writes the selection to the clipboard inside the gesture (iOS's
 * clipboard wants one). Ahead of everything else (capture): while a
 * selection is being drawn the drag is the selection's — its moves are
 * cancelled, so the native scroller never starts — and before the press
 * is long, a drag scrolls as ever.
 */
function pressSelects(
  term: Terminal,
  box: HTMLElement,
  /** A long press the caller takes instead (a block's prompt, 2.3). */
  claims: (row: number, x: number, y: number) => boolean,
): () => void {
  let timer = 0;
  let x0 = 0;
  let y0 = 0;
  let selecting = false;
  let anchor: { col: number; row: number } | null = null;
  const blank = (row: number, col: number) => {
    const ch = term.buffer.active.getLine(row)?.getCell(col)?.getChars() ?? "";
    return ch === "" || ch === " ";
  };
  const selectWord = (at: { col: number; row: number }) => {
    let from = at.col;
    let to = at.col;
    if (!blank(at.row, at.col)) {
      while (from > 0 && !blank(at.row, from - 1)) from -= 1;
      while (to < term.cols - 1 && !blank(at.row, to + 1)) to += 1;
    }
    term.select(from, at.row, to - from + 1);
    anchor = { col: from, row: at.row };
  };
  const extend = (to: { col: number; row: number }) => {
    if (!anchor) return;
    const a = anchor.row * term.cols + anchor.col;
    const b = to.row * term.cols + to.col;
    const first = Math.min(a, b);
    term.select(first % term.cols, Math.floor(first / term.cols), Math.abs(b - a) + 1);
  };
  const start = (e: TouchEvent) => {
    window.clearTimeout(timer);
    selecting = false;
    if (e.touches.length !== 1) return;
    x0 = e.touches[0].clientX;
    y0 = e.touches[0].clientY;
    timer = window.setTimeout(() => {
      const at = cellAt(term, x0, y0);
      if (!at) return;
      if (claims(at.row, x0, y0)) return;
      selecting = true;
      selectWord(at);
    }, LONG_PRESS_MS);
  };
  const move = (e: TouchEvent) => {
    const t = e.touches[0];
    if (!t) return;
    if (!selecting) {
      if (Math.hypot(t.clientX - x0, t.clientY - y0) > PRESS_SLOP) window.clearTimeout(timer);
      return;
    }
    e.preventDefault();
    e.stopImmediatePropagation();
    const at = cellAt(term, t.clientX, t.clientY);
    if (at) extend(at);
  };
  const end = (e: TouchEvent) => {
    window.clearTimeout(timer);
    if (!selecting) return;
    selecting = false;
    // No synthesized mousedown after it: that would clear the selection.
    e.preventDefault();
    e.stopImmediatePropagation();
    const text = term.getSelection();
    if (text) navigator.clipboard?.writeText(text).catch(() => {});
  };
  box.addEventListener("touchstart", start, { capture: true, passive: true });
  box.addEventListener("touchmove", move, { capture: true, passive: false });
  box.addEventListener("touchend", end, { capture: true, passive: false });
  box.addEventListener("touchcancel", end, { capture: true, passive: false });
  return () => {
    window.clearTimeout(timer);
    box.removeEventListener("touchstart", start, true);
    box.removeEventListener("touchmove", move, true);
    box.removeEventListener("touchend", end, true);
    box.removeEventListener("touchcancel", end, true);
  };
}

/**
 * Whether the program, not the history, gets the scroll: it took the
 * mouse (khor turns tmux's on for its own grouped session,
 * `Tmux::grouped_attach_argv`), or it is on the alternate screen, which
 * keeps no history. A wheel then goes to xterm, which encodes it as a
 * report or as arrow keys; the native scroller below is inert.
 */
function programScrolls(term: Terminal): boolean {
  return term.modes.mouseTrackingMode !== "none" || term.buffer.active.type === "alternate";
}

/**
 * **A finger's drag is the wheel when the program scrolls** (2026-10-02,
 * iOS 第三步 A). xterm 6 has no touch code of its own (xterm.js #5489,
 * fixed for 7.0 by #5563), so on a full-screen program or under tmux's
 * mouse a drag becomes the wheel events a trackpad would have sent, at the
 * point under the finger, and xterm encodes them as it encodes every
 * wheel. A plain shell's history is not scrolled here: that is the native
 * scroller's (`scrollsNatively`). A drag that starts sideways is left
 * alone (the page's edge swipe).
 */
function fingerWheels(term: Terminal, box: HTMLElement): () => void {
  let lastY: number | null = null;
  let startX = 0;
  let startY = 0;
  let mine = false;
  let target: EventTarget | null = null;
  const start = (e: TouchEvent) => {
    lastY = e.touches.length === 1 ? e.touches[0].clientY : null;
    startX = e.touches.length === 1 ? e.touches[0].clientX : 0;
    startY = lastY ?? 0;
    mine = false;
    target = e.target;
  };
  const move = (e: TouchEvent) => {
    if (lastY === null || e.touches.length !== 1 || !programScrolls(term)) return;
    const t = e.touches[0];
    if (!mine) {
      // Whose drag this is, once it has gone anywhere: up or down is the
      // terminal's, sideways is the page's.
      const ax = Math.abs(t.clientX - startX);
      const ay = Math.abs(t.clientY - startY);
      if (ax < 4 && ay < 4) return;
      if (ax > ay) {
        lastY = null;
        return;
      }
      mine = true;
    }
    const dy = t.clientY - lastY;
    lastY = t.clientY;
    e.preventDefault();
    e.stopPropagation();
    if (dy === 0) return;
    // Natural scrolling: a finger moving up brings later lines, which
    // is a wheel turned down.
    (target ?? box).dispatchEvent(
      new WheelEvent("wheel", {
        deltaY: -dy,
        deltaMode: WheelEvent.DOM_DELTA_PIXEL,
        clientX: t.clientX,
        clientY: t.clientY,
        bubbles: true,
        cancelable: true,
      }),
    );
  };
  const end = () => {
    lastY = null;
    target = null;
    mine = false;
  };
  box.addEventListener("touchstart", start, { passive: true });
  box.addEventListener("touchmove", move, { passive: false });
  box.addEventListener("touchend", end, { passive: true });
  box.addEventListener("touchcancel", end, { passive: true });
  return () => {
    box.removeEventListener("touchstart", start);
    box.removeEventListener("touchmove", move);
    box.removeEventListener("touchend", end);
    box.removeEventListener("touchcancel", end);
  };
}

/** What each cursor shape looks like when the terminal does not have the
    keys: the block hollow, the bar and the underline as they are. */
const RESTING: Record<CursorShape, "outline" | "bar" | "underline"> = {
  block: "outline",
  bar: "bar",
  underline: "underline",
};

/**
 * **The cursor is the person's until a program asks for its own** (item
 * 16 B). The setting (`lib/term-cursor`) is applied live. A program's
 * DECSCUSR (`CSI n q`: 1–2 block, 3–4 underline, 5–6 bar, odd blinking)
 * sets the shape while it runs, and the blink only if the person's
 * setting lets the cursor blink (item 18). The setting comes back on
 * `CSI 0 q` — as in iTerm2, kitty and WezTerm, where 0 means "the user's
 * default" (xterm.js reads it as a blinking block) — or when a full-screen
 * program leaves the
 * alternate screen (vim's insert-mode bar does not outlive vim). A
 * program's `CSI ? 12 h` / `l`, which xterm.js turns into its
 * `cursorBlink` option for good, is counted as the same kind of request.
 *
 * Handled here rather than by xterm because xterm.js keeps DECSCUSR in its
 * own private modes, which its WebGL renderer reads only when an option
 * changes, so a program's blinking cursor appeared at some later moment
 * (the next theme or font change, the next WebGL reload) rather than when
 * asked.
 */
function cursorFollows(term: Terminal): () => void {
  let mine = termCursor();
  let theirs: { shape?: CursorShape; blink?: boolean } = {};
  let blinkSet = term.options.cursorBlink;
  const apply = () => {
    const shape = theirs.shape ?? mine.shape;
    // **The person's blink setting wins** (item 18, the owner's rule): a
    // program may change the shape; it may make the cursor blink only
    // when the person lets it blink at all.
    const blink = mine.blink && (theirs.blink ?? true);
    if (term.options.cursorStyle !== shape) term.options.cursorStyle = shape;
    if (term.options.cursorInactiveStyle !== RESTING[shape]) term.options.cursorInactiveStyle = RESTING[shape];
    if (term.options.cursorBlink !== blink) term.options.cursorBlink = blink;
    blinkSet = blink;
  };
  const decscusr = term.parser.registerCsiHandler({ intermediates: " ", final: "q" }, (params) => {
    const n = typeof params[0] === "number" ? params[0] : 0;
    if (n === 0) theirs = {};
    else if (n <= 6) theirs = { shape: n <= 2 ? "block" : n <= 4 ? "underline" : "bar", blink: n % 2 === 1 };
    apply();
    return true;
  });
  const parsed = term.onWriteParsed(() => {
    if (term.options.cursorBlink === blinkSet) return;
    // `CSI ? 12 h` / `l`: a program asking for blinking, honoured only
    // when the setting allows it (`apply`).
    theirs = { ...theirs, blink: term.options.cursorBlink };
    apply();
  });
  const swapped = term.buffer.onBufferChange((b) => {
    if (b.type !== "normal" || (theirs.shape === undefined && theirs.blink === undefined)) return;
    theirs = {};
    apply();
  });
  const chosen = onTermCursor((c) => {
    mine = c;
    apply();
  });
  return () => {
    decscusr.dispose();
    parsed.dispose();
    swapped.dispose();
    chosen();
  };
}

/** Prototypes of xterm renderers already taught `blinksOnInput`'s rule. */
const quieted = new WeakSet<object>();

/**
 * **The blink restarts on a key, not on output** (item 16 A). xterm.js's
 * WebGL renderer restarts its blink phase on every cursor move
 * (`WebglRenderer.handleCursorMove` → `CursorBlinkStateManager.
 * restartBlinkAnimation`): a progress line rewritten every 300 ms gave
 * phases of 16 to 1885 ms instead of 600 (measured, item 16). Terminal.app
 * and iTerm2 hold the cursor up while one types and otherwise keep the
 * beat. So the renderer's cursor-move path is run without its restart, and
 * the pane restarts the phase itself when a key goes out (`typed`).
 * Focus still restarts it (xterm's `resume`), as it does in those
 * terminals. xterm's DOM renderer, the fallback after a lost WebGL context
 * (T-23), blinks by a CSS animation of its own and is not changed.
 */
function blinksOnInput(term: Terminal): { quiet: () => void; typed: () => void } {
  type Manager = { restartBlinkAnimation?: () => void };
  type Renderer = { handleCursorMove?: () => void; _cursorBlinkStateManager?: { value?: Manager } };
  const renderer = () =>
    (term as unknown as { _core?: { _renderService?: { _renderer?: { value?: Renderer } } } })._core?._renderService
      ?._renderer?.value;
  return {
    quiet: () => {
      const r = renderer();
      if (!r || !("_cursorBlinkStateManager" in r) || typeof r.handleCursorMove !== "function") return;
      const proto = Object.getPrototypeOf(r) as Renderer;
      if (quieted.has(proto) || typeof proto.handleCursorMove !== "function") return;
      const moved = proto.handleCursorMove;
      proto.handleCursorMove = function (this: Renderer) {
        const m = this._cursorBlinkStateManager?.value;
        if (!m) return moved.call(this);
        m.restartBlinkAnimation = () => {};
        try {
          moved.call(this);
        } finally {
          delete m.restartBlinkAnimation;
        }
      };
      quieted.add(proto);
    },
    typed: () => renderer()?._cursorBlinkStateManager?.value?.restartBlinkAnimation?.(),
  };
}

/** The block a buffer line belongs to: from its prompt to the next one. */
function blockAt(marks: Marks, line: number): Block | null {
  const all = marks.blocks();
  for (let i = all.length - 1; i >= 0; i--) {
    if (all[i].prompt.line <= line) return all[i];
  }
  return null;
}

/** Whether a line is a block's prompt or its typed command (not output). */
function onPromptOf(b: Block, line: number): boolean {
  const last = b.output ? b.output.line - 1 : b.prompt.line;
  return line >= b.prompt.line && line <= Math.max(last, b.prompt.line);
}

/** Buffer lines as text, a wrapped line joined to the one it continues. */
function linesText(term: Terminal, from: number, to: number): string {
  const buf = term.buffer.active;
  let out = "";
  for (let i = from; i < to; i++) {
    const line = buf.getLine(i);
    if (!line) continue;
    if (i > from && !line.isWrapped) out += "\n";
    out += line.translateToString(!buf.getLine(i + 1)?.isWrapped);
  }
  return out.replace(/\s+$/, "");
}

/** A block's output, from where it ran to where the next prompt began. */
function outputOf(term: Terminal, b: Block): string {
  if (!b.output) return "";
  const end = b.end ? b.end.line : term.buffer.active.length;
  return linesText(term, b.output.line, end);
}

/** A block's command: as the shell named it, else as typed. */
function commandOf(term: Terminal, b: Block): string {
  if (b.command) return b.command;
  if (!b.input) return "";
  const end = b.output ? b.output.line : b.input.marker.line + 1;
  return linesText(term, b.input.marker.line, end).slice(b.input.col).trim();
}

/**
 * **Command blocks, drawn** (roadmap 2.3): a hairline where each prompt
 * starts, and once a command has ended, its exit status as a 2 px stripe
 * in the left margin beside the whole block — the state inks, done green
 * for 0 and failed red otherwise. xterm decorations pinned to the marks'
 * markers, so they ride the scrollback; VS Code draws its command
 * decorations the same way (a gutter mark per command, coloured by exit).
 */
function drawBlocks(term: Terminal, marks: Marks): () => void {
  const drawn = new Map<Block, IDisposable[]>();
  const draw = () => {
    for (const b of marks.blocks()) {
      const has = drawn.get(b) ?? [];
      if (has.length === 0) {
        const hair = term.registerDecoration({ marker: b.prompt, width: term.cols, height: 1 });
        hair?.onRender((el) => el.classList.add("term-block-hair"));
        if (hair) has.push(hair);
      }
      if (b.end && has.length === 1) {
        const height = Math.max(1, b.end.line - b.prompt.line);
        const tint = term.registerDecoration({ marker: b.prompt, x: 0, width: 1, height });
        tint?.onRender((el) => {
          el.classList.add("term-block-tint");
          el.dataset.exit = String(b.exit ?? "");
        });
        if (tint) has.push(tint);
      }
      drawn.set(b, has);
    }
  };
  const changed = marks.onChange(draw);
  draw();
  return () => {
    changed.dispose();
    drawn.forEach((ds) => ds.forEach((d) => d.dispose()));
    drawn.clear();
  };
}

/** Scrolls to the previous (`-1`) or next (`1`) prompt from the screen's
    top; past the last one, to the bottom. */
function jumpToPrompt(term: Terminal, marks: Marks, dir: -1 | 1) {
  const top = term.buffer.active.viewportY;
  const lines = marks.blocks().map((b) => b.prompt.line);
  const to = dir < 0 ? lines.filter((l) => l < top).pop() : lines.find((l) => l > top);
  if (to === undefined) {
    if (dir > 0) term.scrollToBottom();
    return;
  }
  term.scrollToLine(to);
}

/** Output quiet this long ends a frame (ms). */
const FRAME_QUIET_MS = 8;
/** The longest a frame is held before it is painted anyway (ms). */
const FRAME_MAX_MS = 60;

/** `CSI ? 25 l` / `h` and `CSI ? 2026 h` / `l`: where a redraw opens and
    closes. */
const OPENS = /\x1b\[\?(25l|2026h)/g;
const CLOSES = /\x1b\[\?(25h|2026l)/g;

/**
 * **Paint whole frames** (item 18 B; the owner: the cursor "blinks" only in
 * Claude Code, and the pinyin preview jumps between the status line, the
 * input and the output). A TUI redraws by hiding the cursor, moving up,
 * repainting and parking the cursor back in its input box; a real terminal
 * shows only the finished frame. Here bytes reached xterm as they arrived —
 * `term_wait` answers the moment any byte lands, so one redraw came as
 * several writes — and xterm painted whatever it had at its next animation
 * frame: the cursor's places mid-redraw were frames on screen, and the
 * composition preview, drawn at the cursor, went with them.
 *
 * So output is held until the frame is whole: until the stream has been
 * quiet `FRAME_QUIET_MS`, and while a redraw is open — a cursor hidden
 * (`?25l`) and not yet shown again, or DEC synchronized output (`?2026h`,
 * which xterm honours by itself once it has the bytes) not yet ended — until
 * it closes, never past `FRAME_MAX_MS`. kitty and WezTerm coalesce output
 * the same way (kitty's `input_delay`/`repaint_delay`, a few ms) and treat
 * mode 2026 as the program's own frame marks.
 */
function framesWhole(term: Terminal): { paint: (bytes: Uint8Array) => void; flush: () => void; drop: () => void } {
  let held: Uint8Array[] = [];
  let since = 0;
  let timer = 0;
  /** A redraw opened and not closed, across the held chunks. */
  let open = false;
  const text = new TextDecoder("latin1");
  const flush = () => {
    window.clearTimeout(timer);
    timer = 0;
    if (held.length === 0) return;
    const n = held.reduce((a, c) => a + c.length, 0);
    const all = new Uint8Array(n);
    let at = 0;
    for (const c of held) {
      all.set(c, at);
      at += c.length;
    }
    held = [];
    since = 0;
    term.write(all);
  };
  const paint = (bytes: Uint8Array) => {
    // The gates' before-and-after switch: written as it arrives.
    if ((window as unknown as { __framesOff?: boolean }).__framesOff) return term.write(bytes);
    held.push(bytes);
    const now = performance.now();
    if (!since) since = now;
    // Which comes last in this chunk, an opening or a closing mark.
    const s = text.decode(bytes);
    let lastOpen = -1;
    let lastClose = -1;
    for (const m of s.matchAll(OPENS)) lastOpen = m.index;
    for (const m of s.matchAll(CLOSES)) lastClose = m.index;
    if (lastOpen > lastClose) open = true;
    else if (lastClose > lastOpen) open = false;
    const left = FRAME_MAX_MS - (now - since);
    if (left <= 0) return flush();
    window.clearTimeout(timer);
    timer = window.setTimeout(flush, open ? left : Math.min(FRAME_QUIET_MS, left));
  };
  const drop = () => {
    window.clearTimeout(timer);
    timer = 0;
    held = [];
    since = 0;
    open = false;
  };
  return { paint, flush, drop };
}

/** A two-finger tap: both fingers down this briefly, barely moving. */
const TWO_TAP_MS = 300;
/** A sideways swipe that steps tabs: this far, mostly sideways, this
    briefly, and not from the screen's left edge, which is the page's own
    edge swipe back (iOS reserves it the same way). */
const SWIPE_PT = 80;
const SWIPE_MS = 400;
const EDGE_PT = 24;

/**
 * **The phone's gestures on a terminal** (roadmap 4.4), the set Blink Shell
 * and Termius use: a two-finger tap is Esc; a pinch sizes the terminal's
 * text, landing on the setting's own steps (`lib/type-size`, every 2 pt
 * from 10 to 22) when the fingers lift; a quick sideways swipe goes to the
 * next or previous session tab.
 */
function phoneGestures(term: Terminal, box: HTMLElement): () => void {
  let fingers = 0;
  let downAt = 0;
  let d0 = 0;
  let scale = 1;
  let travel = 0;
  let x0 = 0;
  let y0 = 0;
  let x1 = 0;
  let y1 = 0;
  const gap = (t: TouchList) => Math.hypot(t[0].clientX - t[1].clientX, t[0].clientY - t[1].clientY);
  const start = (e: TouchEvent) => {
    if (e.touches.length === 1) {
      fingers = 1;
      downAt = performance.now();
      x0 = x1 = e.touches[0].clientX;
      y0 = y1 = e.touches[0].clientY;
      travel = 0;
    } else if (e.touches.length === 2) {
      fingers = 2;
      d0 = gap(e.touches);
      scale = 1;
    }
  };
  const move = (e: TouchEvent) => {
    if (e.touches.length === 2 && fingers === 2) {
      scale = gap(e.touches) / d0;
      travel = Math.max(travel, Math.abs(gap(e.touches) - d0));
    } else if (e.touches.length === 1 && fingers === 1) {
      x1 = e.touches[0].clientX;
      y1 = e.touches[0].clientY;
    }
  };
  const end = (e: TouchEvent) => {
    if (e.touches.length > 0) return;
    const took = performance.now() - downAt;
    if (fingers === 2) {
      if (travel > 12) {
        const t = typeSize();
        const want = Math.round((t.term * scale) / 2) * 2;
        const term_ = Math.max(TERM_MIN, Math.min(TERM_MAX, want));
        if (term_ !== t.term) setTypeSize({ ...t, term: term_ });
      } else if (took < TWO_TAP_MS) {
        term.input("\x1b", true);
      }
    } else if (fingers === 1) {
      const dx = x1 - x0;
      const dy = y1 - y0;
      if (Math.abs(dx) > SWIPE_PT && Math.abs(dx) > 2 * Math.abs(dy) && took < SWIPE_MS && x0 > EDGE_PT) {
        askStepTab(dx < 0 ? 1 : -1);
      }
    }
    fingers = 0;
  };
  box.addEventListener("touchstart", start, { passive: true });
  box.addEventListener("touchmove", move, { passive: true });
  box.addEventListener("touchend", end, { passive: true });
  box.addEventListener("touchcancel", end, { passive: true });
  return () => {
    box.removeEventListener("touchstart", start);
    box.removeEventListener("touchmove", move);
    box.removeEventListener("touchend", end);
    box.removeEventListener("touchcancel", end);
  };
}

/** Whether a box has a size a terminal can be measured in. */
const hasSize = (el: HTMLElement) => el.clientWidth > 0 && el.clientHeight > 0;

/**
 * **Resolves once `box` can be measured for good** (item 15 A): it has a
 * size (xterm's `open` and the fit addon both measure the DOM, and xterm
 * says the element "must be visible (have dimensions)"), and no ancestor
 * is still running a finite animation or transition — the screen's
 * entrance (`screen-in-from-right`), the edge swipe's settle. An endless
 * one (a spinner) is not waited for.
 */
async function settled(box: HTMLElement, signal: AbortSignal): Promise<void> {
  if (!hasSize(box)) {
    await new Promise<void>((done) => {
      const observer = new ResizeObserver(() => {
        if (!hasSize(box)) return;
        observer.disconnect();
        done();
      });
      observer.observe(box);
      signal.addEventListener("abort", () => observer.disconnect());
    });
  }
  const moving: Promise<unknown>[] = [];
  for (let el: Element | null = box; el; el = el.parentElement) {
    for (const a of el.getAnimations?.() ?? []) {
      const end = a.effect?.getComputedTiming().endTime;
      if (a.playState === "running" && typeof end === "number" && Number.isFinite(end)) {
        moving.push(a.finished.catch(() => {}));
      }
    }
  }
  // **Never longer than an entrance** (item 18 A): whatever else runs, the
  // first fit happens `SETTLE_CAP_MS` after the box has a size.
  await Promise.race([Promise.all(moving), new Promise((r) => window.setTimeout(r, SETTLE_CAP_MS))]);
}

/** The longest the first fit waits for an entrance to finish (ms). */
const SETTLE_CAP_MS = 300;

/** Quiet this long after the last scroll event, a scroll has stopped. */
const SETTLE_MS = 140;

/**
 * **The history scrolls the way a page does** (2026-10-03, item 15 B; the
 * owner: 「滚动很卡顿，范围也很小，没有那种浏览网页的拖动感」).
 *
 * xterm 6 replaced its native scroller with VS Code's drawn one
 * (`SmoothScrollableElement`), which answers the wheel by itself and a
 * finger not at all, so the finger scroll this pane had was synthetic:
 * a line per row of travel and an imitated fling. Here the scrolling is
 * the engine's again, the way xterm 5's `.xterm-viewport` and hterm's
 * `ScrollPort` (Blink Shell's renderer) do it: `scroller` scrolls
 * natively (momentum, the edge's bounce, the trackpad's own curve), and
 * `room`, after the terminal in it, is sized to the history above the
 * screen — so its range is the buffer's. The terminal itself is
 * `position: sticky` at the scroller's top, so every touch and click
 * still lands on xterm (selection, taps, links), and a drag or a wheel
 * that xterm does not take scrolls the scroller around it.
 *
 * - **scroller → xterm**: the line under the scroller's top is
 *   `scrollTop / row`; xterm is moved there (`scrollToLine`), and the
 *   screen is shifted by the part of a row left over, so the content
 *   follows the finger by the pixel, not by the line. A scroll that
 *   stops between two rows is carried to the nearer one.
 * - **xterm → scroller**: output, a key that scrolls, a resize — any
 *   change of xterm's line — sets `scrollTop` once; the scroll event
 *   that answers it is recognised and ignored, so the two never chase.
 * - **The wheel**: a wheel the scroller takes never reaches xterm's own
 *   scroller (stopped on the way down), so a Mac's wheel and trackpad are
 *   the system's too. When the program scrolls (`programScrolls`) the
 *   scroller has no range and the wheel goes on to xterm, as before.
 *
 * VS Code's terminal keeps xterm's drawn scroller (it has no touch
 * case); Hyper keeps xterm's too. Termius and Blink draw natively.
 */
function scrollsNatively(
  term: Terminal,
  scroller: HTMLElement,
  room: HTMLElement,
  screen: HTMLElement,
): { sync: () => void; stop: () => void } {
  let row = 0;
  let kicked = false;
  // The `scrollTop` this code set last, until its scroll event comes back.
  let echo: number | null = null;
  let touching = 0;
  let settle = 0;
  let frame = 0;
  const measure = () => {
    const el = term.element?.querySelector(".xterm-screen");
    const h = el ? el.getBoundingClientRect().height / term.rows : 0;
    if (h > 0) row = h;
    return row;
  };
  const shift = (px: number) => {
    const v = px === 0 ? "" : `translateY(${px}px)`;
    if (screen.style.transform !== v) screen.style.transform = v;
  };
  /** xterm's own line → the scroller's range and place. */
  const sync = () => {
    frame = 0;
    if (!measure()) return;
    const b = term.buffer.active;
    const inert = programScrolls(term);
    const height = `${(inert ? 0 : b.baseY) * row}px`;
    if (room.style.height !== height) {
      const first = !kicked && !inert && b.baseY > 0;
      room.style.height = height;
      // **WebKit on iOS, the first time there is anything to scroll**
      // (item 18 A, hypothesis 3): a scroller that had nothing to scroll
      // when its layer was made can keep that answer until something
      // relays it out — the keyboard's resize was that something. Drop
      // and restore its overflow once, so the range is read now.
      if (first) {
        kicked = true;
        scroller.style.overflowY = "hidden";
        void scroller.offsetHeight;
        scroller.style.overflowY = "";
      }
    }
    const line = inert ? 0 : b.viewportY;
    if (Math.round(scroller.scrollTop / row) !== line || inert) {
      const top = line * row;
      if (Math.abs(scroller.scrollTop - top) >= 0.5) {
        echo = top;
        scroller.scrollTop = top;
      }
      shift(0);
    }
  };
  const syncSoon = () => {
    if (!frame) frame = window.requestAnimationFrame(sync);
  };
  /** The scroller's place → xterm's line. */
  const scrolled = () => {
    const top = scroller.scrollTop;
    if (echo !== null && Math.abs(top - echo) < 1) {
      echo = null;
      return;
    }
    echo = null;
    if (programScrolls(term) || !row) return;
    const b = term.buffer.active;
    const line = Math.max(0, Math.min(b.baseY, Math.round(top / row)));
    if (line !== b.viewportY) {
      term.scrollToLine(line);
      // xterm's own scroller can hold a stale range — it does not
      // re-measure when its renderer's size changes without the buffer's
      // (xterm.js #6117, #6172) — and then refuses the line; bring it up
      // to date and ask again.
      if (term.buffer.active.viewportY !== line) {
        (term as unknown as { _core?: { _viewport?: { _sync?: () => void } } })._core?._viewport?._sync?.();
        term.scrollToLine(line);
      }
    }
    // Between two rows: the screen follows by the pixel. Not past either
    // end, where the engine's own bounce moves everything already.
    const max = b.baseY * row;
    shift(top < 0 || top > max ? 0 : Math.max(-row, Math.min(row, line * row - top)));
    window.clearTimeout(settle);
    settle = window.setTimeout(rest, SETTLE_MS);
  };
  /** A scroll that stopped between two rows goes to the nearer one. */
  const rest = () => {
    if (touching > 0) return;
    const top = scroller.scrollTop;
    const to = Math.round(top / row) * row;
    if (Math.abs(top - to) < 0.5) return shift(0);
    const still = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    scroller.scrollTo({ top: to, behavior: still ? "auto" : "smooth" });
  };
  const down = () => {
    touching += 1;
    window.clearTimeout(settle);
  };
  const up = (e: TouchEvent) => {
    touching = e.touches.length;
    window.clearTimeout(settle);
    settle = window.setTimeout(rest, SETTLE_MS);
  };
  // Capture, on the way down: a wheel the scroller will take must not
  // reach xterm's scroller, which would take it first and cancel it.
  const wheel = (e: WheelEvent) => {
    if (!programScrolls(term)) e.stopPropagation();
  };
  scroller.addEventListener("scroll", scrolled, { passive: true });
  scroller.addEventListener("wheel", wheel, { capture: true, passive: true });
  scroller.addEventListener("touchstart", down, { passive: true });
  scroller.addEventListener("touchend", up, { passive: true });
  scroller.addEventListener("touchcancel", up, { passive: true });
  const moved = term.onScroll(syncSoon);
  const wrote = term.onWriteParsed(syncSoon);
  const sized = term.onResize(syncSoon);
  const swapped = term.buffer.onBufferChange(syncSoon);
  const rendered = term.onRender(() => {
    // The first frame drawn knows the row's height.
    if (!row) syncSoon();
  });
  return {
    sync,
    stop: () => {
      window.cancelAnimationFrame(frame);
      window.clearTimeout(settle);
      scroller.removeEventListener("scroll", scrolled);
      scroller.removeEventListener("wheel", wheel, true);
      scroller.removeEventListener("touchstart", down);
      scroller.removeEventListener("touchend", up);
      scroller.removeEventListener("touchcancel", up);
      moved.dispose();
      wrote.dispose();
      sized.dispose();
      swapped.dispose();
      rendered.dispose();
    },
  };
}

/** The open terminals' re-joins, by session (`reopenTerminal`). */
const reopeners = new Map<string, () => Promise<void>>();

/**
 * **Join an open terminal again from scratch** — the reload circle's act
 * (2026-10-03), for anything else that knows a terminal is stuck (the
 * network lane's reconnect strip). Answers false when no pane has that
 * session open; otherwise resolves once the join has answered, joined or
 * not (a failed join says why in the pane).
 */
export async function reopenTerminal(id: string): Promise<boolean> {
  const reopen = reopeners.get(id);
  if (!reopen) return false;
  await reopen();
  return true;
}

export function TerminalPane({
  id,
  machine,
  active = true,
}: {
  id: string;
  /** Whether this is the tab on screen. A kept tab that is not
      (`DetailPane`, T-11) stays attached at the hidden pace and draws
      nothing in the bar. */
  active?: boolean;
  /** The far machine this terminal lives on, by the name people read —
      `null` for one hosted here. What a wait and a reconnect name. */
  machine?: string | null;
}) {
  const boxRef = useRef<HTMLDivElement>(null);
  const mountRef = useRef<HTMLDivElement>(null);
  // The native scroller around the terminal, and the room after it that
  // gives it the history's height (`scrollsNatively`).
  const scrollerRef = useRef<HTMLDivElement>(null);
  const roomRef = useRef<HTMLDivElement>(null);
  const syncScroller = useRef<() => void>(() => {});
  const termRef = useRef<Terminal | null>(null);
  const [gone, setGone] = useState(false);
  // The reload circle's join in flight (`reopen`): its glyph turns.
  const [joining, setJoining] = useState(false);
  const reopenRef = useRef<(() => Promise<void>) | null>(null);
  // Why the terminal never opened, in khor's own words. **Not the same
  // fact as `gone`**: `gone` means the host this pane was watching
  // ended, and 这个终端结束了 is true of it; a failed open means there
  // never was a host to watch, and saying 结束了 about a tmux session
  // sitting right there sends a person looking for a session that
  // ended. Every refusal khor makes here is already a sentence written
  // for a person, and throwing it away is what once flattened all of
  // them into one wrong word.
  const [why, setWhy] = useState<string | null>(null);
  // **Until the attachment is made, the pane says it is coming**
  // (2026-10-02, the phone: a far machine that had dropped off left the
  // board blank for the reach's whole wait). A never-seen region shows
  // its skeleton, never a blank board (guidelines/20-states); the
  // sentence replaces it if the open fails.
  const [opening, setOpening] = useState(true);
  // **When the open began, and which try this is** (round NET-FIX): a
  // wait past a second gets a spinner, past eight the machine's name,
  // the time and 停止等待 / 重试 (`Waiting`). A retry is a new try — the
  // effect below runs again from scratch.
  const [openedAt, setOpenedAt] = useState<number | null>(null);
  const [attempt, setAttempt] = useState(0);
  // The far pipe ended and is being asked for again (`TermChunk::
  // rejoining`): the screen is the last one that arrived, and it says so.
  const [rejoining, setRejoining] = useState(false);
  // Why the last drop produced no paste, in khor's own words. Kept on
  // screen until a drop works (docs/UX.md 失败就地说).
  const [dropWhy, setDropWhy] = useState<string | null>(null);
  const hidden = useHidden();
  const pace = useRef(LIVE_MS);
  const pollNow = useRef<() => void>(() => {});
  // xterm's own textarea, once the terminal is open — the element an
  // input method composes in, and where `useKeysTo` sends focus and
  // stray first keystrokes. The IME handling behind it is xterm's now;
  // the hand-rolled composition guards this file used to carry
  // (keyCode 229, compositionend, the parked caret) live there.
  const keysRef = useRef<HTMLTextAreaElement | null>(null);
  useKeysTo(keysRef, active);
  // Whether the reader is up in history, and whether output arrived
  // while they were — the two sentences the way-back-down control
  // carries. Survived the pivot on purpose: xterm holds the reader's
  // place when output scrolls past, but says nothing about it, and an
  // arrival with no sign is an arrival missed. Same pair of words the
  // conversation uses.
  const [back, setBack] = useState(false);
  const [freshBelow, setFreshBelow] = useState(false);
  // The search: whether the bar is up, what is in it, and where the
  // addon says the matches are (`of` is -1 past the highlight limit).
  const searchRef = useRef<SearchAddon | null>(null);
  const [finding, setFinding] = useState(false);
  const [query, setQuery] = useState("");
  const [hits, setHits] = useState<{ at: number; of: number } | null>(null);
  // The bell's flash, and the timer that ends it.
  const [rang, setRang] = useState(false);
  // **What is being composed, under a finger** (item 18 B): shown in a
  // strip of its own at the screen's foot, not at the cursor, which a
  // redrawing program moves under it.
  const [composingText, setComposingText] = useState<string | null>(null);
  // **A question the pane asks before acting** (roadmap 4.3): a paste of
  // several lines, or a program wanting to read the clipboard.
  const [asking, setAsking] = useState<{ text: string; detail?: string; yes: string; no: string; go: () => void; stop: () => void } | null>(null);
  // A command block's actions (roadmap 2.3), where it was pressed: from a
  // right-click, or a long press on its prompt.
  const [blockMenu, setBlockMenu] = useState<{ x: number; y: number; acts: { name: string; label: string; run: () => void }[] } | null>(null);
  const bellTimer = useRef(0);
  // **Ctrl and Alt, latched from the key row** (`TermKeys`, 2026-10-02;
  // Alt 2026-10-03): each waits for the next key from the row, its sheet
  // or the soft keyboard. A ref as well as state because the `onData`
  // handler below reads them, and that handler is wired once per terminal.
  const coarse = useCoarse();
  /** The keyboard's wake mode (`lib/term-tap`, the terminal lane's 2.2):
      the deck's keyboard circle is drawn in 点击按钮 alone (round UX-3 3a). */
  const wake = useTermWake();
  const [mods, setModsShown] = useState<Mods>({ ctrl: false, alt: false });
  const modsRef = useRef<Mods>({ ctrl: false, alt: false });
  const latch = (m: Mods) => {
    modsRef.current = m;
    setModsShown(m);
  };

  // The block menu goes away on a press anywhere else, Esc, or a scroll.
  useEffect(() => {
    if (!blockMenu) return;
    const away = () => setBlockMenu(null);
    const esc = (e: KeyboardEvent) => {
      if (e.key === "Escape") away();
    };
    document.addEventListener("pointerdown", away);
    document.addEventListener("keydown", esc, true);
    const scroller = scrollerRef.current;
    scroller?.addEventListener("scroll", away, { passive: true });
    return () => {
      document.removeEventListener("pointerdown", away);
      document.removeEventListener("keydown", esc, true);
      scroller?.removeEventListener("scroll", away);
    };
  }, [blockMenu]);

  useLayoutEffect(() => {
    const box = boxRef.current;
    const mount = mountRef.current;
    if (!box || !mount) return;
    const font = fontOf(box);
    const term = new Terminal({
      // The unicode addon is a proposed API; without this flag it throws.
      allowProposedApi: true,
      // **OSC 8 links go where every other link goes** (T-24, the
      // terminal audit: they went to xterm's default `confirm()` and
      // `window.open`, around `open_link`'s whitelist; OscLinkProvider.ts
      // 114). The same gesture as a plain URL (⌘-click, iTerm2's), and
      // the same road (`follow`).
      linkHandler: {
        activate: (ev, uri) => {
          if (ev.metaKey || ev.ctrlKey) follow(uri, machine);
        },
      },
      // **The size reports programs ask for** (T-1: CSI 14 t, 16 t, 18 t
      // went unanswered because `windowOptions` defaults to none;
      // yazi asks 16 t for its cell size). Reports only — nothing here
      // lets a program move, resize or retitle the window.
      windowOptions: { getWinSizePixels: true, getCellSizePixels: true, getWinSizeChars: true },
      // The history this face keeps (`lib/term-history`, item 17; item 15
      // C raised it from tmux's 2000). The parser in `khor_gui_core::term`
      // keeps none, so this is the only history a face has; after a
      // re-attach it holds what the host's ring still has, not more.
      scrollback: termHistory(),
      fontFamily: font.family,
      fontSize: font.size,
      lineHeight: 1,
      theme: themeOf(box),
      // **The cursor is the focus** (2026-10-03): the chosen shape while
      // the terminal has the keys, its hollow or thin form when it does
      // not. The pane draws no ring of its own (`app.css`). The shape and
      // the blink are the person's (`lib/term-cursor`, `cursorFollows`).
      cursorStyle: termCursor().shape,
      cursorInactiveStyle: RESTING[termCursor().shape],
      cursorBlink: termCursor().blink,
      // **⌥-drag is xterm's own selection even when the program asked
      // for the mouse** — iTerm2's gesture. khor turns tmux's mouse on
      // for the tmux sessions it attaches (2026-10-01, so the wheel
      // scrolls tmux's history; `Tmux::grouped_attach_argv`), and from
      // then on a plain drag is tmux's: it selects in copy mode and
      // copies into tmux's own buffer, which never reaches the Mac's
      // clipboard (that would take OSC 52, which this pane does not
      // answer). xterm's escape hatch is Shift elsewhere, but on a Mac
      // it is ⌥ and only with this flag — without it there was no way
      // to select into the clipboard at all. The cost: ⌥-drag is no
      // longer xterm's column selection.
      macOptionClickForcesSelection: true,
    });
    const fit = new FitAddon();
    term.loadAddon(fit);
    // Real east-asian widths — the measured `wideFit` scale hack this
    // replaces existed because the DOM painter had to guess what a
    // renderer that owns its glyph atlas simply knows. **Unicode 15 with
    // grapheme clusters** (T-22, the terminal audit: unicode11 drew ❤️
    // one cell wide, 🇨🇳 as two boxes, 👍🏽 four cells and 👨‍👩‍👧 six),
    // so an emoji sequence is one character of two cells, as in kitty,
    // WezTerm and Ghostty. The addon's README says only to load it; its
    // `activate` registers "15" and "15-graphemes" and selects the
    // latter (0.4.0), so no `activeVersion` is set here.
    term.loadAddon(new UnicodeGraphemesAddon());
    term.open(mount);
    // **OSC 52: a program may write the clipboard, never read it** (T-8,
    // the terminal audit: neither direction was handled). The field's
    // defaults: kitty, Alacritty, WezTerm and Windows Terminal allow the
    // write and refuse (or ask about) the read. The read form (`?`) is
    // answered with nothing, so a program waiting for it times out as it
    // would on any terminal that refuses.
    // **Reading is asked about once per terminal** (roadmap 4.3; T-8 had
    // refused it outright): the answer — kitty's `clipboard_control` and
    // WezTerm ask the same way — holds until the pane closes. A refusal
    // answers nothing, so the program times out as on any terminal that
    // refuses.
    let mayRead: boolean | null = null;
    const readClipboard = (target: string) => {
      void navigator.clipboard
        ?.readText()
        .then((text) => {
          const b64 = btoa(String.fromCharCode(...new TextEncoder().encode(text)));
          send(`\x1b]52;${target};${b64}\x07`);
        })
        .catch(() => {});
    };
    const osc52 = term.parser.registerOscHandler(52, (data) => {
      const semi = data.indexOf(";");
      const payload = semi >= 0 ? data.slice(semi + 1) : "";
      const target = semi >= 0 ? data.slice(0, semi) || "c" : "c";
      if (payload === "?") {
        // A question from history answers nothing (`replay`).
        if (replaying > 0) return true;
        if (mayRead === true) readClipboard(target);
        else if (mayRead === null) {
          setAsking({
            text: gui.clip_read_ask,
            yes: gui.clip_read_allow,
            no: gui.clip_read_deny,
            go: () => {
              mayRead = true;
              readClipboard(target);
            },
            stop: () => {
              mayRead = false;
            },
          });
        }
        return true;
      }
      if (payload === "") return true;
      try {
        const bytes = Uint8Array.from(atob(payload), (c) => c.charCodeAt(0));
        void navigator.clipboard?.writeText(new TextDecoder().decode(bytes)).catch(() => {});
      } catch {
        /* not base64: nothing to write */
      }
      return true;
    });
    // The fastest renderer the webview has; a machine without WebGL
    // silently keeps xterm's DOM renderer. Nothing to install either
    // way — this is a build-time library, not a user dependency.
    //
    // **A lost context falls back, then comes back** (T-23, the terminal
    // audit: losing the WebGL context left the terminal blank, still
    // taking keys, until the context was restored). The addon's own advice
    // (addon-webgl README; WebglRenderer.ts waits 3 s, then only fires
    // the event): dispose it on `onContextLoss`, which puts xterm's DOM
    // renderer back at once, and try WebGL again the next time the page
    // is shown. iOS drops contexts of pages in the background (WebKit
    // bug 261331); a Mac's GPU switch does too.
    const blink = blinksOnInput(term);
    const cursor = cursorFollows(term);
    let webgl: WebglAddon | null = null;
    const useWebgl = () => {
      if (webgl || stoppedWebgl) return;
      try {
        const addon = new WebglAddon();
        addon.onContextLoss(() => {
          if (webgl === addon) webgl = null;
          addon.dispose();
        });
        term.loadAddon(addon);
        webgl = addon;
        blink.quiet();
      } catch {
        /* DOM renderer stands */
      }
    };
    let stoppedWebgl = false;
    const shownAgain = () => {
      if (document.visibilityState === "visible") useWebgl();
    };
    useWebgl();
    document.addEventListener("visibilitychange", shownAgain);
    // …and at once: the addon itself waits 3 s for a restore before it
    // says so, three seconds of a blank terminal. The canvas's own event
    // is heard here first (capture), and the DOM renderer paints now.
    const lost = () => {
      const gone = webgl;
      webgl = null;
      gone?.dispose();
    };
    term.element?.addEventListener("webglcontextlost", lost, true);
    // The search paints its matches as decorations, which is the one
    // proposed API this file leans on besides the unicode addon.
    const search = new SearchAddon();
    term.loadAddon(search);
    searchRef.current = search;
    const found = search.onDidChangeResults((r) => setHits({ at: r.resultIndex, of: r.resultCount }));
    // Links underline on hover and open on ⌘-click, iTerm2's gesture.
    // A plain click on a URL in a terminal is more often the start of a
    // selection than a wish to leave, so the modifier is the intent.
    term.loadAddon(
      new WebLinksAddon((ev, uri) => {
        if (ev.metaKey || ev.ctrlKey) follow(uri, machine);
      }),
    );
    // ⌘F is caught before xterm sees it, so it neither reaches the pty
    // nor falls through to the page's own find. Everything else passes.
    term.attachCustomKeyEventHandler((ev) => {
      if (ev.type === "keydown" && asksToFind(ev)) {
        ev.preventDefault();
        setFinding(true);
        return false;
      }
      const editing = macEditing(ev);
      if (editing) {
        ev.preventDefault();
        term.input(editing, true);
        return false;
      }
      if (asksToClear(ev)) {
        ev.preventDefault();
        // The scrollback and the screen, the prompt line kept: what
        // iTerm2's Clear Buffer leaves. The program is not told.
        term.clear();
        return false;
      }
      return true;
    });
    const rung = term.onBell(() => {
      // The person's bell setting (roadmap 4.2): off is silent; under a
      // finger it is felt as well as seen.
      if (!termBell()) return;
      if (window.matchMedia("(pointer: coarse)").matches) bellHaptic().catch(() => {});
      setRang(true);
      window.clearTimeout(bellTimer.current);
      bellTimer.current = window.setTimeout(() => setRang(false), BELL_MS);
    });
    // Copy on select. The clipboard can refuse (no permission, no
    // secure context on a LAN address) and then ⌘C still works — this
    // is the shortcut, not the only way.
    const selected = term.onSelectionChange(() => {
      const s = term.getSelection();
      if (s) navigator.clipboard?.writeText(s).catch(() => {});
    });
    const shifted = keepShiftedCommits(term);
    const late = keepLateCommits(term);
    const composing = keepCompositionInView(term);
    const field = term.textarea;
    const composed = (e: CompositionEvent) => {
      if (!window.matchMedia("(pointer: coarse)").matches) return;
      setComposingText(e.type === "compositionend" ? null : e.data || null);
    };
    for (const k of ["compositionstart", "compositionupdate", "compositionend"]) {
      field?.addEventListener(k, composed as EventListener);
    }
    // The shell's marks on xterm's buffer (roadmap 2.2, 2.3), from the
    // first byte replayed on.
    const marks = trackMarks(term);
    // For the gates, as `__xterm` is.
    (box as HTMLDivElement & { __marks?: Marks }).__marks = marks;
    // **Paths and URLs open khor's own views** (roadmap 3.1, 3.2): under
    // a mouse a ⌘-click on an underlined path; under a finger a tap.
    const here = machine ?? "";
    const cwdAt = (line: number) => blockAt(marks, line)?.cwd ?? marks.cwd();
    const openPath = (t: { path: string; dir: boolean; line: number | null }) =>
      askOpenPath(here, t.path, t.dir, t.line);
    const linked = term.registerLinkProvider(pathLinks(term, here, cwdAt, openPath));
    const tapped = (row: number, col: number) => {
      const url = urlAt(term, row, col);
      if (url) return follow(url, machine);
      void pathsOn(term, here, cwdAt, row).then((found) => {
        const hit = found.find((f) => col + 1 >= f.x0 && col + 1 <= f.x1);
        if (hit) openPath(hit.target);
      });
    };
    const asks = tapsAskFirst(term, box, marks, tapped);
    const gestures = phoneGestures(term, box);
    fit.fit();
    termRef.current = term;
    keysRef.current = term.textarea ?? null;
    // **The page opened, or this tab was chosen: the terminal has the
    // keys** (2026-10-03, `lib/soft-keyboard`). Now, inside the press that
    // opened it, so a keyboard that was up on this page stays up; under a
    // finger without raising one otherwise.
    const finger = window.matchMedia("(pointer: coarse)").matches;
    if (term.textarea) holdFocus(term.textarea, finger);
    const shown = terminalShown();
    // For the gates: the buffer is the screen, and reading it beats
    // scraping renderer DOM that webgl does not produce. Smoke reads
    // `document.querySelector('[data-terminal]').__xterm`.
    (box as HTMLDivElement & { __xterm?: Terminal }).__xterm = term;

    let stopped = false;
    // **The first second, written down** (item 18 A: on the owner's iPhone
    // a fresh terminal would not scroll until the keyboard came up, and the
    // simulator never showed it). Each frame of the first 1.5 s: the grid,
    // the scroller's range and place; `__termTrace` for the gates and the
    // Web Inspector, the summary to the app's log (`face_note`).
    const born = performance.now();
    const traced: { t: number; what: string; cols: number; rows: number; range: number; top: number; lines: number }[] = [];
    (box as HTMLDivElement & { __termTrace?: typeof traced }).__termTrace = traced;
    const trace = (what: string) => {
      const sc = scrollerRef.current;
      traced.push({
        t: Math.round(performance.now() - born),
        what,
        cols: term.cols,
        rows: term.rows,
        range: sc ? Math.round(sc.scrollHeight - sc.clientHeight) : -1,
        top: sc ? Math.round(sc.scrollTop) : -1,
        lines: term.buffer.active.length,
      });
    };
    let tracing = window.requestAnimationFrame(function each() {
      if (stopped) return;
      trace("frame");
      if (performance.now() - born < 1500) tracing = window.requestAnimationFrame(each);
      else {
        const first = traced.find((x) => x.what === "fit");
        const scrollable = traced.find((x) => x.range > 0);
        faceNote(
          `term ${id}: fit at ${first?.t ?? "never"} ms (${first?.cols}x${first?.rows}), ` +
            `range > 0 at ${scrollable?.t ?? "never"} ms, last range ${traced[traced.length - 1]?.range}`,
        ).catch(() => {});
      }
    });
    let joined = false;
    // Output painted a whole frame at a time (item 18 B).
    const frames = framesWhole(term);
    let seq = 0;
    let timer = 0;
    let busy = false;
    // Which join the screen belongs to (`reopen`): a beat that was in
    // flight across a re-join writes nothing.
    let generation = 0;
    // **Replayed questions answer into nothing** (2026-10-03, the owner's
    // phone: 「11;rgb:…1;2c1$r2 q…」 typed at a prompt). History written into
    // xterm — the attach's ring dump, a reset beat, the catch-up after a
    // long gap — holds the queries programs asked back then, and xterm
    // answers them again. While such bytes are being parsed (until
    // `write`'s callback, which xterm calls once the parser has consumed
    // them), `onData` drops what has the shape of an answer
    // (`lib/term-replies`); keys still go. The xterm.js maintainers' own
    // advice is to replay before wiring the pty (discussions #4869); the
    // pane is wired first so that early keys are not lost, and this is
    // the same thing done at the one place where answers leave.
    let replaying = 0;
    const replay = (bytes: Uint8Array) => {
      replaying += 1;
      term.write(bytes, () => {
        replaying -= 1;
        // The history just landed: the scroller's range is the buffer's
        // now, not whenever xterm next scrolls (item 18 A).
        syncScroller.current();
      });
    };
    /** When the last beat came back: a beat after a long gap (the pane
        was hidden, the app was suspended) is a catch-up, not live. */
    let lastBeat = performance.now();
    const tick = async () => {
      const mine = generation;
      // **The place this ask was made from.** An ask that left before the
      // join set `seq` (a poll the page's own focus effect started while
      // the open was still reaching) comes back with everything since 0 —
      // the history the join had just replayed — and wrote it a second
      // time: a far session's whole history twice on the phone, where the
      // pushed wait holds such an ask open until the ring arrives.
      const from = seq;
      try {
        // **Pushed in the app while the pane is looked at** (round
        // NET-2): the node answers the moment bytes arrive, so an echo
        // no longer waits for the next beat. Elsewhere, the beat.
        const pushed = inApp && pace.current === LIVE_MS;
        const b = await (pushed ? termWait(id, seq, WAIT_MS) : termStream(id, seq));
        if (stopped || mine !== generation || from !== seq) return;
        const gap = performance.now() - lastBeat;
        lastBeat = performance.now();
        if (performance.now() - born < 1500 && (b.bytes || b.reset)) {
          trace(`chunk ${b.bytes.length} from ${seq} to ${b.seq}${b.reset ? " reset" : ""}${b.rejoining ? " rejoining" : ""}`);
        }
        // The feed trimmed past our place (a long time hidden): start
        // the screen over from what the node still holds, rather than
        // writing a stream whose head is missing.
        // A reset starts over: what was held belongs to the screen that
        // goes. A catch-up is history: what was held goes first.
        if (b.reset) {
          frames.drop();
          term.reset();
        }
        if (b.bytes) {
          if (b.reset || gap > CATCH_UP_MS) {
            frames.flush();
            replay(b64bytes(b.bytes));
          } else frames.paint(b64bytes(b.bytes));
        }
        seq = b.seq;
        if (b.gone) setGone(true);
        setRejoining(b.rejoining && !b.gone);
      } catch {
        /* the next beat asks again */
      }
    };
    // **Strictly one request in flight.** The cell-snapshot poll this
    // replaces was idempotent — two answers out of order repainted the
    // same screen. A byte stream is not: the same bytes written twice
    // are twice on the screen, so the loop awaits its tick and only
    // then schedules the next.
    const loop = async () => {
      // Nothing to ask before the join has said where the stream starts.
      if (stopped || busy || !joined) return;
      busy = true;
      try {
        await tick();
      } finally {
        busy = false;
      }
      if (!stopped) {
        window.clearTimeout(timer);
        timer = window.setTimeout(() => void loop(), inApp && pace.current === LIVE_MS ? 0 : pace.current);
      }
    };
    pollNow.current = () => {
      window.clearTimeout(timer);
      void loop();
    };

    let resizing: { dispose(): void } | null = null;
    /** Opens the session and takes its screen. **Again** is `reopen`
        (2026-10-03, the owner: 「term 得有个刷新键…因为有时候会卡住」):
        this face leaves first, the screen is dropped, and the session is
        joined from scratch — the screen and the history the host keeps
        come back as any new attacher gets them. A join that fails says
        why in the pane, as a first join does, and can be pressed again. */
    const join = (again: boolean): Promise<void> => {
      const mine = ++generation;
      window.clearTimeout(timer);
      if (again) setJoining(true);
      else {
        // The waiting clock (`Waiting`): a first join shows its wait with
        // 停止等待 / 重试; a re-join shows the circle's spinner instead.
        setWhy(null);
        setOpenedAt(Date.now());
      }
      const opened = again
        ? termLeave(id)
            .catch(() => {})
            .then(() => termOpen(id, term.cols, term.rows))
        : termOpen(id, term.cols, term.rows);
      return opened
        .then(() => termAttach(id))
        .then((a) => {
          if (stopped || mine !== generation) return;
          setOpening(false);
          setOpenedAt(null);
          setJoining(false);
          // An open that lands after 停止等待 is still an open: the
          // terminal is there, so the pane shows it.
          setWhy(null);
          setGone(false);
          if (again) {
            frames.drop();
            term.reset();
          }
          // Everything the feed holds — it began with the host's ring
          // dump, so the screen arrives *with its history*, the same
          // replay the host gives any attacher. Its old questions get no
          // answers (`replay`).
          trace(`attach ${a.bytes.length} to ${a.seq}`);
          if (a.bytes) replay(b64bytes(a.bytes));
          lastBeat = performance.now();
          seq = a.seq;
          joined = true;
          // Wired only now: a resize racing the open would size a pty
          // the open is about to size again.
          resizing ??= term.onResize(({ cols, rows }) => {
            termResize(id, cols, rows).catch(() => {});
          });
          if (again) {
            fit.fit();
            termResize(id, term.cols, term.rows).catch(() => {});
            term.scrollToBottom();
            // A reload keeps the terminal the focus, and the keyboard as
            // it was.
            if (term.textarea) holdFocus(term.textarea, finger);
          }
          void loop();
        })
        .catch((e: unknown) => {
          if (stopped || mine !== generation) return;
          setOpening(false);
          setOpenedAt(null);
          setJoining(false);
          setWhy(String((e as { message?: string })?.message ?? e));
        });
    };
    const reopenHere = () => join(true);
    reopenRef.current = reopenHere;
    // For the gates, as `__xterm` is.
    (box as HTMLDivElement & { __reopen?: () => Promise<void> }).__reopen = reopenHere;
    reopeners.set(id, reopenHere);

    setOpening(true);
    // **Opened at the size it will have** (T-27, the terminal audit: the
    // pty got 114×40 at open and 114×46 two seconds later — two SIGWINCHs,
    // the first wrong; item 15 A, the owner's phone and Mac: no scrolling
    // after the first entry until the keyboard or a window resize fitted
    // again). The first fit and the pty's first size wait until the box
    // has a size and the screen's entrance has finished (`settled`), then
    // for the frame after; a char size xterm could not measure at `open`
    // is measured now (`resize` to the same size does that, xterm.js #785).
    // Every later change of the box's size fits again (the observer below:
    // the keyboard, the band's padding, a window drag).
    const entering = new AbortController();
    let firstFrame = 0;
    void settled(box, entering.signal).then(() => {
      if (stopped) return;
      firstFrame = window.requestAnimationFrame(() => {
        if (stopped) return;
        term.resize(term.cols, term.rows);
        fit.fit();
        native.sync();
        trace("fit");
        void join(false);
      });
    });

    // Keys travel whole: xterm encodes every key, modifier, mouse
    // report and (bracketed, when the program asked) paste into the
    // bytes a terminal sends, and they go out the same `term_key` road
    // as ever. The immediate re-poll is what makes echo feel wired
    // rather than polled.
    //
    // The latches (`TermKeys`) take the next key xterm hands out — a
    // character typed on the soft keyboard becomes its control form (C →
    // 0x03) and, for Alt, gets an ESC before it — and let go; a mouse
    // report passes and leaves them waiting.
    // **One send at a time, in the order xterm handed them out** (T-3,
    // the terminal audit: on the web face 17 of 51 two-answer asks reached
    // the pty sentinel-first, and fast typing arrived swapped — one HTTP
    // POST per `onData`, nothing ordering them). Everything typed or
    // answered while a send is in flight is coalesced into the next one,
    // so a burst costs one round trip, not one per key. Tauri's own IPC
    // was already ordered; a browser's fetches are not.
    //
    // **And no frame over 64 KiB** (T-16): the host takes at most 1 MiB a
    // frame, and a paste bigger than that used to go as one. A chunk is
    // 16 Ki characters — 64 KiB at most once UTF-8 encoded — and the
    // chunks keep their order like everything else here.
    let outbox = "";
    let sending = false;
    const send = (bytes: string) => {
      outbox += bytes;
      if (sending) return;
      sending = true;
      void (async () => {
        while (outbox && !stopped) {
          // Never split a surrogate pair across two frames.
          let cut = Math.min(outbox.length, CHUNK_CHARS);
          if (cut < outbox.length && /[\ud800-\udbff]/.test(outbox[cut - 1])) cut -= 1;
          const next = outbox.slice(0, cut);
          outbox = outbox.slice(cut);
          await termKey(id, next).catch(() => {});
        }
        sending = false;
        pollNow.current();
      })();
    };
    // **A paste of more than one line asks first** (roadmap 4.3): how many
    // lines and the first, then 粘贴 or 取消 — iTerm2's and Windows
    // Terminal's multi-line paste warning, because a newline in a paste
    // is an Enter a person did not press. Every paste comes through here:
    // the system's (⌘V, iOS's callout) and the key row's (`term.paste`).
    // xterm still brackets what goes when the program asked for that.
    const pasteRaw = term.paste.bind(term);
    const guarded = (text: string) => {
      const lines = text.replace(/\r\n?/g, "\n").replace(/\n+$/, "").split("\n");
      if (lines.length < 2) return pasteRaw(text);
      setAsking({
        text: gui.paste_ask(lines.length),
        detail: lines[0],
        yes: gui.key_paste,
        no: gui.delete_keep,
        go: () => pasteRaw(text),
        stop: () => {},
      });
    };
    term.paste = guarded;
    const pasted = (e: ClipboardEvent) => {
      const text = e.clipboardData?.getData("text/plain") ?? "";
      if (!text) return;
      e.preventDefault();
      e.stopImmediatePropagation();
      guarded(text);
    };
    term.textarea?.addEventListener("paste", pasted, true);
    const data = term.onData((d) => {
      const reply = isReply(d);
      if (replaying > 0 && (reply || isFocusReport(d))) return;
      let out = d;
      const m = modsRef.current;
      // An answer to a live question is not a key: it neither takes a
      // latch nor waits for one.
      if (!reply && !MOUSE_REPORT.test(d)) blink.typed();
      if ((m.ctrl || m.alt) && !reply && !MOUSE_REPORT.test(d)) {
        out = d.length === 1 ? typed(d, m) : d;
        latch({ ctrl: false, alt: false });
      }
      send(out);
    });
    // **Command blocks** (roadmap 2.3): drawn from the marks; a block's
    // actions on a right-click or a long press on its prompt; ⌘↑ ⌘↓, the
    // menu's jumps and the key sheet's two chevrons move between prompts.
    const unblock = drawBlocks(term, marks);
    const actsFor = (b: Block) => {
      const command = commandOf(term, b);
      const acts: { name: string; label: string; run: () => void }[] = [];
      if (b.output) {
        acts.push({
          name: "copy-output",
          label: gui.block_copy_output,
          run: () => void navigator.clipboard?.writeText(outputOf(term, b)).catch(() => {}),
        });
      }
      if (command) {
        acts.push({
          name: "copy-command",
          label: gui.block_copy_command,
          run: () => void navigator.clipboard?.writeText(command).catch(() => {}),
        });
        // Again only at a prompt: into a running program it would be
        // keystrokes, not a command.
        const at = marks.current();
        if (!marks.running() && at?.input && !at.output) {
          acts.push({ name: "rerun", label: gui.block_rerun, run: () => send(`\x15${command}\r`) });
        }
      }
      if (b.output) {
        acts.push({
          name: "to-agent",
          label: gui.block_to_agent,
          run: () => {
            const status = b.exit === undefined || b.exit === null ? "" : `\n(exit ${b.exit})`;
            handOff(`${programOf(command) || "output"}.txt`, `$ ${command}${status}\n\n${outputOf(term, b)}\n`);
            askNewSession(machine ?? "", b.cwd ?? marks.cwd() ?? "");
          },
        });
      }
      return acts;
    };
    const showBlock = (row: number, x: number, y: number, promptOnly: boolean): boolean => {
      const b = blockAt(marks, row);
      if (!b || (promptOnly && !onPromptOf(b, row))) return false;
      const acts = actsFor(b);
      if (acts.length === 0) return false;
      const r = box.getBoundingClientRect();
      setBlockMenu({ x: x - r.left, y: y - r.top, acts });
      return true;
    };
    const rightClick = (e: MouseEvent) => {
      const at = cellAt(term, e.clientX, e.clientY);
      if (at && showBlock(at.row, e.clientX, e.clientY, false)) e.preventDefault();
    };
    box.addEventListener("contextmenu", rightClick);
    (box as HTMLDivElement & { __jump?: (dir: -1 | 1) => void }).__jump = (dir) => jumpToPrompt(term, marks, dir);
    const selects = pressSelects(term, box, (row, x, y) => showBlock(row, x, y, true));
    const dragged = fingerWheels(term, box);
    const native = scrollsNatively(term, scrollerRef.current!, roomRef.current!, mount);
    syncScroller.current = () => {
      native.sync();
      trace("replayed");
    };
    // The way-back-down pair. `viewportY < baseY` is "the reader is up
    // in history" in xterm's own coordinates — on the alternate screen
    // (a full-screen program) baseY stays 0 and neither of these can
    // fire, which is right: that screen has no history to be up in.
    const sync = () => {
      const b = term.buffer.active;
      const behind = b.viewportY < b.baseY;
      setBack(behind);
      if (!behind) setFreshBelow(false);
    };
    const scrolled = term.onScroll(sync);
    const wrote = term.onWriteParsed(() => {
      const b = term.buffer.active;
      if (b.viewportY < b.baseY) setFreshBelow(true);
      else sync();
    });

    // Coalesced to one fit per frame: dragging a window fires
    // ResizeObserver tens of times a second, and a resize is an IPC
    // round plus a PTY resize plus a reflow — per callback that is the
    // drag stutter itself. rAF folds a burst into at most one.
    let raf = 0;
    const observer = new ResizeObserver(() => {
      if (raf) return;
      raf = window.requestAnimationFrame(() => {
        raf = 0;
        fit.fit();
      });
    });
    observer.observe(box);
    // 外观's 终端字号 (the redesign, 2026-10-02): a new size reaches the
    // open terminal at once — xterm re-measures its cells and the fit
    // tells the far side the new rows and columns.
    const sized = onTermSize((px) => {
      term.options.fontSize = px;
      fit.fit();
    });
    // **A new font is measured and fitted in place** (F7.6): open
    // terminals take it without reopening.
    // Scrollback lines (item 17): a new depth reaches the open terminal at once;
    // a smaller one drops the oldest lines, and the scroller's range
    // follows.
    const deepened = onTermHistory((lines) => {
      term.options.scrollback = lines;
      native.sync();
    });
    const fonted = onTermFont((stack) => {
      term.options.fontFamily = stack;
      fit.fit();
    });
    // The symbols face may arrive after the first frame was drawn with a
    // fallback for its glyphs: once the page's fonts are in, redraw.
    void document.fonts?.ready.then(() => {
      if (stopped) return;
      (term as unknown as { clearTextureAtlas?: () => void }).clearTextureAtlas?.();
      term.refresh(0, term.rows - 1);
    });

    return () => {
      sized();
      deepened();
      fonted();
      stopped = true;
      frames.drop();
      window.clearTimeout(timer);
      pollNow.current = () => {};
      if (raf) window.cancelAnimationFrame(raf);
      observer.disconnect();
      data.dispose();
      term.textarea?.removeEventListener("paste", pasted, true);
      dragged();
      native.stop();
      selects();
      unblock();
      box.removeEventListener("contextmenu", rightClick);
      scrolled.dispose();
      wrote.dispose();
      found.dispose();
      rung.dispose();
      selected.dispose();
      shifted();
      late();
      composing();
      for (const k of ["compositionstart", "compositionupdate", "compositionend"]) {
        field?.removeEventListener(k, composed as EventListener);
      }
      asks();
      gestures();
      linked.dispose();
      marks.dispose();
      shown();
      window.clearTimeout(bellTimer.current);
      resizing?.dispose();
      reopenRef.current = null;
      if (reopeners.get(id) === reopenHere) reopeners.delete(id);
      entering.abort();
      window.cancelAnimationFrame(tracing);
      window.cancelAnimationFrame(firstFrame);
      osc52.dispose();
      cursor();
      stoppedWebgl = true;
      document.removeEventListener("visibilitychange", shownAgain);
      term.element?.removeEventListener("webglcontextlost", lost, true);
      term.dispose();
      termRef.current = null;
      searchRef.current = null;
      keysRef.current = null;
      termLeave(id).catch(() => {});
    };
  }, [id, attempt]);

  // Twenty polls a second is what a terminal costs to look at; nobody
  // looking is nobody paying. A shorter pace takes effect at once (a
  // screen ten seconds stale on return is the wrong thing to come back
  // to), a longer one waits out the beat already in flight.
  useEffect(() => {
    const next = hidden || !active ? HIDDEN_MS : LIVE_MS;
    const shorter = next < pace.current;
    pace.current = next;
    if (shorter) pollNow.current();
  }, [hidden, active]);
  // **Chosen again: the terminal has the keys, the screen is current**
  // (`lib/soft-keyboard`, item 11) — without leaving or re-attaching.
  useEffect(() => {
    if (!active) return;
    const field = termRef.current?.textarea;
    if (field) holdFocus(field, window.matchMedia("(pointer: coarse)").matches);
    pollNow.current();
  }, [active]);

  // The theme can flip while a terminal is up, and xterm holds colours,
  // not tokens — so the tokens are re-resolved when the OS scheme or
  // the app's own stamp changes. Watching both is the same three-state
  // rule the stylesheets follow (system unstamped, or data-theme).
  useEffect(() => {
    const box = boxRef.current;
    if (!box) return;
    const apply = () => {
      const term = termRef.current;
      if (term) term.options.theme = themeOf(box);
    };
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    media.addEventListener("change", apply);
    const observer = new MutationObserver(apply);
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme", "class"] });
    return () => {
      media.removeEventListener("change", apply);
      observer.disconnect();
    };
  }, []);

  // Files dropped on this pane land as shell-quoted paths at the cursor
  // (iTerm2's behaviour, 批④; `term_drop` quotes and pastes, and for a
  // remote session transfers the files first and pastes where they
  // landed).
  //
  // **Only in the app, and that is not a shortcut.** tauri intercepts
  // OS drops before the webview sees them, so the real paths arrive on
  // a tauri event and the HTML5 `drop` never fires with anything useful
  // — in the browser a dropped file is a `File` object with no path at
  // all. The position is checked against this pane's own box: the event
  // is the window's, not this element's.
  useLayoutEffect(() => {
    if (!inApp) return;
    let drop: (() => void) | undefined;
    let stopped = false;
    void (async () => {
      try {
        const { getCurrentWebview } = await import("@tauri-apps/api/webview");
        const un = await getCurrentWebview().onDragDropEvent((e) => {
          if (e.payload.type !== "drop") return;
          const box = boxRef.current?.getBoundingClientRect();
          const at = e.payload.position;
          if (!box || !at) return;
          const inside =
            at.x >= box.left && at.x <= box.right && at.y >= box.top && at.y <= box.bottom;
          if (!inside || e.payload.paths.length === 0) return;
          // A cross-machine drop takes as long as the files take; what
          // narrates the wait is the transfer's own row in the corner
          // strip (`use-status-bar`).
          setDropWhy(null);
          termDrop(id, e.payload.paths)
            .then(() => pollNow.current())
            .catch((err: unknown) => {
              setDropWhy(String((err as { message?: string })?.message ?? err));
            });
        });
        if (stopped) un();
        else drop = un;
      } catch {
        // No tauri here. Nothing to listen to and nothing to say.
      }
    })();
    return () => {
      stopped = true;
      drop?.();
    };
  }, [id]);

  // The search bar's three verbs. Typing looks incrementally (the
  // match under the caret grows with the word instead of jumping to
  // the next one), ⏎/⇧⏎ and the two chevrons step, ⎋ and the close
  // put it away and hand the keys back to the terminal.
  const look = (q: string) => {
    setQuery(q);
    const s = searchRef.current;
    if (!s) return;
    if (!q) {
      s.clearDecorations();
      setHits(null);
      return;
    }
    s.findNext(q, { incremental: true, decorations: found() });
  };
  const step = (dir: 1 | -1) => {
    const s = searchRef.current;
    if (!s || !query) return;
    if (dir > 0) s.findNext(query, { decorations: found() });
    else s.findPrevious(query, { decorations: found() });
  };
  const stopFinding = () => {
    searchRef.current?.clearDecorations();
    setFinding(false);
    setQuery("");
    setHits(null);
    termRef.current?.focus();
  };
  // The count beside the field: nothing until there is a word, the
  // lists' own 没有匹配的 for none, "3 / 12" otherwise. Past the
  // highlight limit the addon stops counting and says -1; the place
  // still means something, the total does not.
  // Visible from the start (F4.8; W18): 「0」 before a word is typed.
  const hitsText = !query || !hits ? "0" : hits.of === 0 ? gui.no_matches : hits.of < 0 ? `${hits.at + 1}` : `${hits.at + 1} / ${hits.of}`;

  return (
    <>
    {/* **The find bar is a row of its own above the screen** (the audit
        fix plan F4.8; W18): a glass capsule at the head controls' height,
        pushing the terminal down rather than lying over the lines it
        found — Safari's and Terminal's arrangement. A field, the count,
        the two steps, the way out. ⚠️ 墓碑: an opaque card over the
        screen's top-right corner. */}
    {finding && (
      <div data-term-find-row className="flex flex-none justify-end px-page pt-[var(--day-over)] pb-xxs">
        <div data-term-find className="glass flex h-glass-btn min-w-0 items-center gap-1 rounded-full ps-3 pe-glass-pad">
          <input
            data-term-find-input
            autoFocus
            value={query}
            placeholder={gui.search_terminal}
            onChange={(e) => look(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Escape") {
                e.preventDefault();
                stopFinding();
              } else if (e.key === "Enter") {
                e.preventDefault();
                step(e.shiftKey ? -1 : 1);
              }
            }}
            className="min-w-0 w-48 bg-transparent text-callout outline-none placeholder:text-tertiary"
          />
          <span data-term-find-hits className="min-w-12 text-right text-caption tabular-nums text-muted-foreground">
            {hitsText}
          </span>
          <GlassItem data-term-find-prev aria-label={gui.match_prev} onClick={() => step(-1)}>
            <IconChevronUp className="size-affordance" />
          </GlassItem>
          <GlassItem data-term-find-next aria-label={gui.match_next} onClick={() => step(1)}>
            <IconChevronDown className="size-affordance" />
          </GlassItem>
          <GlassItem data-term-find-close aria-label={gui.close} onClick={stopFinding}>
            <IconClose className="size-affordance" />
          </GlassItem>
        </div>
      </div>
    )}
    <div
      data-terminal
      data-term-bell={rang}
      ref={boxRef}
      // The terminal owns its own two colours; they follow the theme so
      // a plain shell is the app's paper, not a black box. **The paper is
      // the board's** (2026-09-30, 板是纸底是框): this was `--background`,
      // the ground *around* the board, so the open tab above it was the
      // board's grey and the screen under it the ground's black — two
      // pieces where the owner saw one session. It is `--card` now, the
      // same surface the tab joins (`check-palette` reads the box, what
      // xterm was handed and the board, and wants one colour). The font
      // tokens are read off this element by `fontOf` — one source, no
      // copy in JS.
      className={cn(
        "relative min-h-0 flex-1 overflow-hidden bg-[var(--term-bg)] p-1 text-[var(--term-fg)]",
        // The bell: an inset line the terminal's own yellow, gone in
        // `BELL_MS`. Inset so the flash never changes the box's size —
        // a resize on every bell would be a reflow on every bell.
        rang && "shadow-[inset_0_0_0_2px_var(--term-bell)]",
      )}
      style={
        {
          "--term-bg": "var(--card)",
          "--term-fg": "var(--card-foreground)",
          "--term-bell": ANSI16[3],
          fontFamily: "var(--font-term)",
          fontSize: "var(--text-term)",
          lineHeight: "var(--text-term--line-height)",
        } as React.CSSProperties
      }
    >
      {/* xterm's mount. Everything inside is the renderer's own — the
          gates read the buffer through `__xterm`, not this subtree. */}
      <div data-term-scroller ref={scrollerRef} className="h-full w-full overflow-x-hidden overflow-y-auto">
        <div data-term-stick className="sticky top-0 h-full w-full">
          <div
            data-term-screen
            ref={mountRef}
            // Held back while the pipe is being asked for again: the last
            // screen that arrived, not a live one.
            className={cn("h-full w-full transition-opacity", rejoining && "opacity-50")}
          />
        </div>
        <div data-term-room ref={roomRef} aria-hidden />
      </div>
      {asking && (
        <div
          data-term-ask
          role="alertdialog"
          className="glass absolute inset-x-2 bottom-2 z-20 flex flex-col gap-in rounded-lg p-3 font-sans"
        >
          <span className="text-body">{asking.text}</span>
          {asking.detail && (
            <span data-term-ask-detail className="truncate text-callout text-muted-foreground" style={{ fontFamily: "var(--font-term)" }}>
              {asking.detail}
            </span>
          )}
          <span className="flex justify-end gap-in">
            <button
              type="button"
              data-term-ask-no
              onMouseDown={(e) => e.preventDefault()}
              onClick={() => {
                asking.stop();
                setAsking(null);
              }}
              className="min-h-touch rounded-md px-3 text-body hover:bg-fill"
            >
              {asking.no}
            </button>
            <button
              type="button"
              data-term-ask-yes
              onMouseDown={(e) => e.preventDefault()}
              onClick={() => {
                asking.go();
                setAsking(null);
              }}
              className="min-h-touch rounded-md bg-fill-strong px-3 text-body"
            >
              {asking.yes}
            </button>
          </span>
        </div>
      )}
      {composingText && (
        <div
          data-term-composing
          aria-hidden
          className="glass pointer-events-none absolute inset-x-2 bottom-2 z-10 truncate rounded-md px-3 py-1 text-[length:var(--text-term)]"
          style={{ fontFamily: "var(--font-term)" }}
        >
          {composingText}
        </div>
      )}
      {blockMenu && (
        <div
          data-term-block-menu
          role="menu"
          className="glass absolute z-20 flex min-w-44 flex-col rounded-lg p-1"
          style={{
            left: Math.max(8, Math.min(blockMenu.x, (boxRef.current?.clientWidth ?? 0) - 192)),
            top: Math.max(8, Math.min(blockMenu.y, (boxRef.current?.clientHeight ?? 0) - 48 * blockMenu.acts.length)),
          }}
          onPointerDown={(e) => e.stopPropagation()}
        >
          {blockMenu.acts.map((a) => (
            <button
              key={a.name}
              type="button"
              role="menuitem"
              data-term-block-act={a.name}
              onMouseDown={(e) => e.preventDefault()}
              onClick={() => {
                setBlockMenu(null);
                a.run();
              }}
              className="min-h-touch rounded-md px-3 text-left text-body hover:bg-fill"
            >
              {a.label}
            </button>
          ))}
        </div>
      )}
      {/* The way back down, and the only sign that anything arrived
          while the reader was up in history. Two sentences on one
          control: "something came in" and "you are simply not at the
          bottom" are different reasons to press it, and one word for
          both would make the arrival silent. Absent at the bottom,
          which is what lets it go to zero. */}
      {back && (
        <button
          type="button"
          data-term-bottom
          data-fresh={freshBelow}
          onClick={() => {
            termRef.current?.scrollToBottom();
            termRef.current?.focus();
          }}
          // The same floating language as the find bar above it —
          // it was a pill with a shadow of its own (2026-09-28).
          className="absolute bottom-2 left-1/2 z-10 -translate-x-1/2 rounded-lg border bg-popover px-3 py-1 text-caption shadow-(--shadow-float)"
        >
          {freshBelow ? (
            <span className="flex items-center gap-xxs">
              <IconArrow to="down" className="size-3.5" />
              {gui.term_new_output}
            </span>
          ) : (
            gui.to_bottom
          )}
        </button>
      )}
      {/* **Reconnecting, said over the screen it holds** (round
          NET-FIX): the far pipe ended and is being asked for again, so
          the screen under this is the last one that arrived — held back
          in ink, and this pill on top in the pane's floating language
          (the way-back-down control's), so it never sits on the
          program's own text the way a bare caption would. */}
      {rejoining && !gone && !why && (
        <div
          data-term-rejoining
          role="status"
          className="absolute top-2 left-1/2 z-10 flex -translate-x-1/2 items-center gap-xxs rounded-lg border bg-popover px-3 py-1 font-sans text-caption text-muted-foreground shadow-(--shadow-float)"
        >
          <IconReload className="size-3.5 flex-none animate-spin motion-reduce:animate-none" />
          {machine ? gui.term_rejoining(machine) : gui.term_rejoining_here}
        </div>
      )}
      {opening && !why && openedAt !== null && (
        <div data-term-opening className="absolute inset-0 z-10 bg-card font-sans">
          <Waiting
            since={openedAt}
            machine={machine}
            onCancel={() => {
              setOpening(false);
              setOpenedAt(null);
              setWhy(machine ? gui.waiting_stopped(machine) : gui.waiting_stopped_here);
            }}
            onRetry={() => setAttempt((n) => n + 1)}
          />
        </div>
      )}
      {why ? (
        // Centred, not a corner label: there is no screen behind it to
        // annotate, and this sentence is the whole of what the pane has
        // to say — with 重试, which opens it again from the start.
        <div data-term-why className="absolute inset-0 z-10 bg-card font-sans">
          <RegionState state="error" message={why} onRetry={() => setAttempt((n) => n + 1)} />
        </div>
      ) : (
        <div
          className={cn(
            "pointer-events-none absolute right-2 z-10 flex flex-col items-end gap-1 text-caption",
            // Under the search bar while that is up, so the two never
            // overprint each other — and under the day while that floats
            // over the top of the pane (`--day-over`, `DetailPane`).
            finding ? "top-[var(--spacing)]" : "top-[calc(var(--spacing)+var(--day-over))]",
          )}
        >
          {gone && (
            <div data-term-over className="text-muted-foreground">
              {gui.term_over}
            </div>
          )}

          {/* Held to the pane's own width so a sentence wraps here
              instead of pushing the screen sideways. */}
          {dropWhy && (
            <div data-term-drop-why className="max-w-72 text-right text-destructive">
              {dropWhy}
            </div>
          )}
        </div>
      )}
    </div>
    {/* **Under a finger, the keys a soft keyboard lacks and the switch
        for the keyboard** (`TermKeys`, 2026-10-02). On the narrow face
        they go to the session bar (row 2, and row 1's right end); where
        there is no bar — the wide face on an iPad — the row stands here,
        under the screen, and the switch is not drawn. */}
    {/* **Reload, right of ‹** (2026-10-03, the owner: 「term 得有个刷新键，
        可以在返回键右侧，点击后刷新这个终端，因为有时候会卡住」): joins the
        session again from scratch (`reopen`). On the wide face it stands
        at the end of the tab strip, among the page's verbs. While it
        joins the glyph turns and nothing else changes; it stays pressable. */}
    {active && (
    <BarSeat at="lead">
      <GlassCircle
        data-term-reload
        data-joining={joining || undefined}
        aria-label={gui.term_reload}
        onMouseDown={(e) => e.preventDefault()}
        onClick={() => void reopenRef.current?.()}
      >
        <IconReload className={cn(joining && "animate-spin motion-reduce:animate-none")} />
      </GlassCircle>
    </BarSeat>
    )}
    {coarse && active && (
      <>
        {/* **The keyboard circle only in 点击按钮** (round UX-3 3a): in 自动
            and 点击屏幕 a tap on the terminal raises the keyboard, and a
            button for it is a second door; a swipe down lowers it in every
            mode (this pane's own, `terminalKeyboard`). The mode is
            `lib/term-tap`'s (`useTermWake`). */}
        {wake === "button" && (
          <BarSeat at="trail">
            <KeyboardSwitch field={keysRef} />
          </BarSeat>
        )}
        <BarSeat at="under">
          <TermKeys term={termRef} mods={mods} onMods={latch} />
        </BarSeat>
      </>
    )}
    </>
  );
}
