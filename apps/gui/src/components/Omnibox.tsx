// The search box, grown into a scoped one: what used to be free text is
// now free text **plus chips**, one chip per thing the list can be
// narrowed by (用户裁决 2026-08-20 — 芯片类型即各页现有过滤轴).
//
// Three judgments hold this together:
//
// - **A chip is a filter's other face, never a second filter.** The keys
//   here are the same array the filter menu ticks, so ticking a word in
//   the menu puts a chip in the box and taking a chip out unticks it —
//   one piece of state with two faces. Two lists that each remembered
//   their own idea of "what is filtered" is the failure this is built to
//   make impossible, not a bug to be careful about.
// - **The candidates are the node's facts.** Every one of them is minted
//   from what the node sent (a state key it used, a machine in its table,
//   a category on a row); this file receives them and never invents one.
//   A pane with no axes to offer gets no chips at all — which is why the
//   devices pane has none, and not because anything here knows about
//   panes.
// - **A chip is a token.** It goes in whole and comes out whole; there is
//   no half-deleted chip, because half of `dev:turing` is not a thing the
//   list can be filtered by.
//
// The keyboard is the "/" menu's from `ChatView`, deliberately: the same
// ↑↓/Enter/Tab/Escape, the same `onMouseDown` guard that keeps focus in
// the box, and the same composition guard first of all — mid-组词 the
// Enter that picks a candidate belongs to the IME, and a box that read it
// would eat the choice and commit a half-typed word.
import { useEffect, useRef, useState, type ReactNode } from "react";

import type { Avatar } from "@/gen/bindings/Avatar";
import { MachineAvatar } from "@/components/Avatar";
import { IconClose, IconSearch } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";

/** One thing the list can be narrowed by. `key` is the node's own group
    key (`dev:turing`); `label` is what a person reads.

    `hint` and `dim` are one judgment in two halves, and they exist for
    a candidate that **is not a fact about right now**: a path offered
    out of memory while the machine that owns it is not answering. The
    hint names where it came from and the dimming says it is not a
    listing — either alone is the failure, because a remembered path
    that looks exactly like a live one is this box claiming to know
    something it does not. Both are absent on an ordinary candidate,
    which is how "no hint" stays readable as "this is live". */
export type Candidate = {
  key: string;
  label: string;
  face?: Avatar | null;
  hint?: string;
  dim?: boolean;
};

/**
 * One axis the box can offer, with the candidates it has right now.
 *
 * **`role` is the whole of the filter/navigation split**, and it lives
 * here rather than in the mechanism: chips, candidates and the keyboard
 * are the same either way, and what differs is only what committing one
 * *means*.
 *
 * - `chip` (the default): the candidate becomes a chip. What that chip
 *   then does — narrow a list, or name the thing the pane acts on — is
 *   the caller's `onToggle`, which is why a filter axis and a navigation
 *   axis need no different machinery here. A pane that wants one chip at
 *   a time simply replaces instead of appending.
 * - `text`: the candidate is a **completion**, not a chip. Committing it
 *   writes it into the box, because half a path is a real thing to keep
 *   typing and half a filter is not.
 */
export type OmniAxis = {
  key: string;
  label: string;
  candidates: Candidate[];
  role?: "chip" | "text";
};

/**
 * One committed candidate, as a token.
 *
 * **Exported because a second place now wears one**: the address bar's
 * resting face is a breadcrumb, not a box, and the machine still has to
 * be a chip there — same picture, same remove, same distance from what
 * follows it. Copied instead, the two would be one restyle apart from
 * being visibly different tokens for one idea, which is the whole
 * failure `PaneBar`'s head describes about search boxes.
 */
