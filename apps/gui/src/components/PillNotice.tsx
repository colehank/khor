// **The session's pill, when this session cannot go on** (NOTICE-BRIEF,
// the owner 2026-10-08; the notice lab, approved 2026-10-09 「药丸那可以
// 了」, ux-52 §9–10c).
//
// What the pill says comes from `lib/pill-notices`: a transfer that did
// not get there (「图片未传到 Mac-mini」, until 重试 or 取消) and the
// session reconnecting (「重新连接 Mac-mini」 and its seconds, until the
// session is back). **The pill keeps its size**: the kind's mark takes the
// face's seat, the short line the name's room, and the seconds trail it.
// Several at once: the most urgent (reconnecting first), with a count on
// the mark's corner.
//
// **A tap grows the pill, as the Dynamic Island does** — the same glass,
// joined, no gap and no second box: on the phone up and across the
// deck's full width (`NoticeIsland`), on the wide face down from the
// front tab across the board (`NoticePanel`). The pill's own row becomes
// the most urgent notice's row; the others come in above it on the phone
// (the most urgent nearest the pill), below it on the wide face. Each row:
// the mark in the pill's glyph column, the whole sentence, why under it,
// 详情 folded (the machine's own words only there), and the acts at the
// end. A light scrim behind; a tap outside folds it back. Reduced motion:
// no animation.
import { useCallback, useEffect, useLayoutEffect, useRef, useState, type CSSProperties } from "react";
import { createPortal } from "react-dom";

import { MidName } from "@/browse/TabStrip";
import { IconAlert, IconChevronDown, IconChevronRight, IconReload } from "@/components/icons";
import { gui } from "@/gen/catalog";
import { plainOf, type NoticeAct, type PillNotice, type Say } from "@/lib/pill-notices";
import { stillMotion } from "@/lib/travel";
import { cn } from "@/lib/utils";
import { ago, lasting } from "@/words";

/** How long the pill takes to grow, and to fold back. */
export const GROW_MS = 280;
const EASE = "var(--ease-move)";

/**
 * **The click a touch's tap leaves behind** (the lab, found on a real
 * iPhone): the browser makes a click of the tap, at the same spot, after
 * the pointer's up — and by then that spot is the scrim the tap just
 * raised. A click on the scrim this soon after it rose is that one, not a
 * tap outside.
 */
export const GHOST_MS = 400;

function useTick(on: boolean) {
  const [, tick] = useState(0);
  useEffect(() => {
    if (!on) return;
    const t = window.setInterval(() => tick((n) => n + 1), 1000);
    return () => window.clearInterval(t);
  }, [on]);
}

/** The kind's mark, in the face's seat: the reload ring turning while it
    reconnects, the failure's 「!」 in `state-failed`. */
export function NoticeGlyph({ n }: { n: PillNotice }) {
  return n.kind === "rejoin" ? (
    <span data-notice-glyph="rejoin" className="grid size-tab-face flex-none place-items-center text-muted-foreground">
      <IconReload className="size-affordance animate-spin motion-reduce:animate-none" />
    </span>
  ) : (
    <span data-notice-glyph="failed" className="grid size-tab-face flex-none place-items-center text-state-failed">
      <IconAlert className="size-full" />
    </span>
  );
}

/** The short line: words whole, the one name cut in its middle. */
function Line({ say }: { say: Say }) {
  return (
    <span data-notice-short className="flex min-w-0 flex-1 items-center overflow-hidden whitespace-nowrap">
      {say.map((p, i) =>
        typeof p === "string" ? (
          <span key={i} className="flex-none whitespace-pre">
            {p}
          </span>
        ) : (
          <MidName key={i} text={p.name} even />
        ),
      )}
    </span>
  );
}

/** Reconnecting: how long, bare (a lasting state's duration). */
const secondsOf = (n: PillNotice) => (n.since === undefined ? null : lasting(Math.max(1000, Date.now() - n.since)));

/**
 * **The pill's look while something stops its session**: the most urgent
 * notice's mark in the face's seat, with the count on its corner when
 * there are several; its short line; the seconds while it reconnects.
 * `seat` is the face seat's class where the pill is drawn (the deck's,
 * the strip's tab).
 */
