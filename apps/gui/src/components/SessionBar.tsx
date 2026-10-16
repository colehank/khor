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
// the same tab faces (`TabAvatar`, `MidName`), the same `SessionFace` on
// every tab, laid out in a different order: the tabs leave the top of the
// board for the foot of the screen. ⚠️ 墓碑 (ux-q): `TabStrip`
// `at="bottom"`, the wide strip hung from the content — this bar draws
// its own band.
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
import { NoticeIsland, NoticeLook, NoticeScrim, useGrownPill } from "@/components/PillNotice";
import { gui } from "@/gen/catalog";
import { DECK_FOLD, foldStep, groundUnder, inkOver, RESTS_AT_BOTTOM, UnfoldHold, type DeckInk, type FoldMemo } from "@/lib/deck-fold";
import { seatFold } from "@/lib/fold-seat";
import { onKeyboardChange } from "@/lib/soft-keyboard";
import { feel } from "@/lib/haptics";
import { stillMotion } from "@/lib/travel";
import { usePillSignal } from "@/lib/pill-signal";
import { footCovered } from "@/lib/viewport";

type Seats = {
  lead: HTMLElement | null;
  trail: HTMLElement | null;
  above: HTMLElement | null;
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
 * `above` the conversation composer above the deck, `under` the
 * terminal key row below the deck, and `progress` the slot under the pill (UX-31):
 * anything put there is the tab reloading, and the pill runs its line
 * (`PillSlot`). Without a deck around it, `above` and `under` stand where they were
 * written, `lead` goes to the wide face's verbs (`VerbSeat`) where there
 * is one, and `trail`, `pill` and `progress` are not drawn.
 */
export function BarSeat({ at, children }: { at: keyof Seats; children: ReactNode }) {
  const seats = useContext(SeatsContext);
  const verbs = useContext(VerbSeat);
  if (!seats) {
    if (at === "above" || at === "under") return <>{children}</>;
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
  away = false,
  contentMode = false,
  onTabs,
  children,
}: {
  /** **The deck steps away while the 附栏's sheet is up** (UX-37 §4,
      UX-39 §1): the sheet stands on the screen's foot where the deck
      was. Hidden in place, not taken out: the content keeps its height,
      so the terminal under the sheet's half does not refit. */
  away?: boolean;
  /** Expanded session content keeps the terminal's measured deck room. */
  contentMode?: boolean;
  /** A host can present the same tab rows in its existing content sheet. */
  onTabs?: () => void;
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
  const [above, setAbove] = useState<HTMLElement | null>(null);
  const [under, setUnder] = useState<HTMLElement | null>(null);
  const [pill, setPill] = useState<HTMLElement | null>(null);
  const [progress, setProgress] = useState<HTMLElement | null>(null);
  const [sheet, setSheet] = useState(false);
  const seats = useMemo(() => ({ lead, trail, above, under, pill, progress }), [lead, trail, above, under, pill, progress]);
  const at = faces.findIndex((f) => f.id === active);
  const face = at < 0 ? null : faces[at];
  /** **The pill grows while something stops this session** (NOTICE-BRIEF):
      a press on it opens the grown view instead of the 底栏; a transfer
      under way and a plain pill open the 底栏 as before. */
  const stops = face?.interaction?.notices ?? [];
  const signal = usePillSignal(active, face?.interaction);
  const grown = useGrownPill(stops.length);
  /** Grown and something to grow into: the island stands, the pill hides. */
  const island = !contentMode && grown.growth.mounted && stops.length > 0;
  const [deckPill, setDeckPill] = useState<HTMLElement | null>(null);
  const foldGrown = grown.fold;
  useEffect(() => foldGrown(), [active, foldGrown]);
  const [own, setOwn] = useState(false);
  /** The strip pressed holds the deck through the flick (`UnfoldHold`). */
  const hold = useRef(new UnfoldHold());
  const [keys, setKeys] = useState(footCovered);
  const asked = foldedAsked !== undefined;
  // Nothing folds with the keyboard up, the browser's page included.
  const folded = !contentMode && !keys && (foldedAsked ?? own);
  const content = useRef<HTMLDivElement>(null);
  const bar = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    const el = bar.current;
    if (!el || contentMode) return;
    const read = () => el.style.setProperty('--content-deck-reserve', el.getBoundingClientRect().height + 'px');
    const observer = new ResizeObserver(read); observer.observe(el); read();
    return () => observer.disconnect();
  }, [contentMode]);
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
    <div data-session-screen data-content-mode={contentMode || undefined} data-deck-folded={folded || undefined} className="relative flex h-full min-h-0 flex-col bg-card">
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
        // 2026-10-06 21:05 again: 「展开了不要有下面的药丸这一行」). Over
        // the scrim while the pill is grown.
        className={cn("flex flex-none flex-col pb-safe-bottom", away && "invisible", island && "relative z-50")}
      >
        <div ref={setAbove} data-session-bar-above className="flex flex-none flex-col empty:hidden" />
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
              data-pill-signal={signal}
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
          className="relative flex min-h-[calc(var(--glass-btn)+2*var(--gap-in))] flex-none items-start gap-in px-row py-in"
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
              ref={setDeckPill}
              data-deck-pill
              data-pill-signal={signal}
              data-tap={stops.length ? "grow" : "sheet"}
              aria-label={gui.session_tabs}
              className="glass relative flex min-w-0 flex-1 flex-col overflow-hidden rounded-glass-btn"
              // While grown the pill is the island: one piece of glass.
              style={island ? { visibility: "hidden" } : undefined}
            >
              <div data-deck-pill-row className="flex h-glass-btn min-w-0 items-center gap-in ps-1 pe-1">
                <span ref={setPill} data-deck-pill-lead className="flex flex-none items-center empty:hidden" />
                <DeckBand faces={faces} active={active} onFocus={onFocus} onTitle={stops.length ? grown.grow : onTitle} />
                {caption && (
                  // Gives way before the pill's own seats do (UX-22: a page's
                  // failure and its 重试 pushed reload out of the pill).
                  <span data-region-caption className="flex min-w-0 shrink items-center truncate text-caption text-tertiary tabular-nums">
                    {caption}
                  </span>
                )}
                <span ref={setLead} data-session-bar-lead className="flex flex-none items-center empty:hidden" />
              </div>
              <PillSlot loading={loading} seat={setProgress} />
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
                onClick={() => onTabs ? onTabs() : setSheet(true)}
              >
                <TabsGlyph n={faces.length} />
              </GlassCircle>
            </>
          )}
          {island && !editor && <NoticeIsland list={stops} pill={deckPill} growth={grown.growth} seat="size-glass-item-h" />}
        </div>
        <div ref={setUnder} data-session-bar-under className="flex flex-none flex-col empty:hidden" />
      </div>
      {island && <NoticeScrim on={grown.growth.big} openedAt={grown.openedAt} onFold={grown.fold} />}
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
 * is empty. **The pill never grows by it** (NOTICE-BRIEF, the owner
 * 2026-10-08): what stops the session is said in the pill's own row
 * (`components/PillNotice`), and nothing else is. ⚠️ 墓碑 (UX-31 →
 * NOTICE-BRIEF): the window's foot — other rows going busy or failing —
 * hung under a session's pill as a status strip, growing the pill a row.
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
export function PillSlot({ loading = false, seat }: { loading?: boolean; seat: (el: HTMLElement | null) => void }) {
  return (
    <div data-pill-slot className="flex min-w-0 flex-col">
      <span ref={seat} data-pill-progress-seat className="contents" />
      {loading && <PillProgress />}
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
 * **The phone's deck pill is a band** (UX-33 §3, the owner 2026-10-06;
 * UX-49, 2026-10-08): at rest exactly one tab — the current one's face
 * and name, the name cut at the band's edge. A sideways drag moves it
 * and pulls **one** neighbour in, never more.
 *
 * - **A ring** (UX-49 §2): the first tab's previous is the last, the
 *   last one's next the first; with two, either way is the other. With
 *   one the band gives a little (`GIVE`) and springs back.
 * - **The incoming tab hugs the edge it comes in from** (UX-49 §3): from
 *   the right it is right-aligned, from the left left-aligned, in the
 *   room the outgoing one has left — its face and name show from the
 *   first pixel, whatever the names' lengths, and a name too long for
 *   the room is cut. A switch glides it to its left-aligned rest
 *   (`GLIDE_MS`); a drag that does not switch springs back, no glide.
 * - **Letting go switches past `COMMIT_SHARE` of the band, or on a
 *   flick** (UX-49 §5). Crossing that line either way is felt (a light
 *   impact, `feel("threshold")`) and seen: below it the incoming tab is
 *   dim (`DIM`) and the outgoing full; past it the other way round.
 * - **Every end of a drag is one tab at rest** (UX-49 §4): a lift, a
 *   cancel, the system taking the touch, the capture lost, the list or
 *   the front tab changing under the finger.
 *
 * More than one neighbour away: the tabs circle. A tap anywhere on the
 * current segment — face, name or the room after it — is the title's own
 * press (the address editor, the 目录 sheet; UX-35 §1). The face seat
 * takes the shared face assembly (`TabAvatar`, ux-b's). ⚠️ 墓碑 (UX-25):
 * a sideways swipe stepped one tab per flick. ⚠️ 墓碑 (UX-33 → UX-49): a
 * track of three segments, each at least the band wide, panned as one; a
 * drag first panned the current name's overflow, and a release short of
 * half kept that pan — the owner's 「停在一半」, the old tab's face half
 * outside the pill — while a neighbour's face came in only after the
 * blank rest of its segment.
 */
export function DeckBand({
  faces,
  active,
  onFocus,
  onTitle,
  renderFace,
}: {
  faces: TabFace[];
  active: string;
  onFocus: (id: string) => void;
  onTitle?: () => void;
  /** Reuse the same drag for nested documents and browsing sources. */
  renderFace?: (face: TabFace, role: "prev" | "cur" | "next") => ReactNode;
}) {
  const band = useRef<HTMLDivElement>(null);
  const curSeg = useRef<HTMLDivElement>(null);
  const prevSeg = useRef<HTMLDivElement>(null);
  const nextSeg = useRef<HTMLDivElement>(null);
  const n = faces.length;
  const at = faces.findIndex((f) => f.id === active);
  const cur = at < 0 ? null : faces[at];
  const prev = at >= 0 && n > 1 ? faces[(at + n - 1) % n] : null;
  const next = at >= 0 && n > 1 ? faces[(at + 1) % n] : null;
  const drag = useRef<BandDrag | null>(null);
  /** A switch gliding in: where to, and when it lands. */
  const going = useRef<{ id: string; timer: number } | null>(null);
  /** A spring back's end, which puts the neighbour away. */
  const springing = useRef<number | null>(null);
  const latest = useRef({ faces, active, onFocus });
  useLayoutEffect(() => {
    latest.current = { faces, active, onFocus };
  });
  const ease = () => getComputedStyle(document.documentElement).getPropertyValue("--ease-move").trim() || "ease";
  const glide = () => (stillMotion() ? 0 : GLIDE_MS);
  /** Exactly the current tab, nothing moving: every inline style the
      gesture wrote taken off (the segments have none of React's). */
  const rest = () => {
    if (springing.current !== null) window.clearTimeout(springing.current);
    springing.current = null;
    for (const el of [curSeg.current, prevSeg.current, nextSeg.current]) el?.removeAttribute("style");
    band.current?.removeAttribute("data-band-past");
  };
  /** The drag and any switch under way, dropped where they are. */
  const stop = () => {
    const d = drag.current;
    drag.current = null;
    const b = band.current;
    if (d && b?.hasPointerCapture(d.pointer)) b.releasePointerCapture(d.pointer);
    if (going.current) window.clearTimeout(going.current.timer);
    going.current = null;
  };
  // At rest on every change of the front tab or of the list — a drag or
  // a glide under way included (UX-49 §4). The band's width is measured
  // for a file's name; a width that changes under a finger (a seat filled
  // beside the band) leaves the drag going, its next move drawn to the
  // new width.
  const ids = faces.map((f) => f.id).join("\n");
  useLayoutEffect(() => {
    const b = band.current;
    if (!b) return;
    stop();
    rest();
    const measure = () => b.style.setProperty("--band-w", `${b.clientWidth}px`);
    measure();
    const ro = new ResizeObserver(measure);
    ro.observe(b);
    return () => ro.disconnect();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active, ids]);
  useEffect(
    () => () => {
      if (going.current) window.clearTimeout(going.current.timer);
      if (springing.current !== null) window.clearTimeout(springing.current);
    },
    [],
  );
  /** The drag drawn: the current tab moved by `dx`, the incoming one in
      the room it left, hugging its own edge, and the cue. */
  const pull = (dx: number, past: boolean) => {
    const c = curSeg.current;
    if (!c) return;
    c.style.transition = `opacity var(--dur-120) linear`;
    c.style.transform = `translateX(${dx}px)`;
    c.style.opacity = past ? String(DIM) : "";
    const inc = dx < 0 ? nextSeg.current : dx > 0 ? prevSeg.current : null;
    for (const el of [prevSeg.current, nextSeg.current]) {
      if (!el) continue;
      if (el !== inc) {
        el.removeAttribute("style");
        continue;
      }
      el.style.display = "flex";
      el.style.transition = `opacity var(--dur-120) linear`;
      el.style.maxWidth = `${Math.abs(dx)}px`;
      el.style.opacity = past ? "" : String(DIM);
    }
    band.current?.toggleAttribute("data-band-past", past);
  };
  /** Back to rest the way it came: the incoming tab's room closes over
      it, nothing glides. */
  const springBack = () => {
    const ms = glide();
    const c = curSeg.current;
    if (ms === 0 || !c) {
      rest();
      return;
    }
    c.style.transition = `transform ${ms}ms ${ease()}, opacity var(--dur-120) linear`;
    c.style.transform = "none";
    c.style.opacity = "";
    for (const el of [prevSeg.current, nextSeg.current]) {
      if (!el || el.style.display !== "flex") continue;
      el.style.transition = `max-width ${ms}ms ${ease()}, opacity var(--dur-120) linear`;
      el.style.maxWidth = "0";
    }
    band.current?.removeAttribute("data-band-past");
    springing.current = window.setTimeout(rest, ms);
  };
  /** The switch lands: the tab is focused, and a focus that did not
      take (refused, or the tab gone meanwhile) leaves the band at rest
      on what is current. */
  const arrive = () => {
    const g = going.current;
    if (!g) return;
    window.clearTimeout(g.timer);
    going.current = null;
    const { faces: now, active: was, onFocus: focus } = latest.current;
    if (now.some((f) => f.id === g.id)) focus(g.id);
    requestAnimationFrame(() =>
      requestAnimationFrame(() => {
        if (latest.current.active === was && !drag.current && !going.current) rest();
      }),
    );
  };
  /** Letting go switched: the outgoing tab leaves, the incoming one
      takes the band and glides from the edge it hugged to its rest. */
  const commit = (to: TabFace, way: number) => {
    const b = band.current;
    const c = curSeg.current;
    const inc = way < 0 ? nextSeg.current : prevSeg.current;
    if (!b || !c || !inc) return;
    const ms = glide();
    const bw = b.clientWidth;
    if (way < 0) {
      // From the right: unpinned from the right edge where it stands,
      // then carried to the left.
      const from = inc.getBoundingClientRect().left - b.getBoundingClientRect().left;
      inc.style.transition = "none";
      inc.style.left = "0";
      inc.style.right = "auto";
      inc.style.maxWidth = "none";
      inc.style.transform = `translateX(${from}px)`;
      void inc.offsetWidth;
      inc.style.transition = `transform ${ms}ms ${ease()}`;
      inc.style.transform = "none";
    } else {
      // From the left it is at its rest already: its room opens.
      inc.style.transition = `max-width ${ms}ms ${ease()}`;
      inc.style.maxWidth = `${bw}px`;
    }
    inc.style.opacity = "";
    c.style.transition = `transform ${ms}ms ${ease()}`;
    c.style.transform = `translateX(${way < 0 ? -bw : bw}px)`;
    c.style.opacity = "";
    b.removeAttribute("data-band-past");
    going.current = { id: to.id, timer: window.setTimeout(arrive, ms) };
  };
  const onPointerDown = (e: PointerEvent) => {
    // A press while a switch glides lands it now.
    if (going.current) {
      arrive();
      return;
    }
    if (drag.current || (e.target instanceof Element && e.target.closest("input,textarea,[contenteditable=true],[data-band-control]")) || window.getSelection()?.isCollapsed === false) return;
    drag.current = {
      pointer: e.pointerId,
      touch: e.pointerType === "touch",
      touchId: null,
      x0: e.clientX,
      y0: e.clientY,
      moved: false,
      axis: null,
      // Anywhere on the current segment, its face included (UX-35 §1):
      // a short name left most of the pill deaf to a tap.
      onTitle: e.target instanceof Element && e.target.closest('[data-band-seg="cur"]') !== null,
      dx: 0,
      past: false,
      trail: [{ t: e.timeStamp, x: e.clientX }],
    };
    if (!renderFace) band.current?.setPointerCapture(e.pointerId);
  };
  const onPointerMove = (e: PointerEvent) => {
    const d = drag.current;
    const b = band.current;
    if (!d || !b || e.pointerId !== d.pointer) return;
    const raw = e.clientX - d.x0;
    // A press that travels, in any direction, is not a tap; only its
    // sideways part moves the band.
    if (!d.moved && Math.hypot(raw, e.clientY - d.y0) < TAP_SLOP) return;
    if (d.axis === null) d.axis = Math.abs(raw) > Math.abs(e.clientY - d.y0) ? "x" : "y";
    if (d.axis === "y") {
      drag.current = null;
      if (b.hasPointerCapture(d.pointer)) b.releasePointerCapture(d.pointer);
      springBack();
      return;
    }
    if (renderFace && !b.hasPointerCapture(e.pointerId)) b.setPointerCapture(e.pointerId);
    if (!d.moved && springing.current !== null) rest();
    d.moved = true;
    d.trail = [...d.trail, { t: e.timeStamp, x: e.clientX }].filter((p) => e.timeStamp - p.t <= 2 * FLICK_MS);
    if (!prev || !next) {
      // One tab: a rubber band that gives less the further it goes.
      d.dx = Math.sign(raw) * GIVE * (1 - 1 / ((Math.abs(raw) * 0.55) / GIVE + 1));
      pull(d.dx, false);
      return;
    }
    const bw = b.clientWidth;
    d.dx = Math.min(bw, Math.max(-bw, raw));
    const past = Math.abs(d.dx) >= bw * COMMIT_SHARE;
    if (past !== d.past) {
      d.past = past;
      feel("threshold");
    }
    pull(d.dx, past);
  };
  const onPointerUp = (e: Pick<PointerEvent, "pointerId" | "clientX" | "timeStamp">) => {
    const d = drag.current;
    if (!d || e.pointerId !== d.pointer) return;
    drag.current = null;
    if (band.current?.hasPointerCapture(d.pointer)) band.current.releasePointerCapture(d.pointer);
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
    if (d.axis === "y") { springBack(); return; }
    const way = Math.sign(d.dx);
    if (!prev || !next || way === 0) {
      springBack();
      return;
    }
    // The flick: the drag's last `FLICK_MS` of movement, and none if the
    // finger stood still that long before the lift (the lift repeats the
    // last place later, which is not a slowing down). A flick back
    // against the drag lets go of it, past the line or not.
    const last = d.trail[d.trail.length - 1];
    const trail = e.clientX === last.x ? d.trail : [...d.trail, { t: e.timeStamp, x: e.clientX }];
    const end = trail[trail.length - 1];
    const first = trail.find((p) => end.t - p.t <= FLICK_MS) ?? end;
    const v = e.timeStamp - last.t <= FLICK_MS && end !== first ? (end.x - first.x) / Math.max(1, end.t - first.t) : 0;
    const flick = Math.abs(v) >= FLICK_SPEED && Math.abs(d.dx) >= FLICK_MIN;
    if (flick ? Math.sign(v) === way : d.past) commit(way < 0 ? next : prev, way);
    else springBack();
  };
  /** The touch taken away (the system's gesture, a cancel, the capture
      lost): never a switch, always back to rest. */
  const onLost = (e: PointerEvent) => {
    // Touch buttons hold an implicit capture. Transferring it to the
    // band emits a bubbling child loss before the band gains capture;
    // that transfer is not the system cancelling this drag.
    if (e.type === "lostpointercapture" && e.target !== e.currentTarget && band.current?.hasPointerCapture(e.pointerId)) return;
    const d = drag.current;
    if (!d || e.pointerId !== d.pointer) return;
    drag.current = null;
    if (d.moved) springBack();
  };
  // Native touch endings can survive an interrupted Pointer Events
  // stream. They share the same finish path: whichever arrives first
  // consumes the drag, so a normal lift cannot focus twice. Keep the
  // finger's identifier; another finger ending must not finish this one.
  const endings = useRef({ up: onPointerUp, cancel: () => { stop(); rest(); } });
  useLayoutEffect(() => {
    endings.current = { up: onPointerUp, cancel: () => { stop(); rest(); } };
  });
  useEffect(() => {
    const start = (e: TouchEvent) => {
      const d = drag.current;
      if (!d?.touch || d.touchId !== null) return;
      const t = Array.from(e.changedTouches).find((t) => t.target instanceof Node && band.current?.contains(t.target));
      if (t) d.touchId = t.identifier;
    };
    const end = (e: TouchEvent) => {
      const d = drag.current;
      if (!d?.touch) return;
      const t = Array.from(e.changedTouches).find((t) => t.identifier === d.touchId);
      if (!t) return;
      if (e.type === "touchcancel") endings.current.cancel();
      else endings.current.up({ pointerId: d.pointer, clientX: t.clientX, timeStamp: e.timeStamp });
    };
    const cancel = () => endings.current.cancel();
    const hidden = () => {
      if (document.visibilityState !== "visible") cancel();
    };
    window.addEventListener("touchstart", start, { capture: true, passive: true });
    window.addEventListener("touchend", end, { capture: true, passive: true });
    window.addEventListener("touchcancel", end, { capture: true, passive: true });
    window.addEventListener("blur", cancel);
    window.addEventListener("pagehide", cancel);
    document.addEventListener("visibilitychange", hidden);
    return () => {
      window.removeEventListener("touchstart", start, true);
      window.removeEventListener("touchend", end, true);
      window.removeEventListener("touchcancel", end, true);
      window.removeEventListener("blur", cancel);
      window.removeEventListener("pagehide", cancel);
      document.removeEventListener("visibilitychange", hidden);
    };
  }, []);
  const body = (f: TabFace, role: "prev" | "cur" | "next") =>
    // **What stops this session takes the pill's look** (NOTICE-BRIEF):
    // the kind's mark in the face's seat, the short line in the name's
    // room — each tab its own, so a swipe works the same with one showing.
    renderFace ? (role === "cur" && onTitle ? <button type="button" data-deck-title className="flex h-full min-h-touch min-w-0 flex-1 items-center text-left" onClick={e=>{if(e.detail===0)onTitle();}}>{renderFace(f,role)}</button> : renderFace(f,role)) : f.interaction?.notices?.length ? (
      role === "cur" && onTitle ? (
        <button
          type="button"
          data-deck-title
          data-deck-notice={f.interaction?.notices[0].kind}
          onClick={(e) => {
            if (e.detail === 0) onTitle();
          }}
          className="flex h-full min-h-touch min-w-0 flex-1 items-center gap-in text-left text-body"
        >
          <NoticeLook list={f.interaction?.notices} seat="size-glass-item-h" />
        </button>
      ) : (
        <span data-deck-notice={f.interaction?.notices[0].kind} className="flex h-full min-w-0 flex-1 items-center gap-in text-body">
          <NoticeLook list={f.interaction?.notices} seat="size-glass-item-h" />
        </span>
      )
    ) : (
    <>
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
          className={cn("flex min-h-touch items-center text-left text-body", f.interaction?.drop && "min-w-0 flex-1")}
        >
          {f.interaction?.drop ? <MidName text={f.interaction?.drop.name} even /> : f.label}
        </button>
      ) : (
        <span data-deck-title={role === "cur" ? "" : undefined} className={cn("text-body", f.interaction?.drop && "flex min-w-0 flex-1")}>
          {f.interaction?.drop ? <MidName text={f.interaction?.drop.name} even /> : f.label}
        </span>
      )}
    </>
    );
  /** A neighbour: put away until a drag pulls it in, then as wide as the
      room the drag opened, anchored to its own edge. A file going in
      keeps to the band: its name is cut in the middle so the count after
      it always shows (the owner 2026-10-07 13:05). */
  const neighbour = (f: TabFace, role: "prev" | "next", ref: typeof prevSeg) => (
    <div
      ref={ref}
      key={role}
      data-band-seg={role}
      aria-hidden="true"
      className={cn("absolute inset-y-0 hidden items-center overflow-hidden text-foreground", role === "prev" ? "left-0" : "right-0")}
    >
      <div className={cn("flex h-full flex-none items-center gap-in whitespace-nowrap pe-row", (f.interaction?.drop || f.interaction?.notices?.length) && "w-[var(--band-w)]")}>{body(f, role)}</div>
    </div>
  );
  return (
    <div
      ref={band}
      data-deck-band
      className="relative h-full min-w-0 flex-1 overflow-hidden"
      style={{ touchAction: renderFace ? "pan-y" : "none" }}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={onLost}
      onLostPointerCapture={onLost}
    >
      {cur && (
        <div
          ref={curSeg}
          key="cur"
          data-band-seg="cur"
          className="absolute inset-0 flex items-center gap-in overflow-hidden whitespace-nowrap pe-row text-foreground will-change-transform"
        >
          {body(cur, "cur")}
        </div>
      )}
      {prev && neighbour(prev, "prev", prevSeg)}
      {next && neighbour(next, "next", nextSeg)}
    </div>
  );
}

