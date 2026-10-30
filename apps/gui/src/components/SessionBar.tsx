// GUI conversations use a model header and a bottom composer with an
// independent session avatar. Terminals keep the shared bottom deck.
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
import { createContext, useContext, useEffect, useLayoutEffect, useMemo, useRef, useState, type ReactNode, type RefObject } from "react";
import { createPortal } from "react-dom";
import { DeckBand } from "@/components/DeckBand";

import { TabAvatar, type TabFace } from "@/browse/TabStrip";
import { IconBack, IconClose, IconPlus } from "@/components/icons";
import { Dialog, TaskOverlay, DialogTitle } from "@/components/ui/dialog";
import { cn } from "@/lib/utils";
import { WINDOW_DRAG } from "@/components/shell";
import { GlassCircle } from "@/components/Glass";
import { NoticeIsland, NoticeScrim, useGrownPill } from "@/components/PillNotice";
import { gui } from "@/gen/catalog";
import { DECK_FOLD, foldStep, groundUnder, inkOver, RESTS_AT_BOTTOM, UnfoldHold, type DeckInk, type FoldMemo } from "@/lib/deck-fold";
import { seatFold } from "@/lib/fold-seat";
import { onKeyboardChange } from "@/lib/soft-keyboard";
import { usePillSignal } from "@/lib/pill-signal";
import { footCovered } from "@/lib/viewport";