export function NoticeLook({ list, seat, hideMark = false }: { list: PillNotice[]; seat: string; hideMark?: boolean }) {
  const top = list[0];
  useTick(top?.since !== undefined);
  if (!top) return null;
  const seconds = secondsOf(top);
  return (
    <>
      <span data-notice-seat className={cn("relative grid flex-none place-items-center", seat, hideMark && "[&>[data-notice-glyph]]:invisible")}>
        <NoticeGlyph n={top} />
        {list.length > 1 && (
          <span
            data-notice-count={list.length}
            className="absolute -top-0.5 -right-0.5 grid h-affordance min-w-affordance place-items-center rounded-full bg-foreground px-0.5 text-caption leading-none text-background tabular-nums"
          >
            {list.length}
          </span>
        )}
      </span>
      <Line say={top.short} />
      {seconds && (
        <span data-notice-seconds className="-ms-1 flex-none whitespace-nowrap text-caption text-muted-foreground tabular-nums">
          {seconds}
        </span>
      )}
    </>
  );
}

/**
 * **One notice, grown**: the mark in the pill's glyph column, the whole
 * sentence, why under it in `secondary`, how long or how long ago
 * trailing; under that 详情 (the raw words only once opened, with 复制)
 * and the acts, small, at the end. `words` styles all but the mark (the
 * pill's row turning into this one fades the words alone).
 */
function Row({ n, seat, words }: { n: PillNotice; seat: string; words?: CSSProperties }) {
  const [open, setOpen] = useState(false);
  const age = secondsOf(n) ?? ago(n.at);
  return (
    <div data-notice-row={n.kind} data-notice-id={n.id} className="flex flex-col">
      <div className="flex items-start gap-in pe-row">
        <span className="grid h-glass-btn flex-none place-items-center">
          <span className={cn("grid place-items-center", seat)}>
            <NoticeGlyph n={n} />
          </span>
        </span>
        <span data-notice-words className="flex min-h-glass-btn min-w-0 flex-1 flex-col justify-center gap-xxs py-1 text-body" style={words}>
          <span data-notice-whole className="text-foreground">
            {n.whole}
          </span>
          {n.how && (
            <span data-notice-how className="text-callout text-muted-foreground">
              {n.how}
            </span>
          )}
        </span>
        <span className="flex h-glass-btn flex-none items-center text-caption text-tertiary tabular-nums" style={words}>
          {age}
        </span>
      </div>
      {(n.raw || n.acts.length > 0) && (
        <div data-notice-foot className="flex flex-wrap items-center gap-xxs pe-in" style={{ ...words, marginInlineStart: "var(--notice-words)" }}>
          {n.raw && (
            <button
              type="button"
              data-notice-detail={open ? "open" : "shut"}
              aria-expanded={open}
              onClick={() => setOpen((o) => !o)}
              className="-ms-1.5 flex items-center gap-xxs rounded-md px-1.5 py-1 text-callout text-muted-foreground hover:bg-fill"
            >
              {open ? <IconChevronDown className="size-affordance" /> : <IconChevronRight className="size-affordance" />}
              {gui.notice_detail}
            </button>
          )}
          {open && n.raw && (
            <button
              type="button"
              data-notice-copy
              onClick={() => void navigator.clipboard.writeText(n.raw ?? "").catch(() => {})}
              className="rounded-md px-1.5 py-1 text-callout text-muted-foreground hover:bg-fill"
            >
              {gui.copy_text}
            </button>
          )}
          <span className="flex-1" />
          {n.acts.map((a) => (
            <ActButton key={a.key} a={a} />
          ))}
        </div>
      )}
      {open && n.raw && (
        <p
          data-notice-raw
          className="m-0 pe-row pb-1 font-mono text-callout text-muted-foreground select-text [overflow-wrap:anywhere]"
          style={{ ...words, marginInlineStart: "var(--notice-words)" }}
        >
          {n.raw}
        </p>
      )}
      <div className="h-1" />
    </div>
  );
}

function ActButton({ a }: { a: NoticeAct }) {
  return (
    <button
      type="button"
      data-notice-act={a.key}
      onClick={a.run}
      className={cn("rounded-md px-2 py-1 text-callout hover:bg-fill", a.quiet ? "text-muted-foreground" : "text-foreground")}
    >
      {a.word}
    </button>
  );
}

/** The rows, a hairline between each two from the words' column. `lead`
    is the row the pill's own row becomes; `fade` styles the others. */