export function OmniChip({
  candidate,
  onRemove,
}: {
  candidate: Candidate;
  onRemove: () => void;
}) {
  return (
    <span
      data-chip={candidate.key}
      // **No vertical padding of its own any more.** The remove inside
      // it now stands at the finger's floor (tokens.css `--touch`), and
      // that is what a chip is as tall as: padding on top of it would
      // only push the box that holds it further past what the tallest
      // thing in it needs. The chip is a token either way — what
      // changed is that it is a token you can hit.
      className="flex flex-none items-center gap-1 rounded-sm bg-secondary pr-0.5 pl-1.5 text-sm"
    >
      {candidate.face !== undefined && candidate.face !== null && (
        <MachineAvatar face={candidate.face} className="size-kind-mark" />
      )}
      <span data-chip-label>{candidate.label}</span>
      <Button
        size="icon"
        variant="ghost"
        data-chip-remove={candidate.key}
        aria-label={`${gui.remove} ${candidate.label}`}
        // A floor rather than a size: `cn()` cannot lift `h-ctl-md` off
        // the icon variant (button.tsx has the measurement), and
        // `min-*` beats a height without needing to remove it. The
        // glyph does not grow — the box does, and the ghost wash paints
        // that box.
        className="min-h-touch min-w-touch"
        // The box must not lose focus to the press — where there is a
        // box. Harmless where there is not.
        onMouseDown={(e) => e.preventDefault()}
        onClick={onRemove}
      >
        <IconClose />
      </Button>
    </span>
  );
}

