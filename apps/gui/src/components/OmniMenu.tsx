// The list a box offers, and the two places it can be drawn.
//
// **One component, because there is one menu.** It hangs under the box on
// every pane in this app, and on a browsing tab it hangs in a small window
// of the app's own — a native child webview always paints above this
// page's DOM, so a menu drawn here would come out underneath whatever site
// the tab is showing (`show_menu` has the whole account). Two components
// would be one restyle away from being two different menus for one idea,
// which is the failure `OmniChip`'s head describes about copying a token.
//
// **The keyboard never moves.** Even floated, the caret stays in the box
// and ↑↓/Tab/Enter belong to the field it is in; what crosses to the other
// window is the list and which row is lit, and what comes back is a press.
// That is why the window is built unable to take focus.
import { useCallback, useEffect, useLayoutEffect, useRef, useState, type CSSProperties, type Ref } from "react";

import { hear, hideMenu, inApp, menuPick, menuRows, showMenu, sizeMenu } from "@/api";
import type { Avatar } from "@/gen/bindings/Avatar";
import { MachineAvatar } from "@/components/Avatar";
import { IconBrowser, IconCheck } from "@/components/icons";
import { cn } from "@/lib/utils";
import {MenuViewport} from "@/components/ui/menu-viewport";
export {MENU_TALLEST} from "@/components/ui/menu-viewport";

/** One thing the list can be narrowed by. `key` is the node's own group
    key (`dev:turing`); `label` is what a person reads.

    `hint` and `dim` are one judgment in two halves, and they exist for
    a candidate that **is not a fact about right now**: a path offered
    out of memory while the machine that owns it is not answering. The
    hint names where it came from and the dimming says it is not a
    listing — either alone is the failure, because a remembered path
    that looks exactly like a live one is this box claiming to know
    something it does not. Both are absent on an ordinary candidate,
    which is how "no hint" stays readable as "this is live". */
export type Candidate = {
  key: string;
  label: string;
  face?: Avatar | null;
  hint?: string;
  dim?: boolean;
  /** **The letters typed, heavy in the label** (the browser pill, sb-a
      2026-10-09: 「类似目录选择」): where the match starts and how long
      it is. Two numbers rather than markup, because the rows cross to
      the floated window as data. */
  mark?: [number, number];
  /** A fact about the row, at its end: a machine's road (「直连 23 ms」),
      an address's age (「3 小时前」). Unlike `hint` it says nothing about
      where the row came from, so the row is not dimmed. */
  aside?: string;
  /** The one that is already chosen, ticked (the pill's face pressed:
      the current machine). */
  on?: boolean;
  /** A glyph where a face would stand: `page`, an address. */
  glyph?: "page";
};

/**
 * One axis the box can offer, with the candidates it has right now.
 *
 * **`role` is the whole of the filter/navigation split**, and it lives
 * here rather than in the mechanism: chips, candidates and the keyboard
 * are the same either way, and what differs is only what committing one
 * *means*.
 *
 * - `chip` (the default): the candidate becomes a chip. What that chip
 *   then does — narrow a list, or name the thing the pane acts on — is
 *   the caller's `onToggle`, which is why a filter axis and a navigation
 *   axis need no different machinery here. A pane that wants one chip at
 *   a time simply replaces instead of appending.
 * - `text`: the candidate is a **completion**, not a chip. Committing it
 *   writes it into the box, because half a path is a real thing to keep
 *   typing and half a filter is not.
 */
export type OmniAxis = {
  key: string;
  label: string;
  candidates: Candidate[];
  role?: "chip" | "text";
  /** What the axis says in its rows' place when it has none (「没有匹配
      项」); an axis with none and no sentence draws nothing. */
  empty?: string;
};

/** The whole of what a menu is showing, as it crosses to the window that
    draws it: the axes with their candidates, and which row of the flat
    list is lit. **The lit row rides along** because the arrows that move
    it are pressed in the other window — a menu that decided its own
    highlight would answer a different key than the one that was hit. */
export type MenuRows = { groups: OmniAxis[]; at: number };

/** How far under the box the menu hangs. */
const MENU_GAP = 4;

/**
 * The rows themselves. Nothing here knows which of the two places it is
 * being drawn in — the caller passes the box it lives in.
 */