function Rows({ list, seat, lead, fade }: { list: PillNotice[]; seat: string; lead?: { id: string; words: CSSProperties }; fade?: CSSProperties }) {
  useTick(list.some((n) => n.since !== undefined));
  return (
    <div className="flex flex-col font-sans">
      {list.map((n, i) => (
        <div key={n.id} data-row-lead={lead?.id === n.id || undefined} style={lead?.id === n.id ? undefined : fade}>
          {i > 0 && <div className="border-t" style={{ marginInlineStart: "var(--notice-words)" }} />}
          <Row n={n} seat={seat} words={lead?.id === n.id ? lead.words : undefined} />
        </div>
      ))}
    </div>
  );
}

/**
 * **Mounted folded, grown the frame after, folded before it goes.** With
 * reduced motion it is drawn as it ends, at once both ways.
 */
export function useGrowth(open: boolean) {
  const [mounted, setMounted] = useState(open);
  const [big, setBig] = useState(open);
  useEffect(() => {
    if (stillMotion()) {
      setMounted(open);
      setBig(open);
      return;
    }
    if (open) {
      setMounted(true);
      let b = 0;
      const a = requestAnimationFrame(() => (b = requestAnimationFrame(() => setBig(true))));
      return () => {
        cancelAnimationFrame(a);
        cancelAnimationFrame(b);
      };
    }
    setBig(false);
    const t = window.setTimeout(() => setMounted(false), GROW_MS);
    return () => window.clearTimeout(t);
  }, [open]);
  return { mounted, big };
}

/** True from the frame after `ready` first holds: a transition armed
    while the folded clip is being measured would animate the measuring. */
function useArmed(ready: boolean) {
  const [armed, setArmed] = useState(false);
  useEffect(() => {
    if (!ready || armed) return;
    const f = requestAnimationFrame(() => setArmed(true));
    return () => cancelAnimationFrame(f);
  }, [ready, armed]);
  return armed;
}

/** The light scrim behind the grown pill; a tap on it folds the pill,
    unless it is the tap's own leftover click (`GHOST_MS`). `fixed`: over
    the window from `style`'s edges, drawn in a portal (the wide face's,
    under the strip); else over the box it is put in (the phone's screen,
    under the deck). */
export function NoticeScrim({
  on,
  openedAt,
  onFold,
  style,
  fixed = false,
}: {
  on: boolean;
  openedAt: number;
  onFold: () => void;
  style?: CSSProperties;
  fixed?: boolean;
}) {
  const scrim = (
    <div
      data-notice-scrim={on ? "on" : "off"}
      aria-hidden="true"
      onClick={() => {
        if (Date.now() - openedAt < GHOST_MS) return;
        onFold();
      }}
      className={cn(fixed ? "fixed" : "absolute", "inset-0 z-40 bg-(--notice-scrim)")}
      style={{
        opacity: on ? 1 : 0,
        pointerEvents: on ? "auto" : "none",
        transition: stillMotion() ? undefined : `opacity ${GROW_MS}ms linear`,
        ...style,
      }}
    />
  );
  return fixed ? createPortal(scrim, document.body) : scrim;
}

/** Escape folds a grown pill, as it closes a sheet. */
function useEscape(on: boolean, fold: () => void) {
  const latest = useRef(fold);
  latest.current = fold;
  useEffect(() => {
    if (!on) return;
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        latest.current();
      }
    };
    window.addEventListener("keydown", key, true);
    return () => window.removeEventListener("keydown", key, true);
  }, [on]);
}

type Box = { top: number; right: number; bottom: number; left: number };

/**
 * **The pill, grown** (the phone's deck). One piece of glass, laid out at
 * its grown size — the deck's full width inside its margins, up to half
 * the screen, scrolling inside past that — and shown through a clip that
 * starts as the pill's own rounded box and opens to the whole: the pill
 * getting bigger, joined, no second box. Its rows start at the card's own
 * inset; folded they are slid to where the pill's are, so **the pill's row
 * becomes the most urgent notice's row**: the short line gives way to the
 * whole sentence in place as the rows rise with the opening, and the
 * others (above it: the most urgent nearest the pill) come in. Covers the
 * deck's circles while open. Mounted inside the deck (`SessionScreen`).
 */