type Seats = {
  conversation: boolean;
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
export function BarSeat({ at, children }: { at: Exclude<keyof Seats,"conversation">; children: ReactNode }) {
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
export function useComposeNavigation(): boolean {return !!useContext(SeatsContext)?.conversation;}

export function useInBar(): boolean {
  return useContext(SeatsContext) !== null;
}

export type SessionHeaderProps = {
  faces: TabFace[]; active: string; onFocus: (id: string) => void; onBack: () => void;
  onOverview: () => void; barRef: RefObject<HTMLDivElement | null>; children: ReactNode; hidden: boolean;
  onNew?: () => void; newLabel?: string;
};

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
  loadingShare,
  editor,
  editorAbove,
  onEditCancel,
  nativeDeck = false,
  foldOver = true,
  ink,
  away = false,
  contentMode = false,
  onTabs,
  deckAt = "bottom",
  combinedDock = false,
  renderHeader,
  navigation = false,
  children,
}: {
  /** Conversations keep model controls above their thread. */
  deckAt?: "top" | "bottom";
  navigation?: boolean;
  /** Retained for isolated historical layout comparisons. */
  combinedDock?: boolean;
  /** A conversation header, with configuration supplied by its own seat. */
  renderHeader?: (props: SessionHeaderProps) => ReactNode;
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
  /** Number: actual page estimate; null: static unknown page progress.
      Omitted preserves other kinds' existing busy indicator. */
  loadingShare?: number | null;
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
  const conversation=!!renderHeader;
  const seats = useMemo(() => ({ lead, trail, above, under, pill, progress, conversation }), [lead, trail, above, under, pill, progress, conversation]);
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
  const topDeck = deckAt === "top";
  const folded = !topDeck && !combinedDock && !contentMode && !keys && (foldedAsked ?? own);
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
    if (asked || topDeck) return;
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
  }, [active, asked, topDeck]);
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
  const deck = renderHeader ? renderHeader({faces, active, onFocus, onBack, onNew, newLabel, onOverview: () => setSheet(true), barRef: bar, hidden: away || contentMode, children: <span ref={setTrail} data-session-bar-trail className="flex flex-none items-center" />}) : (
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
        className={cn("flex flex-none flex-col", !topDeck && "pb-dock-safe-bottom", away && "invisible", island && "relative z-50")}
      >
        {!topDeck && <div ref={setAbove} data-session-bar-above className="flex flex-none flex-col empty:hidden" />}
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
                data-deck-strip-progress={loading && typeof loadingShare === "number" ? loadingShare : undefined}
                aria-hidden="true"
                className={cn(
                  "absolute inset-x-0 bottom-0 h-px transition-opacity duration-[var(--dur-420)]",
                  !loading && "opacity-0",
                )}
              >
                <span className={cn("block h-full origin-left", loadingShare === null ? "bg-fill-strong" : "bg-brand-strong", loadingShare === undefined && "animate-pulse")} style={typeof loadingShare === "number" ? {transform: `scaleX(${loadingShare})`} : undefined} />
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
                <DeckBand placement={deckAt} faces={faces} active={active} onFocus={onFocus} onTitle={stops.length ? grown.grow : onTitle} onOverview={() => onTabs ? onTabs() : setSheet(true)} />
                {caption && (
                  // Gives way before the pill's own seats do (UX-22: a page's
                  // failure and its 重试 pushed reload out of the pill).
                  <span data-region-caption className="flex min-w-0 shrink items-center truncate text-caption text-tertiary tabular-nums">
                    {caption}
                  </span>
                )}
                <span ref={setLead} data-session-bar-lead className="flex flex-none items-center empty:hidden" />
              </div>
              <PillSlot loading={loading} share={loadingShare} seat={setProgress} />
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
            </>
          )}
          {island && !editor && <NoticeIsland placement={deckAt} list={stops} pill={deckPill} growth={grown.growth} seat="size-glass-item-h" />}
        </div>
        <div ref={setUnder} data-session-bar-under className="flex flex-none flex-col empty:hidden" />
      </div>
  );
  return (
    <div data-session-screen data-session-navigation-root={navigation||undefined} data-deck-at={deckAt} data-content-mode={contentMode || undefined} data-deck-folded={folded || undefined} className="relative flex h-full min-h-0 flex-col bg-card">
      <div data-session-top {...WINDOW_DRAG} className="h-safe-top flex-none" />
      {topDeck && deck}
      <SeatsContext.Provider value={seats}>
        <div ref={content} data-session-content className="flex min-h-0 flex-1 flex-col">
          {children}
        </div>
      </SeatsContext.Provider>
      {!topDeck && (combinedDock ? <div data-session-dock className={cn("glass flex flex-none flex-col", (away || contentMode) && "invisible")}>{deck}</div> : deck)}
      {topDeck && <div ref={setAbove} data-session-composer className={cn("flex flex-none flex-col pb-dock-safe-bottom empty:hidden", (away || contentMode) && "invisible")} />}
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
export function PillSlot({ loading = false, share, seat }: { loading?: boolean; share?: number | null; seat: (el: HTMLElement | null) => void }) {
  return (
    <div data-pill-slot className="flex min-w-0 flex-col">
      <span ref={seat} data-pill-progress-seat className="contents" />
      {loading && <PillProgress share={share} />}
    </div>
  );
}

/** The progress line itself: a view portals one into the `progress` seat
    while its tab reloads; the screen draws one for `loading`. With a
    `share` (item 60, a drop landing) it stands filled that far, 0 to 1,
    instead of running. */
export function PillProgress({ share }: { share?: number | null } = {}) {
  if (share === null) return <span data-pill-pending aria-hidden="true" className="pointer-events-none absolute inset-x-0 bottom-0 h-progress-h bg-fill-strong" />;
  return (
    <span data-pill-progress={share ?? ""} aria-hidden="true" className="pointer-events-none absolute inset-x-0 bottom-0 h-progress-h overflow-hidden">
      <span
        className="block h-full w-full origin-left bg-brand-strong"
        style={share === undefined ? undefined : { transform: `scaleX(${share})` }}
      />
    </span>
  );
}

export { DeckBand } from "@/components/DeckBand";

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
    <TaskOverlay purpose="selection" data-tab-sheet className="!gap-0 overflow-y-auto">
      <DialogTitle className="sr-only">{gui.session_tabs}</DialogTitle>
      <TabSheetRows {...rows}/>
    </TaskOverlay>
  </Dialog>;
}
/** Aligned horizontal content cards; the session switcher keeps its
 * diagonal squares. The count stays in the front card in both cases. */
export function ContentTabsGlyph({n}: {n:number}) {
  return <span data-content-tabs-glyph className="content-tabs-glyph"><TabsGlyph n={n}/></span>;
}
