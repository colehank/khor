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
import { createContext, useContext, useEffect, useLayoutEffect, useMemo, useRef, useState, type PointerEvent, type ReactNode } from "react";
import { createPortal } from "react-dom";

import { MidName, TabAvatar, type TabFace } from "@/browse/TabStrip";
import { BellMark } from "@/components/BellMark";
import { IconBack, IconClose, IconPlus } from "@/components/icons";
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog";
import { cn } from "@/lib/utils";
import { WINDOW_DRAG } from "@/components/shell";
import { GlassCircle } from "@/components/Glass";
import { gui } from "@/gen/catalog";
import { DECK_FOLD, foldStep, groundUnder, inkOver, RESTS_AT_BOTTOM, UnfoldHold, type DeckInk, type FoldMemo } from "@/lib/deck-fold";
import { seatFold } from "@/lib/fold-seat";
import { onKeyboardChange } from "@/lib/soft-keyboard";
import { TRIP_MS } from "@/lib/travel";
import { footCovered } from "@/lib/viewport";

type Seats = {
  lead: HTMLElement | null;
  trail: HTMLElement | null;
  under: HTMLElement | null;
  pill: HTMLElement | null;
  progress: HTMLElement | null;
};

const SeatsContext = createContext<Seats | null>(null);

/** **The open session's verbs on the wide face** (2026-10-03): the end of
    R0's tab strip, where a page's verbs stand. A view's `lead` goes there
    when there is no bar; the provider is the wide shell's (`App`). */
export const VerbSeat = createContext<HTMLElement | null>(null);

/**
 * Something the open tab puts in the deck (round UX-3): `lead` is the
 * pill's right end (the terminal's reconnect, the page's reload), `pill`
 * its left end (the page's ← →), `trail` the circles between the pill and
 * the tabs circle (the terminal's key row, and its keyboard in 点击按钮
 * mode only), and
 * `under` the row below the deck (the keys, the input box, the address
 * while it is edited), and `progress` the slot under the pill (UX-31):
 * anything put there is the tab reloading, and the pill runs its line
 * (`PillSlot`). Without a deck around it, `under` stands where it was
 * written, `lead` goes to the wide face's verbs (`VerbSeat`) where there
 * is one, and `trail`, `pill` and `progress` are not drawn.
 */
