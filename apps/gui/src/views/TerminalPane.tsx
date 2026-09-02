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
//   - **⌘F is the terminal's own search** (`SearchAddon` + the bar in
//     this file), not the page's — a page find would search the DOM of
//     a webgl canvas and find nothing;
//   - **a bell is a flash** on the frame, and **the title a program
//     sets** (OSC 0/2) is shown in the corner: the shell said it, and a
//     title that goes nowhere is a program talking to no one.
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { Unicode11Addon } from "@xterm/addon-unicode11";
import { SearchAddon } from "@xterm/addon-search";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { WebglAddon } from "@xterm/addon-webgl";
import "@xterm/xterm/css/xterm.css";

import { inApp, openLink, termAttach, termDrop, termKey, termLeave, termOpen, termResize, termStream } from "@/api";
import { IconChevronDown, IconChevronUp, IconClose } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { HIDDEN_MS, useHidden } from "@/hooks/use-hidden";
import { useKeysTo } from "@/hooks/use-keys-to";

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

/** Where the search paints what it found: the terminal's own yellow,
    thinned for every match and solid under the one the search stands
    on. Terminal colours rather than app tokens, the same rule as
    `ANSI16` — these sit on top of program output, whose colours are
    the terminal's. */
const FOUND = {
  matchBackground: "#c4a00055",
  matchOverviewRuler: "#c4a000",
  activeMatchBackground: "#c4a000bb",
  activeMatchColorOverviewRuler: "#c4a000",
};

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
 * tokens (`--background`/`--foreground` through `--term-bg`/`--term-fg`)
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
  // What the program running inside called itself (OSC 0/2). Empty
  // until something sets one; a shell that never does says nothing.
  const [title, setTitle] = useState("");
  // The bell's flash, and the timer that ends it.
  const [rang, setRang] = useState(false);
  const bellTimer = useRef(0);

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
    const titled = term.onTitleChange(setTitle);
    // Copy on select. The clipboard can refuse (no permission, no
    // secure context on a LAN address) and then ⌘C still works — this
    // is the shortcut, not the only way.
    const selected = term.onSelectionChange(() => {
      const s = term.getSelection();
      if (s) navigator.clipboard?.writeText(s).catch(() => {});
    });
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
    const data = term.onData((d) => {
      termKey(id, d)
        .then(() => pollNow.current())
        .catch(() => {});
    });
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

    return () => {
      stopped = true;
      window.clearTimeout(timer);
      pollNow.current = () => {};
      if (raf) window.cancelAnimationFrame(raf);
      observer.disconnect();
      data.dispose();
      scrolled.dispose();
      wrote.dispose();
      found.dispose();
      rung.dispose();
      titled.dispose();
      selected.dispose();
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
    s.findNext(q, { incremental: true, decorations: FOUND });
  };
  const step = (dir: 1 | -1) => {
    const s = searchRef.current;
    if (!s || !query) return;
    if (dir > 0) s.findNext(query, { decorations: FOUND });
    else s.findPrevious(query, { decorations: FOUND });
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
  const hitsText = !query || !hits ? "" : hits.of === 0 ? gui.no_matches : hits.of < 0 ? `${hits.at + 1}` : `${hits.at + 1} / ${hits.of}`;

  return (
    <div
      data-terminal
      data-term-bell={rang}
      ref={boxRef}
      // The terminal owns its own two colours; they follow the theme so
      // a plain shell is the app's paper, not a black box. The font
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
          "--term-bg": "var(--background)",
          "--term-fg": "var(--foreground)",
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
      {/* The search bar, over the top-right corner of the screen. A
          field, the count, the two steps, the way out — the shape every
          find bar has, because a person arriving here has used one. */}
      {finding && (
        <div
          data-term-find
          className="absolute right-2 top-1 z-20 flex items-center gap-1 rounded-md border bg-popover p-1 shadow-md"
        >
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
            className="h-ctl-sm w-48 bg-transparent px-2 text-sm outline-none"
          />
          <span data-term-find-hits className="min-w-12 text-right text-xs tabular-nums text-muted-foreground">
            {hitsText}
          </span>
          <Button size="icon" variant="ghost" data-term-find-prev aria-label={gui.match_prev} onClick={() => step(-1)}>
            <IconChevronUp />
          </Button>
          <Button size="icon" variant="ghost" data-term-find-next aria-label={gui.match_next} onClick={() => step(1)}>
            <IconChevronDown />
          </Button>
          <Button size="icon" variant="ghost" data-term-find-close aria-label={gui.close} onClick={stopFinding}>
            <IconClose />
          </Button>
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
          className="absolute bottom-2 left-1/2 z-10 -translate-x-1/2 rounded-full border bg-popover px-3 py-1 text-xs shadow-md"
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
          className="absolute inset-0 grid place-items-center px-6 text-center text-sm text-muted-foreground"
        >
          {why}
        </div>
      ) : (
        <div
          className={cn(
            "pointer-events-none absolute right-2 z-10 flex flex-col items-end gap-1 text-xs",
            // Under the search bar while that is up, so the two never
            // overprint each other.
            finding ? "top-12" : "top-1",
          )}
        >
          {/* What the program calls itself. Muted and on a wash so it
              reads over output without hiding it; `pointer-events-none`
              from the column, so a click lands on the screen beneath. */}
          {title && (
            <div data-term-title className="max-w-72 truncate rounded bg-background/70 px-1 text-muted-foreground">
              {title}
            </div>
          )}
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
  );
}
