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
import { createContext, useContext, useMemo, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";

import { TabAvatar, type TabFace } from "@/browse/TabStrip";
import { IconBack, IconClose, IconPlus } from "@/components/icons";
import { PathCut } from "@/components/PathCut";
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog";
import { cn } from "@/lib/utils";
import { WINDOW_DRAG } from "@/components/shell";
import { GlassCircle } from "@/components/Glass";
import { gui } from "@/gen/catalog";

type Seats = {
  lead: HTMLElement | null;
  trail: HTMLElement | null;
  under: HTMLElement | null;
  pill: HTMLElement | null;
};

const SeatsContext = createContext<Seats | null>(null);

/** **The open session's verbs on the wide face** (2026-10-03): the end of
    R0's tab strip, where a page's verbs stand. A view's `lead` goes there
    when there is no bar; the provider is the wide shell's (`App`). */
export const VerbSeat = createContext<HTMLElement | null>(null);

/**
 * Something the open tab puts in the deck (round UX-3): `lead` is the
 * pill's right end (the terminal's reconnect, the page's reload), `pill`
 * its left end (the page's ← →), `trail` a circle between the pill and
 * the tabs circle (the terminal's keyboard, in 点击按钮 mode only), and
 * `under` the row below the deck (the keys, the input box, the address
 * while it is edited). Without a deck around it, `under` stands where it
 * was written, `lead` goes to the wide face's verbs (`VerbSeat`) where
 * there is one, and `trail` and `pill` are not drawn.
 */
export function BarSeat({ at, children }: { at: "lead" | "trail" | "under" | "pill"; children: ReactNode }) {
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
  onNew,
  newLabel,
  onBack,
  onTitle,
  caption,
  folded = false,
  onUnfold,
  loading = false,
  children,
}: {
  /** **The browser's row folds to a strip while the page is read**
      (round UX-3 §3, built in UX-4 with the phone page's `page:scroll`):
      ‹ and the tabs circle go, the pill becomes a thin strip with the
      title — and a hairline while the page loads — and a tap on it, a
      scroll up or the top of the page brings the row back. The terminal
      never folds. */
  folded?: boolean;
  onUnfold?: () => void;
  /** The page is loading: the folded strip's hairline. */
  loading?: boolean;
  faces: TabFace[];
  active: string;
  onFocus: (id: string) => void;
  onClose: (id: string) => void;
  /** ＋ at the end of the tabs sheet: a new tab of this page's kind (a
      page, a session). The one place a new tab is opened from here. */
  onNew?: () => void;
  /** ＋'s word where the new thing is not a page (「新会话」). */
  newLabel?: string;
  /** Back to the list. */
  onBack: () => void;
  /** The title pressed: the browser's address editor. Nothing on a
      session, whose title is only its name here. */
  onTitle?: () => void;
  /** 过期 / 离线, in the pill after the title — R0's caption on the wide
      face. */
  caption?: ReactNode;
  children: ReactNode;
  /** Kept for the callers that still pass them; the tabs circle opens
      the sheet itself (round UX-3). */
  onSwitch?: () => void;
  switching?: boolean;
}) {
  const [lead, setLead] = useState<HTMLElement | null>(null);
  const [trail, setTrail] = useState<HTMLElement | null>(null);
  const [under, setUnder] = useState<HTMLElement | null>(null);
  const [pill, setPill] = useState<HTMLElement | null>(null);
  const [sheet, setSheet] = useState(false);
  const seats = useMemo(() => ({ lead, trail, under, pill }), [lead, trail, under, pill]);
  const at = faces.findIndex((f) => f.id === active);
  const face = at < 0 ? null : faces[at];
  /** **A sideways swipe on the pill is the next or previous tab**
      (Safari's address bar). Across the pill only, so the content keeps
      its own gestures; a swipe shorter than a third of a circle is a
      press. */
  const swipe = useRef<{ x: number; y: number } | null>(null);
  const step = (by: number) => {
    const next = faces[at + by];
    if (next) onFocus(next.id);
  };
  // ⚠️ Tombstone (item 28, the terminal lane): a 24-pt drag down anywhere over
  // the content lowered the keyboard here. The key row is a portal into
  // the bar, so React's synthetic touch events from the joystick bubbled
  // up into it and the keyboard dropped under the thumb. The one lowering
  // rule is now the terminal's swipe, on its own scroller
  // (`TerminalPane`'s `swipeLowers`).
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
      {/* **The deck** (round UX-3, the owner's ruling 2026-10-04): one
          row of glass at the foot — ‹ back to the list, the pill with the
          tab's own things, and the tabs circle. Nothing at the top; the
          content has the screen. Hidden while the keyboard is up, when
          the row under it (the keys, the box, the address) rides the
          keyboard alone (`app.css`). */}
      <div data-session-bar className="flex flex-none flex-col pb-safe-bottom">
        {folded && (
          <button
            type="button"
            data-deck-strip
            onClick={onUnfold}
            className="relative flex h-deck-strip flex-none items-center justify-center overflow-hidden px-row text-caption text-muted-foreground"
          >
            <span className="min-w-0 truncate">{face?.label ?? ""}</span>
            {loading && <span data-deck-strip-loading aria-hidden="true" className="absolute inset-x-0 bottom-0 h-px animate-pulse bg-brand-strong" />}
          </button>
        )}
        <div
          data-session-deck
          {...WINDOW_DRAG}
          hidden={folded}
          className="flex h-[calc(var(--glass-btn)+2*var(--gap-in))] flex-none items-center gap-in px-row"
        >
          <GlassCircle aria-label={gui.back} data-back onClick={onBack}>
            <IconBack />
          </GlassCircle>
          <div
            data-deck-pill
            aria-label={gui.session_tabs}
            className="glass flex h-glass-btn min-w-0 flex-1 items-center gap-in rounded-full ps-1 pe-1"
            onPointerDown={(e) => {
              swipe.current = { x: e.clientX, y: e.clientY };
            }}
            onPointerUp={(e) => {
              const from = swipe.current;
              swipe.current = null;
              if (!from) return;
              const dx = e.clientX - from.x;
              if (Math.abs(dx) > SWIPE_STEPS && Math.abs(dx) > 2 * Math.abs(e.clientY - from.y)) step(dx < 0 ? 1 : -1);
            }}
          >
            <span ref={setPill} data-deck-pill-lead className="flex flex-none items-center empty:hidden" />
            {face && (
              <span data-deck-face className="grid size-glass-item-h flex-none place-items-center">
                <TabAvatar f={face} bare="size-tab-face" seat="tab" />
              </span>
            )}
            {onTitle ? (
              <button type="button" data-deck-title onClick={onTitle} className="min-w-0 flex-1 truncate text-left text-body">
                {face?.label ?? ""}
              </button>
            ) : (
              <span data-deck-title className="min-w-0 flex-1 truncate text-body">
                {face?.label ?? ""}
              </span>
            )}
            {caption && (
              <span data-region-caption className="flex-none truncate text-caption text-tertiary tabular-nums">
                {caption}
              </span>
            )}
            <span ref={setLead} data-session-bar-lead className="flex flex-none items-center empty:hidden" />
          </div>
          <span ref={setTrail} data-session-bar-trail className="flex flex-none items-center empty:hidden" />
          <GlassCircle
            data-deck-tabs
            data-tab-count
            aria-label={gui.switch_tabs(faces.length)}
            aria-expanded={sheet}
            onClick={() => setSheet(true)}
          >
            <TabsGlyph n={faces.length} />
          </GlassCircle>
        </div>
        <div ref={setUnder} data-session-bar-under className="flex flex-none flex-col empty:hidden" />
      </div>
      <TabSheet
        open={sheet}
        onOpenChange={setSheet}
        faces={faces}
        active={active}
        onFocus={(id) => {
          setSheet(false);
          onFocus(id);
        }}
        onClose={onClose}
        newLabel={newLabel}
        onNew={
          onNew &&
          (() => {
            setSheet(false);
            onNew();
          })
        }
      />
    </div>
  );
}

