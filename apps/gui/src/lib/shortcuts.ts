// **The keyboard, in one place** (2026-09-28:「像一个 Mac app，不像网页」).
//
// Until this the app had two shortcuts, each wired where it was spent —
// ⌘K in `SideHead`, ⌘L in `use-lock` — and a third, ⌘W, inside the
// workspace alone. Mac people learn an app's keys from its menu bar and
// expect the ones every app has: ⌘1–⌘5 walk the rail, ⌘, is the settings,
// ⌘N is 「new」, ⌘W closes the front tab, ⌃⇥ steps tabs. So the map is
// here, the menu bar (`src-tauri/src/menu.rs`) names the same acts, and
// both roads end in `run` below.
//
// # The map
//
// | keys                  | act          | what it does                                    |
// |-----------------------|--------------|-------------------------------------------------|
// | ⌘1 ⌘2 ⌘3 ⌘4 ⌘5        | landing-1…5  | the strip, in its order (外观 → 导航)              |
// | ⌘,                    | settings     | 「…」 › 通用                                      |
// | ⌘L                    | lock         | locks the face (only with a PIN set)            |
// | ⌘K                    | find         | the first row's box, over everything            |
// | ⌘N                    | new          | the landing's 「+」, the same act                 |
// | (menu only)           | import       | 导入会话…                                         |
// | ⌘W                    | close-tab    | the 「…」 page if one is up, else the front tab   |
// | ⌃⇥  ⇧⌘]               | next-tab     | one tab right, wrapping                         |
// | ⌃⇧⇥ ⇧⌘[               | prev-tab     | one tab left, wrapping                          |
// | ⌥⌘S                   | focus-list   | the list's one Tab stop, from anywhere          |
// | ⌃⌘S (Mac only)        | toggle-list  | folds the list, or brings it back (wide)        |
//
// ⌘ is Ctrl off a Mac (Windows, Linux, and a browser there). **On a Mac
// Ctrl is not ⌘**: ⌃W, ⌃K and ⌃L are a text field's and a terminal's own
// keys (delete a word, kill to the end, clear the screen) — ⌃L used to
// lock the app from inside a shell.
//
// # ⌘W with nothing to close does nothing
//
// The front 「…」 page first, then the front tab (a session's, a
// workspace's file, a browsing page). With neither there is no act, and
// on the Mac the menu's 关闭标签页 is greyed. **It is not 「close the
// window」**, which is what the default menu bound it to: closing khor's
// one window quits khor, and khor is the node — the mesh loses this
// machine because a tab key was pressed once too often. ⇧⌘W is the
// window's own close, the red button's road through the draft guard.
//
// # Where keys are not taken
//
// - **Inside a text field they are**, the way Safari's ⌘W works in its
//   address bar: every key here carries ⌘ (or ⌃⇥), and none of them is
//   text. ⌘C / ⌘V / ⌘A / ⌘Z are the Edit menu's, never listed here.
// - **The terminal gives up ⌃⇥** — Terminal.app and iTerm step their tabs
//   on it, and a shell cannot tell ⌃⇥ from ⇥ anyway. Every other key a
//   terminal needs has no ⌘ on it.
// - **Nothing under a dialog**, except ⌘L: a dialog is a
//   question the window is waiting on (Esc, or its buttons, answer it).
// - **Nothing while locked.**
// - **The web face leaves the browser its keys**: ⌘W ⇧⌘W ⌘N ⌘1–⌘5 ⌃⇥ ⌃⇧⇥
//   ⇧⌘] ⇧⌘[ close, open and walk the browser's own tabs, and a page that
//   took them would take them from the person. There is no alternate —
//   every free spelling is some browser's or the system's (⌥⌘←→ Chrome's
//   tabs, ⌃PgDn Firefox's, ⌥1 a character on a Mac, ⌃1 a desktop space):
//   the rail and the tab strip are one press away. ⌘K ⌘L ⌘, stay.
//
// # One press, one act
//
// On the Mac a key reaches the page **first**, and WKWebView re-sends it
// to the menu bar only when the page did not `preventDefault` it. So the
// page prevents every key it runs, and the menu's accelerator fires only
// where the page never saw the key — a browsing tab's own webview, an
// editor window. A menu item picked with the pointer arrives here as the
// same act (`shortcut` from `menu.rs`). `npm run native` counts presses
// through the real WKWebView to hold this; `npm run face` §11 holds the
// page's half.
//
// # Sidebar lists
//
// ↑ ↓ move the keyboard's focus between a list's rows (and its group
// headings and the fold at its foot), Home / End to the ends; ⏎ / Space
// open the focused row — they are buttons. Moving never opens: a tab per
// arrow press would spend the tab limit on rows somebody walked past
// (VS Code's lists, not Finder's). `walkList` is that, for any list.
import { useEffect, useRef, useSyncExternalStore, type KeyboardEvent as ReactKeyboardEvent } from "react";

