import { Fragment, useLayoutEffect, useRef } from "react";

import type { SessionRow } from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { sessionMark } from "@/components/BrandIcons";
import { IconPin } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { ago, groupLabel, word } from "@/words";

/** The corner mark on a row's face: the shape is `sessionMark`'s one
    judgment; this only sizes it. */
function KindMark({ kind, category }: { kind: string; category: string | null }) {
  const M = sessionMark(kind, category);
  return <M className="size-kind-mark-glyph" />;
}

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
export function visibleSessions(rows: SessionRow[], query: string, words: string[]) {
  const q = query.trim().toLowerCase();
  return rows.filter(
    (r) =>
      (words.length === 0 || words.includes(r.word)) &&
      (q === "" ||
        `${r.title} ${r.id} ${r.source?.device ?? ""}`.toLowerCase().includes(q)),
  );
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
      row.style.transition = "none";
      row.style.transform = `translateY(${before - top}px)`;
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
  pinFailed,
}: {
  rows: SessionRow[];
  query: string;
  words: string[];
  selected: string | null;
  onSelect: (row: SessionRow) => void;
  onPin: (row: SessionRow) => void;
  /** Rows whose last pin attempt did not take — see `App`. */
  pinFailed: Set<string>;
}) {
  const shown = visibleSessions(rows, query, words);
  const box = useRowFlip(shown.map((r) => r.id).join("\n"));
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
  return (
    <div ref={box}>
      {shown.map((r, i) => (
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
              handle that does not move (mandala's FilesPane). */}
          {r.group !== "" && r.group !== shown[i - 1]?.group && (
            <div
              data-group={r.group}
              className="px-4 pt-3 pb-1 text-xs text-muted-foreground"
            >
              {groupLabel(r.group)}
            </div>
          )}
          {/* The row is a strip holding two controls, not a button
              itself: the pin cannot be nested inside the button that
              opens the row, because a button inside a button is not a
              thing the DOM has. The strip was cut one batch before the
              pin arrived, so no anchor in here had to move when it
              did. */}
          {/* `data-source` is the machine a reported row came from, as
              an anchor rather than only as painted text — verification
              asserts on data attributes, never on the words, which
              belong to the catalog. Absent on rows that live here,
              which is the same fact `source` itself carries. */}
          <div
          data-row={r.id}
          data-word={r.word}
          data-pinned={r.pinned}
          data-source={r.source?.device}
          data-title={r.title}
          className={cn(
            "flex w-full items-center pr-2 hover:bg-secondary",
            r.id === selected && "bg-accent hover:bg-accent",
          )}
        >
          <button
            type="button"
            data-row-open
            onClick={() => onSelect(r)}
            className="flex min-w-0 flex-1 items-center gap-3 py-2 pl-4 text-left"
          >
            {/* The machine's face carries the row (one per machine, what
                the eye scans by), and its corner wears the kind mark:
                shape says what this session is, colour says how it is —
                one mark answering two things (mandala's .wsbrand rule).
                The word itself stays at the row's end: state must never
                be colour alone. The mark sits on a solid punch-out, or
                the transparent brand strokes dissolve into the face. */}
            {/* `flex`, not inline: an inline wrap grows the baseline's few
                pixels under the face and the mark's corner offset bites
                into that gap instead of the face (mandala's .badged-face
                fix, the flex spelling of display:block+line-height:0). */}
            <span className="relative flex flex-none">
              <MachineAvatar face={r.face} className="size-avatar" />
              <span
                data-kind-mark={r.category ?? r.kind}
                className="absolute -bottom-0.5 -right-0.5 grid size-kind-mark place-items-center rounded-sm bg-card"
                style={{ color: `var(--state-${r.word})` }}
              >
                <KindMark kind={r.kind} category={r.category} />
              </span>
            </span>
            <span className="min-w-0 flex-1">
              <span className="block truncate">{r.title || r.id}</span>
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
                  {r.source && <span className="flex-none">{gui.from_device(r.source.device)}</span>}
                </span>
              )}
            </span>
            <span className="flex flex-none flex-col items-end gap-1 text-xs text-muted-foreground">
              {/* No stamp yet is no age — not an epoch-sized number. */}
              {r.at_ms > 0 && <span>{ago(r.at_ms)}</span>}
              {/* The trailing fine print: the state word (still a word —
                  state is never colour alone, the mark's tint is the
                  second channel not the only one), the unread count, and
                  the borrowed exit's mini face at the row's very corner
                  (docs/NET.md via 记号; absent until `--via` produces
                  one). */}
              <span className="flex items-center gap-1">
                <span data-word-text style={{ color: `var(--state-${r.word})` }}>
                  {word(r.word)}
                </span>
                {r.unread > 0 && (
                  <span
                    data-unread
                    className="min-w-4 rounded-full bg-badge px-1 text-center leading-4 text-badge-foreground"
                  >
                    {r.unread}
                  </span>
                )}
                {r.via && (
                  <span data-via={r.via} className="flex flex-none">
                    <MachineAvatar face={r.via_face} className="size-kind-mark" />
                  </span>
                )}
              </span>
            </span>
          </button>
          {/* The row's second control, and the reason the strip exists:
              a button cannot live inside the button that opens the row.
              **Always drawn, never hover-only** — the narrow face has no
              pointer to reveal it with, and a control you can only reach
              with a mouse is not reachable on a phone. Resting state is
              quiet (muted, tilted); pinned is upright and takes the
              foreground, so the mark is legible without hovering. */}
          {/* When the write did not take, the button says so where it
              was pressed: it goes to the failure colour and its name
              becomes the word for what failed. Without this, a failed
              pin and a missed click are the same picture — a row that
              did not move (docs/UX.md 做了但没变化 / 失败). The name
              tracks the action that was *attempted*, so it is the
              opposite of `r.pinned` from the un-refreshed row. */}
          <Button
            variant="ghost"
            size="icon"
            data-row-pin
            data-on={r.pinned}
            data-pin-failed={pinFailed.has(r.id)}
            aria-label={
              pinFailed.has(r.id)
                ? r.pinned
                  ? gui.unpin_failed
                  : gui.pin_failed
                : r.pinned
                  ? gui.unpin
                  : gui.pin
            }
            onClick={() => onPin(r)}
            className="flex-none text-muted-foreground data-[on=true]:text-foreground data-[pin-failed=true]:text-state-failed"
          >
            <IconPin pinned={r.pinned} />
          </Button>
        </div>
        </Fragment>
      ))}
    </div>
  );
}