export function NoticeIsland({
  list,
  pill,
  growth,
  seat,
  placement = "bottom",
}: {
  list: PillNotice[];
  pill: HTMLElement | null;
  growth: { mounted: boolean; big: boolean };
  seat: string;
  placement?: "top" | "bottom";
}) {
  const island = useRef<HTMLDivElement>(null);
  const scroller = useRef<HTMLDivElement>(null);
  const inner = useRef<HTMLDivElement>(null);
  const [h, setH] = useState(0);
  const [box, setBox] = useState<Box | null>(null);
  const [radius, setRadius] = useState(0);
  /** Where the lead row's first line stands, in the rows and as shown. */
  const [lead, setLead] = useState<{ at: number; shown: number } | null>(null);
  const top = list[0];
  const rows = placement === "top" ? list : [...list].reverse();
  useLayoutEffect(() => {
    const el = inner.current;
    const deck = island.current?.parentElement;
    if (!el || !deck) return;
    const fit = () => {
      const d = deck.getBoundingClientRect();
      const room = placement === "top" ? (window.visualViewport?.height ?? window.innerHeight) - d.top : d.bottom;
      setH(Math.min(el.scrollHeight, Math.round(room / 2)));
    };
    fit();
    const ro = new ResizeObserver(fit);
    ro.observe(el);
    return () => ro.disconnect();
  }, [list.length, placement]);
  useLayoutEffect(() => {
    const s = scroller.current;
    const el = inner.current;
    const me = island.current;
    if (!s || !el || !me || !pill || h === 0) return;
    s.scrollTop = placement === "top" ? 0 : s.scrollHeight;
    const row = el.querySelector<HTMLElement>("[data-row-lead]");
    const at = row ? row.offsetTop + (row.firstElementChild?.matches(".border-t") ? 1 : 0) : null;
    setLead(at === null ? null : { at, shown: at - s.scrollTop });
    const i = me.getBoundingClientRect();
    const p = pill.getBoundingClientRect();
    setBox({ top: p.top - i.top, right: i.right - p.right, bottom: i.bottom - p.bottom, left: p.left - i.left });
    setRadius(p.height / 2);
  }, [h, pill, list.length, placement]);
  const armed = useArmed(h > 0 && box !== null && lead !== null);
  const move = armed && !stillMotion();
  if (!top) return null;
  const clip = growth.big || !box ? `inset(0 round ${radius}px)` : `inset(${box.top}px ${box.right}px ${box.bottom}px ${box.left}px round ${radius}px)`;
  // Folded, the lead row's first line is slid into the pill's place.
  const slide = growth.big || !box || !lead ? 0 : box.top - lead.shown;
  const shift = growth.big || !box ? 0 : box.left;
  const swap = (on: boolean, inMs: number, outMs: number, delay = 0): CSSProperties => ({
    opacity: on ? 1 : 0,
    transition: move ? `opacity ${on ? inMs : outMs}ms linear ${on ? delay : 0}ms` : undefined,
  });
  return (
    <div
      ref={island}
      data-notice-island={growth.big ? "open" : "folded"}
      role="dialog"
      aria-label={plainOf(top.short)}
      // The pill's look as it stands on the deck: its tint over the
      // ground, so the program's colours never show through the words.
      className={cn("glass absolute inset-x-row z-50 overflow-hidden [background:linear-gradient(var(--glass),var(--glass)),var(--card)]", placement === "top" ? "top-in" : "bottom-in")}
      style={{
        height: h,
        borderRadius: radius,
        clipPath: clip,
        WebkitClipPath: clip,
        transition: move ? `clip-path ${GROW_MS}ms ${EASE}, -webkit-clip-path ${GROW_MS}ms ${EASE}` : undefined,
        // Not drawn until the pill's box is measured: never a frame at
        // the grown size before it has grown.
        visibility: growth.big || box ? undefined : "hidden",
      }}
    >
      <div ref={scroller} className="absolute inset-0 overflow-x-hidden overflow-y-auto overscroll-contain">
        <div
          ref={inner}
          className="relative ps-1 [--notice-words:calc(var(--spacing-glass-item-h)+var(--gap-in))]"
          style={{ transform: `translate(${shift}px, ${slide}px)`, transition: move ? `transform ${GROW_MS}ms ${EASE}` : undefined }}
        >
          <Rows list={rows} seat={seat} lead={{ id: top.id, words: swap(growth.big, 160, 90, 70) }} fade={swap(growth.big, 200, 90, 60)} />
          {/* The pill's own look over the lead row's first line: what the
              pill said, until it has become the row. Its mark is the
              row's own, drawn once (two turning rings would turn out of
              step). */}
          {lead && box && (
            <div
              aria-hidden="true"
              data-notice-pill-look
              className="pointer-events-none absolute inset-x-0 flex h-glass-btn items-center gap-in font-sans text-body text-foreground"
              style={{ top: lead.at, width: pill ? pill.getBoundingClientRect().width : undefined, ...swap(!growth.big, 120, 90) }}
            >
              <NoticeLook list={list} seat={seat} hideMark />
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

/**
 * **The front tab, grown** (the wide face). The tab stays the tab — its
 * own face and title again while this says the notice — and the same lit
 * glass carries on below it, across the board's width: one piece, the tab
 * its top. It opens from the tab's own width downward through the same
 * clip; the most urgent notice first, nearest the tab. Drawn in a portal,
 * fixed where the tab and the board are.
 */
export function NoticePanel({ list, tab, board, growth }: { list: PillNotice[]; tab: HTMLElement | null; board: HTMLElement | null; growth: { mounted: boolean; big: boolean } }) {
  const inner = useRef<HTMLDivElement>(null);
  const [h, setH] = useState(0);
  const [at, setAt] = useState<{ left: number; top: number; width: number; tabLeft: number; tabW: number; r: number } | null>(null);
  useLayoutEffect(() => {
    const el = inner.current;
    if (!el || !tab || !board) return;
    const place = () => {
      const t = tab.getBoundingClientRect();
      const b = board.getBoundingClientRect();
      // Across the strip's capsule: the board's width, on its glass edges.
      const left = b.left;
      const width = b.width;
      setAt({ left, top: t.bottom, width, tabLeft: t.left - left, tabW: t.width, r: t.height / 2 });
      setH(Math.min(el.scrollHeight, Math.round((window.innerHeight - t.bottom) / 2)));
    };
    place();
    const ro = new ResizeObserver(place);
    ro.observe(el);
    ro.observe(board);
    window.addEventListener("resize", place);
    return () => {
      ro.disconnect();
      window.removeEventListener("resize", place);
    };
  }, [tab, board, list.length]);
  const armed = useArmed(h > 0 && at !== null);
  const move = armed && !stillMotion();
  if (!list.length) return null;
  const r = at?.r ?? 0;
  // Square where the tab meets it, when the tab stands at its left edge.
  const tl = at && at.tabLeft < 1 ? 0 : r;
  const clip =
    growth.big || !at
      ? `inset(0 round ${tl}px ${r}px ${r}px ${r}px)`
      : `inset(0 ${Math.max(0, at.width - at.tabLeft - at.tabW)}px ${h}px ${at.tabLeft}px)`;
  return createPortal(
    <div
      data-notice-panel={growth.big ? "open" : "folded"}
      role="dialog"
      aria-label={list[0].whole}
      className="glass-sel fixed z-50 overflow-hidden"
      style={{
        left: at?.left,
        top: at?.top,
        width: at?.width,
        height: h,
        borderRadius: `${tl}px ${r}px ${r}px ${r}px`,
        clipPath: clip,
        WebkitClipPath: clip,
        transition: move ? `clip-path ${GROW_MS}ms ${EASE}, -webkit-clip-path ${GROW_MS}ms ${EASE}` : undefined,
        visibility: at ? undefined : "hidden",
      }}
    >
      <div className="max-h-full overflow-y-auto overscroll-contain" style={{ height: h }}>
        <div ref={inner} className="ps-1.5 pt-1 [--notice-words:calc(var(--spacing-tab-face)+var(--gap-in))]">
          <Rows list={list} seat="size-tab-face" />
        </div>
      </div>
    </div>,
    document.body,
  );
}

/**
 * **What a grown pill needs from its owner**: open or not, when it was
 * opened (the ghost click), the growth, and Escape. Folds by itself once
 * nothing stops the session.
 */
export function useGrownPill(count: number) {
  const [open, setOpen] = useState(false);
  const openedAt = useRef(0);
  const growth = useGrowth(open && count > 0);
  useEffect(() => {
    if (count === 0) setOpen(false);
  }, [count]);
  // Stable, so an owner can fold on its own changes (another tab in
  // front) without folding on every render.
  const fold = useCallback(() => setOpen(false), []);
  const grow = useCallback(() => {
    openedAt.current = Date.now();
    setOpen(true);
  }, []);
  useEscape(open, fold);
  return { open, growth, openedAt: openedAt.current, grow, fold };
}

export type GrownPill = ReturnType<typeof useGrownPill>;