import { hear, inApp, menuState, onWebFace, trafficLights } from "@/api";

export type Act =
  | "landing-1"
  | "landing-2"
  | "landing-3"
  | "landing-4"
  | "landing-5"
  | "settings"
  | "lock"
  | "find"
  | "find-list"
  | "new"
  | "new-tab"
  | "import"
  | "close-tab"
  | "next-tab"
  | "prev-tab"
  | "focus-list"
  | "toggle-list"
  | "reload-term"
  | "prev-command"
  | "next-command"
  | "command-history";

/** One way to press an act. `mod` is ⌘ on a Mac and Ctrl elsewhere;
    `ctrl` is the Control key itself on every platform. Matched on `code`
    (the key's place) where a shift changes the character — `]` is `}`
    with ⇧ held — and on `key` where the letter is the point. */
type Chord = { key?: string; code?: string; mod?: true; ctrl?: true; shift?: true; alt?: true; mac?: true };

/** The map. `web: false` is a key the browser owns (see the head). */
export const KEYMAP: { act: Act; chords: Chord[]; web: boolean }[] = [
  { act: "landing-1", chords: [{ code: "Digit1", mod: true }], web: false },
  { act: "landing-2", chords: [{ code: "Digit2", mod: true }], web: false },
  { act: "landing-3", chords: [{ code: "Digit3", mod: true }], web: false },
  { act: "landing-4", chords: [{ code: "Digit4", mod: true }], web: false },
  { act: "landing-5", chords: [{ code: "Digit5", mod: true }], web: false },
  { act: "settings", chords: [{ code: "Comma", mod: true }], web: true },
  { act: "lock", chords: [{ key: "l", mod: true }], web: true },
  { act: "find", chords: [{ key: "k", mod: true }], web: true },
  { act: "new", chords: [{ key: "n", mod: true }], web: false },
  // **⌘T, a new tab where the landing's 「+」 makes one** (UX-7, Safari's
  // and Terminal's key): a session on 会话, a page on 浏览器. Elsewhere the
  // 「+」 makes something that is not a tab, and ⌘T does nothing.
  { act: "new-tab", chords: [{ key: "t", mod: true }], web: false },
  // **⌘F, this column's search** (UX-7; Mail's and Notes' key for the
  // list's own search field): the search circle opened and the caret in
  // its box. A terminal keeps ⌘F for its own search (`TerminalPane`).
  // Not on the web face, where ⌘F is the browser's.
  { act: "find-list", chords: [{ key: "f", mod: true }], web: false },
  { act: "close-tab", chords: [{ key: "w", mod: true }], web: false },
  {
    act: "next-tab",
    chords: [
      { key: "Tab", ctrl: true },
      { code: "BracketRight", mod: true, shift: true },
    ],
    web: false,
  },
  {
    act: "prev-tab",
    chords: [
      { key: "Tab", ctrl: true, shift: true },
      { code: "BracketLeft", mod: true, shift: true },
    ],
    web: false,
  },
  // **⌘R, the terminal in front joined again** (2026-10-03; the reload
  // circle's act, `TerminalPane`). The app's only: in a browser ⌘R is the
  // page's own reload.
  { act: "reload-term", chords: [{ key: "r", mod: true }], web: false },
  // **⌘↑ ⌘↓, from prompt to prompt** in the terminal in front (roadmap
  // 2.3; VS Code's terminal: scroll to previous / next command).
  { act: "prev-command", chords: [{ key: "ArrowUp", mod: true }], web: true },
  { act: "next-command", chords: [{ key: "ArrowDown", mod: true }], web: true },
  // iTerm2's Command History key (roadmap 2.6).
  { act: "command-history", chords: [{ code: "Semicolon", mod: true, shift: true }], web: true },
  // The list's one Tab stop, from anywhere — a terminal included, which
  // keeps Tab for its shell (the audit fix plan F3.8; W25). ⌥⌘S, the
  // sidebar's key in Notes.
  { act: "focus-list", chords: [{ code: "KeyS", mod: true, alt: true }], web: true },
  // **The sidebar's own key** (the audit fix plan F6.7, wide P-5): ⌃⌘S,
  // View › Hide Sidebar in Notes and Finder — the same fold pressing the
  // open landing does. **Mac only** (`mac`): off a Mac `mod` is Ctrl, and
  // the chord would come out as Ctrl+S, which is every editor's save.
  { act: "toggle-list", chords: [{ code: "KeyS", mod: true, ctrl: true, mac: true }], web: false },
];