/** How far sideways a swipe on the pill goes before it changes tab. */
const SWIPE_STEPS = 40;

/** **Two squares and the count** (round UX-3): the tabs circle's figure —
    a page behind a page, and how many there are on the front one. */
/**
 * **Two pages, the front one holding the count** — opaque (the owner,
 * 2026-10-04, his shot `owner-tabs-icon.png`: the back square's stroke
 * showed through the front one, which was filled with the glass's own
 * translucent tint, and the number sat on a crossed line). Both squares
 * are filled with the control's ground as it reads — the glass's tint
 * laid over the page's ground (`--tabs-fill`, `tokens.css`) — so the front
 * covers the back in either theme, and both are drawn in the glyphs' one
 * ink at full strength. The figure is the glass controls' glyph box
 * (`--ctl-glyph`, 22 under a mouse, 26 under a finger) and the squares a
 * share of it, so it scales with its neighbours.
 */
function TabsGlyph({ n }: { n: number }) {
  return (
    <span data-tabs-glyph className="relative block size-ctl-glyph">
      <span
        aria-hidden="true"
        data-tabs-back
        className="absolute top-0 right-0 size-[72%] rounded-[var(--badge-radius)] border-[length:var(--badge-stroke)] border-current bg-(image:--tabs-fill) bg-background"
      />
      <span
        data-tabs-front
        className="absolute bottom-0 left-0 grid size-[72%] place-items-center rounded-[var(--badge-radius)] border-[length:var(--badge-stroke)] border-current bg-(image:--tabs-fill) bg-background text-[length:var(--badge-text)] leading-none font-title tabular-nums"
      >
        {n}
      </span>
    </span>
  );
}