export function BarSeat({ at, children }: { at: "lead" | "trail" | "under" | "pill" | "progress"; children: ReactNode }) {
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
  folded: foldedAsked,
  onUnfold,
  loading = false,
  editor,
  editorAbove,
  onEditCancel,
  nativeDeck = false,
  foldOver = true,
  ink,
  status,
  away = false,
  children,
}: {
  /** **The deck steps away while the 附栏's sheet is up** (UX-37 §4,
      UX-39 §1): the sheet stands on the screen's foot where the deck
      was. Hidden in place, not taken out: the content keeps its height,
      so the terminal under the sheet's half does not refit. */
  away?: boolean;
  /** **The status strip under the pill** (UX-31, the owner 2026-10-06:
      the window's foot — what is happening elsewhere — hangs under the
      deck's pill on the phone, like Safari's progress bar): the slot's
      content once no load runs. A session's; a page and a file show
      nothing after a load. */
  status?: ReactNode;
  /** **The folded capsule sits in the home indicator's band** (item 62,
      the owner 2026-10-07: half of it covered the terminal's last rows):
      the fold frees the deck's whole height to the content, which ends at
      the capsule's top (`app.css`). `false` where the content is a native
      view (a Mac window's page): the capsule keeps a room of its own
      above the band. ⚠️ Tombstone (UX-29): the capsule floated over the
      content's foot. */
  foldOver?: boolean;
  /** The capsule's ink, where the screen cannot sample what is behind it
      (the phone's native page says its ground, `page:ground`). Left out,
      the screen samples its own content under the capsule. */
  ink?: DeckInk;
  /** **The deck is drawn natively over the page** (UX-13, an iPhone's
      browser, `lib/native-deck`): the DOM deck keeps its place and its
      layout — it is what the native one is read from, and what a native
      press clicks — but is not painted. */
  nativeDeck?: boolean;
  /** **The pill is the address editor while this is given** (UX-18, the
      owner 2026-10-05: the browser has one deck): the pill's own things
      give way to the editor drawn inside its glass — the pill's ← → and
      reload with them — and 取消 stands where the tabs circle was until
      the editor is put away. */
  editor?: ReactNode;
  /** A row over the deck while the editor is up — the browser's machine
      chips (UX-42 row 13), the choices Safari stands over the keyboard. */
  editorAbove?: ReactNode;
  onEditCancel?: () => void;
  /** **The deck folds to a strip while the content is read** (round UX-3
      §3; UX-13 for every content page, `lib/deck-fold`): ‹, the pill and
      the circles give way to a thin strip with the title — and a
      hairline while a page loads — and a tap on it, a move back toward
      rest or rest itself brings the row back. The browser says it from
      its native page's `page:scroll` and passes it here; left out, the
      screen folds by its own content's scrollers (a terminal's, a
      conversation's: rest is their bottom). */
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
  const [progress, setProgress] = useState<HTMLElement | null>(null);
  const [sheet, setSheet] = useState(false);
  const seats = useMemo(() => ({ lead, trail, under, pill, progress }), [lead, trail, under, pill, progress]);
  const at = faces.findIndex((f) => f.id === active);
  const face = at < 0 ? null : faces[at];
  const [own, setOwn] = useState(false);
  /** The strip pressed holds the deck through the flick (`UnfoldHold`). */
  const hold = useRef(new UnfoldHold());
  const [keys, setKeys] = useState(footCovered);
  const asked = foldedAsked !== undefined;
  // Nothing folds with the keyboard up, the browser's page included.
  const folded = !keys && (foldedAsked ?? own);
  const content = useRef<HTMLDivElement>(null);
  const bar = useRef<HTMLDivElement>(null);
  /** **The capsule's ink from what is behind it** (UX-29): the content
      under the capsule's centre, sampled as the fold happens and as the
      content scrolls on, a few times a second. */
  const [sampled, setSampled] = useState<DeckInk>("dark");
  const sampledAt = useRef(0);
  const sample = () => {
    const cap = bar.current?.querySelector<HTMLElement>("[data-deck-strip]");
    if (!cap) return;
    const b = cap.getBoundingClientRect();
    setSampled(inkOver(groundUnder(b.left + b.width / 2, b.top + b.height / 2, bar.current)));
  };
  /** **Midway between the last line and the foot** (item 72, `lib/fold-seat`):
      the capsule is lifted into the part-row a terminal's grid leaves
      empty above the band. Seated before the ink is sampled under it. */
  useLayoutEffect(() => {
    const room = bar.current?.querySelector<HTMLElement>("[data-deck-fold]");
    if (!folded || !room || !content.current) return;
    return seatFold(room, content.current);
  }, [folded, active]);
  useLayoutEffect(() => {
    if (folded && ink === undefined) sample();
  }, [folded, ink, active]);
  /** The capsule pressed: the deck back, held through the flick. */
  const unfold = () => {
    hold.current.asked();
    setOwn(false);
    onUnfold?.();
  };
  /** **Folding by the content's own scrollers** (UX-13): a terminal's or a
      conversation's, measured from their bottom. A move away or back
      counts only while the scroller kept its height — the fold itself
      grows the content, which is not anybody reading — but reaching the
      bottom always unfolds. Nothing folds with the keyboard up. A new
      tab starts unfolded. */
  useEffect(() => {
    setOwn(false);
    if (asked) return;
    const el = content.current;
    if (!el) return;
    const memo: FoldMemo = { turned: null };
    const last = new WeakMap<Element, { away: number; h: number }>();
    const on = (e: Event) => {
      const t = e.target;
      if (!(t instanceof HTMLElement) || !t.matches(RESTS_AT_BOTTOM)) return;
      // A terminal kept mounted under the one on screen scrolls on its
      // own output; it is not what the person reads.
      if (t.closest("[data-kept-terminal]:not([data-active])")) return;
      const now = { away: Math.max(0, t.scrollHeight - t.clientHeight - t.scrollTop), h: t.clientHeight };
      const was = last.get(t);
      last.set(t, now);
      if (!was || footCovered()) return;
      // Rest is rest whatever the height did; a direction is read only
      // from a scroller that kept its height.
      if (was.h !== now.h) {
        if (now.away <= 1) {
          hold.current.asked();
          setOwn(false);
        }
        return;
      }
      const next = foldStep(memo, now.away, now.away > was.away, 1);
      // **Under a finger a terminal folds by the finger** (item 50,
      // `DECK_FOLD` below): its scroller's travel only brings the deck back.
      if (next === true && t.matches("[data-term-scroller]") && window.matchMedia("(pointer: coarse)").matches) return;
      if (!hold.current.allows() && next === true) return;
      // **An unfold holds the deck through the refit** (UX-29): the deck
      // coming back shrinks the content, a terminal refits its grid, and
      // the scroller's geometry settles over a few events — measured 60 →
      // 64 px from the bottom with the same height, which read as leaving
      // and folded it straight back. The same hold as the strip's tap.
      if (next === false) hold.current.asked();
      if (next !== null) setOwn(next);
      if (ink === undefined && e.timeStamp - sampledAt.current > SAMPLE_MS && bar.current?.querySelector("[data-deck-strip]")) {
        sampledAt.current = e.timeStamp;
        sample();
      }
    };
    el.addEventListener("scroll", on, { capture: true, passive: true });
    // **The terminal's finger folds it too** (item 50, `DECK_FOLD`): over
    // a program holding the mouse the drag is the program's wheel and the
    // scroll rule above reads nothing; the pane tells the deck instead.
    const told = (e: Event) => {
      if (footCovered()) return;
      const fold = (e as CustomEvent<boolean>).detail;
      if (!fold) hold.current.asked();
      setOwn(fold);
    };
    el.addEventListener(DECK_FOLD, told);
    return () => {
      el.removeEventListener("scroll", on, { capture: true });
      el.removeEventListener(DECK_FOLD, told);
    };
  }, [active, asked]);
  useEffect(
    () =>
      onKeyboardChange(() => {
        // The system keyboard, or khor's panel in its place (UX-31):
        // nothing folds over either.
        const up = footCovered();
        setKeys(up);
        if (up) setOwn(false);
      }),
    [],
  );
  // ⚠️ Tombstone (item 44, the terminal lane, 2026-10-06): a folded
  // terminal kept its grid here — whole rows of what the fold freed went
  // to a band the terminal drew history into (`data-folded`,
  // `--fold-band`, term's item 32), and the rest of the deck's height
  // stayed reserved as the deck's ground. The owner withdrew 「网格不重排」:
  // the fold frees the deck's whole height to the content, the terminal's
  // box grows by it and its grid refits, as it does for the keyboard.
  /** **A sideways swipe on the pill is the next or previous tab**
      (Safari's address bar). Across the pill only, so the content keeps
      its own gestures; a swipe shorter than a third of a circle is a
      press. */
  // ⚠️ Tombstone (item 28, the terminal lane): a 24-pt drag down anywhere over
  // the content lowered the keyboard here. The key row is a portal into
  // the bar, so React's synthetic touch events from the joystick bubbled
  // up into it and the keyboard dropped under the thumb. The one lowering
  // rule is now the terminal's swipe, on its own scroller
  // (`TerminalPane`'s `swipeLowers`).
  return (
    <div data-session-screen data-deck-folded={folded || undefined} className="flex h-full min-h-0 flex-col bg-card">
      {/* The top of the glass is the platform's: a phone's status bar
          (`--safe-top`), or on a Mac window dragged narrow the corner its
          three buttons stand in (`app.css`, `[data-session-top]` under
          `data-traffic`) — which is a handle on the window like L0 was. */}
      <div data-session-top {...WINDOW_DRAG} className="h-safe-top flex-none" />
      <SeatsContext.Provider value={seats}>
        <div ref={content} data-session-content className="flex min-h-0 flex-1 flex-col">
          {children}
        </div>
      </SeatsContext.Provider>
      {/* **The deck** (round UX-3, the owner's ruling 2026-10-04): one
          row of glass at the foot — ‹ back to the list, the pill with the
          tab's own things, and the tabs circle. Nothing at the top; the
          content has the screen. Hidden while the keyboard is up, when
          the row under it (the box, the address) rides the keyboard
          alone (`app.css`) — but a terminal's stays, over its key row
          (UX-12, `views/TerminalPane`'s `KeyRowSeats`). */}
      <div
        ref={bar}
        data-session-bar
        data-folded={folded || undefined}
        data-native-deck={nativeDeck || undefined}
        data-fold-over={foldOver || undefined}
        data-deck-away={away || undefined}
        // The deck steps away while the sheet is up (UX-37 §4; the owner
        // 2026-10-06 21:05 again: 「展开了不要有下面的药丸这一行」).
        className={cn("flex flex-none flex-col pb-safe-bottom", away && "invisible")}
      >
        {folded && (
          // **Safari's capsule** (UX-29, the owner 2026-10-06): one small
          // glass capsule centred at the foot with the title alone, its
          // ink from what is behind it. **In the home indicator's band**
          // (item 62): with nothing seated under the deck it takes the
          // band (`--deck-fold`, `app.css`) and the content ends at its
          // top; a row seated under the deck (a conversation's box, the
          // key row) or a native page (`foldOver` false) gives it a room
          // of its own above that row. Never over the content. The whole
          // room is its press.
          // ⚠️ 墓碑 (UX-13): a 24-pt strip across the width. ⚠️ Tombstone
          // (UX-29): the capsule floated over the content's foot.
          <div
            data-deck-fold
            onClick={(e) => {
              if (e.target === e.currentTarget) unfold();
            }}
            className="relative flex h-[calc(var(--deck-strip)+var(--gap-in))] flex-none items-end justify-center"
          >
            <button
              type="button"
              data-deck-strip
              data-deck-ink={ink ?? sampled}
              onClick={unfold}
              className="glass absolute bottom-[var(--gap-in)] flex h-deck-strip max-w-[calc(100%-2*var(--page-inset))] animate-in items-center justify-center overflow-hidden rounded-full px-row text-callout fade-in"
            >
              <span className="min-w-0 truncate">{face?.label ?? ""}</span>
              {/* **One steady pulse** (UX-22): drawn the whole time and
                  lit while the page loads, so a load that hops does not
                  restart the breath — mounted per state, every hop began
                  it again from its first frame. */}
              <span
                data-deck-strip-loading={loading || undefined}
                aria-hidden="true"
                className={cn(
                  "absolute inset-x-0 bottom-0 h-px transition-opacity duration-[var(--dur-420)]",
                  !loading && "opacity-0",
                )}
              >
                <span className="block h-full animate-pulse bg-brand-strong" />
              </span>
            </button>
          </div>
        )}
        {editor && editorAbove && (
          <div data-deck-editor-above className="flex-none pt-in">
            {editorAbove}
          </div>
        )}
        <div
          data-session-deck
          {...WINDOW_DRAG}
          hidden={folded}
          // One glass row tall with the pill at one row; a pill grown by
          // its slot (UX-31) grows the deck under the circles, which keep
          // the first row.
          className="flex min-h-[calc(var(--glass-btn)+2*var(--gap-in))] flex-none items-start gap-in px-row py-in"
        >
          {/* ‹ gives the editor its room (UX-38 §3): Safari's address
              field runs the width, with 取消 beside it. */}
          {!editor && (
            <GlassCircle aria-label={gui.back} data-back onClick={onBack}>
              <IconBack />
            </GlassCircle>
          )}
          {editor ? (
            <div data-deck-pill data-deck-editing className="glass flex h-glass-btn min-w-0 flex-1 items-center rounded-full">
              {editor}
            </div>
          ) : (
            // **The pill and its slot** (UX-31): the seats in one row of
            // the capsule's height, and under them one slot — the
            // progress line along the pill's own bottom edge while the tab
            // reloads, else the status strip for a session — in the same
            // glass. The corner is the one-row capsule's, so a pill grown
            // by a strip keeps the same ends.
            <div
              data-deck-pill
              aria-label={gui.session_tabs}
              className="glass relative flex min-w-0 flex-1 flex-col overflow-hidden rounded-glass-btn"
            >
              <div data-deck-pill-row className="flex h-glass-btn min-w-0 items-center gap-in ps-1 pe-1">
                <span ref={setPill} data-deck-pill-lead className="flex flex-none items-center empty:hidden" />
                <DeckBand faces={faces} active={active} onFocus={onFocus} onTitle={onTitle} />
                {caption && (
                  // Gives way before the pill's own seats do (UX-22: a page's
                  // failure and its 重试 pushed reload out of the pill).
                  <span data-region-caption className="flex min-w-0 shrink items-center truncate text-caption text-tertiary tabular-nums">
                    {caption}
                  </span>
                )}
                <span ref={setLead} data-session-bar-lead className="flex flex-none items-center empty:hidden" />
              </div>
              <PillSlot loading={loading} seat={setProgress}>
                {status}
              </PillSlot>
            </div>
          )}
          {editor ? (
            <button
              type="button"
              data-deck-edit-cancel
              onClick={onEditCancel}
              className="glass flex h-glass-btn flex-none items-center rounded-full px-row text-body"
            >
              {gui.delete_keep}
            </button>
          ) : (
            <>
              <span ref={setTrail} data-session-bar-trail className="flex flex-none items-center gap-in empty:hidden" />
              <GlassCircle
                data-deck-tabs
                data-tab-count
                aria-label={gui.switch_tabs(faces.length)}
                aria-expanded={sheet}
                onClick={() => setSheet(true)}
              >
                <TabsGlyph n={faces.length} />
              </GlassCircle>
            </>
          )}
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

/** How often, at most, the folded capsule looks at what is behind it. */
const SAMPLE_MS = 250;

/**
 * **One slot under every tab pill** (UX-31 §3, the owner 2026-10-06):
 * while the tab reloads — a session's re-attach, a page's load, a file's
 * re-read — it is Safari's progress line, a 2-pt run along the pill's own
 * bottom edge, left to right, in the pill's glass; when the load ends it
 * is empty and folds. A session shows the status strip there after
 * (`status`); a page and a file show nothing. Progress and status never
 * share it: progress wins while a load runs (`app.css` hides the status
 * under a running line).
 *
 * Two ways to say "loading": the screen's own `loading` (the browser's
 * page) and the `progress` seat, which the view that holds the tab fills
 * while it reloads (`BarSeat at="progress"`: a terminal's join). The
 * wide strip's pills draw the same line from the same fact (UX-33 §2).
 *
 * ⚠️ 墓碑 (UX-36 → UX-39 §4, the owner 2026-10-06 22:10): a session's
 * slot held its day at rest, the 24 hours flattened along the pill's
 * foot. No pill draws a day now; it is 最近's first row.
 */
export function PillSlot({
  loading = false,
  seat,
  children,
}: {
  loading?: boolean;
  seat: (el: HTMLElement | null) => void;
  children?: ReactNode;
}) {
  return (
    <div data-pill-slot className="flex min-w-0 flex-col">
      <span ref={seat} data-pill-progress-seat className="contents" />
      {loading && <PillProgress />}
      {children && (
        <div data-pill-status className="flex min-w-0 items-center">
          {children}
        </div>
      )}
    </div>
  );
}

/** The progress line itself: a view portals one into the `progress` seat
    while its tab reloads; the screen draws one for `loading`. With a
    `share` (item 60, a drop landing) it stands filled that far, 0 to 1,
    instead of running. */
export function PillProgress({ share }: { share?: number } = {}) {
  return (
    <span data-pill-progress={share ?? ""} aria-hidden="true" className="pointer-events-none absolute inset-x-0 bottom-0 h-progress-h overflow-hidden">
      <span
        className="block h-full w-full origin-left bg-brand-strong"
        style={share === undefined ? undefined : { transform: `scaleX(${share})` }}
      />
    </span>
  );
}

/**
 * **The phone's deck pill is a band** (UX-33 §3, the owner 2026-10-06):
 * at rest exactly one tab — the current one's face and name, the name cut
 * at the band's edge — and under it a track of three, the previous, this
 * and the next. A sideways drag first pans the name's own overflow; at
 * its end it keeps going and pulls **one** neighbour in, never more;
 * released past half, that tab becomes current; released short, it
 * springs back. More than one neighbour: the tabs circle. A tap anywhere
 * on the current segment — face, name or the room after it — is the
 * title's own press (the address editor, the 目录 sheet; UX-35 §1). The
 * face seat takes the shared face assembly (`TabAvatar`, ux-b's). ⚠️ 墓碑
 * (UX-25): a sideways swipe stepped one tab per flick.
 */
export function DeckBand({
  faces,
  active,
  onFocus,
  onTitle,
}: {
  faces: TabFace[];
  active: string;
  onFocus: (id: string) => void;
  onTitle?: () => void;
}) {
  const band = useRef<HTMLDivElement>(null);
  const track = useRef<HTMLDivElement>(null);
  const at = faces.findIndex((f) => f.id === active);
  const cur = at < 0 ? null : faces[at];
  const prev = at > 0 ? faces[at - 1] : null;
  const next = at >= 0 && at + 1 < faces.length ? faces[at + 1] : null;
  /** The track's x, and the drag under way. */
  const x = useRef(0);
  const drag = useRef<{ x0: number; y0: number; start: number; moved: boolean; onTitle: boolean } | null>(null);
  const put = (nx: number, animate: boolean) => {
    const t = track.current;
    if (!t) return;
    x.current = nx;
    t.style.transition = animate ? `transform ${TRIP_MS}ms var(--ease-move, ease)` : "none";
    t.style.transform = `translateX(${nx}px)`;
  };
  /** The track's geometry: `home` is this tab's segment at the band's
      left edge, `over` the name's own travel past the band. */
  const geometry = () => {
    const b = band.current;
    const t = track.current;
    if (!b || !t) return null;
    const bw = b.clientWidth;
    const seg = [...t.children] as HTMLElement[];
    const prevW = prev ? seg[0].offsetWidth : 0;
    const curW = seg[prev ? 1 : 0]?.offsetWidth ?? bw;
    return { bw, home: -prevW, over: Math.max(0, curW - bw) };
  };
  // Home on every change of tab or of the band's width.
  useLayoutEffect(() => {
    const b = band.current;
    if (!b) return;
    const settle = () => {
      b.style.setProperty("--band-w", `${b.clientWidth}px`);
      const g = geometry();
      if (g) put(g.home, false);
    };
    settle();
    const ro = new ResizeObserver(settle);
    ro.observe(b);
    return () => ro.disconnect();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active, faces.length]);
  const onPointerDown = (e: PointerEvent) => {
    drag.current = {
      x0: e.clientX,
      y0: e.clientY,
      start: x.current,
      moved: false,
      // Anywhere on the current segment, its face included (UX-35 §1):
      // a short name left most of the pill deaf to a tap.
      onTitle: e.target instanceof Element && e.target.closest('[data-band-seg="cur"]') !== null,
    };
    band.current?.setPointerCapture(e.pointerId);
  };
  const onPointerMove = (e: PointerEvent) => {
    const d = drag.current;
    const g = geometry();
    if (!d || !g) return;
    const dx = e.clientX - d.x0;
    // A press that travels, in any direction, is not a tap; only its
    // sideways part pans the band.
    if (!d.moved && Math.hypot(dx, e.clientY - d.y0) < 6) return;
    d.moved = true;
    // Never past one neighbour: the previous fully in, or the next fully
    // in — and no neighbour there, no further than the name's own travel.
    const left = next ? g.home - g.over - g.bw : g.home - g.over;
    const right = prev ? g.home + g.bw : g.home;
    put(Math.min(right, Math.max(left, d.start + dx)), false);
  };
  const onPointerUp = () => {
    const d = drag.current;
    drag.current = null;
    const g = geometry();
    if (!d || !g) return;
    // **A press that did not move is the title's** (the address editor,
    // the 目录 sheet), wherever on the current segment it began: with the
    // pointer captured by the band the click lands here, not on the
    // title's button — that one answers the keyboard. ⚠️ 墓碑 (UX-35 §1):
    // only a press begun on the name's text counted, so 「khor」 left the
    // rest of the pill doing nothing.
    if (!d.moved) {
      if (d.onTitle) onTitle?.();
      return;
    }
    const nx = x.current;
    if (prev && nx > g.home + g.bw * 0.5) {
      put(g.home + g.bw, true);
      onFocus(prev.id);
      return;
    }
    if (next && nx < g.home - g.over - g.bw * 0.5) {
      put(g.home - g.over - g.bw, true);
      onFocus(next.id);
      return;
    }
    put(Math.min(g.home, Math.max(g.home - g.over, nx)), true);
  };
  const segment = (f: TabFace, role: "prev" | "cur" | "next") => (
    <div
      key={f.id}
      data-band-seg={role}
      className={cn(
        "flex h-full min-w-[var(--band-w)] flex-none items-center gap-in whitespace-nowrap pe-row",
        // A file going in keeps to the band: its name is cut in the
        // middle so the count after it always shows (the owner
        // 2026-10-07 13:05), not panned.
        f.drop && "w-[var(--band-w)]",
        role === "cur" ? "text-foreground" : "text-tertiary",
      )}
    >
      <span data-deck-face className="grid size-glass-item-h flex-none place-items-center">
        <TabAvatar f={f} bare="size-tab-face" seat="tab" />
      </span>
      {f.rang && <BellMark />}
      {role === "cur" && onTitle ? (
        <button
          type="button"
          data-deck-title
          onClick={(e) => {
            if (e.detail === 0) onTitle();
          }}
          // The finger's floor on the title (the 44 touch rule): the band
          // row is 36 tall, so the button wears it and the row clips it.
          className={cn("flex min-h-touch items-center text-left text-body", f.drop && "min-w-0 flex-1")}
        >
          {f.drop ? <MidName text={f.drop.name} even /> : f.label}
        </button>
      ) : (
        <span data-deck-title={role === "cur" ? "" : undefined} className={cn("text-body", f.drop && "flex min-w-0 flex-1")}>
          {f.drop ? <MidName text={f.drop.name} even /> : f.label}
        </span>
      )}
    </div>
  );
  return (
    <div
      ref={band}
      data-deck-band
      className="relative h-full min-w-0 flex-1 touch-none overflow-hidden"
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={onPointerUp}
    >
      <div ref={track} data-deck-band-track className="absolute inset-y-0 left-0 flex items-center will-change-transform">
        {prev && segment(prev, "prev")}
        {cur && segment(cur, "cur")}
        {next && segment(next, "next")}
      </div>
    </div>
  );
}

// ⚠️ 墓碑 (UX-25 → UX-33): `usePillSwipe`, a sideways flick on the pill
// stepping one tab; the band above is the one pill gesture now.

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
export function TabsGlyph({ n }: { n: number }) {
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
export function TabSheet({
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
                {/* **One line, the title** (UX-20). ⚠️ 墓碑: the machine or
                    the path under it, cut at the start (`PathCut`). */}
                <span data-tab-sheet-name className="min-w-0 flex-1 truncate text-body text-foreground">
                  {f.title ?? f.label}
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