export function Omnibox({
  label,
  query,
  onQuery,
  axes,
  chosen,
  onToggle,
  onSubmit,
  lead,
  autoFocus,
  onLeave,
}: {
  /** Names the box and stands in as its placeholder — the pane's, not
      this component's, for the same reason the plain box took it. */
  label: string;
  query: string;
  onQuery: (q: string) => void;
  axes: OmniAxis[];
  /** The keys currently on: the filter's array, not a copy. */
  chosen: string[];
  onToggle: (key: string) => void;
  /**
   * Enter with nothing to choose from: the box has been typed into and
   * the person means "go". Only a pane that can *do* something with free
   * text passes one — where the box only narrows a list, the list is
   * already narrowed and there is nothing left for Enter to do.
   */
  onSubmit?: (text: string) => void;
  /**
   * The glyph in front of the box. Defaults to the magnifier, which is
   * what this control is on every pane that narrows a list — and is
   * wrong on the one that does not: an address bar wearing a magnifier
   * says "type words and I will look for them", which is the opposite
   * of "this is where you are".
   */
  lead?: ReactNode;
  /** Take the caret on mount. For a box that exists only because
      somebody just pressed the thing it replaced — pressing a
      breadcrumb and then having to click the box is one act done
      twice. */
  autoFocus?: boolean;
  /**
   * Done being typed in: Escape with no menu up, or focus gone
   * elsewhere. **Only a caller with somewhere to go back to passes
   * one** — a pane's search box is that pane's permanent furniture and
   * has no other state to return to, so it passes none and nothing
   * about it changes.
   */
  onLeave?: () => void;
}) {
  const [open, setOpen] = useState(false);
  const [pick, setPick] = useState(0);
  const box = useRef<HTMLInputElement>(null);

  // Everything not already on, filtered by what has been typed — matched
  // on the **label**, because that is the string the person is looking
  // at. Matching the key would mean typing `dev:` to find a machine.
  //
  // **The two roles match differently, and that is not a detail.** A
  // chip is *found*: you remember part of a machine's name and type it,
  // so a substring of the label is the right test. A completion is
  // *continued*: what is in the box is the beginning of the answer, so
  // the test is whether the candidate starts with it. Matching a
  // completion by substring would offer `/a/b/inner/` for the text
  // `/x/y/inn` — a path on a branch nobody is standing on.
  const q = query.trim().toLowerCase();
  const offered = axes
    .map((a) => ({
      ...a,
      candidates: a.candidates.filter((c) =>
        a.role === "text"
          ? c.key.toLowerCase().startsWith(q)
          : !chosen.includes(c.key) && (q === "" || c.label.toLowerCase().includes(q)),
      ),
    }))
    .filter((a) => a.candidates.length > 0);
  const flat = offered.flatMap((a) => a.candidates);
  // A pick that outlived the list it pointed into would commit whatever
  // slid into that position.
  const at = Math.min(pick, Math.max(0, flat.length - 1));

  useEffect(() => {
    setPick(0);
  }, [query]);

  /** Which axis a candidate came from — committing means two different
      things on the two roles, and the candidate alone does not say. */
  const axisOfCandidate = (key: string) =>
    axes.find((a) => a.candidates.some((c) => c.key === key));

  const commit = (key: string) => {
    if (axisOfCandidate(key)?.role === "text") {
      // A completion, not a chip: it goes into the box so typing can
      // carry on from it. The key *is* the completed text.
      onQuery(key);
    } else {
      onToggle(key);
      // The text was the way to find the chip; once it is a chip the
      // text has done its job, and leaving it would go on filtering.
      onQuery("");
    }
    setPick(0);
    box.current?.focus();
  };

  const chips = chosen
    .map((key) => {
      for (const a of axes) {
        const found = a.candidates.find((c) => c.key === key);
        if (found) return found;
      }
      // A key with no candidate behind it is still on, and still has to
      // be removable — that is the same rule the filter menu follows for
      // a ticked word whose last row left. The key stands in for a label
      // nobody can look up any more.
      return { key, label: key } as Candidate;
    })
    .filter(Boolean);

  return (
    <div className="relative min-w-0 flex-1">
      <div
        data-omnibox
        className={cn(
          // The strip is as tall as the floor and no taller: the box
          // inside it is the thing a finger lands on, so the padding
          // that used to sit above and below it would now only be
          // distance between the field's edge and the target — bought
          // at the cost of every bar this control rides in. Wrapped
          // chips still have `gap-1` between rows.
          "flex min-h-touch w-full flex-wrap items-center gap-1 rounded-md px-2",
          "focus-within:ring-2 focus-within:ring-ring/50",
        )}
        // A click anywhere on the strip lands in the box — the chips do
        // not take focus, so the whole thing behaves as one field.
        onMouseDown={(e) => {
          if (e.target === e.currentTarget) {
            e.preventDefault();
            box.current?.focus();
            setOpen(true);
          }
        }}
      >
        {lead ?? <IconSearch className="pointer-events-none flex-none text-muted-foreground" />}
        {chips.map((c) => (
          <OmniChip key={c.key} candidate={c} onRemove={() => onToggle(c.key)} />
        ))}
        <input
          ref={box}
          data-pane-search
          data-omni-input
          type="search"
          // eslint-disable-next-line jsx-a11y/no-autofocus
          autoFocus={autoFocus}
          aria-label={label}
          placeholder={chips.length === 0 ? label : ""}
          value={query}
          // **The one target on a browsing landing that is the landing**
          // — where you are and where you go — and it measured 18 tall,
          // the height of its own line of text. An `<input>` has no
          // pseudo-elements to hang a bigger hit area on, so its box is
          // its target and the box takes the floor (tokens.css
          // `--touch`). The strip above sheds its padding in exchange,
          // which is why the bar this rides in grows by four and not by
          // fourteen.
          className="min-h-touch min-w-0 flex-1 bg-transparent text-base outline-none placeholder:text-muted-foreground md:text-sm"
          onFocus={() => setOpen(true)}
          onBlur={() => {
            setOpen(false);
            onLeave?.();
          }}
          onChange={(e) => onQuery(e.target.value)}
          onKeyDown={(e) => {
            // First, before every branch: mid-composition these keys are
            // the IME's (`ChatView`'s "/" menu, same reason).
            if (e.nativeEvent.isComposing) return;
            // Second, and before the "nothing to choose from" return
            // below: Escape has to mean something in both cases. With a
            // menu up it puts the menu away; with none it is the way out
            // of the box for whoever has an out (see `onLeave`). Left
            // where it used to sit — after that return — the second
            // meaning was unreachable, which is exactly the state an
            // address bar spends most of its time in.
            if (e.key === "Escape") {
              e.preventDefault();
              if (open && flat.length > 0) setOpen(false);
              else onLeave?.();
              return;
            }
            if (e.key === "Backspace" && query === "" && chosen.length > 0) {
              // Whole, never half: the chip is one token, and there is
              // no such thing as filtering by part of a machine's name.
              e.preventDefault();
              onToggle(chosen[chosen.length - 1]);
              return;
            }
            if (flat.length === 0) {
              // Nothing to choose from, so Enter belongs to the pane: a
              // path typed out in full offers no completion and still
              // has to be openable.
              if (e.key === "Enter" && onSubmit) {
                e.preventDefault();
                onSubmit(query);
              }
              return;
            }
            if (!open) return;
            if (e.key === "ArrowDown" || e.key === "ArrowUp") {
              e.preventDefault();
              const step = e.key === "ArrowDown" ? 1 : flat.length - 1;
              setPick((p) => (Math.min(p, flat.length - 1) + step) % flat.length);
              return;
            }
            // **Tab completes, Enter goes.** On a chip axis the two are
            // one act — a chip is finished the moment it is chosen. On a
            // text axis they are not: Tab fills in the segment under the
            // cursor and leaves the caret to keep typing, Enter means
            // the text as it stands is the thing to open. Folding them
            // together would make it impossible to open anything whose
            // name is a prefix of something else.
            if (e.key === "Tab") {
              e.preventDefault();
              commit(flat[at].key);
              return;
            }
            if (e.key === "Enter") {
              e.preventDefault();
              if (axisOfCandidate(flat[at].key)?.role === "text") onSubmit?.(query);
              else commit(flat[at].key);
              return;
            }
          }}
        />
      </div>
      {open && flat.length > 0 && (
        <div
          data-omni-menu
          className="absolute top-full left-0 z-20 mt-1 max-h-60 w-full overflow-y-auto rounded-md border bg-popover p-1 shadow-md"
        >
          {offered.map((a) => (
            <div key={a.key}>
              <div data-omni-axis={a.key} className="px-2 py-1 text-xs text-muted-foreground">
                {a.label}
              </div>
              {a.candidates.map((c) => {
                const n = flat.indexOf(c);
                return (
                  <button
                    key={c.key}
                    type="button"
                    data-omni-item={c.key}
                    data-on={n === at}
                    className={cn(
                      // A menu row is chosen with a finger as often as
                      // with the keyboard, and the two ways have to aim
                      // at the same thing: `bg-accent` marks the picked
                      // row and the row is the floor tall, so what the
                      // arrow keys highlight is exactly what a press
                      // lands on (tokens.css `--touch`).
                      "flex min-h-touch w-full cursor-pointer items-center gap-2 rounded-sm px-2 py-1 text-left text-sm",
                      n === at && "bg-accent",
                      c.dim && "text-muted-foreground",
                    )}
                    // The box must not lose focus to the press, or the
                    // chip would land in a field nobody is typing in.
                    onMouseDown={(e) => e.preventDefault()}
                    onClick={() => commit(c.key)}
                  >
                    {c.face !== undefined && c.face !== null && (
                      <MachineAvatar face={c.face} className="size-kind-mark" />
                    )}
                    <span className="min-w-0 flex-1 truncate">{c.label}</span>
                    {/* Where it came from, when it came from memory
                        rather than from the machine. It rides the row it
                        describes — there is nowhere else for it to be
                        that would still be beside the thing it is
                        about. */}
                    {c.hint && (
                      <span data-omni-hint className="flex-none text-xs text-muted-foreground">
                        {c.hint}
                      </span>
                    )}
                  </button>
                );
              })}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