/** **The tabs, as a sheet from the bottom** (round UX-3): each open tab
    with its face, its name, where it is, and × — the switcher's rows —
    and ＋ at the end, the one place a new tab is opened from the deck. */
function TabSheet({
  open,
  onOpenChange,
  faces,
  active,
  onFocus,
  onClose,
  onNew,
  newLabel,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  faces: TabFace[];
  active: string;
  onFocus: (id: string) => void;
  onClose: (id: string) => void;
  onNew?: () => void;
  /** What ＋ opens, where it is not a page (a session). */
  newLabel?: string;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent form data-tab-sheet className="max-h-[70dvh] !gap-0 overflow-y-auto">
        <DialogTitle className="sr-only">{gui.session_tabs}</DialogTitle>
        <div className="flex min-w-0 flex-col">
          {faces.map((f) => (
            <div
              key={f.id}
              data-tab-sheet-row={f.id}
              data-on={f.id === active || undefined}
              className={cn("flex min-h-list-row min-w-0 items-center gap-row rounded-md px-in", f.id === active && "bg-fill-strong")}
            >
              <button
                type="button"
                data-tab-sheet-open={f.id}
                onClick={() => onFocus(f.id)}
                className="flex min-w-0 flex-1 items-center gap-row text-left"
              >
                <TabAvatar f={f} bare="size-list-face" seat="row" />
                <span className="flex min-w-0 flex-1 flex-col">
                  <span className="truncate text-body text-foreground">{f.label}</span>
                  {(f.machine || f.path) && (
                    // A path keeps its tail, cut at the start like every
                    // path in a line too short for it (`PathCut`).
                    <PathCut text={f.machine ?? f.path} className="block min-w-0 text-callout text-muted-foreground" />
                  )}
                </span>
              </button>
              <button
                type="button"
                data-tab-sheet-close={f.id}
                aria-label={gui.close}
                onClick={() => onClose(f.id)}
                className="grid min-h-touch min-w-touch flex-none place-items-center text-muted-foreground"
              >
                <IconClose />
              </button>
            </div>
          ))}
          {onNew && (
            <button
              type="button"
              data-tab-sheet-new
              onClick={onNew}
              className="flex min-h-list-row items-center gap-row rounded-md px-in text-left text-body text-foreground"
            >
              <span className="grid size-list-face flex-none place-items-center">
                <IconPlus />
              </span>
              {newLabel ?? gui.new_tab}
            </button>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