export function OmniMenu({
  menuRef,
  rows,
  onPick,
  className,
  style,
}: {
  menuRef?: Ref<HTMLDivElement>;
  rows: MenuRows;
  onPick: (key: string) => void;
  className?: string;
  style?: CSSProperties;
}) {
  const flat = rows.groups.flatMap((a) => a.candidates);
  return (
    <div
      ref={menuRef}
      data-omni-menu
      data-menu-frame
      style={style}
      // The one menu's material (the audit fix plan F1.6 / F3.4).
      className={cn("glass overflow-hidden rounded-menu border-0 bg-glass-menu shadow-[var(--glass-in),var(--glass-shadow)]", className)}
    >
      <MenuViewport key={flat.map(c=>c.key).join("\0")} active={rows.at} room={style?.maxHeight}>
      {rows.groups.map((a) => (
        <div key={a.key}>
          {/* An axis with no label is a list of one kind — the path
              box's completions (`PathField`), where a caption over the
              rows would repeat what the box already says. */}
          {a.label !== "" && (
            <div data-omni-axis={a.key} className="px-row pt-1.5 pb-1 text-caption text-muted-foreground">
              {a.label}
            </div>
          )}
          {a.candidates.length === 0 && a.empty && (
            <div data-omni-empty className="px-row py-2 text-body text-muted-foreground">
              {a.empty}
            </div>
          )}
          {a.candidates.map((c) => {
            const n = flat.indexOf(c);
            return (
              <button
                key={c.key}
                type="button"
                data-omni-item={c.key}
                data-on={n === rows.at}
                className={cn(
                  // A menu row is chosen with a finger as often as
                  // with the keyboard, and the two ways have to aim
                  // at the same thing: `fill-strong` marks the picked
                  // row (a chosen row, like a selected list row — not
                  // the brand wash, README 颜色) and the row is the floor
                  // tall, so what the arrow keys highlight is exactly
                  // what a press lands on (tokens.css `--touch`).
                  "flex min-h-menu-item w-full items-center gap-2 rounded-menu-item px-row text-left text-body",
                  n === rows.at && "glass-sel",
                  c.dim && "text-muted-foreground",
                )}
                // The box must not lose focus to the press — the caret
                // stays in the field whichever window this list is drawn
                // in, because that field is what the arrows and Enter
                // belong to.
                onMouseDown={(e) => e.preventDefault()}
                // **The release, not the press**, and it stays that way
                // floated: pressing a row and sliding off it has always
                // meant "no", and moving the pick to `mousedown` to be
                // safe against a window stealing focus would have taken
                // that away from every other box in the app to fix a
                // window only one of them has.
                onClick={() => onPick(c.key)}
              >
                {c.face !== undefined && c.face !== null && (
                  <MachineAvatar face={c.face} className="size-kind-mark flex-none" />
                )}
                {c.glyph === "page" && <IconBrowser className="size-kind-mark flex-none text-muted-foreground" />}
                <span className="min-w-0 flex-1 truncate">
                  {c.mark ? (
                    <>
                      {c.label.slice(0, c.mark[0])}
                      <span data-omni-mark className="font-title">
                        {c.label.slice(c.mark[0], c.mark[0] + c.mark[1])}
                      </span>
                      {c.label.slice(c.mark[0] + c.mark[1])}
                    </>
                  ) : (
                    c.label
                  )}
                </span>
                {c.aside && (
                  <span data-omni-aside className="flex-none text-caption tabular-nums text-muted-foreground">
                    {c.aside}
                  </span>
                )}
                {c.on && (
                  <span data-omni-on className="flex flex-none">
                    <IconCheck className="size-affordance" />
                  </span>
                )}
                {/* Where it came from, when it came from memory
                    rather than from the machine. It rides the row it
                    describes — there is nowhere else for it to be
                    that would still be beside the thing it is
                    about. */}
                {c.hint && (
                  <span data-omni-hint className="flex-none text-caption text-muted-foreground">
                    {c.hint}
                  </span>
                )}
              </button>
            );
          })}
        </div>
      ))}
      </MenuViewport>
    </div>
  );
}

/**
 * Draws the menu in a window of the app's own, over anything the app has
 * laid on top of its page.
 *
 * Answers **whether that took**. `false` is not a failure and the caller
 * must draw the list itself when it comes back: a browser tab and a phone
 * have no such window, and neither of them has a page webview covering
 * khor's overlays either — the in-page menu is the right one there. A
 * desktop refusal (a window that would not build) lands in the same place,
 * which is the one behaviour worth having: 一档接不上不会变成「没有值」,
 * and a menu that quietly became nothing would leave the box unusable.
 */
