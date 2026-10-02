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
import { useCallback, useEffect, useLayoutEffect, useRef, useState, type CSSProperties } from "react";

import { hear, hideMenu, inApp, menuPick, menuRows, showMenu, sizeMenu } from "@/api";
import type { Avatar } from "@/gen/bindings/Avatar";
import { MachineAvatar } from "@/components/Avatar";
import { cn } from "@/lib/utils";

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
};

/** The whole of what a menu is showing, as it crosses to the window that
    draws it: the axes with their candidates, and which row of the flat
    list is lit. **The lit row rides along** because the arrows that move
    it are pressed in the other window — a menu that decided its own
    highlight would answer a different key than the one that was hit. */
export type MenuRows = { groups: OmniAxis[]; at: number };

/**
 * How tall a menu may get before it scrolls inside itself.
 *
 * **A number rather than `max-h-60`, and it has to be one**: the floated
 * menu's window is sized in pixels from what this measures, so a cap
 * written as a class would be a second spelling that the app could not
 * read — and the two would part company the first time either moved.
 */
export const MENU_TALLEST = 240;

/** How far under the box the menu hangs. */
const MENU_GAP = 4;

/**
 * The rows themselves. Nothing here knows which of the two places it is
 * being drawn in — the caller passes the box it lives in.
 */
export function OmniMenu({
  rows,
  onPick,
  className,
  style,
}: {
  rows: MenuRows;
  onPick: (key: string) => void;
  className?: string;
  style?: CSSProperties;
}) {
  const flat = rows.groups.flatMap((a) => a.candidates);
  return (
    <div
      data-omni-menu
      style={{ maxHeight: MENU_TALLEST, ...style }}
      // The one menu's material (the audit fix plan F1.6 / F3.4).
      className={cn("glass overflow-y-auto rounded-menu border-0 bg-glass-menu p-menu-inset shadow-[var(--glass-in),var(--glass-shadow)]", className)}
    >
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
                  <MachineAvatar face={c.face} className="size-kind-mark" />
                )}
                <span className="min-w-0 flex-1 truncate">{c.label}</span>
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
  useEffect(() => hear<string>("menu:pick", (key) => said.current(key)), []);

  const on = float && able;
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
    <div ref={box}>
      <OmniMenu
        rows={rows}
        onPick={(key) => {
          menuPick(key).catch(() => {});
        }}
      />
    </div>
  );
}