const IS_MAC = /mac/i.test(navigator.platform || navigator.userAgent);

type Keyish = Pick<KeyboardEvent, "key" | "code" | "metaKey" | "ctrlKey" | "shiftKey" | "altKey">;

/**
 * Which act a key press is, on which face — `null` for none. Pure, so
 * the face gate can ask it about the web face from inside the app's.
 */
export function actOf(e: Keyish, face: "app" | "web", mac = IS_MAC): Act | null {
  const cmd = mac ? e.metaKey : e.ctrlKey;
  for (const { act, chords, web } of KEYMAP) {
    if (face === "web" && !web) continue;
    for (const c of chords) {
      if (c.mac && !mac) continue;
      // ⌃⇥ is Control on every platform; off a Mac that is also `mod`.
      const wantCtrl = !!c.ctrl || (!!c.mod && !mac);
      const wantMeta = !!c.mod && mac;
      if (e.ctrlKey !== wantCtrl || e.metaKey !== wantMeta || e.shiftKey !== !!c.shift || e.altKey !== !!c.alt) continue;
      if (c.mod && !cmd) continue;
      if (c.code !== undefined && e.code !== c.code) continue;
      if (c.key !== undefined && e.key.toLowerCase() !== c.key.toLowerCase()) continue;
      return act;
    }
  }
  return null;
}

/**
 * **The key an act answers to, as a menu writes it** (the audit fix plan
 * F6.7, wide P-2): 「⌘N」 on a Mac — modifiers in the system's order,
 * ⌃⌥⇧⌘ — and 「Ctrl+N」 elsewhere. `null` where this face does not take
 * the key (a browser's own, on the web face), so a menu never offers a
 * key that would do something else.
 */
export function keyLabel(act: Act, face: "app" | "web" = onWebFace ? "web" : "app", mac = IS_MAC): string | null {
  const entry = KEYMAP.find((k) => k.act === act);
  if (!entry || (face === "web" && !entry.web)) return null;
  const c = entry.chords.find((x) => !x.mac || mac);
  if (!c) return null;
  const name = c.key ? (c.key.length === 1 ? c.key.toUpperCase() : c.key) : (c.code ?? "").replace(/^Key|^Digit/, "").replace("Comma", ",");
  if (mac) return `${c.ctrl ? "⌃" : ""}${c.alt ? "⌥" : ""}${c.shift ? "⇧" : ""}${c.mod ? "⌘" : ""}${name}`;
  return [c.ctrl || c.mod ? "Ctrl" : "", c.alt ? "Alt" : "", c.shift ? "Shift" : "", name].filter(Boolean).join("+");
}