export function useFloatedMenu(
  anchor: React.RefObject<HTMLElement | null>,
  rows: MenuRows | null,
  onPick: (key: string) => void,
  float: boolean,
): boolean {
  // **Optimistic, and that is a judgment about flicker.** In the app the
  // float is the ordinary answer; starting at `false` would draw the
  // in-page menu for a frame and then take it away under the eyes of the
  // person who just pressed the box.
  const [able, setAble] = useState(inApp);
  const said = useRef(onPick);
  said.current = onPick;
  /** **A press is this box's only while this box's rows are up**: the
      window and its `menu:pick` are the app's one, and every box with a
      floated menu hears it — the browser pill's field and a search box
      can both be mounted (sb-a, 2026-10-09). */
  const showing = useRef(false);
  useEffect(() => hear<string>("menu:pick", (key) => showing.current && said.current(key)), []);

  const on = float && able;
  showing.current = on && rows !== null;
  // The rows are rebuilt on every render, so the effect below is keyed on
  // what is *in* them. Keyed on the object it would ask the app to move a
  // window on every keystroke that changed nothing.
  const same = rows ? JSON.stringify(rows) : "";
  const ship = useCallback(() => {
    if (!on) return;
    if (!rows) {
      hideMenu().catch(() => {});
      return;
    }
    const el = anchor.current;
    if (!el) return;
    const r = el.getBoundingClientRect();
    showMenu(
      {
        x: r.left,
        y: r.bottom + MENU_GAP,
        w: r.width,
        // This viewport is how the app finds the screen: it subtracts it
        // from the window frame to learn how much of that frame is
        // chrome (`on_screen`). Nothing here does the arithmetic and
        // nothing writes down a titlebar.
        view: window.innerHeight,
        across: window.innerWidth,
      },
      rows,
    ).then(
      (took) => {
        if (!took) setAble(false);
      },
      // **Loud, and it earned that on the first try.** This handler was
      // `() => setAble(false)`, so when the app refused to build the
      // window the only sign was a menu that went on being drawn in the
      // page — the exact silence a missing capabilities file hid behind
      // for a day (`onPageWord`). Falling back is right; falling back
      // without saying why is not.
      (e: unknown) => {
        console.error("khor: the overlay window refused, menu stays in the page", e);
        setAble(false);
      },
    );
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [on, same, anchor]);

  useEffect(ship, [ship]);

  // The box moves when the window does, and a menu pinned to where the box
  // used to be is worse than none.
  useEffect(() => {
    window.addEventListener("resize", ship);
    return () => window.removeEventListener("resize", ship);
  }, [ship]);

  // Gone from the screen with the menu still up: the window is the app's
  // and outlives this component, so leaving would leave it hanging there.
  useEffect(
    () => () => {
      hideMenu().catch(() => {});
    },
    [],
  );

  return on;
}

/**
 * The whole of the app's small overlay window (`isMenuFace`).
 *
 * **It pulls once and then listens**, because the window and its first
 * rows are decided in the same breath and a webview still loading its page
 * cannot be listening yet — an event sent then reaches nobody, and the
 * first press of the address bar would open an empty menu.
 */
export function MenuFace() {
  const [rows, setRows] = useState<MenuRows | null>(null);
  useEffect(() => {
    menuRows().then(
      (r) => setRows(r ?? null),
      () => {},
    );
    return hear<MenuRows>("menu:rows", setRows);
  }, []);

  // **The window is sized from what was actually laid out.** Working the
  // height out on the app's side would mean writing down a row height,
  // and that number goes wrong the first time the type scale moves — and
  // goes wrong silently, as a menu with its last row cut off. Watched
  // rather than measured once per render, so a face that arrives late or
  // a font that loads late resizes the window instead of being clipped.
  const box = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    const el = box.current;
    if (!el) return;
    const tell = () => {
      sizeMenu(document.documentElement.scrollHeight).catch(() => {});
    };
    const watch = new ResizeObserver(tell);
    watch.observe(el);
    tell();
    return () => watch.disconnect();
  }, [rows]);

  // Nothing to show is not a state this window sits in — the app hides it
  // — so an empty render is a frame between two lists, and drawing a bare
  // bordered box for it would flash a white rectangle over the page.
  if (!rows || rows.groups.length === 0) return null;
  return (
    <div ref={box} data-menu-native>
      <OmniMenu
        rows={rows}
        onPick={(key) => {
          menuPick(key).catch(() => {});
        }}
      />
    </div>
  );
}
