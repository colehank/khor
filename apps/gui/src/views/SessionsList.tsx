import { Fragment, useLayoutEffect, useMemo, useRef, useState } from "react";

import { takeover, takeoverTerm, type SessionRow } from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { RowMenu, type RowAction } from "@/components/RowMenu";
import { SessionFace } from "@/components/SessionFace";
import { EditableName } from "@/components/EditableName";
import { IconBack } from "@/components/icons";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { closeWarning } from "@/views/DetailPane";
import { ago, axisOf, groupLabel, placeLabels, quietFor, valueOf, word } from "@/words";

/**
 * Which rows the pane bar's search and filter leave standing.
 *
 * The filter matches on the state **key**, the one the node sent — never
 * on the displayed word, and never on a state this layer worked out for
 * itself (docs/UX.md 状态呈现). Search reads what the row shows plus its
 * id, since the id is what the CLI prints and what people paste.
 *
 * **The machine a row came from is part of what the row shows**, so it
 * is part of what search reads — on a list gathering sessions from
 * several machines, "which ones are on turing" is the obvious question
 * to ask a search box, and it used to come back empty.
 *
 * Only reported rows carry that name, which is the same rule as
 * everything else here rather than a gap: `source` is the offline axis,
 * set on rows another device told us about, and a row living on this
 * machine prints no machine name. Search matches what is on the screen —
 * so a local row is not findable by this machine's name, because the row
 * never says it.
 */
export function visibleSessions(rows: SessionRow[], query: string, ticked: string[]) {
  const q = query.trim().toLowerCase();
  // **AND across axes, OR within one.** Both halves are decisions, not
  // defaults: ticking 忙碌 and 待批 means "either of these two", while
  // ticking 忙碌 and a machine means "busy ones on that machine". The
  // OR half is also what this list has always done with the state
  // words, so the two new axes join the rule rather than changing it.
  const wanted = new Map<string, Set<string>>();
  for (const key of ticked) {
    const axis = axisOf(key);
    const set = wanted.get(axis) ?? new Set<string>();
    set.add(valueOf(key));
    wanted.set(axis, set);
  }
  // What the row answers on each axis, in the node's own terms: the
  // state key it sent, the machine it lives on, and the category — with
  // the empty string for "could not tell", which is exactly the key the
  // node groups those rows under (`cat:` with nothing after it).
  const answers = (r: SessionRow): Record<string, string> => ({
    "state:": r.word,
    "dev:": r.home,
    "cat:": r.category ?? "",
    // The empty string for a row with no place, which is exactly the
    // key the node groups those under (`at:` with nothing after it) —
    // so ticking that group filters to it, like every other axis.
    "at:": r.place ?? "",
  });
  return rows.filter((r) => {
    const on = answers(r);
    for (const [axis, set] of wanted) if (!set.has(on[axis] ?? "")) return false;
    return (
      q === "" ||
      `${r.title} ${r.id} ${r.source?.device ?? ""} ${r.place ?? ""}`
        .toLowerCase()
        .includes(q)
    );
  });
}

/** How far a row is currently displaced from where it is laid out —
    the translateY of whatever transform it is wearing, mid-animation or
    not. `none` is the resting case and matrices are what the computed
    style hands back, so both are read rather than assumed. */
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
 * a pin lands, a machine reports in — and on a 2s poll the whole order
 * changes between two frames with nothing to say which row went where.
 * FLIP is the answer that needs no bookkeeping: measure where every row
 * *was*, let React paint where they now are, then put each one back
 * where it started and release it in the same tick.
 *
 * **`offsetTop`, not `getBoundingClientRect`**: the list is inside a
 * scroller, and a rect is relative to the viewport — scrolling one pixel
 * would read as every row having moved and set the whole list walking.
 *
 * **Rows only, never the headings.** A group heading is a boundary the
 * rows themselves declare (it exists because a row carried it), so a
 * heading is not a thing that moves — it appears and disappears, and
 * animating that would be animating an idea rather than an object.
 *
 * Under `prefers-reduced-motion` the whole measurement is skipped, not
 * merely given a zero duration: setting a transform and removing it is
 * two paints either way, and at zero duration that is a flicker instead
 * of a move.
 */
