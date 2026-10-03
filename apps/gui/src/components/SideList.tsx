// **The list inside every sidebar — one, for all five** (docs/SHELL.md
// 第四轮 第 1 条:「所有展开的一级侧边栏内容显示逻辑要一致, 都是如session
// 那样, 可以被filter, 置顶的显示也是收起与展开, 形式完全一致, 不必写多份
// 代码, 公用一份即可」).
//
// `ListPane` is the sidebar's shell (ground, head, scroller, seam) and
// this is what goes inside it. Sessions, machines, workspaces, browsing
// tabs and accounts each hand in rows — what the face and its corner
// are, the two lines, the menu past 置顶 and 删除 — and everything a
// person can *learn once* about a sidebar is here and only here:
//
//   filter   one rule: what the box holds, against what each row says it
//            can be found by (`find`);
//   groups   a heading where the group changes between neighbours, and
//            the heading folds its rows away — the pinned group is one of
//            those, which is how every column gets 「置顶的显示也是收起与
//            展开」 without a line of its own;
//   row      a face with its kind mark (`FaceMark`), a title over a
//            subtitle, whatever the column puts at the right end;
//   motion   rows walk to their new places (`useRowFlip`);
//   the old  rows past their time fold behind one line at the foot.
//
// It came out of `SessionsList`, which was the one list that already
// did all of this; the other four had each grown part of it.
import { Fragment, useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";

import { EmptyState } from "@/components/RegionState";
import { DeleteDialog, type Pending } from "@/components/DeleteDialog";
import { FaceMark, type FaceOf, type KindMark } from "@/components/FaceMark";
import { IconChevronRight } from "@/components/icons";
import { RowMenu, type RowAction } from "@/components/RowMenu";
import { useNarrow } from "@/hooks/use-narrow";
import { walkList } from "@/lib/shortcuts";
import { useRowSwipe } from "@/hooks/use-row-swipe";
import { cn } from "@/lib/utils";
import { gui } from "@/gen/catalog";
import { useSetting } from "@/settings";

type Data = Record<string, string | number | boolean | null | undefined>;

/** One row, as a column hands it in. */
export type SideRow = {
  /** Stable, and the anchor the row answers to (`data-side-row`). */
  id: string;
  /** The group this row sits under: `""` for none, `"pin"` for the
      pinned group every column has. A column whose groups come from the
      node (sessions' arrangements) hands those keys in as they are. */
  group: string;
  pinned: boolean;
  /** Whose face: a machine's record (`{ machine }`, drawn by `FaceMark`
      at the row's size, the ink of its mark read off the same record),
      or a drawing that is not a face (`{ art }`, already sized — the 额度
      dial), which never carries a mark (`FaceOf`). */
  face: FaceOf;
  /** What kind of thing the row is — drawn on the face or on its corner
      as the 外观 setting says (`FaceMark`, 2026-10-02). Absent draws the
      face alone (the 额度 row's mark opens its title instead,
      2026-09-30). */
  mark?: KindMark;
  /** A session's state ring. */
  halo?: string;
  title: ReactNode;
  /** The row's name as plain text, for the delete window's question
      (「删除 mandala？」) — `title` is a drawing, and may be an editor. */
  name?: string;
  sub?: ReactNode;
  /** The right end of the row, inside what opens it. */
  trailing?: ReactNode;
  /** Beside the row and outside what opens it — a control of its own
      (a tab's ×) or a drawing of state (a workspace's chevron). */
  after?: ReactNode;
  /** What the box finds this row by. **What the row says**, and nothing
      it does not: a match on a field nobody can see leaves a person
      looking at a row that has nothing to do with what they typed. */
  find: string;
  selected?: boolean;
  /** Greyed as a whole (a session whose word has gone quiet). */
  dim?: boolean;
  /** Folded behind the line at the foot until asked for. */
  old?: boolean;
  menu: RowAction[];
  /** Where pressing the row goes; absent leaves it a drawing. */
  onOpen?: () => void;
  /** Anchors a column's older gates still read, on the row and on the
      button that opens it. */
  rowData?: Data;
  openData?: Data;
};

/** How far a row is currently displaced from where it is laid out —
    the translateY of whatever transform it is wearing, mid-animation or
    not. */
function offsetOf(row: HTMLElement): number {
  const t = getComputedStyle(row).transform;
  if (!t || t === "none") return 0;
  try {
    return new DOMMatrixReadOnly(t).m42;
  } catch {
    return 0;
  }
}

/**
 * Rows walk to their new places instead of teleporting there.
 *
 * The list re-sorts under the reader — a session turns 待批 and floats,
 * a pin lands — and on a poll the whole order changes between two
 * frames with nothing to say which row went where. FLIP needs no
 * bookkeeping: measure where every row *was*, let React paint where they
 * now are, put each one back where it started and release it.
 *
 * **`offsetTop`, not `getBoundingClientRect`**: the list is inside a
 * scroller, and a rect would read one pixel of scrolling as every row
 * having moved. **Rows only, never headings**: a heading appears and
 * disappears, it does not move. Under `prefers-reduced-motion` the
 * measurement is skipped, not given a zero duration — that would be a
 * flicker instead of a move.
 */
function useRowFlip(deps: string) {
  const box = useRef<HTMLDivElement>(null);
  const was = useRef(new Map<string, number>());
  useLayoutEffect(() => {
    const el = box.current;
    if (!el) return;
    const still = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const now = new Map<string, number>();
    for (const row of el.querySelectorAll<HTMLElement>("[data-side-row]")) {
      const id = row.dataset.sideRow;
      if (!id) continue;
      const top = row.offsetTop;
      now.set(id, top);
      const before = was.current.get(id);
      if (still || before === undefined || before === top) continue;
      // **Where the row is *seen*, not where it is laid out**: a second
      // reorder can land mid-flight, and starting from the layout
      // position would snap it back first — the jump FLIP exists to
      // remove. The leftover is carried and the transition retargets.
      const carried = offsetOf(row);
      row.style.transition = "none";
      row.style.transform = `translateY(${before - top + carried}px)`;
      requestAnimationFrame(() => {
        row.style.transition = "";
        row.style.transform = "";
      });
    }
    was.current = now;
  }, [deps]);
  return box;
}

/** Where the keyboard's focus stops in a list: each row's open button,
    each group's heading, the fold at the foot — in reading order. */
const WALK = "[data-side-open], [data-side-group], [data-old-fold]";

/** The focus ring on a list's buttons: the brand's ring colour, drawn
    inside the row so the next row does not cover it (README: the ring is
    one of `brand-strong`'s three posts). */
const RING = "rounded-md focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring";

/** Whether a row answers what is typed in the box: a plain substring,
    any case — the reading every box in this app gives a typed word. */
export function finds(query: string, text: string): boolean {
  const q = query.trim().toLowerCase();
  return q === "" || text.toLowerCase().includes(q);
}

export function SideList({
  rows,
  loading = false,
  query = "",
  label,
  empty,
  emptyHow,
  none,
  old,
  mark,
  filtered = false,
  headData,
  rest = gui.group_rest,
}: {
  rows: SideRow[];
  /** The column has not been answered yet: a skeleton, not `empty`. */
  loading?: boolean;
  /** What is in the head's box. */
  query?: string;
  /** A group's heading, from its key. */
  label: (group: string) => string;
  /** Drawn when the column has no rows at all. */
  empty?: ReactNode;
  /** **Where the way to fill it lives**, under `empty` (UX-4: one empty
      state, `EmptyState`'s sentence and hint, on every list): 「点击上方的
      ＋…」. Not said under `none` — a filter is undone, not filled. */
  emptyHow?: ReactNode;
  /** Drawn when the box has filtered every row away — a different fact
      from 「there are none」, and the wrong one would be a lie. */
  none?: ReactNode;
  /** The words on the fold at the foot: how many are behind it, and
      the word for putting them back. Absent where a column has no old
      rows to fold. */
  old?: { more: (n: number) => string; less: string };
  /** An extra anchor on the list itself, for a column's older gates. */
  mark?: Data;
  /** Whether the column has narrowed the rows by something other than
      the box (sessions' ticked words) — the fold at the foot stands
      aside for that as it does for a query. */
  filtered?: boolean;
  /** Extra anchors on a group's heading, for a column's older gates. */
  headData?: (group: string) => Data;
  /** The heading over the rows in no group, once a group stands above
      them — the column's word for its own order (sessions: 最近). */
  rest?: string;
}) {
  // **No selected row on the narrow face** (2026-10-02, the owner on a
  // phone: going back to a list still showed the row it had opened). A
  // selected row says "this is what is open beside me"; on the narrow
  // face the list and what it opened are never on screen together — the
  // list is one screen, the row's page the next — so coming back, there
  // is nothing beside it to point at. Every landing's list is this one
  // component, so this is the one place. The wide face is unchanged.
  const narrow = useNarrow();
  const shown = rows.filter((r) => finds(query, r.find));
  // **A query reads everything**, the fold included: it is the person
  // asking for a row wherever it is. The fold is the untouched view's.
  const searching = query.trim() !== "" || filtered;
  const fresh = searching ? shown : shown.filter((r) => !r.old);
  const aged = searching ? [] : shown.filter((r) => r.old);
  const [showOld, setShowOld] = useState(false);
  /**
   * Groups folded away, by key — so a fold survives a re-poll, a re-sort
   * and a filter that empties the group and then fills it again. **Every
   * group folds**, not only the pinned one: singling it out would be a
   * list of special cases where one rule does. Kept per list: each
   * column remembers its own (「收起状态各列各记」).
   */
  const [shut, setShut] = useState<Set<string>>(() => new Set());
  const shutOf = (group: string) => shut.has(group);
  const toggle = (group: string) =>
    setShut((was) => {
      const next = new Set(was);
      if (!next.delete(group)) next.add(group);
      return next;
    });
  // **删除 asks first, in one window for every column** (第四轮 第 3 条),
  // until 不再提醒 turns `confirm-delete` off. Only an explicit off takes
  // the question away: a setting that has not arrived reads as the
  // default, which asks.
  const asking = useSetting("confirm-delete") !== "off";
  const [pending, setPending] = useState<Pending | null>(null);
  // **The word says whether a question follows** (docs/design-system
  // 「带省略号表示还会再问一次」): 「删除…」 while the window will come
  // up, the plain word once 不再提醒 has taken it away — it is the same
  // press either way, and only one of the two labels is true at a time.
  /** What a slid row offers (F7.5): its pin and its destructive item,
      as the menu has them (and through the same ask). */
  const swipeOf = (r: SideRow): RowAction[] =>
    menuOf(r).filter((a) => a.key === "pin" || a.destructive);
  const menuOf = (r: SideRow): RowAction[] =>
    r.menu.map((a) =>
      a.ask === undefined || !asking
        ? a
        : {
            ...a,
            label: a.askLabel ?? a.label,
            confirm: undefined,
            run: () => setPending({ name: r.name, says: a.ask ?? "", run: a.run }),
          },
    );
  const box = useRowFlip(
    (showOld ? [...fresh, ...aged] : fresh)
      .filter((r) => !shutOf(r.group))
      .map((r) => r.id)
      .join("\n"),
  );
  // Slide a row left for its pin and its delete (F7.5, narrow).
  // On only while the rows are drawn: the loading and empty faces are
  // other elements, and the list box comes and goes with them.
  useRowSwipe(box, narrow && shown.length > 0 && !(loading && rows.length === 0));

  // **What matched is marked in the rows** (the audit fix plan F3.4;
  // N20): every place the typed word stands in a row's title or second
  // line, painted with the CSS highlight registry so no row's drawing has
  // to know about the search (`app.css` `::highlight(khor-find)`).
  useEffect(() => {
    const reg = (CSS as unknown as { highlights?: Map<string, unknown> }).highlights;
    const Mark = (window as unknown as { Highlight?: new (...r: Range[]) => unknown }).Highlight;
    const el = box.current;
    const q = query.trim().toLowerCase();
    if (!reg || !Mark) return;
    if (!el || q === "") {
      reg.delete("khor-find");
      return;
    }
    const ranges: Range[] = [];
    const walk = document.createTreeWalker(el, NodeFilter.SHOW_TEXT, {
      acceptNode: (n) =>
        n.parentElement?.closest("[data-side-title], [data-side-sub]") ? NodeFilter.FILTER_ACCEPT : NodeFilter.FILTER_REJECT,
    });
    for (let n = walk.nextNode(); n; n = walk.nextNode()) {
      const t = (n.textContent ?? "").toLowerCase();
      for (let i = t.indexOf(q); i >= 0; i = t.indexOf(q, i + q.length)) {
        const r = new Range();
        r.setStart(n, i);
        r.setEnd(n, i + q.length);
        ranges.push(r);
      }
    }
    reg.set("khor-find", new Mark(...ranges));
    return () => {
      reg.delete("khor-find");
    };
  });

  if (loading && rows.length === 0) {
    // **One skeleton, shaped like the rows that are coming** (V18): the
    // face's disc in its slot, a title and a second line over the inset
    // hairline — the same x's as a row — on every list.
    return (
      <div data-side-list {...mark} data-loading aria-busy="true" className={narrow ? "pb-in" : "px-in pb-in"}>
        {["w-[56%]", "w-[42%]", "w-[64%]", "w-[48%]"].map((w, i) => (
          <div key={i} data-skeleton className={cn("flex min-h-list-row items-stretch gap-row", narrow ? "ps-group" : "ps-row")}>
            <span className="flex w-list-face flex-none items-center justify-center">
              <span className="size-avatar rounded-full bg-fill" />
            </span>
            <span className={cn("flex min-w-0 flex-1 flex-col justify-center gap-1.5 border-b py-1", narrow ? "pe-group" : "pe-row")}>
              <span className={cn("h-3.5 rounded-full bg-fill", w)} />
              <span className="h-3 w-[30%] rounded-full bg-fill" />
            </span>
          </div>
        ))}
      </div>
    );
  }
  if (shown.length === 0) {
    return (
      <div data-side-list {...mark} data-empty className="flex min-h-0 flex-1 flex-col">
        <EmptyState hint={rows.length === 0 ? emptyHow : undefined}>{rows.length === 0 ? empty : none}</EmptyState>
      </div>
    );
  }

  /** **The list is one Tab stop** (the audit fix plan F3.8; W25): the
      picked row, or the first, takes Tab; the arrows walk the rest
      (`walkList`). ⌥⌘S puts the focus there (`lib/shortcuts`). */
  const visible = (showOld ? [...fresh, ...aged] : fresh).filter((r) => !shutOf(r.group));
  /** **A row's React key: its id, made unique where two rows share one**
      (2026-10-03). Two machines with the same name share one
      conversation (`chat/<name>`, the node's persisted session id), and
      the node lists it once per machine — two rows with one id, which
      React folds together. The id stays what the row is (what it opens,
      selects, pins); only the key gains the occurrence. The sharing itself
      is the node's to settle and waits on the owner. */
  const keyOf = new Map<SideRow, string>();
  {
    const seen = new Map<string, number>();
    for (const r of [...fresh, ...aged]) {
      const n = seen.get(r.id) ?? 0;
      seen.set(r.id, n + 1);
      keyOf.set(r, n === 0 ? r.id : `${r.id}#${n}`);
    }
  }
  const stop = (visible.find((r) => r.selected) ?? visible[0])?.id;
  const one = (r: SideRow, prev: SideRow | undefined, size: number) => {
    const open = !shutOf(r.group);
    // **The row of the redesign** (2026-10-02): the face, then the words
    // over an inset hairline that starts where the words do — Apple's
    // lists. `list-row` / `list-face` are the pointer's sizes (68 / 40
    // under a finger, 56 / 32 under a mouse). The hairline is the text
    // block's foot, so a picked row's grey block (wide) hides its own.
    const picked = !narrow && !!r.selected;
    const button = (
      <button
        type="button"
        data-side-open
        {...r.openData}
        tabIndex={r.id === stop ? 0 : -1}
        onClick={r.onOpen}
        className={cn(
          "flex min-h-list-row min-w-0 flex-1 items-stretch gap-row text-left",
          narrow ? "ps-group" : "ps-row",
          RING,
        )}
      >
        {/* **One leading slot** (the audit fix plan F2.1): the face's
            width on every list, so the words start on one x whatever the
            face is — a session's, a machine's, the 额度 dial. */}
        <span data-side-face-slot className="flex w-list-face flex-none items-center justify-center">
          {"machine" in r.face ? (
            <FaceMark face={r.face} halo={r.halo} mark={r.mark} />
          ) : (
            <FaceMark face={r.face} halo={r.halo} />
          )}
        </span>
        <span
          data-side-text
          className={cn(
            "flex min-w-0 flex-1 items-center gap-row border-b py-1",
            narrow ? "pe-group" : "pe-row",
            picked && "border-transparent",
          )}
        >
        <span className="min-w-0 flex-1">
          {/* **One title**: `headline` (13 / 600 under a mouse, 17 / 600
              under a finger), on every list and whether or not the row
              is the picked one — a picked row shows by its block, never
              by its weight (F2.1; SYS-05, N15, W09). */}
          <span data-side-title className="block min-w-0 truncate text-headline">
            {r.title}
          </span>
          {/* Always drawn, empty where a row has nothing to say under its
              name — an empty block takes no height, and every row keeps
              one shape. */}
          <span data-side-sub className="block min-w-0 truncate text-callout text-muted-foreground">
            {r.sub}
          </span>
        </span>
        {r.trailing}
        {/* A row that opens a page says so on the narrow face, where the
            page is the next screen (F2.1). */}
        {narrow && r.onOpen && (
          <IconChevronRight data-side-drill className="size-affordance flex-none text-tertiary" />
        )}
        </span>
      </button>
    );
    return (
      <Fragment key={keyOf.get(r) ?? r.id}>
        {/* One rule where the pinned group ends and the rest begins
            (docs/SHELL.md 第七轮 第 1 条), noticed between neighbours like
            the heading below — so it is drawn only while both groups
            have rows. A folded group's rows are not drawn but its last
            one is still this row's neighbour, which leaves the rule
            right under the heading. The seam's colour: `border-t` is
            `--color-border`, the same token `ListPane`'s seam wears. */}
        {/* ⚠️ 墓碑 (the redesign): a rule stood here between the pinned
            group and the rest. Every row has its own hairline now, and
            the next group's heading is the boundary. */}
        {/* A heading starts where the group changes between neighbouring
            rows: the rows arrive grouped, so this notices a boundary
            rather than deciding one, and a heading can never be drawn
            over nothing. It is also the group's handle; what pressing it
            does next is said by the chevron and `aria-expanded`, and the
            count shows only while the group is shut. */}
        {/* **A group's extent is visible** (F2.2; N05, W07): once a group
            stands above them, the rows in none get a heading of their
            own, so a folded 置顶 does not seem to own the whole list. */}
        {(r.group !== "" ? r.group !== prev?.group : prev !== undefined && prev.group !== "") && (
          <button
            type="button"
            data-side-group={r.group}
            data-group={r.group}
            data-group-open={open}
            {...headData?.(r.group)}
            aria-expanded={open}
            tabIndex={-1}
            onClick={() => toggle(r.group)}
            className={cn(
              "flex min-h-touch w-full items-end gap-xxs pt-in pb-xxs text-left text-group text-muted-foreground",
              narrow ? "px-group" : "rounded-md px-row",
              RING,
            )}
          >
            {/* The family's chevron at 12 (components/GroupHeader), right
                when shut and turned a quarter when open — it is centred
                on 8,8, so turned it IS `chevronDown`. Not the back arrow
                turned: that path sits 0.4 off centre, which is what read
                as crooked (2026-09-26「展开按钮的icon甚至都没有整齐」). */}
            <IconChevronRight className={cn("size-affordance flex-none transition-transform", open && "rotate-90")} />
            <span data-group-label className="min-w-0 truncate">{r.group === "" ? rest : label(r.group)}</span>
            {/* The count only while the group is folded: open, its rows
                are the count. */}
            {!open && (
              <span data-group-size className="ml-auto flex-none font-body tabular-nums">
                {size}
              </span>
            )}
          </button>
        )}
        {open && (
          <div
            data-side-row={r.id}
            data-row-swipe={narrow && swipeOf(r).length > 0 ? "" : undefined}
            data-side-pinned={r.pinned}
            data-side-in={r.group}
            data-side-menu={r.menu.length > 0}
            data-selected={picked}
            {...r.rowData}
            // **Selected is painted by `app.css`, keyed on
            // `data-selected`** (第十五轮 第 2 条, D6): the brand's solid
            // green with its own ink while the window is in front, grey
            // with the ordinary ink when it is not — two states and an
            // ink that changes with them, which a class here could not
            // say without knowing about the window.
            // A picked row is a rounded grey block on the wide face and
            // nothing on the narrow one, where the list is a screen of its
            // own and the picked thing is the next screen (the redesign).
            className={cn(
              "flex w-full items-center",
              !narrow && "rounded-lg",
              !picked && !narrow && "hover:bg-fill",
              r.dim && !picked && "opacity-55",
            )}
          >
            {/* **The menu's trigger is the button, not the strip around
                it**: the trigger carries the long press's
                `onPointerDown`, and a press handler on a bare `<div>` is
                a target only a pointer can find. On the button, focus
                reaches it and the keyboard's Menu key opens it. */}
            {narrow && swipeOf(r).length > 0 ? (
              <>
                {/* **Slid left, the row shows what can be done to it**
                    (F7.5; `hooks/use-row-swipe`): the menu's own items,
                    in its words, revealed from the right edge. */}
                <div data-row-face className="flex min-w-0 flex-1 items-center">
                  {r.menu.length > 0 ? <RowMenu items={menuOf(r)}>{button}</RowMenu> : button}
                  {r.after}
                </div>
                <div data-row-actions>
                  <div data-row-actions-inner className="flex h-full">
                    {swipeOf(r).map((a) => (
                      <button
                        key={a.key}
                        type="button"
                        data-row-action={a.key}
                        data-destructive={a.destructive || undefined}
                        onClick={(e) => {
                          const row = e.currentTarget.closest<HTMLElement>("[data-row-swipe]");
                          row?.removeAttribute("data-row-revealed");
                          row?.style.removeProperty("--row-reveal");
                          void Promise.resolve().then(a.run).catch(() => {});
                        }}
                        className="min-w-(--row-action-w) flex-none px-in text-callout"
                      >
                        {a.label}
                      </button>
                    ))}
                  </div>
                </div>
              </>
            ) : (
              <>
                {r.menu.length > 0 ? <RowMenu items={menuOf(r)}>{button}</RowMenu> : button}
                {r.after}
              </>
            )}
          </div>
        )}
      </Fragment>
    );
  };
  /** One run of rows and the headings they declare. **A group is
      counted inside its run**: the tail holds rows of its own, and a
      heading above counting rows from below would point at things its
      fold cannot show. */
  const run = (list: SideRow[]) => {
    const size = new Map<string, number>();
    for (const r of list) size.set(r.group, (size.get(r.group) ?? 0) + 1);
    return list.map((r, i) => one(r, list[i - 1], size.get(r.group) ?? 0));
  };
  return (
    <div
      ref={box}
      data-side-list
      {...mark}
      // ↑ ↓ Home End walk the rows, the headings and the fold; ⏎ / Space
      // open the one with the ring — they are buttons (`lib/shortcuts`).
      onKeyDown={(e) => walkList(e, WALK)}
      className={narrow ? "pb-in" : "px-in pb-in"}
    >
      <DeleteDialog pending={pending} onDone={() => setPending(null)} />
      {run(fresh)}
      {/* The fold at the foot. A control, not a heading: its word is
          what pressing it does next. It never renders over nothing. */}
      {old && aged.length > 0 && (
        <button
          type="button"
          data-old-fold
          data-open={showOld}
          tabIndex={-1}
          onClick={() => setShowOld((v) => !v)}
          className={cn(
            "flex min-h-row-1 w-full items-center text-left text-caption text-tertiary hover:bg-fill",
            narrow ? "px-group" : "rounded-md px-row",
            RING,
          )}
        >
          {showOld ? old.less : old.more(aged.length)}
        </button>
      )}
      {showOld && run(aged)}
    </div>
  );
}
