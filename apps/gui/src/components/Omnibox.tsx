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
import { useEffect, useRef, useState } from "react";

import type { Avatar } from "@/gen/bindings/Avatar";
import { MachineAvatar } from "@/components/Avatar";
import { IconClose, IconSearch } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";

/** One thing the list can be narrowed by. `key` is the node's own group
    key (`dev:turing`); `label` is what a person reads. */
export type Candidate = { key: string; label: string; face?: Avatar | null };

/** One axis of narrowing, with the candidates it offers right now. */
export type OmniAxis = { key: string; label: string; candidates: Candidate[] };

export function Omnibox({
  label,
  query,
  onQuery,
  axes,
  chosen,
  onToggle,
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
}) {
  const [open, setOpen] = useState(false);
  const [pick, setPick] = useState(0);
  const box = useRef<HTMLInputElement>(null);

  // Everything not already on, filtered by what has been typed — matched
  // on the **label**, because that is the string the person is looking
  // at. Matching the key would mean typing `dev:` to find a machine.
  const q = query.trim().toLowerCase();
  const offered = axes
    .map((a) => ({
      ...a,
      candidates: a.candidates.filter(
        (c) => !chosen.includes(c.key) && (q === "" || c.label.toLowerCase().includes(q)),
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

  const commit = (key: string) => {
    onToggle(key);
    // The text was the way to find the chip; once it is a chip the text
    // has done its job, and leaving it would go on filtering by it.
    onQuery("");
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
          "flex min-h-ctl-md w-full flex-wrap items-center gap-1 rounded-md px-2 py-1",
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
        <IconSearch className="pointer-events-none flex-none text-muted-foreground" />
        {chips.map((c) => (
          <span
            key={c.key}
            data-chip={c.key}
            className="flex flex-none items-center gap-1 rounded-sm bg-secondary py-0.5 pr-0.5 pl-1.5 text-sm"
          >
            {c.face !== undefined && c.face !== null && (
              <MachineAvatar face={c.face} className="size-kind-mark" />
            )}
            <span data-chip-label>{c.label}</span>
            <Button
              size="icon"
              variant="ghost"
              data-chip-remove={c.key}
              aria-label={`${gui.remove} ${c.label}`}
              className="size-ctl-sm"
              onMouseDown={(e) => e.preventDefault()}
              onClick={() => onToggle(c.key)}
            >
              <IconClose />
            </Button>
          </span>
        ))}
        <input
          ref={box}
          data-pane-search
          data-omni-input
          type="search"
          aria-label={label}
          placeholder={chips.length === 0 ? label : ""}
          value={query}
          className="min-w-0 flex-1 bg-transparent text-base outline-none placeholder:text-muted-foreground md:text-sm"
          onFocus={() => setOpen(true)}
          onBlur={() => setOpen(false)}
          onChange={(e) => onQuery(e.target.value)}
          onKeyDown={(e) => {
            // First, before every branch: mid-composition these keys are
            // the IME's (`ChatView`'s "/" menu, same reason).
            if (e.nativeEvent.isComposing) return;
            if (e.key === "Backspace" && query === "" && chosen.length > 0) {
              // Whole, never half: the chip is one token, and there is
              // no such thing as filtering by part of a machine's name.
              e.preventDefault();
              onToggle(chosen[chosen.length - 1]);
              return;
            }
            if (!open || flat.length === 0) return;
            if (e.key === "ArrowDown" || e.key === "ArrowUp") {
              e.preventDefault();
              const step = e.key === "ArrowDown" ? 1 : flat.length - 1;
              setPick((p) => (Math.min(p, flat.length - 1) + step) % flat.length);
              return;
            }
            if (e.key === "Enter" || e.key === "Tab") {
              e.preventDefault();
              commit(flat[at].key);
              return;
            }
            if (e.key === "Escape") {
              e.preventDefault();
              setOpen(false);
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
                      "flex w-full cursor-pointer items-center gap-2 rounded-sm px-2 py-1 text-left text-sm",
                      n === at && "bg-accent",
                    )}
                    // The box must not lose focus to the press, or the
                    // chip would land in a field nobody is typing in.
                    onMouseDown={(e) => e.preventDefault()}
                    onClick={() => commit(c.key)}
                  >
                    {c.face !== undefined && c.face !== null && (
                      <MachineAvatar face={c.face} className="size-kind-mark" />
                    )}
                    {c.label}
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