function useRowFlip(deps: string) {
  const box = useRef<HTMLDivElement>(null);
  const was = useRef(new Map<string, number>());
  useLayoutEffect(() => {
    const el = box.current;
    if (!el) return;
    const still = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const now = new Map<string, number>();
    for (const row of el.querySelectorAll<HTMLElement>("[data-row]")) {
      const id = row.dataset.row;
      if (!id) continue;
      const top = row.offsetTop;
      now.set(id, top);
      const before = was.current.get(id);
      // A row that was not on screen last time has no "from" to come
      // out of — it arrived, which is a different event and not this
      // one's to narrate.
      if (still || before === undefined || before === top) continue;
      // **Where the row is *seen*, not where it is laid out.** A second
      // reorder can land while the first is still running, and then the
      // row is mid-flight: its layout position is `before` but the eye
      // is looking at `before + carried`. Starting the new move from the
      // layout position would snap it back that far first, which is the
      // jump FLIP exists to remove — so the leftover is carried into the
      // new offset and the animation redirects from wherever it had got
      // to. This is also why the motion is a transition rather than a
      // keyframe animation: a transition to a new value simply retargets.
      const carried = offsetOf(row);
      row.style.transition = "none";
      row.style.transform = `translateY(${before - top + carried}px)`;
      requestAnimationFrame(() => {
        row.style.transition = "";
        row.style.transform = "";
      });
    }
    was.current = now;
    // `deps` is the order as a string: the rows arrive as new objects on
    // every poll, so identity says nothing, and the only change worth
    // measuring for is one that moved somebody.
  }, [deps]);
  return box;
}