// ── the acts a landing lends while it is on screen ─────────────────────

/** A strip of tabs the keys can close and step through. */
export type TabHost = {
  /** How many tabs the strip holds. */
  count: number;
  /** Whether one is in front — what ⌘W would close. */
  front: boolean;
  close: () => void;
  step: (by: 1 | -1) => void;
};
/** The landing's 「+」, by its own name; `tab` when what it makes is a
    tab (⌘T runs it too). */
export type NewAct = { label: string; run: () => void; tab?: boolean };

type Lent = { tabs: (() => TabHost | null) | null; new: (() => NewAct | null) | null };
type Entry = { id: number; get: () => unknown };
const lent: Record<keyof Lent, Entry[]> = { tabs: [], new: [] };
let snapshot: Lent = { tabs: null, new: null };
const hearers = new Set<() => void>();
let nextId = 1;

function changed() {
  snapshot = {
    tabs: (lent.tabs.at(-1)?.get as Lent["tabs"]) ?? null,
    new: (lent.new.at(-1)?.get as Lent["new"]) ?? null,
  };
  for (const h of hearers) h();
}

/**
 * Lends one act while the caller is mounted; the latest lender wins.
 *
 * **Re-lent only when its shape changes** (`shape`: the tab count, what is
 * in front, the 「+」's name) — the functions are read through a ref at
 * the moment they run. Lending on every render would loop: a lend tells
 * the shell, the shell re-renders its children, a child builds a fresh
 * object and lends it again.
 */
function useLend<K extends keyof Lent>(kind: K, v: (Lent[K] extends (() => infer T) | null ? T : never) | null, shape: string) {
  const id = useRef(0);
  if (id.current === 0) id.current = nextId++;
  const latest = useRef(v);
  latest.current = v;
  const on = v !== null;
  useEffect(() => {
    if (!on) return;
    const mine = id.current;
    const list = lent[kind];
    list.push({ id: mine, get: () => latest.current });
    changed();
    return () => {
      const i = list.findIndex((x) => x.id === mine);
      if (i >= 0) list.splice(i, 1);
      changed();
    };
  }, [kind, on, shape]);
}

/** The strip on screen lends its tabs to ⌘W and ⌃⇥ (`null`: not now). */
export const useTabHost = (host: TabHost | null) =>
  useLend("tabs", host, host === null ? "" : `${host.count}:${host.front}`);
/** The landing's 「+」 lends itself to ⌘N (`PaneBar`, seated in the first
    row — the one 「+」 a landing has). */
export const useNewAct = (act: NewAct | null) => useLend("new", act, act ? `${act.label}:${!!act.tab}` : "");

const subscribe = (h: () => void) => {
  hearers.add(h);
  return () => hearers.delete(h);
};
const lentNow = () => snapshot;

// ── the shell's half ───────────────────────────────────────────────────

/** Anything **open** on top of the page (`use-keys-to`'s reading: a
    closing Radix content keeps `data-state="closed"` for its exit). */
const ON_TOP = '[role="dialog"]:not([data-state="closed"]), [role="alertdialog"]:not([data-state="closed"])';

/** What the shell itself answers for. */
export type ShellActs = {
  locked: boolean;
  /** The rail's landing, 0–4, reached the way its glyph reaches it. */
  go: (i: number) => void;
  settings: () => void;
  /** Puts the 「…」 page away, when one is up. */
  closePage: (() => void) | null;
  /** Locks, when a PIN makes that possible. */
  lock: (() => void) | null;
  find: () => void;
  import: () => void;
  /** Folds the list or brings it back, where there is one to fold. */
  toggleList: (() => void) | null;
  /** Whether that list is folded now — the menu's word follows it. */
  listFolded: boolean;
  /** The strip's landings in its order: the View menu's (`menu.rs`). */
  order: string[];
};

/**
 * The keyboard and the menu bar, installed once by the shell. Every key
 * the map knows is run here, and so is every menu item the Mac app's
 * menu bar sends back.
 */
