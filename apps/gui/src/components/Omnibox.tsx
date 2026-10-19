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

import { MachineAvatar } from "@/components/Avatar";
import { IconClose, IconSearch } from "@/components/icons";
import { OmniMenu, useFloatedMenu, type Candidate, type MenuRows, type OmniAxis } from "@/components/OmniMenu";
import { Button } from "@/components/ui/button";
import { gui } from "@/gen/catalog";
import { useCoarse } from "@/hooks/use-coarse";
import { cn } from "@/lib/utils";
import { useFieldDismiss } from "@/hooks/use-field-dismiss";
import { imeKey } from "@/lib/ime";

// **The rows live with the rows.** A menu is drawn in two places now — in
// this page, and in a small window of the app's own when a browsing tab's
// page would otherwise cover it — so the list, its shape and its cap moved
// to the one file both draw from (`components/OmniMenu`). Re-exported
// because every pane in this app names its axes from here, and a rename
// across all of them buys nothing.
export type { Candidate, MenuRows, OmniAxis };

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
      className="flex flex-none items-center gap-1 rounded-sm bg-secondary pr-0.5 pl-1.5 text-callout"
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
        <IconClose className="size-affordance" />
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
  float,
  field = false,
  trail,
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
  /**
   * Draw the completions in a window of the app's own rather than in this
   * page.
   *
   * **For the one box that has something painted over it.** A browsing
   * tab's page is a native child webview and those always paint above
   * this page's DOM, so the address bar's completions came out
   * underneath the site — 「不对,应该是浮在页面上的」. Nowhere else in
   * this app has that problem, and a window is not free (it is a second
   * webview, and a click into it is a click into another window), so
   * every other box goes on drawing its list where it always did.
   *
   * Asking for it is not getting it: a browser tab and a phone answer no
   * and the list is drawn here (`useFloatedMenu`).
   */
  float?: boolean;
  /**
   * **This box is a list's search field** (docs/design-system
   * components/SearchField): the strip is marked `data-search-field`
   * and its words wear the field's type — `body`, the placeholder in
   * `tertiary` — so a landing's box reads the same whether it can
   * narrow by chips or not. The address bar is not a search field and
   * keeps its own.
   */
  field?: boolean;
  /** What stands at the strip's right end — a seated box's ⌘K hint. */
  trail?: ReactNode;
}) {
  const [open, setOpen] = useState(false);
  const [pick, setPick] = useState(0);
  /** Whether the arrows have moved the pick since the text changed. */
  const [walked, setWalked] = useState(false);
  const coarse = useCoarse();
  const box = useRef<HTMLInputElement>(null);
  // What the menu hangs under, in both places it can be drawn: the strip
  // is the control, and a completion list narrower than the box it
  // completes reads as belonging to something else.
  const strip = useRef<HTMLDivElement>(null);
  useFieldDismiss(open, () => [strip.current], () => {
    if (document.activeElement === box.current) box.current?.blur();
    else { setOpen(false); onLeave?.(); }
  });

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
    setWalked(false);
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

  /**
   * A row was pressed, with a finger or a mouse.
   *
   * **Not `commit`, and the difference is Chrome's.** Tab *completes* —
   * it writes the candidate into the box and leaves the caret to keep
   * typing — because half a path is a real thing to go on typing. A
   * press is not that: nobody points at a row in order to have it typed
   * out for them, they point at it to go there. Measured 2026-09-09 with
   * somebody pressing a suggestion and reporting 「点击内容无反应」 — the
   * press had landed, the address had gone into the box, and nothing had
   * happened, which from the outside is indistinguishable from a dead
   * control.
   *
   * A chip axis has no such split: a chip is finished the moment it is
   * chosen, so there the press and Tab are the same act.
   */
  const press = (key: string) => {
    if (axisOfCandidate(key)?.role !== "text") {
      commit(key);
      return;
    }
    onQuery(key);
    setPick(0);
    onSubmit?.(key);
  };

  const shown = open && flat.length > 0;
  const floated = useFloatedMenu(
    strip,
    shown ? { groups: offered, at } : null,
    press,
    float === true,
  );

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
    <div ref={strip} className="relative min-w-0 flex-1">
      <div
        data-omnibox
        data-search-field={field ? "" : undefined}
        className={cn(
          // The strip is as tall as the floor and no taller: the box
          // inside it is the thing a finger lands on, so the padding
          // that used to sit above and below it would now only be
          // distance between the field's edge and the target — bought
          // at the cost of every bar this control rides in. Wrapped
          // chips still have `gap-1` between rows.
          "flex min-h-touch w-full flex-wrap items-center gap-xxs rounded-sm bg-fill px-2",
          // The text field's focus ring (components/TextField,
          // 2026-09-28): 2 px of `ring` flush outside the strip — it was
          // a half-strength glow here and nothing on the plain box.
          "focus-within:outline-2 focus-within:outline-offset-0 focus-within:outline-ring",
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
        {lead ?? <IconSearch className="pointer-events-none flex-none text-tertiary" />}
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
          className={cn(
            "min-h-touch min-w-0 flex-1 bg-transparent text-body outline-none",
            field ? "placeholder:text-tertiary" : "placeholder:text-muted-foreground md:text-callout",
          )}
          onFocus={() => setOpen(true)}
          // **Focus is not the only way in, and Escape is why.** Escape
          // with a menu up puts the menu away and leaves the caret where
          // it is, so the next press on a box that already has focus
          // fires no `focus` at all — and the list never came back until
          // somebody typed or clicked away and returned. Measured
          // 2026-09-09; it reads as a control that works once.
          onMouseDown={() => setOpen(true)}
          onBlur={() => {
            // **A window of khor's own is not somewhere else.** Floated,
            // the menu is a second window, and on a platform where that
            // window can take the pointer this field blurs between the
            // press on a row and the answer coming back — leaving here
            // would put the resting face up and throw away the
            // completion that was just chosen. `hasFocus` is false only
            // when the whole document lost focus, which a click inside
            // this page never does.
            //
            // The menu window is built unable to become key, so on macOS
            // this should not fire at all; it is here for the platforms
            // where that lever is weaker.
            //
            // The box therefore also stays open when the whole app is
            // switched away from, which is Chrome's behaviour and is the
            // one worth having: coming back finds the half-typed address
            // where it was left. The *menu* does not stay on screen with
            // it — the app takes it down when the face loses focus and
            // puts it back when the face returns (`on_window_event`).
            if (floated && !document.hasFocus()) return;
            setOpen(false);
            onLeave?.();
          }}
          onChange={(e) => onQuery(e.target.value)}
          onKeyDown={(e) => {
            // First, before every branch: mid-composition these keys are
            // the IME's (`ChatView`'s "/" menu, same reason).
            if (imeKey(e)) return;
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
              setWalked(true);
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
        {/* **A clear button inside the field** (F3.4; N20), once there is
            something to clear: the × beside the field leaves search,
            this empties it. */}
        {field && (query !== "" || chosen.length > 0) && (
          <button
            type="button"
            data-search-clear
            aria-label={gui.search_clear}
            onMouseDown={(e) => e.preventDefault()}
            onClick={() => {
              onQuery("");
              for (const k of chosen) onToggle(k);
              box.current?.focus();
            }}
            className="grid size-6 flex-none place-items-center rounded-full bg-fill-strong text-muted-foreground"
          >
            <IconClose className="size-affordance" />
          </button>
        )}
        {trail}
      </div>
      {/* **Here only when it is not floating.** The floated one is the
          same component in the app's own small window, put there by
          `useFloatedMenu` above — one menu, two places to draw it. */}
      {shown && !floated && (
        <OmniMenu
          // **Nothing is picked in advance under a finger** (F3.4; N20):
          // a highlighted first row is the keyboard's promise of what
          // Enter does, and a finger has no Enter.
          rows={{ groups: offered, at: coarse && !walked ? -1 : at }}
          onPick={press}
          className="absolute top-full z-20 mt-2"
          // From the list column's edge where the box is seated in the
          // left block's first row, from the box's own edge anywhere
          // else (tokens.css `--menu-inset`).
          style={{ left: "var(--menu-inset)", width: "calc(100% - var(--menu-inset))" }}
        />
      )}
    </div>
  );
}