// Order and words come from the node untouched: the list never
// re-derives a judgment the backend already made (docs/UX.md). That
// includes the order pinned rows arrive in — `Node::sessions` floats
// them, this file only paints the mark. **There is no comparison
// function in here, and adding one is the bug.**
export function SessionsList({
  rows,
  query,
  words,
  selected,
  onSelect,
  onPin,
  onClose,
  pinFailed,
  closeRefused,
  showVia = true,
}: {
  rows: SessionRow[];
  query: string;
  words: string[];
  selected: string | null;
  onSelect: (row: SessionRow) => void;
  /** **Rejects when the write did not take** — that is how the menu
      knows to stay open on the item that was pressed, instead of
      closing over a row that did not move (`RowMenu`, `App`). */
  onPin: (row: SessionRow) => Promise<void>;
  /** Rejects the same way, and for the same reason; a refusal also
      lands in `closeRefused` as a whole sentence. */
  onClose: (row: SessionRow) => Promise<void>;
  /** Rows whose last pin attempt did not take — see `App`. */
  pinFailed: Set<string>;
  /** What the node said when it would not close a row, by row id. */
  closeRefused: Map<string, string>;
  /** Whether a row says which machine it came through (工作区批③).
      **Off where the surface already said it once**: the workspace
      pane's column is one machine's directory, named in the header
      directly above it, and a face repeated on every row underneath is
      the same fact said twice — which docs/UX.md spends on nothing.
      On by default, because the sessions landing gathers rows from the
      whole mesh and there the machine is the thing the eye scans by. */
  showVia?: boolean;
}) {
  /** What the node said when it would not change a row's form, by row
      id. Owned here rather than in `App` because nothing else needs
      it: unlike a pin, the conversion has no optimistic state to keep
      — the row comes back in its new form on the next poll, or it does
      not come back at all and this sentence says why. */
  const [turnRefused, setTurnRefused] = useState<Map<string, string>>(new Map());
  const shown = visibleSessions(rows, query, words);
  // The fold (退场批): old news — dead, looked at, past the threshold,
  // all judged by the node (`SessionRow::stale`) — sits behind one line
  // at the list's end. **Searching and filtering read everything**: a
  // query is the person asking for a row wherever it is, so the fold
  // applies only to the untouched default view. Nothing is deleted —
  // the rows are in `shown` either way.
  const searching = query.trim() !== "" || words.length > 0;
  const fresh = searching ? shown : shown.filter((r) => !r.stale);
  const old = searching ? [] : shown.filter((r) => r.stale);
  const [showOld, setShowOld] = useState(false);
  /**
   * Groups folded away, by the node's own group key.
   *
   * **Keyed by the group, not by a position or a mode**, so a fold
   * survives everything that is not about it: a re-poll, a re-sort, a
   * filter that empties the group and a later one that fills it again.
   * Changing the arrangement brings different keys, so another mode's
   * groups are neither shut nor remembered — they are simply not in
   * here.
   *
   * **Every group folds, not only 置顶.** Singling that one out would
   * mean writing `khor_node::list::GROUP_PINNED` into this file — a
   * backend constant copied into the face — and it would show the
   * moment somebody changes the arrangement: one heading wearing a
   * chevron among neighbours that do not. In the default arrangement
   * (`Arrange::Recent`) pinned rows are the only group there is; every
   * other row carries an empty key and draws no heading at all, so on
   * that screen this is the pinned fold and nothing else.
   *
   * Session-lived, like the tail's fold below: a fold is a view of this
   * list right now, not a setting, and two folds on one list may not
   * answer to two different lifetimes.
   */
  const [shut, setShut] = useState<Set<string>>(() => new Set());
  const shutOf = (group: string) => group !== "" && shut.has(group);
  const toggleGroup = (group: string) =>
    setShut((was) => {
      const next = new Set(was);
      if (!next.delete(group)) next.add(group);
      return next;
    });
  const box = useRowFlip(
    (showOld ? [...fresh, ...old] : fresh)
      // What is on the glass, not what survived the filter: a fold
      // changes which rows exist to be measured, and a row that comes
      // back arrives rather than moves.
      .filter((r) => !shutOf(r.group))
      .map((r) => r.id)
      .join("\n"),
  );
  // Place headings, told apart where two checkouts share a name
  // (`placeLabels`). Over the rows on screen, because those are the
  // groups that can collide with each other.
  const placeNames = useMemo(() => placeLabels(shown.map((r) => r.group)), [shown]);
  if (shown.length === 0) {
    // "Nothing here" and "nothing matched" are different facts, and the
    // wrong one is a lie the user has no way to catch: someone who
    // filtered down to zero would read "还没有 session" and believe the
    // machine had lost their work.
    return (
      <div data-empty className="p-4 text-sm text-muted-foreground">
        {rows.length === 0 ? gui.empty_sessions : gui.no_matches}
      </div>
    );
  }
  /**
   * What a row can be told to do, on right-click and on long-press.
   *
   * **Two items, and the second one is why there is a menu at all.** A
   * menu holding one thing is a menu that teaches people not to open
   * menus, which is the same judgment `App` makes about an empty "+".
   *
   * **Rename is deliberately not here.** It already has a way in — the
   * title's own double-click and F2 (`EditableName`) — and a second
   * path to one action is the thing this repo keeps taking back out.
   * The two that are here had no way in that a finger could reach:
   * the pin was a hover-only button, and close lived one screen away in
   * the detail header. 复制 id came the same road on 2026-09-02 — the
   * detail header sheds it (「id用户知道也没用」), and its one real
   * reader, a person wiring this session up from another machine,
   * finds it here beside the other things done *to* a row.
   */
  /**
   * Changing what a session **is**: the same act both ways round, and
   * the row's menu is where it belongs.
   *
   * **It used to be a tab.** The detail pane carried a 终端/对话 strip
   * whose second face, on a conversation, was a card reading "there is
   * no terminal here, press this to make one" — a conversion wearing a
   * view's clothes, on a strip that cost every session two controls.
   * Now the pane shows the form the host is actually running
   * (`DetailPane`) and this is the one way to change it, beside 置顶
   * and 关闭 which also change a row rather than look at one.
   *
   * **Absent, not greyed, where it cannot happen.** A row on another
   * machine, a shell with no conversation to have, a vendor khor has
   * no shim for: none of them has a second form, so none of them gets
   * a word about one.
   *
   * Both directions carry a confirm, because both end something that
   * is running: which form is ending is the one thing a person must
   * not have to guess from the label (`takeover_*` in the catalog).
   */
  const turnInto = (r: SessionRow): RowAction[] => {
    const refused = turnRefused.get(r.id);
    const said = (e: unknown) =>
      setTurnRefused((m) => new Map(m).set(r.id, String(e instanceof Error ? e.message : e)));
    // Somebody else's machine hosts it; the act happens where the
    // process is.
    if (r.source) return [];
    if (r.kind === "gui") {
      return [
        {
          key: "to-a-terminal",
          label: gui.to_a_terminal,
          confirm: r.word === "busy" ? gui.takeover_term_busy : gui.takeover_term,
          note: refused,
          run: async () => {
            try {
              await takeoverTerm(r.id);
            } catch (e) {
              said(e);
              throw e;
            }
          },
        },
      ];
    }
    // Only a vendor khor can speak for has a conversation to move
    // into — the same judgment the offer inside the read-only chat
    // used to make.
    if (r.category !== "claude") return [];
    return [
      {
        key: "to-a-chat",
        label: gui.to_a_chat,
        confirm: r.word === "busy" ? gui.takeover_busy : gui.takeover_calm,
        note: refused,
        run: async () => {
          try {
            await takeover(r.id);
          } catch (e) {
            said(e);
            throw e;
          }
        },
      },
    ];
  };
  const actions = (r: SessionRow): RowAction[] => [
    ...turnInto(r),
    {
      key: "pin",
      // The word is what pressing it *does*, so it is the opposite of
      // the row's current state — which is also what makes this item
      // say what the state is. When the last attempt did not take, the
      // word becomes the one for what failed, said on the very thing
      // that was pressed (docs/UX.md 做了但没变化 / 失败). It tracks the
      // action that was **attempted**, so it stays the opposite of
      // `r.pinned` read off the un-refreshed row.
      label: pinFailed.has(r.id)
        ? r.pinned
          ? gui.unpin_failed
          : gui.pin_failed
        : r.pinned
          ? gui.unpin
          : gui.pin,
      failed: pinFailed.has(r.id),
      run: () => onPin(r),
    },
    {
      // The id is paste material, not reading material. The menu
      // closing is the success face; a clipboard refusal rejects, so
      // the menu stays open where the press was made (RowMenu's rule:
      // rejecting is how an action says it did not happen).
      key: "copy-id",
      label: gui.copy_id,
      run: () => navigator.clipboard.writeText(r.id),
    },
    {
      key: "close",
      label: gui.close,
      // This row's own sentence, from the one place that map lives
      // (`DetailPane.closeWarning`). Writing the kinds out again here
      // would be a second source that compiles, runs, and says the
      // wrong thing.
      confirm: closeWarning(r.kind),
      note: closeRefused.get(r.id),
      run: () => onClose(r),
    },
  ];
  const strip = (r: SessionRow, prev: SessionRow | undefined, size: number) => {
    const open = !shutOf(r.group);
    return (
        <Fragment key={r.id}>
          {/* A heading starts where the group changes between
              neighbouring rows. The rows arrive grouped, so this notices
              a boundary rather than deciding one — and it means the
              empty group (the mode that does not group) prints no
              headings at all, with no special case.
              **An empty group cannot be drawn here**, and not by luck: a
              heading only exists because a row carried it, so there is
              no path that renders a group with nothing under it. A
              heading over nothing is worse than no heading — it is a
              handle that does not move (mandala's FilesPane). **A shut
              group is not a counter-example**: its rows are still here,
              still declaring the boundary, and one press brings them
              back — which is precisely why the fold is drawn from the
              rows rather than from a list of group names. */}
          {r.group !== "" && r.group !== prev?.group && (
            // The heading is also the group's handle. **Unlike the fold
            // line at the tail, a heading has a second job** — it names
            // the group — so what pressing it does next is said by the
            // chevron and by `aria-expanded`, not by swapping the word
            // out. That line at the end of the list can swap its word
            // because naming is all it would otherwise do.
            //
            // The count appears only while the group is shut: open, the
            // rows count themselves.
            <button
              type="button"
              data-group={r.group}
              data-group-open={open}
              aria-expanded={open}
              onClick={() => toggleGroup(r.group)}
              // A full-width strip, so only its height was ever under
              // the floor — `min-h-touch` grows the band the words sit
              // in and the hover wash is that band, exactly as the
              // tail's fold does it.
              className="flex min-h-touch w-full items-center gap-1 px-4 text-left text-xs font-label text-muted-foreground hover:bg-secondary"
            >
              {/* The family's only chevron, turned (icons.tsx is one
                  family, no exceptions, and rotating a mark to say which
                  way it means is what `IconPin` already does): pointing
                  down is open, pointing right is shut. */}
              <IconBack className={cn("flex-none", open ? "-rotate-90" : "rotate-180")} />
              <span className="min-w-0 truncate">{placeNames.get(r.group) ?? groupLabel(r.group)}</span>
              {!open && (
                <span data-group-size className="tabular-nums">
                  {size}
                </span>
              )}
            </button>
          )}
          {/* The row is a strip, not a button itself — it was cut that
              way for a pin that had to sit outside the button that
              opens the row (a button inside a button is not a thing the
              DOM has). **The pin is gone and the strip stays**, now for
              a different reason: right-click and long-press belong to
              the whole row, and the strip is what "the whole row" is.
              **Where the pinned fact went**: it used to be said twice —
              by the group heading a pinned row sits under and by a pin
              that stayed drawn on the row. The heading is `Node::sessions`'
              own judgment and is on screen in all four arrangements; the
              row's copy was the one that needed a pointer to appear.
              What replaces it is more exact than either: the menu's own
              first item reads 置顶 or 取消置顶, which states the row's
              current state at the moment somebody asks. */}
          {/* `data-source` is the machine a reported row came from, as
              an anchor rather than only as painted text — verification
              asserts on data attributes, never on the words, which
              belong to the catalog. Absent on rows that live here,
              which is the same fact `source` itself carries.
              `data-home` is its sibling and a **different fact**: which
              machine the session lives on, present on every row
              (`SessionRow::home`). The machine filter reads that one —
              an axis built on `source` would be missing this machine. */}
          {/* `data-quiet` greys the whole strip (词的保鲜期批): the
              word has outlived its believability — the machine is
              reachable, the signal is not. The word itself stands; the
              age line below says how long the silence is. Same axis as
              离线, worn the same way (dimmed, never re-worded). */}
          {open && (
          <div
          data-row={r.id}
          // **The front row, said in the DOM and not only in a colour**
          // (2026-09-16). It was `bg-accent` and nothing else, which a
          // gate can only read by comparing two computed colours; the
          // workspace list's own rows have carried this attribute since
          // the day 「一个工作区被选中了没有任何状态」 was answered, and
          // the two lists are one gesture.
          data-selected={r.id === selected}
          data-mux={r.mux}
          data-word={r.word}
          data-pinned={r.pinned}
          data-quiet={r.quiet}
          data-source={r.source?.device}
          data-home={r.home}
          data-title={r.title}
          className={cn(
            // No `group/row` any more: that name existed for exactly one
            // reader, the pin's `group-hover/row:opacity-100`, and a
            // named group nobody names back is a handle left tied to
            // something that is gone.
            // No day strip on the row any more: the band moved under
            // the open session's header (2026-09-02), where it is one
            // fact about one session rather than a timeline per row.
            "flex w-full items-center hover:bg-secondary",
            r.id === selected && "bg-accent hover:bg-accent",
            r.quiet && "opacity-55",
          )}
        >
          {/* **The menu's trigger is this button, not the strip around
              it**, and the `pr-2` moved in here so that the button is
              the whole row and the two are the same target either way.
              The reason is not tidiness: the trigger carries an
              `onPointerDown` (that is the long press), and a press
              handler on a bare `<div>` is a target only a pointer can
              find — `check-face`'s section 4 asserts against exactly
              that, and it would have been right to. On the button, the
              handler sits on something the DOM already admits is a
              target: focus reaches it, and a keyboard's Menu key fires
              `contextmenu` on the focused element, which is what opens
              this. */}
          <RowMenu items={actions(r)}>
          <button
            type="button"
            data-row-open
            onClick={() => onSelect(r)}
            className="flex min-w-0 flex-1 items-center gap-3 py-2 pr-2 pl-4 text-left"
          >
            {/* The machine's face carries the row (one per machine, what
                the eye scans by); around it the state's halo, on its
                corner the kind mark, at the row's end the word. The
                assembly and its three-channels judgment live in
                `SessionFace` now — the tabs and the narrow detail
                header draw the same one, which is the whole point of it
                being one component. The word stays on the row
                regardless: state may never be colour alone, which is
                also what lets the halo *be* a colour. */}
            <SessionFace face={r.face} word={r.word} kind={r.kind} category={r.category} />
            <span className="min-w-0 flex-1">
              {/* Double-click the title text to rename it in place —
                  the same component the detail header wears, so the two
                  cannot drift. The hot zone is the text alone: the rest
                  of the row keeps opening the row, which is also what
                  the first click of the double-click does — selecting
                  the very row being renamed. */}
              <EditableName
                id={r.id}
                title={r.title}
                className="block font-title"
                inputClassName="w-full font-title"
                // Inside the row's own button: not a control here
                // (`EditableName`'s head says why the path lives in the
                // detail header instead).
                nested
              />
              {/* The preview line: the newest thing said or shown, the
                  kind's own judgment carried on the row (`last`). The
                  machine a reported row came from rides the same line —
                  both are "where this row has been", not state. */}
              {(r.last || r.source) && (
                <span className="flex items-baseline gap-2 overflow-hidden whitespace-nowrap text-sm text-muted-foreground">
                  {r.last && (
                    <span data-last className="min-w-0 flex-1 truncate">
                      {r.last}
                    </span>
                  )}
                  {/* `data-row-source` names the fact for the smoke:
                      the detail header used to carry the origin and no
                      longer does (2026-09-02 标题瘦身), so the row's
                      own line is where「a local row names no machine,
                      a reported one does」is asserted. */}
                  {r.source && (
                    <span data-row-source className="flex-none">{gui.from_device(r.source.device)}</span>
                  )}
                </span>
              )}
            </span>
            {/* The row's right column, in the two tiers tokens.css names
                (三 tiers 注释): **the state word is this row's `fact`,
                the timestamp is `aux`.** They used to be one thing — same
                size, same grey, the stamp on top — and the definition
                that hands out the tiers points at exactly these two as
                opposite examples ("a reading, a state word, a count" against
                "ids, ages, denominators"). Two lines the vocabulary calls
                different, wearing the same clothes, is a block with no
                reading order: you find the word by knowing where it sits.

                **Rank said once.** The word takes the size axis and the
                first line; it keeps the weight it always had (400), because
                a line that is both a size above its neighbours and heavier
                is shouting (ink weights 注释), and it keeps the colour it
                always had, because that channel is already spoken for by
                the six words themselves — a second job for it would be the
                brand green's mistake again.

                Ordering the fact above the note is the other half of the
                same move, and it lands the column on the left half's lines:
                title beside word, preview beside age. */}
            <span className="flex flex-none flex-col items-end gap-1 text-muted-foreground">
              {/* The state word (still a word — state is never colour
                  alone, the halo's tint is the second channel not the only
                  one), the unread count, and the borrowed exit's mini face
                  at the row's very corner (docs/NET.md via 记号; absent
                  until `--via` produces one). */}
              <span className="flex items-center gap-1 text-fact">
                <span data-word-text style={{ color: `var(--state-${r.word})` }}>
                  {word(r.word)}
                </span>
                {/* The count takes no size of its own: it rides the fact
                    line, and "a count" is one of the three things the
                    fact tier is defined by. The pill's own `leading-4`
                    fixes its height either way — measured at both sizes,
                    16 tall in a 21 line, so this is the tier speaking and
                    not a fit. */}
                {r.unread > 0 && (
                  <span
                    data-unread
                    className="min-w-4 rounded-full bg-badge px-1 text-center leading-4 text-badge-foreground"
                  >
                    {r.unread}
                  </span>
                )}
                {showVia && r.via && (
                  <span data-via={r.via} className="flex flex-none">
                    <MachineAvatar face={r.via_face} className="size-kind-mark" />
                  </span>
                )}
              </span>
              {/* No stamp yet is no age — not an epoch-sized number.
                  On a quiet row the age line IS the note: 「N 天没消息」
                  says the same duration with the fact that matters. */}
              {r.at_ms > 0 &&
                (r.quiet ? (
                  <span data-quiet-note className="text-aux">
                    {quietFor(r.at_ms)}
                  </span>
                ) : (
                  <span className="text-aux tabular-nums">{ago(r.at_ms)}</span>
                ))}
            </span>
          </button>
          </RowMenu>
        </div>
          )}
        </Fragment>
    );
  };
  /** One run of rows, plus the headings those rows declare.
      **A group is counted inside its run**, never across the list: the
      tail below holds rows of its own, and a heading down there
      reporting a number that included rows from above would be a count
      pointing at things the fold cannot show. */
  const run = (list: SessionRow[]) => {
    const size = new Map<string, number>();
    for (const r of list) size.set(r.group, (size.get(r.group) ?? 0) + 1);
    return list.map((r, i) => strip(r, list[i - 1], size.get(r.group) ?? 0));
  };
  return (
    <div ref={box}>
      {run(fresh)}
      {/* The folded tail. A control, not a heading: pressing it is what
          changes the view, and its word is what pressing it does next
          (the pin's rule). It never renders over nothing — `old` empty
          means no line at all. */}
      {old.length > 0 && (
        <button
          type="button"
          data-old-fold
          data-open={showOld}
          onClick={() => setShowOld((v) => !v)}
          // A full-width strip, so only its height was ever under the
          // floor; `min-h-touch` with the items centred keeps the words
          // where they were and grows the band they sit in — and the
          // hover wash is that band, not a smaller one inside it.
          className="flex min-h-touch w-full items-center px-4 py-2 text-left text-xs text-muted-foreground hover:bg-secondary"
        >
          {showOld ? gui.collapse_old : gui.old_sessions(old.length)}
        </button>
      )}
      {showOld && run(old)}
    </div>
  );
}