export function useShortcuts(acts: ShellActs) {
  const now = useSyncExternalStore(subscribe, lentNow);
  const live = useRef({ acts, now });
  live.current = { acts, now };

  useEffect(() => {
    const face = onWebFace ? "web" : "app";
    /** Runs one act; false when nothing applies (the key is then left
        to whoever else wants it — on a Mac, the greyed menu's beep). */
    const run = (act: Act): boolean => {
      const { acts, now: lender } = live.current;
      const now = { tabs: lender.tabs?.() ?? null, new: lender.new?.() ?? null };
      if (acts.locked) return false;
      if (act !== "lock" && document.querySelector(ON_TOP)) return false;
      if (act.startsWith("landing-")) {
        acts.go(Number(act.slice(-1)) - 1);
        return true;
      }
      switch (act) {
        case "settings":
          acts.settings();
          return true;
        case "lock":
          if (!acts.lock) return false;
          acts.lock();
          return true;
        case "find":
          // **⌘K in a terminal clears it** (T-17, iTerm2's and Terminal's
          // ⌘K): left to the terminal (`TerminalPane`), not the app's
          // find, while the terminal has the keys.
          if (IS_MAC && document.activeElement?.classList.contains("xterm-helper-textarea")) return false;
          acts.find();
          return true;
        case "import":
          acts.import();
          return true;
        case "new":
          if (!now.new) return false;
          now.new.run();
          return true;
        case "new-tab":
          if (!now.new?.tab) return false;
          now.new.run();
          return true;
        case "find-list": {
          if (document.activeElement?.classList.contains("xterm-helper-textarea")) return false;
          const open = [...document.querySelectorAll<HTMLElement>("[data-pane-bar] [data-search-open]")].find((e) => e.offsetParent);
          const box = [...document.querySelectorAll<HTMLInputElement>("[data-pane-bar] input")].find((e) => e.offsetParent);
          if (open) open.click();
          else if (box) box.focus();
          else return false;
          return true;
        }
        case "close-tab":
          if (acts.closePage) acts.closePage();
          else if (now.tabs?.front) now.tabs.close();
          else return false;
          return true;
        case "focus-list": {
          const row = document.querySelector<HTMLElement>('[data-side-list] [data-side-open][tabindex="0"]');
          if (!row) return false;
          // A key put it there: the ring shows at once (W-V6).
          document.documentElement.dataset.modality = "keyboard";
          row.focus({ focusVisible: true } as FocusOptions);
          return true;
        }
        case "toggle-list":
          if (!acts.toggleList) return false;
          acts.toggleList();
          return true;
        case "reload-term": {
          // Not under a finger: there the circle is the way.
          if (window.matchMedia("(pointer: coarse)").matches) return false;
          const box = document.querySelector<HTMLElement & { __reopen?: () => Promise<void> }>(
            "[data-detail-pane] [data-kept-terminal][data-active] [data-terminal]",
          );
          if (!box?.__reopen) return false;
          void box.__reopen();
          return true;
        }
        case "prev-command":
        case "next-command": {
          // A text field of the page's own keeps ⌘↑ ⌘↓ for its caret.
          const at = document.activeElement;
          const box = document.querySelector<HTMLElement & { __jump?: (dir: -1 | 1) => void }>(
            "[data-detail-pane] [data-kept-terminal][data-active] [data-terminal]",
          );
          if (!box?.__jump) return false;
          if (at instanceof HTMLInputElement || (at instanceof HTMLTextAreaElement && !box.contains(at))) return false;
          box.__jump(act === "prev-command" ? -1 : 1);
          return true;
        }
        case "command-history": {
          const box = document.querySelector<HTMLElement & { __history?: () => void }>(
            "[data-detail-pane] [data-kept-terminal][data-active] [data-terminal]",
          );
          if (!box?.__history) return false;
          box.__history();
          return true;
        }
        case "next-tab":
        case "prev-tab":
          if (acts.closePage || !now.tabs || now.tabs.count < 2) return false;
          now.tabs.step(act === "next-tab" ? 1 : -1);
          return true;
      }
      return false;
    };
    const key = (e: KeyboardEvent) => {
      if (e.defaultPrevented || e.isComposing) return;
      const act = actOf(e, face);
      if (act === null || !run(act)) return;
      // Prevented so WKWebView does not hand the same key to the menu
      // bar's accelerator — and stopped so a terminal does not also type
      // it (xterm reads keydown on its own textarea, under this).
      e.preventDefault();
      e.stopPropagation();
    };
    // Capture, on the window: before a terminal, an input or a menu can
    // take the key for itself.
    window.addEventListener("keydown", key, true);
    const unhear = hear<string>("shortcut", (act) => {
      run(act as Act);
    });
    return () => {
      window.removeEventListener("keydown", key, true);
      unhear();
    };
  }, []);

  // **The menu bar's enabled items and the 「+」's name**, told to the Mac
  // app whenever they change (`menu.rs`). Only there: no other face has a
  // menu bar to tell.
  const tabs = now.tabs?.() ?? null;
  const state = JSON.stringify({
    close: acts.closePage !== null || !!tabs?.front,
    step: acts.closePage === null && (tabs?.count ?? 0) >= 2,
    new: now.new?.()?.label ?? null,
    lock: acts.lock !== null,
    locked: acts.locked,
    order: acts.order,
    list: acts.toggleList ? acts.listFolded : null,
  });
  useEffect(() => {
    if (!inApp || !trafficLights) return;
    menuState(JSON.parse(state)).catch(() => {
      /* a menu that is a beat behind still runs the right act: `run`
         decides again when the key or the item arrives */
    });
  }, [state]);

  useNoWebMenu();
}