/** The band's drag (`DeckBand`). */
type BandDrag = {
  pointer: number;
  touch: boolean;
  touchId: number | null;
  x0: number;
  y0: number;
  moved: boolean;
  axis: "x" | "y" | null;
  onTitle: boolean;
  /** How far the current tab stands from its rest. */
  dx: number;
  /** Past the line: letting go switches. */
  past: boolean;
  /** The last `FLICK_MS` of positions, for the flick. */
  trail: { t: number; x: number }[];
};
/** **When letting go switches** (UX-49 §5): a drag across this share of
    the band — about 100 pt of an iPhone's pill. */
const COMMIT_SHARE = 0.4;
/** …or a flick: at least this fast (px per ms, 400 pt/s — Android's
    pager's fling floor) over the last `FLICK_MS` of the drag, and at
    least `FLICK_MIN` px from the start. */
const FLICK_SPEED = 0.4;
const FLICK_MS = 80;
const FLICK_MIN = 12;
/** The cue: what letting go would leave behind is drawn at this. */
const DIM = 0.4;
/** The switch's glide to rest and the spring back (「about 0.2 s」). */
const GLIDE_MS = 200;
/** One tab: how far the band gives, approached and never reached. */
const GIVE = 28;
/** A press that travels less than this is a tap. */
const TAP_SLOP = 6;

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
export type TabSheetRowsProps = {
  faces: TabFace[]; active: string; onFocus: (id: string) => void; onClose: (id: string) => void;
  onNew?: () => void; newLabel?: string;
  canClose?: (face: TabFace) => boolean;
  actions?: (face: TabFace) => ReactNode;
  rowData?: (face: TabFace) => Record<string,string|boolean|undefined>;
};
/** The same rows in the terminal tab sheet and the session's content
 * overview. Both own navigation; the row owns face, name and close geometry. */
