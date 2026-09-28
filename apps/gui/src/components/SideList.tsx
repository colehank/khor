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
//   row      a face with a corner mark (`FaceMark`), a title over a
//            subtitle, whatever the column puts at the right end;
//   motion   rows walk to their new places (`useRowFlip`);
//   the old  rows past their time fold behind one line at the foot.
//
// It came out of `SessionsList`, which was the one list that already
// did all of this; the other four had each grown part of it.
import { Fragment, useLayoutEffect, useRef, useState, type ReactNode } from "react";

import { EmptyState, RegionState } from "@/components/RegionState";
import { DeleteDialog, type Pending } from "@/components/DeleteDialog";
import { FaceMark } from "@/components/FaceMark";
import { IconChevronRight } from "@/components/icons";
import { RowMenu, type RowAction } from "@/components/RowMenu";
import { cn } from "@/lib/utils";
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
  /** The face, already sized. */
  face: ReactNode;
  /** The corner mark and what it says (a key, for the gates). `beside`
      stands it next to the face rather than on it — for a face with
      words in its middle (`FaceMark`). */
  badge?: { mark: string; icon: ReactNode; data?: Record<string, string>; beside?: boolean };
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
  none,
  old,
  mark,
  filtered = false,
  headData,
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
}) {
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
  const shutOf = (group: string) => group !== "" && shut.has(group);
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

  if (loading && rows.length === 0) {
    return (
      <div data-side-list {...mark} data-loading>
        <RegionState state="loading-never" />
      </div>
    );
  }
  if (shown.length === 0) {
    return (
      <div data-side-list {...mark} data-empty className="flex min-h-0 flex-1 flex-col">
        <EmptyState>{rows.length === 0 ? empty : none}</EmptyState>
      </div>
    );
  }

  const one = (r: SideRow, prev: SideRow | undefined, size: number) => {
    const open = !shutOf(r.group);
    const button = (
      <button
        type="button"
        data-side-open
        {...r.openData}
        onClick={r.onOpen}
        className="flex min-h-row-2 min-w-0 flex-1 items-center gap-row px-in py-1 text-left"
      >
        <FaceMark
          face={r.face}
          halo={r.halo}
          badge={r.badge?.icon}
          mark={r.badge?.mark}
          badgeData={r.badge?.data}
          beside={r.badge?.beside}
        />
        <span className="min-w-0 flex-1">
          <span data-side-title className="block min-w-0">
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
      </button>
    );
    return (
      <Fragment key={r.id}>
        {/* One rule where the pinned group ends and the rest begins
            (docs/SHELL.md 第七轮 第 1 条), noticed between neighbours like
            the heading below — so it is drawn only while both groups
            have rows. A folded group's rows are not drawn but its last
            one is still this row's neighbour, which leaves the rule
            right under the heading. The seam's colour: `border-t` is
            `--color-border`, the same token `ListPane`'s seam wears. */}
        {prev?.group === "pin" && r.group !== "pin" && (
          <div data-side-list-rule aria-hidden className="mx-in my-xxs border-t" />
        )}
        {/* A heading starts where the group changes between neighbouring
            rows: the rows arrive grouped, so this notices a boundary
            rather than deciding one, and a heading can never be drawn
            over nothing. It is also the group's handle; what pressing it
            does next is said by the chevron and `aria-expanded`, and the
            count shows only while the group is shut. */}
        {r.group !== "" && r.group !== prev?.group && (
          <button
            type="button"
            data-side-group={r.group}
            data-group={r.group}
            data-group-open={open}
            {...headData?.(r.group)}
            aria-expanded={open}
            onClick={() => toggle(r.group)}
            className="flex min-h-touch w-full items-center gap-xxs rounded-md px-in pt-in pb-xxs text-left text-group text-tertiary"
          >
            {/* The family's chevron at 12 (components/GroupHeader), right
                when shut and turned a quarter when open — it is centred
                on 8,8, so turned it IS `chevronDown`. Not the back arrow
                turned: that path sits 0.4 off centre, which is what read
                as crooked (2026-09-26「展开按钮的icon甚至都没有整齐」). */}
            <IconChevronRight className={cn("size-3 flex-none transition-transform", open && "rotate-90")} />
            <span data-group-label className="min-w-0 truncate">{label(r.group)}</span>
            <span data-group-size className="ml-auto flex-none font-body tabular-nums">
              {size}
            </span>
          </button>
        )}
        {open && (
          <div
            data-side-row={r.id}
            data-side-pinned={r.pinned}
            data-side-in={r.group}
            data-side-menu={r.menu.length > 0}
            data-selected={!!r.selected}
            {...r.rowData}
            // **Selected is painted by `app.css`, keyed on
            // `data-selected`** (第十五轮 第 2 条, D6): the brand's solid
            // green with its own ink while the window is in front, grey
            // with the ordinary ink when it is not — two states and an
            // ink that changes with them, which a class here could not
            // say without knowing about the window.
            className={cn("flex w-full items-center rounded-md", !r.selected && "hover:bg-fill", r.dim && !r.selected && "opacity-55")}
          >
            {/* **The menu's trigger is the button, not the strip around
                it**: the trigger carries the long press's
                `onPointerDown`, and a press handler on a bare `<div>` is
                a target only a pointer can find. On the button, focus
                reaches it and the keyboard's Menu key opens it. */}
            {r.menu.length > 0 ? <RowMenu items={menuOf(r)}>{button}</RowMenu> : button}
            {r.after}
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
    <div ref={box} data-side-list {...mark} className="px-in pb-in">
      <DeleteDialog pending={pending} onDone={() => setPending(null)} />
      {run(fresh)}
      {/* The fold at the foot. A control, not a heading: its word is
          what pressing it does next. It never renders over nothing. */}
      {old && aged.length > 0 && (
        <button
          type="button"
          data-old-fold
          data-open={showOld}
          onClick={() => setShowOld((v) => !v)}
          className="flex min-h-row-1 w-full items-center rounded-md px-in text-left text-caption text-tertiary hover:bg-fill"
        >
          {showOld ? old.less : old.more(aged.length)}
        </button>
      )}
      {showOld && run(aged)}
    </div>
  );
}
