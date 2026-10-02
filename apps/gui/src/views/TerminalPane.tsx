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
// of it — grid metrics, wide chars (unicode11), IME composition, every
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
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { Unicode11Addon } from "@xterm/addon-unicode11";
import { SearchAddon } from "@xterm/addon-search";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { WebglAddon } from "@xterm/addon-webgl";
import "@xterm/xterm/css/xterm.css";

import { inApp, openLink, termAttach, termDrop, termKey, termLeave, termOpen, termResize, termStream } from "@/api";
import { GlassItem } from "@/components/Glass";
import { IconChevronDown, IconChevronUp, IconClose } from "@/components/icons";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { HIDDEN_MS, useHidden } from "@/hooks/use-hidden";
import { useKeysTo } from "@/hooks/use-keys-to";
import { useCoarse } from "@/hooks/use-coarse";
import { BarSeat } from "@/components/SessionBar";
import { onTermSize } from "@/lib/type-size";
import { onTermFont } from "@/lib/term-font";
import { KeyboardSwitch, TermKeys, ctrlOf } from "@/views/TermKeys";

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

/** The scrollback xterm keeps, matched to the parser's own
    (`khor_gui_core::term::SCROLLBACK`) so the two authorities never
    disagree about how far back "back" goes. */
const SCROLLBACK = 2000;

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
function asksToFind(ev: KeyboardEvent): boolean {
  if (ev.key.toLowerCase() !== "f" || ev.altKey) return false;
  return IS_MAC ? ev.metaKey && !ev.ctrlKey : ev.ctrlKey && ev.shiftKey;
}

/** Where a link goes. In the app, through khor (`open_link`: the
    scheme whitelist is the same one the conversation pane's links go
    through). In a browser, the browser's own new tab — `open` on the
    serving machine would open it on the wrong screen when the face is
    being read from a phone. */