export function TabSheetRows({faces,active,onFocus,onClose,onNew,newLabel,canClose,actions,rowData}: TabSheetRowsProps) {
  return (
        <div className="flex min-w-0 flex-col">
          {faces.map((f) => (
            <div
              {...rowData?.(f)}
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
              {actions?.(f)}
              {(!canClose || canClose(f)) && <button
                type="button"
                data-tab-sheet-close={f.id}
                aria-label={gui.close}
                onClick={() => onClose(f.id)}
                className="grid min-h-touch min-w-touch flex-none place-items-center text-muted-foreground"
              >
                <IconClose />
              </button>}
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
  );
}
export function TabSheet({open,onOpenChange,...rows}: TabSheetRowsProps & {open:boolean;onOpenChange:(open:boolean)=>void}) {
  return <Dialog open={open} onOpenChange={onOpenChange}>
    <DialogContent form data-tab-sheet className="max-h-[70dvh] !gap-0 overflow-y-auto">
      <DialogTitle className="sr-only">{gui.session_tabs}</DialogTitle>
      <TabSheetRows {...rows}/>
    </DialogContent>
  </Dialog>;
}
/** Aligned horizontal content cards; the session switcher keeps its
 * diagonal squares. The count stays in the front card in both cases. */
export function ContentTabsGlyph({n}: {n:number}) {
  return <span data-content-tabs-glyph className="content-tabs-glyph"><TabsGlyph n={n}/></span>;
}