// ── the page's right-click ─────────────────────────────────────────────

/**
 * **No browser menu over the chrome** (2026-09-28). WKWebView answers a
 * right-click anywhere with its own menu — Reload, and Inspect Element in
 * a build with the inspector — which no Mac app has over its sidebar.
 * The system's menu stays where it is useful: in a text field (cut, copy,
 * paste, spelling) and over selected text (copy, look up). khor's own
 * menus (a row's, `RowMenu`) open as before: they prevent the event
 * themselves, earlier, and this reads that.
 *
 * On the window, **in the bubble phase**: Radix opens a row's menu only
 * if nobody prevented the event before it (`composeEventHandlers`), so a
 * capture listener here would close every row menu in the app.
 */
function useNoWebMenu() {
  useEffect(() => {
    const menu = (e: MouseEvent) => {
      if (e.defaultPrevented) return;
      const t = e.target instanceof Element ? e.target : null;
      if (t?.closest('input, textarea, select, [contenteditable]:not([contenteditable="false"])')) return;
      const sel = window.getSelection();
      if (sel && !sel.isCollapsed && t && sel.containsNode(t, true)) return;
      e.preventDefault();
    };
    window.addEventListener("contextmenu", menu);
    return () => window.removeEventListener("contextmenu", menu);
  }, []);
}

// ── sidebar lists ──────────────────────────────────────────────────────

/**
 * ↑ ↓ Home End between a list's items, for an `onKeyDown` on the list.
 * `items` selects what the focus stops on, inside the list. Moving never
 * presses: ⏎ and Space are the buttons' own.
 */
export function walkList(e: ReactKeyboardEvent<HTMLElement>, items: string) {
  if (e.altKey || e.metaKey || e.ctrlKey || e.shiftKey) return;
  if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(e.key)) return;
  const all = [...e.currentTarget.querySelectorAll<HTMLElement>(items)].filter(
    (el) => el.getClientRects().length > 0,
  );
  const at = all.indexOf(document.activeElement as HTMLElement);
  if (at < 0) return;
  const to =
    e.key === "Home"
      ? 0
      : e.key === "End"
        ? all.length - 1
        : Math.max(0, Math.min(all.length - 1, at + (e.key === "ArrowDown" ? 1 : -1)));
  e.preventDefault();
  all[to]?.focus();
  all[to]?.scrollIntoView({ block: "nearest" });
}