function follow(uri: string) {
  if (inApp) openLink(uri).catch(() => {});
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
    replaced the hand-measured `wideFit` ratio. */
function fontOf(el: HTMLElement): { family: string; size: number; line: number } {
  const cs = getComputedStyle(el);
  const size = Number.parseFloat(cs.fontSize) || 13;
  const lineHeight = Number.parseFloat(cs.lineHeight);
  return {
    family: cs.fontFamily,
    size,
    // xterm takes a multiplier; the token is a length.
    line: Number.isFinite(lineHeight) && size > 0 ? lineHeight / size : 1.2,
  };
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

/** A report xterm writes for the mouse (SGR `ESC [ <`, X10 `ESC [ M`) —
    not a key, so a latched Ctrl lets it through and keeps waiting. */
const MOUSE_REPORT = /^\x1b\[(<|M)/;

/**
 * **A finger's drag scrolls, whatever the program asked for** (2026-10-02,
 * iOS 第三步 A). xterm scrolls its own history under a drag, but a
 * full-screen program has none: the alternate screen keeps no scrollback,
 * and a program that took the mouse (khor turns tmux's on for its own
 * grouped session, `Tmux::grouped_attach_argv`) wants wheel reports, not a
 * moved viewport. On a desktop the wheel does that — xterm turns it into
 * the report, or into arrow keys on the alternate screen. A finger has no
 * wheel, so in exactly those two cases its vertical drag is turned into
 * the wheel events a trackpad would have sent, at the point under the
 * finger, and xterm encodes them the way it encodes every other wheel.
 * Elsewhere the drag is left to xterm's own touch scrolling.
 */
function dragsAsWheel(term: Terminal, box: HTMLElement): () => void {
  let lastY: number | null = null;
  let target: EventTarget | null = null;
  const wants = () => term.modes.mouseTrackingMode !== "none" || term.buffer.active.type === "alternate";
  const start = (e: TouchEvent) => {
    lastY = e.touches.length === 1 ? e.touches[0].clientY : null;
    target = e.target;
  };
  const move = (e: TouchEvent) => {
    if (lastY === null || e.touches.length !== 1 || !wants()) return;
    const t = e.touches[0];
    const dy = t.clientY - lastY;
    lastY = t.clientY;
    // Taken either way: the page has nothing to scroll, and xterm's own
    // drag would move a viewport that has no history to show.
    e.preventDefault();
    e.stopPropagation();
    if (dy === 0) return;
    // Natural scrolling: a finger moving up brings later lines, which is
    // a wheel turned down.
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

export function TerminalPane({ id }: { id: string }) {
  const boxRef = useRef<HTMLDivElement>(null);
  const mountRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<Terminal | null>(null);
  const [gone, setGone] = useState(false);
  // Why the terminal never opened, in khor's own words. **Not the same
  // fact as `gone`**: `gone` means the host this pane was watching
  // ended, and 这个终端结束了 is true of it; a failed open means there
  // never was a host to watch, and saying 结束了 about a tmux session
  // sitting right there sends a person looking for a session that
  // ended. Every refusal khor makes here is already a sentence written
  // for a person, and throwing it away is what once flattened all of
  // them into one wrong word.
  const [why, setWhy] = useState<string | null>(null);
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
  useKeysTo(keysRef, true);
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
  const bellTimer = useRef(0);
  // **Ctrl, latched from the key row** (`TermKeys`, 2026-10-02): it waits
  // for the next key from either the row or the soft keyboard. A ref as
  // well as state because the `onData` handler below reads it, and that
  // handler is wired once per terminal.
  const coarse = useCoarse();
  const [ctrl, setCtrlShown] = useState(false);
  const ctrlRef = useRef(false);
  const latch = (on: boolean) => {
    ctrlRef.current = on;
    setCtrlShown(on);
  };

  useLayoutEffect(() => {
    const box = boxRef.current;
    const mount = mountRef.current;
    if (!box || !mount) return;
    const font = fontOf(box);
    const term = new Terminal({
      // unicode11 is a proposed API; without this flag the addon throws.
      allowProposedApi: true,
      scrollback: SCROLLBACK,
      fontFamily: font.family,
      fontSize: font.size,
      lineHeight: font.line,
      theme: themeOf(box),
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
    // renderer that owns its glyph atlas simply knows.
    term.loadAddon(new Unicode11Addon());
    term.unicode.activeVersion = "11";
    term.open(mount);
    // The fastest renderer the webview has; a machine without WebGL
    // silently keeps xterm's DOM renderer. Nothing to install either
    // way — this is a build-time library, not a user dependency.
    try {
      term.loadAddon(new WebglAddon());
    } catch {
      /* DOM renderer stands */
    }
    // The search paints its matches as decorations, which is the one
    // proposed API this file leans on besides unicode11.
    const search = new SearchAddon();
    term.loadAddon(search);
    searchRef.current = search;
    const found = search.onDidChangeResults((r) => setHits({ at: r.resultIndex, of: r.resultCount }));
    // Links underline on hover and open on ⌘-click, iTerm2's gesture.
    // A plain click on a URL in a terminal is more often the start of a
    // selection than a wish to leave, so the modifier is the intent.
    term.loadAddon(
      new WebLinksAddon((ev, uri) => {
        if (ev.metaKey || ev.ctrlKey) follow(uri);
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
      return true;
    });
    const rung = term.onBell(() => {
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
    fit.fit();
    termRef.current = term;
    keysRef.current = term.textarea ?? null;
    // For the gates: the buffer is the screen, and reading it beats
    // scraping renderer DOM that webgl does not produce. Smoke reads
    // `document.querySelector('[data-terminal]').__xterm`.
    (box as HTMLDivElement & { __xterm?: Terminal }).__xterm = term;

    let stopped = false;
    let seq = 0;
    let timer = 0;
    let busy = false;
    const tick = async () => {
      try {
        const b = await termStream(id, seq);
        if (stopped) return;
        // The feed trimmed past our place (a long time hidden): start
        // the screen over from what the node still holds, rather than
        // writing a stream whose head is missing.
        if (b.reset) term.reset();
        if (b.bytes) term.write(b64bytes(b.bytes));
        seq = b.seq;
        if (b.gone) setGone(true);
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
      if (stopped || busy) return;
      busy = true;
      try {
        await tick();
      } finally {
        busy = false;
      }
      if (!stopped) {
        window.clearTimeout(timer);
        timer = window.setTimeout(() => void loop(), pace.current);
      }
    };
    pollNow.current = () => {
      window.clearTimeout(timer);
      void loop();
    };

    termOpen(id, term.cols, term.rows)
      .then(() => termAttach(id))
      .then((a) => {
        if (stopped) return;
        // Everything the feed holds — it began with the host's ring
        // dump, so the screen arrives *with its history*, the same
        // replay the host gives any attacher.
        if (a.bytes) term.write(b64bytes(a.bytes));
        seq = a.seq;
        // Wired only now: a resize racing the open would size a pty
        // the open is about to size again.
        term.onResize(({ cols, rows }) => {
          termResize(id, cols, rows).catch(() => {});
        });
        void loop();
      })
      .catch((e: unknown) => {
        setWhy(String((e as { message?: string })?.message ?? e));
      });

    // Keys travel whole: xterm encodes every key, modifier, mouse
    // report and (bracketed, when the program asked) paste into the
    // bytes a terminal sends, and they go out the same `term_key` road
    // as ever. The immediate re-poll is what makes echo feel wired
    // rather than polled.
    //
    // A latched Ctrl (`TermKeys`) takes the next key xterm hands out — a
    // character typed on the soft keyboard becomes its control form (C →
    // 0x03) — and lets go; a mouse report passes and leaves it waiting.
    const data = term.onData((d) => {
      let out = d;
      if (ctrlRef.current && !MOUSE_REPORT.test(d)) {
        out = ctrlOf(d) ?? d;
        ctrlRef.current = false;
        setCtrlShown(false);
      }
      termKey(id, out)
        .then(() => pollNow.current())
        .catch(() => {});
    });
    const dragged = dragsAsWheel(term, box);
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
      fonted();
      stopped = true;
      window.clearTimeout(timer);
      pollNow.current = () => {};
      if (raf) window.cancelAnimationFrame(raf);
      observer.disconnect();
      data.dispose();
      dragged();
      scrolled.dispose();
      wrote.dispose();
      found.dispose();
      rung.dispose();
      selected.dispose();
      shifted();
      window.clearTimeout(bellTimer.current);
      term.dispose();
      termRef.current = null;
      searchRef.current = null;
      keysRef.current = null;
      termLeave(id).catch(() => {});
    };
  }, [id]);

  // Twenty polls a second is what a terminal costs to look at; nobody
  // looking is nobody paying. A shorter pace takes effect at once (a
  // screen ten seconds stale on return is the wrong thing to come back
  // to), a longer one waits out the beat already in flight.
  useEffect(() => {
    const next = hidden ? HIDDEN_MS : LIVE_MS;
    const shorter = next < pace.current;
    pace.current = next;
    if (shorter) pollNow.current();
  }, [hidden]);

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
      <div data-term-screen ref={mountRef} className="h-full w-full" />
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
          {freshBelow ? gui.new_below : gui.to_bottom}
        </button>
      )}
      {why ? (
        // Centred, not a corner label: there is no screen behind it to
        // annotate, and this sentence is the whole of what the pane has
        // to say.
        <div
          data-term-why
          className="absolute inset-0 grid place-items-center px-6 text-center text-callout text-muted-foreground"
        >
          {why}
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
    {coarse && (
      <>
        <BarSeat at="trail">
          <KeyboardSwitch field={keysRef} />
        </BarSeat>
        <BarSeat at="under">
          <TermKeys term={termRef} ctrl={ctrl} onCtrl={latch} />
        </BarSeat>
      </>
    )}
    </>
  );
}
