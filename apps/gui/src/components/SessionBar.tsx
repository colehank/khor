// **A session on the narrow face: the content on top, the way back and
// the tabs at the foot** (2026-10-02, iOS 第三步 A).
//
// The owner, showing another phone's terminal: 「我可能希望手机版的 session
// 布局大概是这样。上方完全交给内容，下方是操作与标签页」, and then three
// answers: inside a session the landings' bar is not shown, a back control
// returns to the list; tabs that do not fit scroll sideways like the wide
// strip; a conversation has the same layout, with its input box where a
// terminal has its keys. And the principle under all three: **the narrow
// layout is the phone layout** — one implementation, keyed on three
// switches that are never conflated:
//
//   width    the narrow face decides this layout (a Mac window dragged
//            narrow gets it too — `App` mounts this on `narrow` alone);
//   pointer  a finger decides the touch things: the keys a soft keyboard
//            lacks and the switch for the keyboard (`TermKeys`,
//            `KeyboardSwitch` — each asks `(pointer: coarse)` itself, so
//            the wide face on a finger has them too, in place);
//   can-host decides which entries exist (`DeviceRow.hosts`, 第二步) and
//            is not read here at all.
//
// So this is **not a second session page**. It is the same `DetailPane`,
// the same `TabStrip`, the same `SessionFace` on every tab, laid out in a
// different order: the strip leaves the top of the board for the foot of
// the screen, and it hangs from the content instead of standing under it
// (`TabStrip` `at="bottom"`: the open tab is the paper joined to the
// session above it).
//
// # The bar
//
//   row 1   ‹ · the open sessions (scrolling) · the count when they do
//           not fit · stale · the keyboard switch (a finger only)
//   row 2   whatever the open session seats under it: a terminal's keys
//           (a finger only), a conversation's input box (always)
//
// Row 1 is `--pane-head`, the same first row a tab stands in at the top
// of the wide face — 38 under a mouse, 48 under a finger. Row 2 is
// `--key-row` for the keys, and the input box's own height for a
// conversation. Under both, the phone's home indicator (`--safe-bottom`),
// which goes to nothing while the soft keyboard is up: the shell is sized
// to what the keyboard leaves (`lib/viewport`), so the bar lands on the
// keys and the terminal above it refits.
//
// # Where the open session's verbs went (一个动作一个家)
//
// On the wide face R0 is the strip, and the open session's own verbs ride
// **on its tab**: pressing the tab in front again shows its day, a
// double-click on the name renames, the × closes the view; R0's right end
// holds only the stale line and, once the strip scrolls, the count. Here
// the strip moved, so those verbs moved with it, unchanged — they are the
// tab's — and row 1's right end is R0's right end: the count, the stale
// line, and the one verb the wide face has no use for, the keyboard
// switch. Creating something stays on the list's ＋, a row's acts stay in
// its long-press menu. Nothing is offered twice: the reference's second
// 「hide keyboard」 key at the end of the key row is not drawn, the
// switch in row 1 is its one home.
//
// # Seats
//
// The open session decides what goes in the bar, and it is three views
// deep (`DetailPane` → `TerminalPane` / `ChatView` / `MachineChat`), so
// the bar hands down two seats and a view puts things in them through a
// portal (`BarSeat`) — the arrangement the window's first row already
// uses for a landing's head (`SideSeat`, `NarrowHead`). **Where there is
// no bar** (the wide face, a workspace's session column) a view's `under`
// renders in place, exactly where it always stood, and its `trail` is
// not drawn: the wide face has no row that belongs to the open session.
import { createContext, useContext, useMemo, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";

import { TabStrip, type TabFace } from "@/browse/TabStrip";
import { IconBack } from "@/components/icons";
import { WINDOW_DRAG } from "@/components/shell";
import { GlassCircle } from "@/components/Glass";
import { gui } from "@/gen/catalog";

type Seats = { lead: HTMLElement | null; trail: HTMLElement | null; under: HTMLElement | null };

const SeatsContext = createContext<Seats | null>(null);

/** **The open session's verbs on the wide face** (2026-10-03): the end of
    R0's tab strip, where a page's verbs stand. A view's `lead` goes there
    when there is no bar; the provider is the wide shell's (`App`). */
export const VerbSeat = createContext<HTMLElement | null>(null);

/**
 * Something the open session puts in the bar: `under` is row 2 (the keys,
 * the input box), `trail` is row 1's right end (the keyboard switch),
 * `lead` is row 1 right after ‹ (the terminal's reload). Without a bar
 * around it, `under` stands where it was written, `lead` goes to the wide
 * face's verbs (`VerbSeat`) where there is one, and `trail` is not drawn.
 */
export function BarSeat({ at, children }: { at: "lead" | "trail" | "under"; children: ReactNode }) {
  const seats = useContext(SeatsContext);
  const verbs = useContext(VerbSeat);
  if (!seats) {
    if (at === "under") return <>{children}</>;
    return at === "lead" && verbs ? createPortal(children, verbs) : null;
  }
  const el = seats[at];
  return el ? createPortal(children, el) : null;
}

/** Whether the open session is in the bar's screen — a view that lays
    itself out differently there (the day's float) asks this. */
export function useInBar(): boolean {
  return useContext(SeatsContext) !== null;
}

export function SessionScreen({
  faces,
  active,
  onFocus,
  onClose,
  onSwitch,
  switching,
  onBack,
  caption,
  children,
}: {
  faces: TabFace[];
  active: string;
  onFocus: (id: string) => void;
  onClose: (id: string) => void;
  /** The count's press, once the strip has stopped showing every tab —
      the narrow shell's switcher, the wide strip's rule (`TabStrip`). */
  onSwitch: () => void;
  switching: boolean;
  /** Back to the list. */
  onBack: () => void;
  /** 过期 / 离线, after the strip — R0's caption on the wide face. */
  caption?: ReactNode;
  children: ReactNode;
}) {
  const [lead, setLead] = useState<HTMLElement | null>(null);
  const [trail, setTrail] = useState<HTMLElement | null>(null);
  const [under, setUnder] = useState<HTMLElement | null>(null);
  const seats = useMemo(() => ({ lead, trail, under }), [lead, trail, under]);
  return (
    <div data-session-screen className="flex h-full min-h-0 flex-col bg-card">
      {/* The top of the glass is the platform's: a phone's status bar
          (`--safe-top`), or on a Mac window dragged narrow the corner its
          three buttons stand in (`app.css`, `[data-session-top]` under
          `data-traffic`) — which is a handle on the window like L0 was. */}
      <div data-session-top {...WINDOW_DRAG} className="h-safe-top flex-none" />
      <SeatsContext.Provider value={seats}>
        <div data-session-content className="flex min-h-0 flex-1 flex-col">
          {children}
        </div>
      </SeatsContext.Provider>
      {/* No rule over the bar: the change of ground is the edge, and a
          line would cut the open tab off from the session it is joined
          to — the strip's own rule at the top (`TabStrip`). */}
      <div data-session-bar className="flex flex-none flex-col pb-safe-bottom">
        {/* Row 1 is R0 moved to the foot: a handle on the window where
            it is blank, like R0 (`shell.ts`). */}
        <div data-session-bar-tabs data-session-tabs aria-label={gui.session_tabs} {...WINDOW_DRAG} className="flex h-pane-head flex-none">
          <TabStrip
            glass
            at="bottom"
            faces={faces}
            active={active}
            onFocus={onFocus}
            onClose={onClose}
            onSwitch={onSwitch}
            switching={switching}
            // The way out, pinned before the scroller (`TabStrip` `head`)
            // so it never scrolls away with the tabs. The narrow face's
            // only exit, so it is at the finger's floor.
            // Right after it, the open session's own verbs (the
            // terminal's reload): two circles side by side, not a capsule.
            head={
              <>
                <GlassCircle aria-label={gui.back} data-back onClick={onBack}>
                  <IconBack />
                </GlassCircle>
                <span ref={setLead} data-session-bar-lead className="flex flex-none items-center gap-in empty:hidden" />
              </>
            }
            tail={
              <>
                {caption && (
                  <span data-region-caption className="flex-none truncate px-xxs text-caption text-tertiary tabular-nums">
                    {caption}
                  </span>
                )}
                <span ref={setTrail} data-session-bar-trail className="flex flex-none items-center empty:hidden" />
              </>
            }
          />
        </div>
        <div ref={setUnder} data-session-bar-under className="flex flex-none flex-col empty:hidden" />
      </div>
    </div>
  );
}
